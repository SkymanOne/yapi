//! Codex's WebSocket transport: connections, pi's per-session socket cache
//! and response continuation.
//!
//! Port of the WebSocket half of `packages/ai/src/api/openai-codex-responses.ts`
//! in pi `v1.0.0`. A session keeps one socket per account between requests,
//! closed after five idle minutes or once it is 55 minutes old.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use base64::Engine as _;
use futures_util::{FutureExt as _, SinkExt as _, StreamExt as _};
use serde_json::{Map, Value, json};
use sha1::Digest as _;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::tungstenite::Message as Frame;
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
    /// Idle, with the generation that its idle timer expires.
    Idle(Box<Cached>, u64),
}

struct Cached {
    socket: Socket,
    created: Instant,
    continuation: Option<Continuation>,
}

static SOCKETS: LazyLock<Mutex<HashMap<Key, Slot>>> = LazyLock::new(Default::default);
static GENERATION: AtomicU64 = AtomicU64::new(0);
/// Sessions whose WebSocket failed; they use SSE for the rest of the process.
static FALLBACK: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);

/// Whether `session`'s WebSocket failed earlier in this process.
pub(super) fn fallback_active(session: Option<&str>) -> bool {
    session.is_some_and(|session| lock(&FALLBACK).contains(session))
}

/// Sends `session`'s later requests over SSE.
pub(super) fn record_failure(session: Option<&str>) {
    if let Some(session) = session {
        lock(&FALLBACK).insert(session.to_owned());
    }
}

/// Closes the idle sockets `session_id` keeps, as pi does when a session ends.
pub fn close_session(session_id: &str) {
    let mut sockets = lock(&SOCKETS);
    let keys: Vec<Key> = sockets
        .keys()
        .filter(|(session, _)| session == session_id)
        .cloned()
        .collect();
    for key in keys {
        if let Some(Slot::Idle(cached, _)) = sockets.remove(&key) {
            close(cached.socket, "debug_close");
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
    let one_off = |socket| Lease {
        socket,
        key: None,
        created: Instant::now(),
        continuation: None,
    };
    let Some(key) = key else {
        return Ok(one_off(open().await?));
    };
    let busy = {
        let mut sockets = lock(&SOCKETS);
        match sockets.remove(&key) {
            Some(Slot::Busy) => {
                sockets.insert(key.clone(), Slot::Busy);
                true
            }
            Some(Slot::Idle(mut cached, _)) => {
                let fresh = cached.created.elapsed() < MAX_AGE;
                if fresh && is_open(&mut cached.socket) {
                    sockets.insert(key.clone(), Slot::Busy);
                    return Ok(Lease {
                        socket: cached.socket,
                        key: Some(key),
                        created: cached.created,
                        continuation: cached.continuation,
                    });
                }
                close(
                    cached.socket,
                    if fresh {
                        "done"
                    } else {
                        "connection_age_limit"
                    },
                );
                false
            }
            None => false,
        }
    };
    if busy {
        return Ok(one_off(open().await?));
    }
    let socket = open().await?;
    lock(&SOCKETS).insert(key.clone(), Slot::Busy);
    Ok(Lease {
        socket,
        key: Some(key),
        created: Instant::now(),
        continuation: None,
    })
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
    /// closes or errs, when `cancel` fires, and after the HTTP idle timeout
    /// without a message.
    pub(super) async fn next(&mut self, cancel: &CancellationToken) -> Result<String, Failure> {
        loop {
            let read = self.socket.next();
            let frame = match http::idle_timeout() {
                Some(limit) => cancel
                    .run_until_cancelled(tokio::time::timeout(limit, read))
                    .await
                    .map(|frame| {
                        frame.map_err(|_| {
                            Failure::error(format!(
                                "WebSocket idle timeout after {}ms",
                                limit.as_millis()
                            ))
                        })
                    }),
                None => cancel.run_until_cancelled(read).await.map(Ok),
            };
            let frame =
                frame.unwrap_or_else(|| Err(Failure::error(http::ABORTED_DURING_STREAM)))?;
            match frame {
                Some(Ok(Frame::Text(text))) => return Ok(text.to_string()),
                Some(Ok(Frame::Binary(bytes))) => {
                    return Ok(String::from_utf8_lossy(&bytes).into_owned());
                }
                Some(Ok(Frame::Close(frame))) => return Err(Failure::closed(frame)),
                Some(Ok(_)) => {}
                Some(Err(_)) | None => return Err(Failure::error(WEBSOCKET_ERROR)),
            }
        }
    }

    /// Returns a cached socket to its session when `keep` and it is still
    /// open, to close after five idle minutes; closes it otherwise.
    pub(super) fn release(mut self, keep: bool) {
        let Some(key) = self.key.take() else {
            return close(self.socket, "done");
        };
        let mut sockets = lock(&SOCKETS);
        if !keep || !is_open(&mut self.socket) {
            if matches!(sockets.get(&key), Some(Slot::Busy)) {
                sockets.remove(&key);
            }
            return close(self.socket, "done");
        }
        let generation = GENERATION.fetch_add(1, Ordering::Relaxed);
        let cached = Cached {
            socket: self.socket,
            created: self.created,
            continuation: self.continuation,
        };
        sockets.insert(key.clone(), Slot::Idle(Box::new(cached), generation));
        tokio::spawn(async move {
            tokio::time::sleep(IDLE_TTL).await;
            let mut sockets = lock(&SOCKETS);
            if matches!(sockets.get(&key), Some(Slot::Idle(_, current)) if *current == generation)
                && let Some(Slot::Idle(cached, _)) = sockets.remove(&key)
            {
                close(cached.socket, "idle_timeout");
            }
        });
    }
}

/// Whether nothing has closed `socket` while no one read it: pi's
/// `readyState` check, made by reading what has already arrived.
fn is_open(socket: &mut Socket) -> bool {
    loop {
        match socket.next().now_or_never() {
            None => return true,
            Some(Some(Ok(Frame::Ping(_) | Frame::Pong(_)))) => {}
            Some(_) => return false,
        }
    }
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
    let key =
        base64::engine::general_purpose::STANDARD.encode(yapi_types::time::random_bytes::<16>());
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
            .is_some_and(|accept| accept.as_bytes() == accept_key(&key).as_bytes());
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

/// RFC 6455's `Sec-WebSocket-Accept` for `key`.
fn accept_key(key: &str) -> String {
    let digest = sha1::Sha1::new()
        .chain_update(key)
        .chain_update("258EAFA5-E914-47DA-95CA-C5AB0DC85B11")
        .finalize();
    base64::engine::general_purpose::STANDARD.encode(digest)
}
