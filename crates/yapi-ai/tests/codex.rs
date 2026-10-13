//! The Codex backend's transports, against a mock server.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::Path;

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
    let mut model = builtin_models("openai-codex")
        .into_iter()
        .find(|model| model.id == "gpt-5.5")
        .unwrap();
    model.base_url = server.url();
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
    yapi_ai::api::close_codex_websockets("ws-closed");
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
