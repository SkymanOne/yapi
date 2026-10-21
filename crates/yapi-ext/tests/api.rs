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

/// Dialogs that stay open until cancelled, counting those opened.
#[derive(Default)]
struct Waiting(std::sync::atomic::AtomicUsize);

impl yapi_core::extensions::ExtensionUi for Waiting {
    fn has_ui(&self) -> bool {
        true
    }

    fn notify(&self, _message: &str, _kind: yapi_core::extensions::NotifyKind) {}

    fn confirm(
        &self,
        _title: &str,
        _message: &str,
        dialog: yapi_core::extensions::DialogOptions,
    ) -> futures_util::future::BoxFuture<'static, bool> {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Box::pin(async move {
            match dialog.cancel {
                Some(cancel) => cancel.cancelled().await,
                None => std::future::pending().await,
            }
            false
        })
    }
}

/// A dialog's `signal` dismisses it with its default answer, and one that
/// is already aborted opens no dialog (#205).
#[tokio::test(flavor = "multi_thread")]
async fn dialog_signal_dismisses() {
    let dir = scratch("api-dialog-signal");
    let host = load(
        &dir,
        r#"export default function (pi) {
	pi.registerCommand("probe", {
		handler: async (_args, ctx) => {
			const later = new AbortController();
			setTimeout(() => later.abort(), 100);
			const answer = await ctx.ui.confirm("T", "M", { signal: later.signal });
			const early = await ctx.ui.confirm("T", "M", { signal: AbortSignal.abort() });
			pi.appendEntry("dialogs", { answer, early });
		},
	});
}"#,
    )
    .await;
    let session = common::session(&Faux::new([]), &dir, host.for_session());
    let ui = Arc::new(Waiting::default());
    session
        .bind_extensions(ui.clone(), Mode::Rpc, None, None)
        .await;
    let probed = probe(&session, "/probe", "dialogs");
    let answered = tokio::time::timeout(std::time::Duration::from_secs(10), probed)
        .await
        .expect("the signal left the dialog open");
    assert_eq!(answered, json!({"answer": false, "early": false}));
    assert_eq!(ui.0.load(std::sync::atomic::Ordering::SeqCst), 1);
}

/// `ctx.executeTool` passes the nested tool's partial results to
/// `onUpdate`, and `signal` cancels the nested call (#206).
#[tokio::test(flavor = "multi_thread")]
async fn nested_call_options() {
    let dir = scratch("api-nested-options");
    let host = load(
        &dir,
        r#"export default function (pi) {
	pi.registerTool({
		name: "nest", label: "Nest", description: "Nests", parameters: { type: "object", properties: {} },
		async execute(_id, _params, _signal, _onUpdate, ctx) {
			let updates = 0;
			await ctx.executeTool("bash", { command: "echo a; sleep 0.2; echo b" }, { onUpdate: () => updates++ });
			const controller = new AbortController();
			setTimeout(() => controller.abort(), 200);
			const started = Date.now();
			const aborted = await ctx.executeTool("bash", { command: "sleep 5; echo done" }, { signal: controller.signal });
			pi.appendEntry("nested", { updated: updates > 0, isError: aborted.isError, quick: Date.now() - started < 3000 });
			return { content: [{ type: "text", text: "ok" }] };
		},
	});
}"#,
    )
    .await;
    let faux = Faux::new([
        Response::tool_call("call-1", "nest", json!({})),
        Response::text("done"),
    ]);
    let session = common::session_with_tools(&faux, &dir, host.for_session(), &["read", "bash"]);
    bind(&session).await;
    session.prompt("go", Vec::new()).await.unwrap();
    assert_eq!(
        custom_entries(&session, "nested"),
        [json!({"updated": true, "isError": true, "quick": true})]
    );
}

/// `pi.exec` and the local bash operations resolve once the process exits,
/// without waiting for a descendant that holds its output open (#209).
#[tokio::test(flavor = "multi_thread")]
async fn exec_ignores_descendants() {
    let dir = scratch("api-exec-exit");
    let host = load(
        &dir,
        r#"import { createLocalBashOperations } from "@earendil-works/pi-coding-agent";
export default function (pi) {
	pi.registerCommand("probe", {
		handler: async (_args, ctx) => {
			let started = Date.now();
			const result = await pi.exec("sh", ["-c", "sleep 3 & echo started"]);
			const exec = Date.now() - started;
			started = Date.now();
			let output = "";
			const bash = await createLocalBashOperations().exec("sleep 3 & echo started", ctx.cwd, { onData: (data) => (output += data) });
			const operations = Date.now() - started;
			pi.appendEntry("exec", { stdout: result.stdout, code: result.code, output, exitCode: bash.exitCode, quick: exec < 2000 && operations < 2000 });
		},
	});
}"#,
    )
    .await;
    let session = common::session(&Faux::new([]), &dir, host.for_session());
    bind(&session).await;
    assert_eq!(
        probe(&session, "/probe", "exec").await,
        json!({"stdout": "started\n", "code": 0, "output": "started\n", "exitCode": 0, "quick": true})
    );
}

/// `pi.getAllTools()`, `getHeader()` and the session manager's context
/// builders give pi's shapes (#210).
#[tokio::test(flavor = "multi_thread")]
async fn tools_and_session_shapes() {
    let dir = scratch("api-shapes");
    let host = load(
        &dir,
        r#"export default function (pi) {
	pi.registerTool({
		name: "probe_tool", label: "Probe", description: "Probes", parameters: { type: "object", properties: {} },
		promptGuidelines: ["Probe first."], annotations: { readOnlyHint: true }, exposure: "deferred",
		async execute() { return { content: [] }; },
	});
	pi.registerCommand("probe", {
		handler: async (_args, ctx) => {
			const tools = pi.getAllTools();
			const own = tools.find((tool) => tool.name === "probe_tool");
			const read = tools.find((tool) => tool.name === "read");
			const sm = ctx.sessionManager;
			const projection = sm.buildSessionProjection();
			pi.appendEntry("shapes", {
				own: [own.promptGuidelines, own.annotations, own.exposure, own.sourceInfo.source],
				read: [read.exposure, read.sourceInfo.path, read.sourceInfo.source, "annotations" in read],
				header: sm.getHeader().type,
				entries: sm.buildContextEntries().map((entry) => entry.type),
				projection: [projection.entries.length, projection.messages.length, projection.thinkingLevel, projection.model.modelId],
			});
		},
	});
}"#,
    )
    .await;
    let session = common::session(&Faux::new([]), &dir, host.for_session());
    bind(&session).await;
    assert_eq!(
        probe(&session, "/probe", "shapes").await,
        json!({
            "own": [["Probe first."], {"readOnlyHint": true}, "deferred", "cli"],
            "read": ["direct", "builtin:read", "builtin", false],
            "header": "session",
            "entries": ["model_change", "thinking_level_change"],
            "projection": [2, 0, "off", "faux-1"],
        })
    );
}
