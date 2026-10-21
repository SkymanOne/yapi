//! pi's extension API as extensions call it in a session: what `pi` and
//! `ctx` give back and do, matched against pi `v1.0.0`.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

mod common;

use std::path::Path;
use std::sync::Arc;

use common::{cli_source, custom_entries, engine, eventually, options, scratch};
use serde_json::{Value, json};
use yapi_ai::faux::{Faux, Response};
use yapi_core::agent_session::AgentSession;
use yapi_core::extensions::{Mode, NoUi};
use yapi_ext::ExtensionHost;
use yapi_types::message::Message;

/// Loads extension `source` from a file in `dir`.
async fn load(dir: &Path, source: &str) -> Arc<ExtensionHost> {
    let path = dir.join("ext.ts");
    std::fs::write(&path, source).unwrap();
    let host = ExtensionHost::load(&engine(), options(dir), &[cli_source(&path)])
        .await
        .unwrap();
    assert!(host.errors().is_empty(), "{:?}", host.errors());
    host
}

/// Binds `session` to its extensions without a UI.
async fn bind(session: &AgentSession) {
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
}

/// Runs `command` and waits for the custom entry `kind` it appends.
async fn probe(session: &AgentSession, command: &str, kind: &str) -> Value {
    session.prompt(command, Vec::new()).await.unwrap();
    eventually(kind, || !custom_entries(session, kind).is_empty()).await;
    custom_entries(session, kind).remove(0)
}

/// `pi.getCommands()` lists prompt templates and skills after the
/// extensions' commands, as pi's does (#208).
#[tokio::test(flavor = "multi_thread")]
async fn commands_list_resources() {
    let dir = scratch("api-commands");
    let host = load(
        &dir,
        r#"export default function (pi) {
	pi.registerCommand("probe", {
		handler: async () => pi.appendEntry("commands", pi.getCommands().map((c) => [c.name, c.source, c.sourceInfo.path])),
	});
}"#,
    )
    .await;
    let source = |path: &str| yapi_types::rpc::SourceInfo {
        path: path.into(),
        source: "local".into(),
        scope: "user".into(),
        origin: "top-level".into(),
        base_dir: None,
    };
    let mut config = common::session_config(&Faux::new([]), &dir, host.for_session(), &["read"]);
    config
        .resources
        .templates
        .push(yapi_core::resources::PromptTemplate {
            name: "hello".into(),
            description: "Says hello".into(),
            argument_hint: None,
            content: "Hello".into(),
            file_path: dir.join("hello.md"),
            source: source("hello.md"),
        });
    config
        .resources
        .skills
        .push(yapi_types::system_prompt::Skill {
            name: "foo".into(),
            description: "Does foo".into(),
            file_path: dir.join("foo/SKILL.md"),
            base_dir: dir.join("foo"),
            source: source("foo/SKILL.md"),
            disable_model_invocation: false,
        });
    let session = AgentSession::new(config);
    bind(&session).await;
    let path = dir.join("ext.ts").to_string_lossy().into_owned();
    assert_eq!(
        probe(&session, "/probe", "commands").await,
        json!([
            ["probe", "extension", path],
            ["hello", "prompt", "hello.md"],
            ["skill:foo", "skill", "foo/SKILL.md"],
        ])
    );
}

/// The request's system prompt, as the model reads it.
fn request_prompt(faux: &Faux) -> String {
    let requests = faux.requests();
    let Some(Message::System(system)) = requests[0].first() else {
        panic!("no system message: {:?}", requests[0]);
    };
    system.text()
}

/// `ctx.getSystemPrompt()` gives the full prompt before the first request:
/// the base options' while idle, the run's once it starts (#202).
#[tokio::test(flavor = "multi_thread")]
async fn system_prompt_before_request() {
    let dir = scratch("api-system-prompt");
    let host = load(
        &dir,
        r#"export default function (pi) {
	pi.on("session_start", (_event, ctx) => pi.appendEntry("prompt", ctx.getSystemPrompt()));
	pi.on("agent_start", (_event, ctx) => pi.appendEntry("prompt", ctx.getSystemPrompt()));
}"#,
    )
    .await;
    let faux = Faux::new([Response::text("done")]);
    let session = common::session(&faux, &dir, host.for_session());
    bind(&session).await;
    session.prompt("hi", Vec::new()).await.unwrap();
    let sent = request_prompt(&faux);
    assert!(sent.contains("- read:"), "{sent}");
    assert_eq!(
        custom_entries(&session, "prompt"),
        [json!(sent), json!(sent)]
    );
}

/// `before_agent_start`'s `systemPrompt` and `ctx.getSystemPrompt()` render
/// the options as handlers have edited them so far (#203).
#[tokio::test(flavor = "multi_thread")]
async fn before_agent_start_rerenders() {
    let dir = scratch("api-rerender");
    let host = load(
        &dir,
        r#"export default function (pi) {
	pi.on("before_agent_start", (event, ctx) => {
		event.systemPromptOptions.appendSystemPrompt += "\nMARKER";
		pi.appendEntry("seen", { event: event.systemPrompt.includes("MARKER"), ctx: ctx.getSystemPrompt().includes("MARKER") });
	});
	pi.on("before_agent_start", (event) => ({ systemPrompt: `${event.systemPrompt}\nSECOND` }));
}"#,
    )
    .await;
    let faux = Faux::new([Response::text("done")]);
    let session = common::session(&faux, &dir, host.for_session());
    bind(&session).await;
    session.prompt("hi", Vec::new()).await.unwrap();
    assert_eq!(
        custom_entries(&session, "seen"),
        [json!({"event": true, "ctx": true})]
    );
    let sent = request_prompt(&faux);
    assert!(
        sent.contains("MARKER") && sent.ends_with("\nSECOND"),
        "{sent}"
    );
}
