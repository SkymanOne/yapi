//! Requests that bypass the `httpProxy` setting. The setting is process-wide,
//! so these tests have their own binary.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

use serde_json::json;
use tokio_util::sync::CancellationToken;
use yapi_ai::aws::{AwsEnv, default_credentials};
use yapi_mock::{Cassette, Interaction, MockServer};

/// AWS container and instance metadata carry tokens and credentials in plain
/// HTTP, which the SDK never sends through a proxy.
#[tokio::test]
async fn aws_metadata_unproxied() {
    // A proxy that refuses connections fails every request sent through it.
    let closed = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let proxy = format!("http://{}", closed.local_addr().unwrap());
    drop(closed);
    yapi_ai::http::set_settings_proxy(Some(&proxy));

    let credentials = json!({"AccessKeyId": "AKIDMETA", "SecretAccessKey": "secret"});
    let mut roles = Interaction::json(
        "GET",
        "/latest/meta-data/iam/security-credentials/",
        200,
        &json!(null),
    );
    roles.response.chunks = vec!["role".into()];
    let server = MockServer::local(Cassette {
        interactions: vec![
            Interaction::json("GET", "/credentials", 200, &credentials),
            Interaction::json("PUT", "/latest/api/token", 200, &json!("token")),
            roles,
            Interaction::json(
                "GET",
                "/latest/meta-data/iam/security-credentials/role",
                200,
                &credentials,
            ),
        ],
    })
    .await
    .unwrap();
    let base = server.url();
    let container = format!("{base}/credentials");
    let home = std::env::temp_dir().join(format!("yapi-proxy-{}", std::process::id()));
    let home = home.to_str().unwrap();
    for source in [
        ("AWS_CONTAINER_CREDENTIALS_FULL_URI", container.as_str()),
        ("AWS_EC2_METADATA_SERVICE_ENDPOINT", base.as_str()),
    ] {
        let env = AwsEnv::from_vars([("HOME", home), source]);
        let found = default_credentials(&env, None, None, &CancellationToken::new()).await;
        assert_eq!(found.unwrap().access_key_id, "AKIDMETA", "{source:?}");
    }
    server.finish().unwrap();
}
