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

/// `ctx.model` and `ctx.thinkingLevel` read the session when read, so a
/// command sees the model and level it just set (#207).
#[tokio::test(flavor = "multi_thread")]
async fn context_reads_live_state() {
    let dir = scratch("api-live-state");
    let host = load(
        &dir,
        r#"export default function (pi) {
	pi.registerCommand("probe", {
		handler: async (_args, ctx) => {
			const before = ctx.model.id;
			await pi.setModel(ctx.modelRegistry.find("faux", "faux-2"));
			pi.setThinkingLevel("low");
			pi.appendEntry("state", { before, after: ctx.model.id, level: ctx.thinkingLevel });
		},
	});
}"#,
    )
    .await;
    let mut config = common::session_config(&Faux::new([]), &dir, host.for_session(), &["read"]);
    let mut second = common::faux_model();
    second.id = "faux-2".into();
    second.reasoning = true;
    config
        .registry
        .register_provider("faux", vec![common::faux_model(), second]);
    let session = AgentSession::new(config);
    bind(&session).await;
    assert_eq!(
        probe(&session, "/probe", "state").await,
        json!({"before": "faux-1", "after": "faux-2", "level": "low"})
    );
}

/// `ctx.signal` is the run's abort signal, which `ctx.abort()` aborts, and
/// `undefined` while no run is active (#204).
#[tokio::test(flavor = "multi_thread")]
async fn context_signal_aborts() {
    let dir = scratch("api-signal");
    let host = load(
        &dir,
        r#"export default function (pi) {
	pi.on("tool_call", async (_event, ctx) => {
		const signal = ctx.signal;
		let heard = false;
		signal?.addEventListener("abort", () => (heard = true));
		ctx.abort();
		await new Promise((resolve) => setTimeout(resolve, 200));
		pi.appendEntry("signal", { aborted: signal?.aborted ?? null, heard });
	});
	pi.registerCommand("probe", {
		handler: async (_args, ctx) => pi.appendEntry("idle", { signal: ctx.signal === undefined }),
	});
}"#,
    )
    .await;
    let faux = Faux::new([
        Response::tool_call("call-1", "read", json!({"path": "a.txt"})),
        Response::text("done"),
    ]);
    let session = common::session(&faux, &dir, host.for_session());
    bind(&session).await;
    assert_eq!(
        probe(&session, "/probe", "idle").await,
        json!({"signal": true})
    );
    session.prompt("go", Vec::new()).await.unwrap();
    eventually("signal", || !custom_entries(&session, "signal").is_empty()).await;
    assert_eq!(
        custom_entries(&session, "signal"),
        [json!({"aborted": true, "heard": true})]
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
