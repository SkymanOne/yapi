//! The Codex backend's transports, against a mock server.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};
use yapi_ai::api::Apis;
use yapi_ai::catalog::builtin_models;
use yapi_ai::stream::{Request, StreamOptions, Transport};
use yapi_mock::{Cassette, Interaction, MockServer, RecordedRequest};
use yapi_types::message::{AssistantMessage, Content, Message, StopReason, UserMessage};

/// An access token whose claims carry the account id `acct_mock`.
const TOKEN: &str = "eyJhbGciOiJub25lIn0.eyJodHRwczovL2FwaS5vcGVuYWkuY29tL2F1dGgiOnsiY2hhdGdwdF9hY2NvdW50X2lkIjoiYWNjdF9tb2NrIn19.sig";

fn cassette(name: &str) -> Cassette {
    Cassette::load(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/cassettes/openai-codex-responses")
            .join(name),
    )
    .unwrap()
}

/// The text response over SSE.
fn sse() -> Interaction {
    cassette("text.json").interactions.remove(0)
}

/// A WebSocket exchange answering with `messages`.
fn exchange(messages: Vec<String>) -> Interaction {
    let mut interaction = Interaction::json("GET", "/codex/responses", 101, &json!(null));
    interaction.response.headers.clear();
    interaction.response.chunks = messages;
    interaction
}

/// The text response over a WebSocket.
fn text() -> Interaction {
    let messages = sse()
        .response
        .chunks
        .iter()
        .flat_map(|chunk| chunk.lines())
        .filter_map(|line| line.strip_prefix("data: "))
        .map(str::to_owned)
        .collect();
    exchange(messages)
}

fn user(text: &str) -> Message {
    Message::User(UserMessage {
        content: Content::Text(text.into()),
        timestamp: 1,
    })
}

/// Options for a session's request over `transport`.
fn session(id: &str, transport: Option<Transport>) -> StreamOptions {
    StreamOptions {
        session_id: Some(id.into()),
        transport,
        ..StreamOptions::default()
    }
}

async fn ask_with(
    server: &MockServer,
    messages: Vec<Message>,
    options: StreamOptions,
) -> AssistantMessage {
    ask_at(&server.url(), messages, options).await
}

async fn ask_at(url: &str, messages: Vec<Message>, options: StreamOptions) -> AssistantMessage {
    let mut model = builtin_models("openai-codex")
        .into_iter()
        .find(|model| model.id == "gpt-5.5")
        .unwrap();
    model.base_url = url.to_owned();
    // Grammar-constrained tools travel as custom tools.
    model
        .compat
        .get_or_insert_with(Default::default)
        .insert("supportsOpenAIGrammarTools".into(), json!(true));
    let request = Request {
        model,
        messages,
        options: StreamOptions {
            api_key: Some(TOKEN.into()),
            ..options
        },
    };
    Apis::default().stream(request).result().await.unwrap()
}

async fn ask(server: &MockServer, options: StreamOptions) -> AssistantMessage {
    ask_with(server, vec![user("Say hello")], options).await
}

fn body(request: &RecordedRequest) -> Value {
    serde_json::from_str(&request.body).unwrap()
}

#[tokio::test]
async fn sse_bodies_are_zstd_compressed() {
    let server = MockServer::local(cassette("text.json")).await.unwrap();
    let options = StreamOptions {
        transport: Some(Transport::Sse),
        ..StreamOptions::default()
    };
    let message = ask(&server, options).await;
    assert_eq!(message.error_message, None);
    let requests = server.finish().unwrap();
    assert_eq!(requests[0].headers["content-encoding"], "zstd");
    // The mock decodes the body; raw zstd would not parse.
    assert_eq!(body(&requests[0])["model"], "gpt-5.5");
}

#[tokio::test]
async fn a_failed_websocket_sends_the_session_to_sse() {
    let not_found = Interaction::json("GET", "/codex/responses", 404, &json!({}));
    let server = MockServer::local(Cassette {
        interactions: vec![not_found, sse(), sse()],
    })
    .await
    .unwrap();
    let first = ask(&server, session("ws-fallback", Some(Transport::Auto))).await;
    assert_eq!(first.stop_reason, StopReason::Stop);
    let diagnostic = &first.diagnostics.unwrap()[0];
    assert_eq!(diagnostic["error"]["message"], "WebSocket error");
    assert_eq!(diagnostic["details"]["fallbackTransport"], "sse");
    let second = ask(&server, session("ws-fallback", Some(Transport::Auto))).await;
    assert_eq!(second.diagnostics, None);
    let methods: Vec<String> = server
        .finish()
        .unwrap()
        .into_iter()
        .map(|request| request.method)
        .collect();
    assert_eq!(methods, ["GET", "POST", "POST"]);
}

#[tokio::test]
async fn a_full_connection_limit_retries_once() {
    let limit = json!({"type": "error", "code": "websocket_connection_limit_reached", "message": "Too many connections"});
    let server = MockServer::local(Cassette {
        interactions: vec![exchange(vec![limit.to_string()]), text()],
    })
    .await
    .unwrap();
    let message = ask(&server, session("ws-limit", Some(Transport::Auto))).await;
    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.diagnostics, None);
    assert_eq!(server.finish().unwrap().len(), 2);
}

#[tokio::test]
async fn continuations_send_the_new_input_and_recover_a_lost_response() {
    let missing = json!({"type": "error", "status": 400, "error": {"code": "previous_response_not_found", "message": "Previous response not found."}});
    let server = MockServer::local(Cassette {
        interactions: vec![text(), exchange(vec![missing.to_string()]), text()],
    })
    .await
    .unwrap();
    let options = || session("ws-continue", Some(Transport::Auto));
    let first = ask(&server, options()).await;
    let messages = vec![
        user("Say hello"),
        Message::Assistant(Box::new(first)),
        user("Again"),
    ];
    let second = ask_with(&server, messages, options()).await;
    assert_eq!(second.stop_reason, StopReason::Stop);
    let requests = server.finish().unwrap();
    let delta = body(&requests[1]);
    assert_eq!(delta["type"], "response.create");
    assert_eq!(delta["previous_response_id"], "resp_mock_text");
    assert_eq!(
        delta["input"],
        json!([{"role": "user", "content": [{"type": "input_text", "text": "Again"}]}])
    );
    let retried = body(&requests[2]);
    assert_eq!(retried.get("previous_response_id"), None);
    let input = retried["input"].as_array().unwrap();
    assert_eq!(input[0]["content"][0]["text"], "Say hello");
    assert_eq!(input.last(), delta["input"].get(0));
}

#[tokio::test]
async fn continuations_need_an_explicit_transport() {
    let server = MockServer::local(Cassette {
        interactions: vec![text(), text()],
    })
    .await
    .unwrap();
    let first = ask(&server, session("ws-default", None)).await;
    let messages = vec![
        user("Say hello"),
        Message::Assistant(Box::new(first)),
        user("Again"),
    ];
    ask_with(&server, messages, session("ws-default", None)).await;
    let requests = server.finish().unwrap();
    assert_eq!(body(&requests[1]).get("previous_response_id"), None);
}

#[tokio::test]
async fn a_failure_after_events_ends_the_request() {
    let created = sse().response.chunks[0]
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap()
        .to_owned();
    // The socket closes after the exchange, before a terminal event.
    let server = MockServer::local(Cassette {
        interactions: vec![exchange(vec![created]), sse()],
    })
    .await
    .unwrap();
    let first = ask(&server, session("ws-late", Some(Transport::Auto))).await;
    assert_eq!(first.stop_reason, StopReason::Error);
    assert_eq!(
        first.error_message.as_deref(),
        Some("WebSocket closed 1005")
    );
    let details = &first.diagnostics.unwrap()[0]["details"];
    assert_eq!(details["phase"], "after_message_stream_start");
    assert_eq!(details.get("fallbackTransport"), None);
    let second = ask(&server, session("ws-late", Some(Transport::Auto))).await;
    assert_eq!(second.stop_reason, StopReason::Stop);
    assert_eq!(server.finish().unwrap()[1].method, "POST");
}

#[tokio::test]
async fn closing_a_session_closes_its_socket() {
    let server = MockServer::local(Cassette {
        interactions: vec![text(), text()],
    })
    .await
    .unwrap();
    let options = || session("ws-closed", Some(Transport::Auto));
    let first = ask(&server, options()).await;
    yapi_ai::api::close_codex_websockets(Some("ws-closed"));
    let messages = vec![
        user("Say hello"),
        Message::Assistant(Box::new(first)),
        user("Again"),
    ];
    ask_with(&server, messages, options()).await;
    // A new socket has no response to continue.
    let requests = server.finish().unwrap();
    assert_eq!(body(&requests[1]).get("previous_response_id"), None);
}

#[tokio::test]
async fn continuations_follow_a_grammar_tool_call() {
    let event = |value: Value| value.to_string();
    let call = json!({"id": "ctc_1", "type": "custom_tool_call", "call_id": "call_1", "name": "codemode", "input": "return 1;"});
    let server = MockServer::local(Cassette {
        interactions: vec![
            exchange(vec![
                event(json!({"type": "response.created", "response": {"id": "resp_cm"}})),
                event(json!({"type": "response.output_item.added", "output_index": 0, "item": call})),
                event(json!({"type": "response.output_item.done", "output_index": 0, "item": call})),
                event(json!({"type": "response.completed", "response": {"id": "resp_cm", "status": "completed"}})),
            ]),
            text(),
        ],
    })
    .await
    .unwrap();
    let message = |value: Value| serde_json::from_value::<Message>(value).unwrap();
    let system = message(json!({
        "role": "system", "content": "Use tools.", "timestamp": 0,
        "toolsAdded": [{
            "name": "codemode", "description": "Run code",
            "parameters": {"type": "object", "properties": {"code": {"type": "string"}}, "required": ["code"]},
            "constrainedSampling": {"type": "grammar", "variants": {"openai_lark": "start: /.+/"}},
        }],
    }));
    let options = || session("ws-grammar", Some(Transport::Auto));
    let first = ask_with(&server, vec![system.clone(), user("Run it")], options()).await;
    assert_eq!(first.stop_reason, StopReason::ToolUse);
    let result = message(json!({
        "role": "toolResult", "toolCallId": "call_1|ctc_1", "toolName": "codemode",
        "content": [{"type": "text", "text": "1"}], "isError": false, "timestamp": 2,
    }));
    let messages = vec![
        system,
        user("Run it"),
        Message::Assistant(Box::new(first)),
        result,
    ];
    ask_with(&server, messages, options()).await;
    let delta = body(&server.finish().unwrap()[1]);
    assert_eq!(delta["previous_response_id"], "resp_cm");
    assert_eq!(
        delta["input"],
        json!([{"type": "custom_tool_call_output", "call_id": "call_1", "output": "1"}])
    );
}

fn stats(session: &str) -> Value {
    serde_json::to_value(yapi_ai::api::codex_websocket_stats(session).unwrap()).unwrap()
}

/// The first event of the text response, for exchanges that stop after it.
fn created() -> String {
    sse().response.chunks[0]
        .lines()
        .find_map(|line| line.strip_prefix("data: "))
        .unwrap()
        .to_owned()
}

/// A server that never answers WebSocket upgrades and answers POSTs with the
/// text response over SSE.
async fn hanging_upgrades() -> String {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let body: String = sse().response.chunks.concat();
    tokio::spawn(async move {
        loop {
            let (mut socket, _) = listener.accept().await.unwrap();
            let body = body.clone();
            tokio::spawn(async move {
                let mut head = Vec::new();
                while !head.ends_with(b"\r\n\r\n") {
                    let Ok(byte) = socket.read_u8().await else {
                        return;
                    };
                    head.push(byte);
                }
                if head.starts_with(b"GET") {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    return;
                }
                let head = String::from_utf8_lossy(&head).to_lowercase();
                let length = head
                    .lines()
                    .find_map(|line| line.strip_prefix("content-length:"))
                    .and_then(|value| value.trim().parse().ok())
                    .unwrap_or(0);
                let mut request = vec![0; length];
                let _ = socket.read_exact(&mut request).await;
                let response = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: text/event-stream\r\ncontent-length: {}\r\n\r\n{body}",
                    body.len()
                );
                let _ = socket.write_all(response.as_bytes()).await;
            });
        }
    });
    url
}

#[tokio::test]
async fn upgrades_carry_pis_headers() {
    let server = MockServer::local(Cassette {
        interactions: vec![text(), text()],
    })
    .await
    .unwrap();
    ask(&server, session("ws-headers", Some(Transport::Auto))).await;
    // Without a session, the request id is a fresh UUIDv7.
    let one_off = StreamOptions {
        transport: Some(Transport::Auto),
        ..StreamOptions::default()
    };
    ask(&server, one_off).await;
    let requests = server.finish().unwrap();
    let headers = &requests[0].headers;
    assert_eq!(headers["openai-beta"], "responses_websockets=2026-02-06");
    assert_eq!(headers["session-id"], "ws-headers");
    assert_eq!(headers["x-client-request-id"], "ws-headers");
    // reqwest's default, which Node's WebSocket sends too.
    assert_eq!(headers["accept"], "*/*");
    assert!(!headers.contains_key("content-type"));
    let id = &requests[1].headers["x-client-request-id"];
    assert_eq!((id.len(), &id[14..15]), (36, "7"));
    assert_eq!(&requests[1].headers["session-id"], id);
}

#[tokio::test]
async fn the_websocket_transport_sends_full_requests() {
    let server = MockServer::local(Cassette {
        interactions: vec![text(), text()],
    })
    .await
    .unwrap();
    let options = || session("ws-explicit", Some(Transport::Websocket));
    let first = ask(&server, options()).await;
    let messages = vec![
        user("Say hello"),
        Message::Assistant(Box::new(first)),
        user("Again"),
    ];
    ask_with(&server, messages, options()).await;
    let requests = server.finish().unwrap();
    assert_eq!(body(&requests[1]).get("previous_response_id"), None);
    assert_eq!(stats("ws-explicit")["connectionsReused"], 1);
    assert_eq!(stats("ws-explicit")["cachedContextRequests"], 0);
}

#[tokio::test]
async fn a_second_connection_limit_falls_back() {
    let limit = json!({"type": "error", "code": "websocket_connection_limit_reached", "message": "Too many connections"});
    let server = MockServer::local(Cassette {
        interactions: vec![
            exchange(vec![limit.to_string()]),
            exchange(vec![limit.to_string()]),
            sse(),
        ],
    })
    .await
    .unwrap();
    let message = ask(&server, session("ws-limit-twice", Some(Transport::Auto))).await;
    assert_eq!(message.stop_reason, StopReason::Stop);
    let diagnostic = &message.diagnostics.unwrap()[0];
    assert_eq!(
        diagnostic["error"],
        json!({"name": "CodexApiError", "message": "Codex error: Too many connections", "code": "websocket_connection_limit_reached"})
    );
    assert_eq!(diagnostic["details"]["fallbackTransport"], "sse");
    assert_eq!(stats("ws-limit-twice")["sseFallbacks"], 1);
}

#[tokio::test]
async fn a_busy_socket_gets_a_one_off_connection() {
    let mut slow = text();
    slow.response.chunk_delay_ms = 20;
    let server = MockServer::local(Cassette {
        interactions: vec![slow, text(), text()],
    })
    .await
    .unwrap();
    let options = || session("ws-busy", Some(Transport::Auto));
    let first = ask(&server, options());
    let second = async {
        tokio::time::sleep(Duration::from_millis(30)).await;
        ask(&server, options()).await
    };
    let (first, second) = tokio::join!(first, second);
    assert_eq!(
        (first.stop_reason, second.stop_reason),
        (StopReason::Stop, StopReason::Stop)
    );
    assert_eq!(stats("ws-busy")["connectionsCreated"], 2);
    // The one-off socket closed; the cached one serves the next request.
    ask(&server, options()).await;
    assert_eq!(stats("ws-busy")["connectionsReused"], 1);
    server.finish().unwrap();
}

#[tokio::test]
async fn connecting_times_out_or_aborts() {
    let url = hanging_upgrades().await;
    let timed = StreamOptions {
        websocket_connect_timeout_ms: Some(50),
        ..session("ws-connect-timeout", Some(Transport::Auto))
    };
    let message = ask_at(&url, vec![user("Say hello")], timed).await;
    assert_eq!(message.stop_reason, StopReason::Stop);
    let diagnostic = &message.diagnostics.unwrap()[0];
    assert_eq!(
        diagnostic["error"]["message"],
        "WebSocket connect timeout after 50ms"
    );
    let aborted = session("ws-connect-abort", Some(Transport::Auto));
    let cancel = aborted.cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel.cancel();
    });
    let message = ask_at(&url, vec![user("Say hello")], aborted).await;
    assert_eq!(message.stop_reason, StopReason::Aborted);
    assert_eq!(
        message.error_message.as_deref(),
        Some("Request was aborted")
    );
    assert_eq!(message.diagnostics, None);
}

#[tokio::test]
async fn streams_abort_close_and_idle_as_pi_does() {
    let mut slow = exchange(vec![created(), created()]);
    slow.response.chunk_delay_ms = 200;
    let mut too_big = exchange(vec![created()]);
    too_big.response.close = Some(yapi_mock::Close {
        code: 1009,
        reason: String::new(),
    });
    let mut quiet = exchange(vec![created()]);
    quiet.response.chunk_delay_ms = 300;
    let server = MockServer::local(Cassette {
        interactions: vec![slow, too_big, quiet, sse()],
    })
    .await
    .unwrap();

    let aborted = session("ws-abort", Some(Transport::Auto));
    let cancel = aborted.cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(300)).await;
        cancel.cancel();
    });
    let message = ask(&server, aborted).await;
    assert_eq!(message.stop_reason, StopReason::Aborted);
    assert_eq!(
        message.error_message.as_deref(),
        Some("Request was aborted")
    );
    assert_eq!(message.diagnostics, None);

    let message = ask(&server, session("ws-too-big", Some(Transport::Auto))).await;
    assert_eq!(
        message.error_message.as_deref(),
        Some("WebSocket closed 1009 message too big")
    );
    assert_eq!(
        message.diagnostics.unwrap()[0]["error"]["code"],
        json!(1009)
    );

    let idle = StreamOptions {
        timeout_ms: Some(50.0),
        ..session("ws-idle", Some(Transport::Auto))
    };
    let message = ask(&server, idle).await;
    assert_eq!(message.stop_reason, StopReason::Stop);
    let diagnostic = &message.diagnostics.unwrap()[0];
    assert_eq!(
        diagnostic["error"]["message"],
        "WebSocket idle timeout after 50ms"
    );
    assert_eq!(diagnostic["details"]["fallbackTransport"], "sse");
}

#[tokio::test(start_paused = true)]
async fn sockets_close_when_idle_and_age_out() {
    let server = MockServer::local(Cassette {
        interactions: (0..16).map(|_| text()).collect(),
    })
    .await
    .unwrap();
    // No timers but the cache's own, so the paused clock moves only as the
    // test advances it.
    let options = || StreamOptions {
        websocket_connect_timeout_ms: Some(0),
        ..session("ws-age", Some(Transport::Websocket))
    };
    ask(&server, options()).await;
    tokio::time::advance(Duration::from_secs(301)).await;
    ask(&server, options()).await;
    assert_eq!(stats("ws-age")["connectionsCreated"], 2);
    // Used every four minutes, the socket lasts 55 minutes.
    for _ in 0..14 {
        tokio::time::advance(Duration::from_secs(240)).await;
        ask(&server, options()).await;
    }
    assert_eq!(stats("ws-age")["connectionsCreated"], 3);
    assert_eq!(stats("ws-age")["connectionsReused"], 13);
    server.finish().unwrap();
}
