//! The Codex backend's transports, against a mock server.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::path::Path;

use serde_json::Value;
use yapi_ai::api::Apis;
use yapi_ai::catalog::builtin_models;
use yapi_ai::stream::{Request, StreamOptions, Transport};
use yapi_mock::{Cassette, MockServer};
use yapi_types::message::{AssistantMessage, Content, Message, UserMessage};

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

async fn ask(server: &MockServer, options: StreamOptions) -> AssistantMessage {
    let mut model = builtin_models("openai-codex")
        .into_iter()
        .find(|model| model.id == "gpt-5.5")
        .unwrap();
    model.base_url = server.url();
    let request = Request {
        model,
        messages: vec![Message::User(UserMessage {
            content: Content::Text("Say hello".into()),
            timestamp: 1,
        })],
        options: StreamOptions {
            api_key: Some(TOKEN.into()),
            ..options
        },
    };
    Apis::default().stream(request).result().await.unwrap()
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
    let body: Value = serde_json::from_str(&requests[0].body).unwrap();
    assert_eq!(body["model"], "gpt-5.5");
}
