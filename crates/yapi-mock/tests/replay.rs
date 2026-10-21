//! The mock server against a real HTTP client.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use std::net::SocketAddr;
use std::path::Path;
use std::time::{Duration, Instant};

use yapi_mock::{Cassette, Error, Interaction, MockServer, RequestMatch, Response};

const ANY_PORT: ([u8; 4], u16) = ([127, 0, 0, 1], 0);

fn sample() -> Cassette {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/cassettes/anthropic-messages/text.json");
    Cassette::load(&path).unwrap()
}

fn interaction(method: &str, path: &str, response: Response) -> Interaction {
    Interaction {
        request: RequestMatch {
            method: method.into(),
            path: path.into(),
        },
        response,
    }
}

fn reply(status: u16, chunks: &[&str]) -> Response {
    Response {
        status,
        headers: Default::default(),
        chunks: chunks.iter().map(|chunk| chunk.to_string()).collect(),
        body_base64: None,
        chunk_delay_ms: 0,
        close: None,
    }
}

async fn start(cassette: Cassette) -> MockServer {
    MockServer::start(SocketAddr::from(ANY_PORT), cassette)
        .await
        .unwrap()
}

#[tokio::test]
async fn streams_and_records() {
    let cassette = sample();
    let expected_body: String = cassette.interactions[0].response.chunks.concat();
    let server = start(cassette).await;

    let mut response = reqwest::Client::new()
        .post(format!("{}/v1/messages?beta=true", server.url()))
        .header("x-api-key", "test-key")
        .header("content-type", "application/json")
        .body(r#"{"model":"claude-sonnet-4-5","stream":true}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(response.headers()["content-type"], "text/event-stream");
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.unwrap() {
        body.extend_from_slice(&chunk);
    }
    assert_eq!(String::from_utf8(body).unwrap(), expected_body);

    let requests = server.finish().unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];
    assert_eq!(
        (request.method.as_str(), request.path.as_str()),
        ("POST", "/v1/messages")
    );
    assert_eq!(request.query.as_deref(), Some("beta=true"));
    assert_eq!(request.headers["x-api-key"], yapi_mock::REDACTED);
    assert_eq!(
        request.body,
        r#"{"model":"claude-sonnet-4-5","stream":true}"#
    );
}

#[tokio::test]
async fn serves_interactions_in_order() {
    let mut rate_limited = reply(429, &[r#"{"error":{"type":"rate_limit_error"}}"#]);
    rate_limited
        .headers
        .insert("retry-after".into(), "1".into());
    let server = start(Cassette {
        interactions: vec![
            interaction("POST", "/v1/chat/completions", rate_limited),
            interaction(
                "POST",
                "/v1/chat/completions",
                reply(200, &["data: [DONE]\n\n"]),
            ),
        ],
    })
    .await;

    let client = reqwest::Client::new();
    let url = format!("{}/v1/chat/completions", server.url());
    let first = client.post(&url).send().await.unwrap();
    assert_eq!(first.status(), 429);
    assert_eq!(first.headers()["retry-after"], "1");
    assert_eq!(
        first.text().await.unwrap(),
        r#"{"error":{"type":"rate_limit_error"}}"#
    );
    let second = client.post(&url).send().await.unwrap();
    assert_eq!(second.text().await.unwrap(), "data: [DONE]\n\n");

    assert_eq!(server.finish().unwrap().len(), 2);
}

#[tokio::test]
async fn unexpected_request_500() {
    let server = start(sample()).await;

    let response = reqwest::get(format!("{}/v1/models", server.url()))
        .await
        .unwrap();
    assert_eq!(response.status(), 500);
    assert!(
        response
            .text()
            .await
            .unwrap()
            .contains("expected POST /v1/messages")
    );

    let Err(Error::Unsatisfied(problems)) = server.finish() else {
        panic!("mismatch not reported");
    };
    assert_eq!(
        problems,
        ["request 1 (GET /v1/models): expected POST /v1/messages"]
    );
}

#[tokio::test]
async fn unused_and_extra_reported() {
    let server = start(sample()).await;
    let Err(Error::Unsatisfied(problems)) = server.finish() else {
        panic!("unused interaction not reported");
    };
    assert_eq!(problems, ["never requested: POST /v1/messages"]);

    let empty = start(Cassette::default()).await;
    let response = reqwest::Client::new()
        .post(format!("{}/v1/messages", empty.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 500);
    assert!(empty.finish().is_err());
}

#[tokio::test]
async fn delays_each_chunk() {
    let mut slow = reply(200, &["a", "b", "c"]);
    slow.chunk_delay_ms = 30;
    let server = start(Cassette {
        interactions: vec![interaction("POST", "/stream", slow)],
    })
    .await;

    let started = Instant::now();
    let response = reqwest::Client::new()
        .post(format!("{}/stream", server.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.text().await.unwrap(), "abc");
    assert!(started.elapsed() >= Duration::from_millis(90));
}

/// Opens a WebSocket to `path` with the handshake of RFC 6455's example.
async fn websocket(
    server: &MockServer,
    path: &str,
) -> tokio_tungstenite::WebSocketStream<tokio::net::TcpStream> {
    use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
    let mut stream = tokio::net::TcpStream::connect(server.url().trim_start_matches("http://"))
        .await
        .unwrap();
    let request = format!(
        "GET {path} HTTP/1.1\r\nHost: mock\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\n\r\n"
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut head = Vec::new();
    while !head.ends_with(b"\r\n\r\n") {
        head.push(stream.read_u8().await.unwrap());
    }
    let head = String::from_utf8(head).unwrap().to_lowercase();
    assert!(head.starts_with("http/1.1 101"), "{head}");
    assert!(head.contains("sec-websocket-accept: s3pplmbitxaq9kygzzhzrbk+xoo="));
    tokio_tungstenite::WebSocketStream::from_raw_socket(
        stream,
        tokio_tungstenite::tungstenite::protocol::Role::Client,
        None,
    )
    .await
}

#[tokio::test]
async fn replays_websocket_exchanges() {
    use futures_util::{SinkExt as _, StreamExt as _};
    use tokio_tungstenite::tungstenite::Message;
    let exchange = |chunks: &[&str]| interaction("GET", "/ws", reply(101, chunks));
    let first = start(Cassette {
        interactions: vec![
            exchange(&["a", "b"]),
            exchange(&["c"]),
            interaction("POST", "/ws", reply(200, &["done"])),
        ],
    })
    .await;

    let mut socket = websocket(&first, "/ws").await;
    let mut received = Vec::new();
    socket.send(Message::text("one")).await.unwrap();
    received.push(socket.next().await.unwrap().unwrap());
    received.push(socket.next().await.unwrap().unwrap());
    socket.send(Message::text("two")).await.unwrap();
    received.push(socket.next().await.unwrap().unwrap());
    assert_eq!(received, ["a", "b", "c"].map(Message::text));
    // The next interaction is not an exchange, so the socket closes.
    assert!(matches!(
        socket.next().await.unwrap().unwrap(),
        Message::Close(_)
    ));
    let response = reqwest::Client::new()
        .post(format!("{}/ws", first.url()))
        .send()
        .await
        .unwrap();
    assert_eq!(response.text().await.unwrap(), "done");

    let mut closing = exchange(&["d"]);
    closing.response.chunk_delay_ms = 20;
    closing.response.close = Some(yapi_mock::Close {
        code: 1009,
        reason: "big".into(),
    });
    let server = start(Cassette {
        interactions: vec![closing],
    })
    .await;
    let mut socket = websocket(&server, "/ws").await;
    socket.send(Message::text("three")).await.unwrap();
    let started = Instant::now();
    assert_eq!(socket.next().await.unwrap().unwrap(), Message::text("d"));
    assert!(started.elapsed() >= Duration::from_millis(20));
    let Message::Close(Some(frame)) = socket.next().await.unwrap().unwrap() else {
        panic!("no close frame");
    };
    assert_eq!(
        (u16::from(frame.code), frame.reason.as_str()),
        (1009, "big")
    );
    server.finish().unwrap();

    let requests = first.finish().unwrap();
    let seen: Vec<(&str, &str)> = requests
        .iter()
        .map(|request| (request.method.as_str(), request.body.as_str()))
        .collect();
    assert_eq!(seen, [("GET", "one"), ("GET", "two"), ("POST", "")]);
}
