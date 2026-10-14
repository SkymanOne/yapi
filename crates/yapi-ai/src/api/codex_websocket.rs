//! Codex's WebSocket transport: connections, pi's per-session socket cache
//! and response continuation.
//!
//! Port of the WebSocket half of `packages/ai/src/api/openai-codex-responses.ts`
//! in pi `v1.0.0`. A session keeps one socket per account between requests,
//! closed after five idle minutes or once it is 55 minutes old. While idle, a
//! task reads the socket, as Node does: it answers pings, drops stray
//! messages and notices a close.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use futures_util::{SinkExt as _, StreamExt as _};
use serde::Serialize;
use serde_json::{Map, Value, json};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tokio::time::Instant;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as Frame;
use tokio_tungstenite::tungstenite::handshake::client::generate_key;
use tokio_tungstenite::tungstenite::handshake::derive_accept_key;
use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
use tokio_tungstenite::tungstenite::protocol::{CloseFrame, Role};
use tokio_util::sync::CancellationToken;
use yapi_types::sync::lock;

use crate::http::{self, Headers};
use crate::stream::{Transport, now_ms};

type Socket = WebSocketStream<reqwest::Upgraded>;

/// A session id and an account id.
type Key = (String, String);

const IDLE_TTL: Duration = Duration::from_secs(5 * 60);
const MAX_AGE: Duration = Duration::from_secs(55 * 60);
const DEFAULT_CONNECT_TIMEOUT_MS: u64 = 15_000;
/// What pi reports for any failure the runtime's WebSocket gives no detail on.
const WEBSOCKET_ERROR: &str = "WebSocket error";

enum Slot {
    /// A request is using the socket.
    Busy,
    Idle(Idle),
}

/// A cached socket between requests, held by its reader task.
struct Idle {
    /// Stops the reader: `None` hands the socket back, a reason closes it.
    stop: oneshot::Sender<Option<&'static str>>,
    /// The socket, unless it closed or was closed.
    reader: JoinHandle<Option<Socket>>,
    created: Instant,
    continuation: Option<Continuation>,
    generation: u64,
}

static SOCKETS: LazyLock<Mutex<HashMap<Key, Slot>>> = LazyLock::new(Default::default);
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Sessions whose WebSocket failed; they use SSE for the rest of the process.
static FALLBACK: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);
static STATS: LazyLock<Mutex<HashMap<String, DebugStats>>> = LazyLock::new(Default::default);

/// pi's `OpenAICodexWebSocketDebugStats` for one session.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DebugStats {
    requests: u64,
    connections_created: u64,
    connections_reused: u64,
    cached_context_requests: u64,
    store_true_requests: u64,
    full_context_requests: u64,
    delta_requests: u64,
    last_input_items: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_delta_input_items: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_previous_response_id: Option<String>,
    websocket_failures: u64,
    sse_fallbacks: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    websocket_fallback_active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_web_socket_error: Option<String>,
}

fn with_stats(session: &str, update: impl FnOnce(&mut DebugStats)) {
    update(lock(&STATS).entry(session.to_owned()).or_default());
}

/// pi's `getOpenAICodexWebSocketDebugStats`: what `session_id`'s WebSocket
/// requests did in this process.
pub fn debug_stats(session_id: &str) -> Option<DebugStats> {
    lock(&STATS).get(session_id).cloned()
}

/// pi's `resetOpenAICodexWebSocketDebugStats`: forgets the statistics and
/// the SSE fallback of `session_id`, or of every session.
pub fn reset_debug_stats(session_id: Option<&str>) {
    match session_id {
        Some(session) => {
            lock(&STATS).remove(session);
            lock(&FALLBACK).remove(session);
        }
        None => {
            lock(&STATS).clear();
            lock(&FALLBACK).clear();
        }
    }
}

/// Whether `session`'s WebSocket failed earlier in this process.
pub(super) fn fallback_active(session: Option<&str>) -> bool {
    session.is_some_and(|session| lock(&FALLBACK).contains(session))
}

/// Counts a request of `session` as pi's `processWebSocketStream` does.
pub(super) fn count_request(session: &str, reused: bool, continues: bool, request: &Value) {
    with_stats(session, |stats| {
        stats.requests += 1;
        if reused {
            stats.connections_reused += 1;
        } else {
            stats.connections_created += 1;
        }
        if continues {
            stats.cached_context_requests += 1;
        }
        if request["store"] == true {
            stats.store_true_requests += 1;
        }
        let items = request["input"].as_array().map_or(0, Vec::len);
        stats.last_input_items = items;
        match request["previous_response_id"].as_str() {
            Some(previous) => {
                stats.delta_requests += 1;
                stats.last_delta_input_items = Some(items);
                stats.last_previous_response_id = Some(previous.to_owned());
            }
            None => {
                stats.full_context_requests += 1;
                stats.last_delta_input_items = None;
                stats.last_previous_response_id = None;
            }
        }
    });
}

/// pi's `recordWebSocketSseFallback`: a request of `session` goes over SSE.
pub(super) fn record_fallback(session: Option<&str>) {
    if let Some(session) = session {
        let active = fallback_active(Some(session));
        with_stats(session, |stats| {
            stats.sse_fallbacks += 1;
            stats.websocket_fallback_active = Some(active);
        });
    }
}

/// pi's `recordWebSocketFailure`: `session`'s later requests use SSE.
pub(super) fn record_failure(session: Option<&str>, message: &str) {
    if let Some(session) = session {
        lock(&FALLBACK).insert(session.to_owned());
        with_stats(session, |stats| {
            stats.websocket_failures += 1;
            stats.last_web_socket_error = Some(message.to_owned());
            stats.websocket_fallback_active = Some(true);
        });
    }
}

/// pi's `closeOpenAICodexWebSocketSessions`: closes the sockets
/// `session_id` keeps, or every session's. A socket in use closes when its
/// request ends.
pub fn close_sessions(session_id: Option<&str>) {
    let mut sockets = lock(&SOCKETS);
    let keys: Vec<Key> = sockets
        .keys()
        .filter(|(session, _)| session_id.is_none_or(|id| session == id))
        .cloned()
        .collect();
    for key in keys {
        if let Some(Slot::Idle(idle)) = sockets.remove(&key) {
            let _ = idle.stop.send(Some("debug_close"));
        }
    }
}

/// Why a WebSocket request failed.
#[derive(Debug)]
pub(super) struct Failure {
    /// The connection failed, as opposed to the backend reporting an error
    /// or sending a message that is not JSON. Only these fall back to SSE.
    pub(super) transport: bool,
    /// pi's error class, for the diagnostic.
    name: &'static str,
    pub(super) message: String,
    /// The backend's error code, or a close code.
    code: Option<Value>,
}

impl Failure {
    /// A failure pi raises as a plain `Error`.
    pub(super) fn error(message: impl Into<String>) -> Failure {
        Failure {
            transport: true,
            name: "Error",
            message: message.into(),
            code: None,
        }
    }

    /// An `error` or `response.failed` event, with its code.
    pub(super) fn api(message: String, code: Option<&str>) -> Failure {
        Failure {
            transport: false,
            name: "CodexApiError",
            message,
            code: code.map(Value::from),
        }
    }

    /// A message that is not JSON.
    pub(super) fn protocol(message: String) -> Failure {
        Failure {
            transport: false,
            name: "CodexProtocolError",
            message,
            code: None,
        }
    }

    fn closed(frame: Option<CloseFrame>) -> Failure {
        let (code, reason) = frame.map_or((1005, String::new()), |frame| {
            (u16::from(frame.code), frame.reason.to_string())
        });
        let reason = if reason.is_empty() && code == 1009 {
            "message too big".to_owned()
        } else {
            reason
        };
        Failure {
            transport: true,
            name: "WebSocketCloseError",
            message: format!("WebSocket closed {code} {reason}")
                .trim_end()
                .to_owned(),
            code: Some(Value::from(code)),
        }
    }

    /// The backend's error code.
    pub(super) fn api_code(&self) -> Option<&str> {
        self.code
            .as_ref()
            .and_then(Value::as_str)
            .filter(|_| !self.transport)
    }

    /// pi's `provider_transport_failure` diagnostic: the failure with the
    /// `transport` setting, whether events had arrived, which decides the SSE
    /// fallback, and the request's size in bytes.
    pub(super) fn diagnostic(
        &self,
        transport: Transport,
        events: bool,
        request_bytes: usize,
    ) -> Value {
        let mut error = Map::new();
        error.insert("name".into(), json!(self.name));
        error.insert("message".into(), json!(self.message));
        if let Some(code) = &self.code {
            error.insert("code".into(), code.clone());
        }
        let mut details = Map::new();
        details.insert("configuredTransport".into(), json!(transport));
        if !events {
            details.insert("fallbackTransport".into(), json!("sse"));
        }
        details.insert("eventsEmitted".into(), json!(events));
        let phase = if events {
            "after_message_stream_start"
        } else {
            "before_message_stream_start"
        };
        details.insert("phase".into(), json!(phase));
        details.insert("requestBytes".into(), json!(request_bytes));
        json!({
            "type": "provider_transport_failure",
            "timestamp": now_ms(),
            "error": error,
            "details": details,
        })
    }
}

/// The exchange a cached socket last carried.
pub(super) struct Continuation {
    /// The full request, as sent before any delta was taken.
    pub(super) body: Value,
    pub(super) response_id: String,
    /// The response's output as input items.
    pub(super) items: Vec<Value>,
}

/// The request to send on a socket that last carried `continuation`: when
/// `body` repeats that request with more input after the response's items,
/// only the new input, referencing the response. Otherwise `body`, and the
/// continuation is dropped.
pub(super) fn continued(body: &Value, continuation: &mut Option<Continuation>) -> Value {
    let Some(last) = continuation else {
        return body.clone();
    };
    match delta(body, last) {
        Some(input) if !last.response_id.is_empty() => {
            let mut next = body.clone();
            next["previous_response_id"] = json!(last.response_id);
            next["input"] = Value::Array(input);
            next
        }
        _ => {
            *continuation = None;
            body.clone()
        }
    }
}

/// pi's `getCachedWebSocketInputDelta`. Requests compare as serialized JSON.
fn delta(body: &Value, last: &Continuation) -> Option<Vec<Value>> {
    let rest = |body: &Value| {
        let mut rest = body.as_object()?.clone();
        rest.shift_remove("input");
        rest.shift_remove("previous_response_id");
        Some(yapi_types::json::stringify(&rest))
    };
    if rest(body)? != rest(&last.body)? {
        return None;
    }
    let input = |body: &Value| body["input"].as_array().cloned().unwrap_or_default();
    let current = input(body);
    let mut baseline = input(&last.body);
    baseline.extend(last.items.iter().cloned());
    let same =
        |(a, b): (&Value, &Value)| yapi_types::json::stringify(a) == yapi_types::json::stringify(b);
    if current.len() < baseline.len() || !current.iter().zip(&baseline).all(same) {
        return None;
    }
    Some(current[baseline.len()..].to_vec())
}

/// The message for a request: `response.create` with the body's fields.
pub(super) fn frame(body: &Value) -> String {
    let mut frame = Map::new();
    frame.insert("type".into(), json!("response.create"));
    if let Some(fields) = body.as_object() {
        frame.extend(fields.clone());
    }
    yapi_types::json::stringify(&frame)
}

/// A socket for one request.
pub(super) struct Lease {
    socket: Socket,
    /// The cache slot the socket returns to; `None` for a one-off socket.
    key: Option<Key>,
    created: Instant,
    /// Whether the socket served an earlier request.
    pub(super) reused: bool,
    /// The exchange the socket last carried.
    pub(super) continuation: Option<Continuation>,
}

/// A socket to `url` for `key`'s session and account: its idle cached one,
/// else a new one that is cached once released. While the cached socket is
/// busy, and without a key, the socket is a one-off.
pub(super) async fn acquire(
    url: &str,
    headers: &Headers,
    key: Option<Key>,
    cancel: &CancellationToken,
    connect_timeout_ms: Option<u64>,
) -> Result<Lease, Failure> {
    let open = || connect(url, headers, cancel, connect_timeout_ms);
    let lease = |socket, key, created| Lease {
        socket,
        key,
        created,
        reused: false,
        continuation: None,
    };
    let Some(key) = key else {
        return Ok(lease(open().await?, None, Instant::now()));
    };
    let found = {
        let mut sockets = lock(&SOCKETS);
        let found = sockets.remove(&key);
        if found.is_some() {
            sockets.insert(key.clone(), Slot::Busy);
        }
        found
    };
    match found {
        Some(Slot::Busy) => return Ok(lease(open().await?, None, Instant::now())),
        Some(Slot::Idle(idle)) => {
            let _ = idle.stop.send(None);
            if let Ok(Some(socket)) = idle.reader.await {
                if idle.created.elapsed() < MAX_AGE {
                    return Ok(Lease {
                        reused: true,
                        continuation: idle.continuation,
                        ..lease(socket, Some(key), idle.created)
                    });
                }
                close(socket, "connection_age_limit");
            }
        }
        None => {}
    }
    let socket = match open().await {
        Ok(socket) => socket,
        Err(failure) => {
            remove_busy(&key);
            return Err(failure);
        }
    };
    lock(&SOCKETS).insert(key.clone(), Slot::Busy);
    Ok(lease(socket, Some(key), Instant::now()))
}

/// Forgets `key`'s slot when a request holds it.
fn remove_busy(key: &Key) {
    let mut sockets = lock(&SOCKETS);
    if matches!(sockets.get(key), Some(Slot::Busy)) {
        sockets.remove(key);
    }
}

impl Lease {
    /// Whether the socket returns to the session's cache.
    pub(super) fn cached(&self) -> bool {
        self.key.is_some()
    }

    /// Sends `text` as one message.
    pub(super) async fn send(&mut self, text: String) -> Result<(), Failure> {
        self.socket
            .send(Frame::text(text))
            .await
            .map_err(|_| Failure::error(WEBSOCKET_ERROR))
    }

    /// The next text or binary message, as text. Fails when the socket
    /// closes or errs, when `cancel` fires, and once `idle` passes without a
    /// message; pings and pongs do not count.
    pub(super) async fn next(
        &mut self,
        cancel: &CancellationToken,
        idle: Option<Duration>,
    ) -> Result<String, Failure> {
        let deadline = idle.map(|idle| (idle, tokio::time::Instant::now() + idle));
        loop {
            let read = self.socket.next();
            let frame = match deadline {
                Some((idle, at)) => cancel
                    .run_until_cancelled(tokio::time::timeout_at(at, read))
                    .await
                    .map(|frame| {
                        frame.map_err(|_| {
                            Failure::error(format!(
                                "WebSocket idle timeout after {}ms",
                                idle.as_millis()
                            ))
                        })
                    }),
                None => cancel.run_until_cancelled(read).await.map(Ok),
            };
            let frame =
                frame.unwrap_or_else(|| Err(Failure::error(http::ABORTED_DURING_STREAM)))?;
            // pi ignores empty messages, and they do not restart its idle timer.
            match frame {
                Some(Ok(Frame::Text(text))) if !text.is_empty() => return Ok(text.to_string()),
                Some(Ok(Frame::Binary(bytes))) if !bytes.is_empty() => {
                    return Ok(String::from_utf8_lossy(&bytes).into_owned());
                }
                Some(Ok(Frame::Close(frame))) => return Err(Failure::closed(frame)),
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return Err(Failure::error(WEBSOCKET_ERROR)),
            }
        }
    }

    /// Returns a cached socket to its session when `keep` and its slot is
    /// still the request's, to close after five idle minutes; closes it
    /// otherwise.
    pub(super) fn release(self, keep: bool) {
        let Some(key) = self.key else {
            return close(self.socket, "done");
        };
        let mut sockets = lock(&SOCKETS);
        if !matches!(sockets.get(&key), Some(Slot::Busy)) {
            return close(self.socket, "done");
        }
        if !keep {
            sockets.remove(&key);
            return close(self.socket, "done");
        }
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
        let (stop, stopped) = oneshot::channel();
        let reader = tokio::spawn(read_idle(self.socket, stopped, key.clone(), generation));
        sockets.insert(
            key,
            Slot::Idle(Idle {
                stop,
                reader,
                created: self.created,
                continuation: self.continuation,
                generation,
            }),
        );
    }
}

/// Reads an idle socket until a request takes it back, which gets the
/// socket, or until it closes, errs or idles for five minutes, which ends it
/// and its slot.
async fn read_idle(
    mut socket: Socket,
    mut stopped: oneshot::Receiver<Option<&'static str>>,
    key: Key,
    generation: u64,
) -> Option<Socket> {
    let expiry = tokio::time::sleep(IDLE_TTL);
    tokio::pin!(expiry);
    let reason = loop {
        tokio::select! {
            stop = &mut stopped => match stop {
                Ok(None) => return Some(socket),
                Ok(Some(reason)) => break Some(reason),
                Err(_) => break Some("done"),
            },
            () = &mut expiry => break Some("idle_timeout"),
            frame = socket.next() => match frame {
                Some(Ok(Frame::Close(_)) | Err(_)) | None => break None,
                // pi drops what arrives while no request listens.
                Some(Ok(_)) => {}
            },
        }
    };
    {
        let mut sockets = lock(&SOCKETS);
        if matches!(sockets.get(&key), Some(Slot::Idle(idle)) if idle.generation == generation) {
            sockets.remove(&key);
        }
    }
    if let Some(reason) = reason {
        close(socket, reason);
    }
    None
}

/// Closes `socket` with a normal close and `reason`, without waiting.
fn close(mut socket: Socket, reason: &'static str) {
    tokio::spawn(async move {
        let frame = CloseFrame {
            code: CloseCode::Normal,
            reason: reason.into(),
        };
        let _ = socket.close(Some(frame)).await;
    });
}

/// Opens a WebSocket to `url`, an `http` or `https` URL, through an HTTP/1.1
/// upgrade.
async fn connect(
    url: &str,
    headers: &Headers,
    cancel: &CancellationToken,
    timeout_ms: Option<u64>,
) -> Result<Socket, Failure> {
    let key = generate_key();
    let request = headers
        .apply(http::upgrade_client().get(url))
        .header("connection", "Upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .header("sec-websocket-key", &key);
    let open = async {
        let failed = |_| Failure::error(WEBSOCKET_ERROR);
        let response = request.send().await.map_err(failed)?;
        let accepted = response
            .headers()
            .get("sec-websocket-accept")
            .is_some_and(|accept| {
                accept.as_bytes() == derive_accept_key(key.as_bytes()).as_bytes()
            });
        if response.status() != reqwest::StatusCode::SWITCHING_PROTOCOLS || !accepted {
            return Err(Failure::error(WEBSOCKET_ERROR));
        }
        let upgraded = response.upgrade().await.map_err(failed)?;
        Ok(WebSocketStream::from_raw_socket(upgraded, Role::Client, None).await)
    };
    let timeout_ms = timeout_ms.unwrap_or(DEFAULT_CONNECT_TIMEOUT_MS);
    let open = async {
        if timeout_ms == 0 {
            return open.await;
        }
        tokio::time::timeout(Duration::from_millis(timeout_ms), open)
            .await
            .unwrap_or_else(|_| {
                Err(Failure::error(format!(
                    "WebSocket connect timeout after {timeout_ms}ms"
                )))
            })
    };
    cancel
        .run_until_cancelled(open)
        .await
        .unwrap_or_else(|| Err(Failure::error(http::ABORTED_DURING_STREAM)))
}
