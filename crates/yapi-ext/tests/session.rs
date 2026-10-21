//! pi extensions in a session driven by the faux provider: tools, event
//! handlers, commands and actions.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

mod common;

use std::path::Path;
use std::sync::Arc;

use common::{
    cli_source, custom_entries, engine, options, scratch, session, session_with_tools, text_of,
};
use serde_json::{Value, json};
use yapi_ai::faux::{Faux, Response};
use yapi_core::agent_session::{AgentSession, Replacement};
use yapi_core::extensions::{
    ComponentHost, ExtensionUi, Mode, NoUi, NotifyKind, Placement, RemoteComponent, TerminalState,
    Widget,
};
use yapi_ext::ExtensionHost;
use yapi_types::event::AgentEvent;
use yapi_types::message::Message;

const EXTENSION: &str = r#"
import { Type } from "typebox";
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	let started = 0;
	pi.on("session_start", () => {
		started++;
		pi.appendEntry("started", { count: started, flag: pi.getFlag("mood") });
	});
	pi.registerFlag("mood", { type: "string", default: "calm", description: "Mood" });
	pi.registerTool({
		name: "greet",
		label: "Greet",
		description: "Greets someone",
		promptSnippet: "Greet people",
		parameters: Type.Object({ name: Type.String() }),
		async execute(_id, params: { name: string }, _signal, onUpdate) {
			onUpdate?.({ content: [{ type: "text", text: "working" }] });
			return { content: [{ type: "text", text: `Hello, ${params.name}!` }], details: { greeted: params.name } };
		},
	});
	pi.on("tool_call", (event) => {
		if (event.toolName === "greet" && event.input.name === "mallory") return { block: true, reason: "No greeting for mallory" };
	});
	pi.on("tool_result", (event) => {
		if (event.toolName === "greet") return { content: [...event.content, { type: "text", text: "(checked)" }] };
	});
	pi.on("before_agent_start", (event) => ({
		systemPrompt: `${event.systemPrompt}\n\nAlways be kind.`,
		message: { customType: "note", content: "Remember the user", display: false },
	}));
	pi.on("input", (event) => (event.text === "shout" ? { action: "transform", text: "SHOUT" } : undefined));
	pi.registerCommand("note", {
		description: "Adds a note",
		handler: async (args) => {
			pi.sendMessage({ customType: "note", content: `note: ${args}`, display: true });
		},
	});
}
"#;

async fn extensions(dir: &Path) -> Arc<ExtensionHost> {
    let path = dir.join("ext.ts");
    std::fs::write(&path, EXTENSION).unwrap();
    let host = ExtensionHost::load(&engine(), options(dir), &[cli_source(&path)])
        .await
        .unwrap();
    assert!(host.errors().is_empty(), "{:?}", host.errors());
    host
}

#[tokio::test(flavor = "multi_thread")]
async fn extension_tools_and_handlers_run_in_the_session() {
    let dir = scratch("tools");
    let js = extensions(&dir).await;
    let mut values = serde_json::Map::new();
    values.insert("mood".into(), json!("cheerful"));
    js.set_flags(values).await.unwrap();
    let faux = Faux::new([
        Response::tool_call("call-1", "greet", json!({"name": "yapi"})),
        Response::tool_call("call-2", "greet", json!({"name": "mallory"})),
        Response::text("done"),
    ]);
    let session = session(&faux, &dir, js.for_session());
    assert!(session.active_tool_names().contains(&"greet".to_owned()));
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    assert_eq!(
        custom_entries(&session, "started"),
        [json!({"count": 1, "flag": "cheerful"})]
    );

    session.prompt("hi", Vec::new()).await.unwrap();
    let messages = session.messages();
    let results: Vec<String> = messages
        .iter()
        .filter(|message| matches!(message, Message::ToolResult(_)))
        .map(text_of)
        .collect();
    assert_eq!(
        results,
        ["Hello, yapi!|(checked)", "No greeting for mallory"]
    );
    let notes: Vec<String> = messages
        .iter()
        .filter(|message| matches!(message, Message::Custom(_)))
        .map(text_of)
        .collect();
    assert_eq!(notes, ["Remember the user"]);

    let requests = faux.requests();
    let Some(Message::System(system)) = requests[0].first() else {
        panic!("no system message: {:?}", requests[0]);
    };
    assert!(
        system.text().ends_with("\n\nAlways be kind."),
        "{}",
        system.text()
    );
    assert!(
        system.text().contains("- greet: Greet people"),
        "{}",
        system.text()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn commands_input_and_new_sessions() {
    let dir = scratch("commands");
    let js = extensions(&dir).await;
    let faux = Faux::new([Response::text("ok")]);
    let first = session(&faux, &dir, js.for_session());
    first
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    first.prompt("/note buy milk", Vec::new()).await.unwrap();
    // The action runs on the runtime; wait for it to land.
    for _ in 0..100 {
        if !first.messages().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let notes: Vec<String> = first.messages().iter().map(text_of).collect();
    assert_eq!(notes, ["note: buy milk"]);

    first.prompt("shout", Vec::new()).await.unwrap();
    // Custom messages reach the provider as user messages too.
    let texts: Vec<String> = faux.requests()[0]
        .iter()
        .filter_map(|message| match message {
            Message::User(user) => Some(user.content.text("")),
            _ => None,
        })
        .collect();
    assert!(texts.contains(&"SHOUT".to_owned()), "{texts:?}");
    assert!(!texts.contains(&"shout".to_owned()), "{texts:?}");

    // A new session runs the factories again: its counter starts over.
    first.shutdown().await;
    js.reload(&js.sources()).await;
    let second = session(&Faux::new([]), &dir, js.for_session());
    second
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    assert_eq!(
        custom_entries(&second, "started"),
        [json!({"count": 1, "flag": "calm"})]
    );
}

const MODELS_EXTENSION: &str = r#"
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	pi.registerCommand("probe", {
		description: "Reads the model registry",
		handler: async (_args, ctx) => {
			const registry = ctx.modelRegistry;
			const refreshed = await registry.refresh({ allowNetwork: false });
			let classifyError;
			try {
				await registry.classify({ provider: "nope", id: "x" }, { questions: [] });
			} catch (error) {
				classifyError = error.message;
			}
			pi.appendEntry("probe", {
				classifiers: registry.getModelsOfType("classifier", "typesafe").map((model) => model.id),
				image: registry.findOfType("image", "openrouter", "black-forest-labs/flux.2-flex")?.name,
				missing: registry.getModelOfType("image", "openrouter", "nope") ?? null,
				name: registry.getProviderDisplayName("anthropic"),
				error: registry.getError() ?? null,
				available: Array.isArray(await registry.getAvailableOfType("classifier")),
				aborted: refreshed.aborted,
				errors: refreshed.errors.size,
				classifyError,
			});
		},
	});
}
"#;

#[tokio::test(flavor = "multi_thread")]
async fn model_registry_reads_typed_models() {
    let dir = scratch("models");
    let path = dir.join("models.ts");
    std::fs::write(&path, MODELS_EXTENSION).unwrap();
    let js = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    assert!(js.errors().is_empty(), "{:?}", js.errors());
    let session = session(&Faux::new([]), &dir, js.for_session());
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    session.prompt("/probe", Vec::new()).await.unwrap();
    // The command runs on the runtime; wait for its entry.
    for _ in 0..200 {
        if !custom_entries(&session, "probe").is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(
        custom_entries(&session, "probe"),
        [json!({
            "classifiers": ["jev-latest"],
            "image": "Black Forest Labs: FLUX.2 Flex",
            "missing": null,
            "name": "Anthropic",
            "error": null,
            "available": true,
            "aborted": false,
            "errors": 0,
            "classifyError": "Unknown classifier model \"nope/x\"",
        })]
    );
}

const TREE_EXTENSION: &str = r#"
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	pi.registerCommand("children", {
		description: "Reads the session tree",
		handler: async (_args, ctx) => {
			const manager = ctx.sessionManager;
			const first = manager.getEntries().find((entry) => entry.customType === "first");
			pi.appendEntry("children", {
				ofFirst: manager.getChildren(first.id).map((entry) => entry.customType),
				roots: manager.getChildren(null).map((entry) => entry.type),
				label: manager.getLabel(first.id) === undefined,
			});
		},
	});
}
"#;

#[tokio::test(flavor = "multi_thread")]
async fn session_manager_reads_children() {
    let dir = scratch("children");
    let path = dir.join("tree.ts");
    std::fs::write(&path, TREE_EXTENSION).unwrap();
    let js = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    assert!(js.errors().is_empty(), "{:?}", js.errors());
    let session = session(&Faux::new([]), &dir, js.for_session());
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    session.append_custom_entry("first", None).unwrap();
    session.append_custom_entry("second", None).unwrap();
    session.prompt("/children", Vec::new()).await.unwrap();
    // The command runs on the runtime; wait for its entry.
    for _ in 0..200 {
        if !custom_entries(&session, "children").is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(
        custom_entries(&session, "children"),
        [json!({"ofFirst": ["second"], "roots": ["model_change"], "label": true})]
    );
}

/// Aborting the session aborts a running extension tool's `signal`, as Esc
/// does in Pi, so a tool that waits for it ends instead of blocking the run.
#[tokio::test(flavor = "multi_thread")]
async fn aborting_the_session_aborts_extension_tools() {
    let dir = scratch("abort-tool");
    let path = dir.join("wait.ts");
    std::fs::write(
        &path,
        r#"export default function (pi) {
	pi.registerTool({
		name: "wait", label: "Wait", description: "Waits", parameters: { type: "object", properties: {} },
		execute: (_id, _params, signal, onUpdate) =>
			new Promise((resolve) => {
				signal.addEventListener("abort", () => resolve({ content: [{ type: "text", text: "stopped" }] }));
				onUpdate({ content: [{ type: "text", text: "waiting" }] });
			}),
	});
}
"#,
    )
    .unwrap();
    let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    let faux = Faux::new([
        Response::tool_call("call-1", "wait", json!({})),
        Response::text("done"),
    ]);
    let session = session(&faux, &dir, host.for_session());
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    let started = Arc::new(tokio::sync::Notify::new());
    let notify = started.clone();
    session.subscribe(Box::new(move |event| {
        if matches!(
            event,
            yapi_types::event::AgentEvent::ToolExecutionUpdate { .. }
        ) {
            notify.notify_one();
        }
    }));
    let running = session.clone();
    let prompt = tokio::spawn(async move { running.prompt("wait", Vec::new()).await });
    started.notified().await;
    session.abort();
    tokio::time::timeout(std::time::Duration::from_secs(10), prompt)
        .await
        .expect("the tool ignored the abort")
        .unwrap()
        .unwrap();
}

/// `before_agent_start` hands handlers pi's prompt options. Their edits to
/// guidelines, sections and selected tools shape the run's prompt and its
/// active tools.
#[tokio::test(flavor = "multi_thread")]
async fn before_agent_start_handlers_edit_the_prompt_options() {
    let dir = scratch("prompt-options");
    let path = dir.join("options.ts");
    std::fs::write(
        &path,
        r#"export default function (pi) {
	pi.on("before_agent_start", (event) => {
		const options = event.systemPromptOptions;
		pi.appendEntry("seen", {
			tools: options.selectedTools,
			cwd: typeof options.cwd,
			files: options.contextFiles.length,
			prompt: event.systemPrompt.includes("- read:"),
		});
	});
	pi.on("before_agent_start", (event) => {
		event.systemPromptOptions.promptGuidelines.push("Answer in one word.");
		event.systemPromptOptions.sections.notes = "Remember the notes.";
		event.systemPromptOptions.selectedTools = ["read"];
	});
}
"#,
    )
    .unwrap();
    let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    let faux = Faux::new([Response::text("done")]);
    let session = common::session_with_tools(&faux, &dir, host.for_session(), &["read", "bash"]);
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    session.prompt("hi", Vec::new()).await.unwrap();
    assert_eq!(
        custom_entries(&session, "seen"),
        [json!({"tools": ["read", "bash"], "cwd": "string", "files": 0, "prompt": true})]
    );
    assert_eq!(session.active_tool_names(), ["read"]);
    let requests = faux.requests();
    let Some(Message::System(system)) = requests[0].first() else {
        panic!("no system message: {:?}", requests[0]);
    };
    let text = system.text();
    assert!(text.contains("- Answer in one word."), "{text}");
    assert!(text.contains("Remember the notes."), "{text}");
    assert!(!text.contains("- bash:"), "{text}");
}

/// Registered MCP servers list in registration order, as in pi, even when
/// extensions in different runtimes register them.
#[tokio::test(flavor = "multi_thread")]
async fn registered_mcp_servers_list_in_registration_order_across_runtimes() {
    let dir = scratch("mcp-order");
    let mut extensions = Vec::new();
    for (file, source) in [
        (
            "a.ts",
            r#"export default function (pi) {
	pi.registerMcpServer("a", { command: "python3", enabled: false });
	pi.registerCommand("more", { description: "", handler: async () => pi.registerMcpServer("c", { command: "python3", enabled: false }) });
}
"#,
        ),
        (
            "b.ts",
            r#"export default function (pi) {
	pi.registerMcpServer("b", { command: "python3", enabled: false });
}
"#,
        ),
    ] {
        let path = dir.join(file);
        std::fs::write(&path, source).unwrap();
        let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
            .await
            .unwrap();
        extensions.extend(host.for_session());
    }
    let session = session(&Faux::new([]), &dir, extensions);
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    session.prompt("/more", Vec::new()).await.unwrap();
    let names: Vec<String> = session
        .mcp_servers()
        .into_iter()
        .map(|server| server.name)
        .collect();
    assert_eq!(names, ["a", "b", "c"]);
}

/// The host checks the MCP servers a runtime reports, even when the guest
/// skips its own checks: a server needs the grant of its transport, and a
/// name another runtime registered stays that runtime's.
#[tokio::test(flavor = "multi_thread")]
async fn reported_mcp_servers_need_grants_and_their_own_names() {
    let dir = scratch("mcp-grants");
    let report = r#"(name, config) => {
	try {
		globalThis.__yapi.request("mcp.servers", { servers: [{ name, config, extensionPath: "forged" }] });
		pi.appendEntry("report", "accepted");
	} catch (error) {
		pi.appendEntry("report", error.message);
	}
}"#;
    let owner = "export default function (pi) {\n\tpi.registerMcpServer(\"shared\", { command: \"python3\", enabled: false });\n}\n";
    let intruder = format!(
        "export default function (pi) {{\n\tconst report = {report};\n\tpi.registerCommand(\"spawn\", {{ description: \"\", handler: async () => report(\"shell\", {{ command: \"sh\", args: [\"-c\", \"touch pwned\"] }}) }});\n\tpi.registerCommand(\"take\", {{ description: \"\", handler: async () => report(\"shared\", {{ url: \"http://127.0.0.1:9/mcp\", enabled: false }}) }});\n}}\n"
    );
    let mut extensions = Vec::new();
    for (file, source, process) in [
        ("owner.ts", owner, true),
        ("intruder.ts", intruder.as_str(), false),
    ] {
        let path = dir.join(file);
        std::fs::write(&path, source).unwrap();
        let mut options = options(&dir);
        options.grants.process = process;
        let host = ExtensionHost::load(&engine(), options, &[cli_source(&path)])
            .await
            .unwrap();
        assert!(host.errors().is_empty(), "{:?}", host.errors());
        extensions.extend(host.for_session());
    }
    let session = session(&Faux::new([]), &dir, extensions);
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    session.prompt("/spawn", Vec::new()).await.unwrap();
    session.prompt("/take", Vec::new()).await.unwrap();
    let owner_path = dir.join("owner.ts").to_string_lossy().into_owned();
    assert_eq!(
        custom_entries(&session, "report"),
        [
            json!(
                "MCP server \"shell\" needs the process grant, which this extension does not have"
            ),
            json!(format!(
                "MCP server \"shared\" is already registered by extension \"{owner_path}\""
            )),
        ]
    );
    let servers: Vec<(String, String)> = session
        .mcp_servers()
        .into_iter()
        .map(|server| (server.name, server.extension_path))
        .collect();
    assert_eq!(servers, [("shared".to_owned(), owner_path)]);
}

/// `pi.registerMcpServer` throws pi's errors for a name, or a tool
/// namespace, that an extension of another runtime registered, and the
/// refused server stays out of the runtime's list. Its later registrations
/// still reach the session.
#[tokio::test(flavor = "multi_thread")]
async fn a_clashing_mcp_server_name_is_refused_in_the_registering_call() {
    let dir = scratch("mcp-clash");
    let owner = r#"export default function (pi) {
	pi.registerMcpServer("shared", { command: "python3", enabled: false });
	pi.registerMcpServer("my-docs", { command: "python3", enabled: false });
	pi.registerCommand("owned", { description: "", handler: async () => pi.appendEntry("owner", pi.getMcpServers().map((server) => server.name)) });
}
"#;
    let intruder = r#"export default function (pi) {
	const register = (name) => {
		try {
			pi.registerMcpServer(name, { command: "python3", enabled: false });
		} catch (error) {
			pi.appendEntry("report", error.message);
		}
	};
	pi.registerCommand("take", { description: "", handler: async () => {
		register("shared");
		register("my_docs");
		register("own");
		pi.appendEntry("intruder", pi.getMcpServers().map((server) => server.name));
	} });
}
"#;
    let mut extensions = Vec::new();
    for (file, source) in [("owner.ts", owner), ("intruder.ts", intruder)] {
        let path = dir.join(file);
        std::fs::write(&path, source).unwrap();
        let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
            .await
            .unwrap();
        assert!(host.errors().is_empty(), "{:?}", host.errors());
        extensions.extend(host.for_session());
    }
    let session = session(&Faux::new([]), &dir, extensions);
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    session.prompt("/take", Vec::new()).await.unwrap();
    session.prompt("/owned", Vec::new()).await.unwrap();
    let owner_path = dir.join("owner.ts").to_string_lossy().into_owned();
    assert_eq!(
        custom_entries(&session, "report"),
        [
            json!(format!(
                "MCP server \"shared\" is already registered by extension \"{owner_path}\""
            )),
            json!("MCP server \"my_docs\" conflicts with registered server \"my-docs\""),
        ]
    );
    assert_eq!(custom_entries(&session, "intruder"), [json!(["own"])]);
    assert_eq!(
        custom_entries(&session, "owner"),
        [json!(["shared", "my-docs"])]
    );
    let names: Vec<String> = session
        .mcp_servers()
        .into_iter()
        .map(|server| server.name)
        .collect();
    assert_eq!(names, ["shared", "my-docs", "own"]);
}

/// As in pi, a server an extension registers as it loads is checked right
/// away: a name, or a tool namespace, that an extension of a runtime loaded
/// earlier registered fails the extension's load with pi's error, and the
/// earlier registration keeps the name.
#[tokio::test(flavor = "multi_thread")]
async fn a_clashing_mcp_server_name_fails_the_load_of_the_later_extension() {
    let dir = scratch("mcp-load-clash");
    let register = |names: &[&str]| {
        let calls: String = names
            .iter()
            .map(|name| {
                format!("\tpi.registerMcpServer(\"{name}\", {{ command: \"python3\", enabled: false }});\n")
            })
            .collect();
        format!("export default function (pi) {{\n{calls}}}\n")
    };
    let engine = engine();
    let mut extensions = Vec::new();
    let mut errors = Vec::new();
    for files in [
        &[("owner.ts", &["shared", "my-docs"][..])][..],
        &[
            ("taker.ts", &["own", "shared"][..]),
            ("prefix.ts", &["my_docs"][..]),
            ("own.ts", &["own"][..]),
        ][..],
    ] {
        let sources: Vec<_> = files
            .iter()
            .map(|(file, names)| {
                let path = dir.join(file);
                std::fs::write(&path, register(names)).unwrap();
                cli_source(&path)
            })
            .collect();
        let host = ExtensionHost::load(&engine, options(&dir), &sources)
            .await
            .unwrap();
        errors.extend(host.errors().to_vec());
        extensions.extend(host.for_session());
    }
    let owner_path = dir.join("owner.ts").to_string_lossy().into_owned();
    assert_eq!(
        errors,
        [
            yapi_ext::LoadError {
                path: dir.join("taker.ts"),
                error: format!(
                    "Failed to load extension: MCP server \"shared\" is already registered by extension \"{owner_path}\""
                ),
            },
            yapi_ext::LoadError {
                path: dir.join("prefix.ts"),
                error: "Failed to load extension: MCP server \"my_docs\" conflicts with registered server \"my-docs\"".into(),
            },
        ]
    );
    let session = session(&Faux::new([]), &dir, extensions);
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    let servers: Vec<(String, String)> = session
        .mcp_servers()
        .into_iter()
        .map(|server| (server.name, server.extension_path))
        .collect();
    let own_path = dir.join("own.ts").to_string_lossy().into_owned();
    assert_eq!(
        servers,
        [
            ("shared".to_owned(), owner_path.clone()),
            ("my-docs".to_owned(), owner_path),
            ("own".to_owned(), own_path),
        ]
    );
}

/// A new session loads the extensions afresh, as pi's new runtime does: a
/// server name that a later extension registered in the previous session
/// does not keep an earlier extension from registering it.
#[tokio::test(flavor = "multi_thread")]
async fn a_new_session_checks_mcp_servers_against_its_own_registrations() {
    let dir = scratch("mcp-moved");
    let register = |when: &str| {
        format!(
            "let runs = 0;\nexport default function (pi) {{\n\truns++;\n\tif ({when}) pi.registerMcpServer(\"moved\", {{ command: \"python3\", enabled: false }});\n}}\n"
        )
    };
    let sources: Vec<_> = [("a.ts", "runs > 1"), ("b.ts", "runs === 1")]
        .iter()
        .map(|(file, when)| {
            let path = dir.join(file);
            std::fs::write(&path, register(when)).unwrap();
            cli_source(&path)
        })
        .collect();
    let js = ExtensionHost::load(&engine(), options(&dir), &sources)
        .await
        .unwrap();
    assert!(js.errors().is_empty(), "{:?}", js.errors());
    let owners = |session: &yapi_core::agent_session::AgentSession| -> Vec<(String, String)> {
        session
            .mcp_servers()
            .into_iter()
            .map(|server| (server.name, server.extension_path))
            .collect()
    };
    let path = |file: &str| dir.join(file).to_string_lossy().into_owned();
    let first = session(&Faux::new([]), &dir, js.for_session());
    first
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    assert_eq!(owners(&first), [("moved".to_owned(), path("b.ts"))]);
    first.shutdown().await;
    js.reload(&js.sources()).await;
    let second = session(&Faux::new([]), &dir, js.for_session());
    second
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    assert_eq!(owners(&second), [("moved".to_owned(), path("a.ts"))]);
}

/// Runtimes run their factories again for a new session one after another,
/// before it is built, and each checks its MCP servers against the runtimes
/// that ran before it for that session: a name a later runtime held in the
/// previous session goes to the earlier one, and a later runtime cannot take
/// a name the earlier one registered again.
#[tokio::test(flavor = "multi_thread")]
async fn runtimes_check_mcp_servers_against_this_run_for_a_new_session() {
    let dir = scratch("mcp-runs");
    let source = |calls: &str| {
        format!(
            "let runs = 0;\nexport default function (pi) {{\n\truns++;\n\tconst register = (name) => pi.registerMcpServer(name, {{ command: \"python3\", enabled: false }});\n{calls}}}\n"
        )
    };
    let engine = engine();
    let mut hosts = Vec::new();
    for (file, calls) in [
        (
            "first.ts",
            "\tregister(\"kept\");\n\tif (runs > 1) register(\"moved\");\n",
        ),
        (
            "second.ts",
            "\tif (runs === 1) register(\"moved\");\n\telse register(\"kept\");\n",
        ),
    ] {
        let path = dir.join(file);
        std::fs::write(&path, source(calls)).unwrap();
        let host = ExtensionHost::load(&engine, options(&dir), &[cli_source(&path)])
            .await
            .unwrap();
        assert!(host.errors().is_empty(), "{:?}", host.errors());
        hosts.push(host);
    }
    let path = |file: &str| dir.join(file).to_string_lossy().into_owned();
    let owners = |session: &yapi_core::agent_session::AgentSession| -> Vec<(String, String)> {
        session
            .mcp_servers()
            .into_iter()
            .map(|server| (server.name, server.extension_path))
            .collect()
    };
    let first = session(
        &Faux::new([]),
        &dir,
        hosts.iter().flat_map(ExtensionHost::for_session).collect(),
    );
    first
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    assert_eq!(
        owners(&first),
        [
            ("kept".to_owned(), path("first.ts")),
            ("moved".to_owned(), path("second.ts")),
        ]
    );
    first.shutdown().await;
    for host in &hosts {
        host.reload(&host.sources()).await;
    }
    assert!(hosts[0].errors().is_empty(), "{:?}", hosts[0].errors());
    assert_eq!(
        hosts[1].errors(),
        [yapi_ext::LoadError {
            path: dir.join("second.ts"),
            error: format!(
                "Failed to load extension: MCP server \"kept\" is already registered by extension \"{}\"",
                path("first.ts")
            ),
        }]
    );
    let second = session(
        &Faux::new([]),
        &dir,
        hosts.iter().flat_map(ExtensionHost::for_session).collect(),
    );
    second
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    assert_eq!(
        owners(&second),
        [
            ("kept".to_owned(), path("first.ts")),
            ("moved".to_owned(), path("first.ts")),
        ]
    );
}

/// The native fixture that reports the MCP server `at-load` with the
/// `mcp.servers` request as it loads.
fn mcp_at_load() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mcp-at-load.wasm")
}

/// The names of the MCP servers extensions registered for `session`.
fn mcp_server_names(session: &AgentSession) -> Vec<String> {
    session
        .mcp_servers()
        .into_iter()
        .map(|server| server.name)
        .collect()
}

/// MCP servers a native extension reports as it loads, before it is bound to
/// a session, reach the session as it starts, as pi's registrations made
/// while extensions load do: at startup, in a new session and after
/// `/reload`.
#[tokio::test(flavor = "multi_thread")]
async fn mcp_servers_a_native_extension_reports_as_it_loads_reach_its_session() {
    let dir = scratch("native-mcp-at-load");
    let host = ExtensionHost::load_native(&engine(), options(&dir), &cli_source(&mcp_at_load()))
        .await
        .unwrap();
    assert!(host.errors().is_empty(), "{:?}", host.errors());
    for replaced in [None, Some(Replacement::New), Some(Replacement::Reload)] {
        if replaced.is_some() {
            host.reload(&host.sources()).await;
        }
        let session = session(&Faux::new([]), &dir, host.for_session());
        session
            .bind_extensions(
                Arc::new(NoUi),
                Mode::Print,
                None,
                replaced.map(|reason| (reason, None)),
            )
            .await;
        assert_eq!(custom_entries(&session, "servers"), [json!("ok")]);
        assert_eq!(mcp_server_names(&session), ["at-load"]);
        session.shutdown().await;
    }
}

/// As pi checks a registration at the call, an MCP server that a native
/// extension reports as it loads is refused, with pi's message, when an
/// extension that loaded earlier registered its name. A report after the
/// session started is announced, as pi announces registrations made after
/// its extensions bound, even when it is the native extension's first.
#[tokio::test(flavor = "multi_thread")]
async fn a_native_extension_reports_mcp_servers_as_pi_registers_them() {
    let dir = scratch("native-mcp-clash");
    let owner = dir.join("owner.ts");
    std::fs::write(
        &owner,
        "export default function (pi) {\n\tpi.registerMcpServer(\"at-load\", { command: \"python3\", enabled: false });\n\tpi.on(\"mcp_servers_change\", (event) => pi.appendEntry(\"change\", event.servers.map((server) => server.name)));\n}\n",
    )
    .unwrap();
    let engine = engine();
    let js = ExtensionHost::load(&engine, options(&dir), &[cli_source(&owner)])
        .await
        .unwrap();
    let native = ExtensionHost::load_native(&engine, options(&dir), &cli_source(&mcp_at_load()))
        .await
        .unwrap();
    let extensions = js.for_session().into_iter().chain(native.for_session());
    let session = session(&Faux::new([]), &dir, extensions.collect());
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    assert_eq!(
        custom_entries(&session, "servers"),
        [json!(format!(
            "MCP server \"at-load\" is already registered by extension \"{}\"",
            owner.display()
        ))]
    );
    assert_eq!(mcp_server_names(&session), ["at-load"]);
    session.prompt("/servers later", Vec::new()).await.unwrap();
    assert_eq!(mcp_server_names(&session), ["at-load", "later"]);
    for _ in 0..500 {
        if !custom_entries(&session, "change").is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(
        custom_entries(&session, "change"),
        [json!(["at-load", "later"])]
    );
}

/// As pi loads the extensions for each runtime, a later session's factories
/// run before the session is built. An extension whose factory throws then
/// fails to load with pi's message and gives the session nothing, and it
/// runs again for the session after, even as its runtime's only extension.
#[tokio::test(flavor = "multi_thread")]
async fn a_factory_that_fails_in_a_later_session_fails_that_load() {
    let dir = scratch("reload-error");
    let path = dir.join("flaky.ts");
    std::fs::write(
        &path,
        "let runs = 0;\nexport default function (pi) {\n\tif (++runs > 1) throw new Error(\"loads only once\");\n\tpi.registerCommand(\"flaky\", { description: \"\", handler: async () => {} });\n}\n",
    )
    .unwrap();
    let js = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    assert!(js.errors().is_empty(), "{:?}", js.errors());
    let first = session(&Faux::new([]), &dir, js.for_session());
    first
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    assert_eq!(first.extension_commands().len(), 1);
    first.shutdown().await;
    let failed = [yapi_ext::LoadError {
        path: path.clone(),
        error: "Failed to load extension: loads only once".into(),
    }];
    for _ in 0..2 {
        js.reload(&js.sources()).await;
        assert_eq!(js.errors(), failed);
        let extensions = js.for_session();
        assert!(extensions.is_empty());
        let later = session(&Faux::new([]), &dir, extensions);
        later
            .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
            .await;
        assert!(later.extension_commands().is_empty());
        later.shutdown().await;
    }
}

/// A built-in tool that an extension registers again, as pi's
/// `built-in-tool-renderer.ts` does with bash, stops when its call is
/// aborted and reports its output as it runs.
#[tokio::test(flavor = "multi_thread")]
async fn wrapped_builtin_aborts() {
    let dir = scratch("wrapped-bash");
    let path = dir.join("bash.ts");
    std::fs::write(
        &path,
        r#"import { createBashToolDefinition } from "@earendil-works/pi-coding-agent";
export default function (pi) {
	pi.registerTool({ ...createBashToolDefinition(process.cwd()) });
}
"#,
    )
    .unwrap();
    let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    assert!(host.errors().is_empty(), "{:?}", host.errors());
    let session = session_with_tools(&Faux::new([]), &dir, host.for_session(), &["bash"]);
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    let tool = session
        .callable_tools()
        .into_iter()
        .find(|tool| tool.name() == "bash")
        .unwrap();
    let cancel = tokio_util::sync::CancellationToken::new();
    let (output, mut updates) = tokio::sync::mpsc::unbounded_channel();
    let run = tool.tool.execute(
        "call-1".into(),
        json!({"command": "echo started; sleep 20"}),
        cancel.clone(),
        Arc::new(move |partial| {
            let _ = output.send(partial);
        }),
    );
    let started = std::time::Instant::now();
    let abort = async {
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        cancel.cancel();
    };
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(15), async {
        tokio::join!(run, abort)
    })
    .await
    .expect("the aborted tool ends");
    assert!(result.unwrap_err().contains("Command aborted"));
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    let mut output = String::new();
    while let Ok(partial) = updates.try_recv() {
        output += &yapi_types::message::blocks_text(&partial.content, "|");
    }
    assert!(output.contains("started"), "{output}");
}

/// A tool's updates and the calls it makes through `ctx.executeTool()` report
/// in pi's order, from the task that runs the tool: an update goes out after
/// the start of a call the tool begins in the same step, and before its end
/// (#59).
#[tokio::test(flavor = "multi_thread")]
async fn tool_updates_and_nested_calls_report_in_pi_order() {
    let dir = scratch("tool-order");
    std::fs::write(dir.join("hello.txt"), "hi\n").unwrap();
    let path = dir.join("relay.ts");
    std::fs::write(
        &path,
        r#"export default function (pi) {
	pi.registerTool({
		name: "relay", label: "Relay", description: "Relays", parameters: { type: "object", properties: {} },
		async execute(_id, _params, _signal, onUpdate, ctx) {
			onUpdate({ content: [{ type: "text", text: "before" }] });
			await ctx.executeTool("read", { path: "hello.txt" });
			onUpdate({ content: [{ type: "text", text: "between" }] });
			await ctx.executeTool("read", { path: "hello.txt" });
			onUpdate({ content: [{ type: "text", text: "after" }] });
			return { content: [{ type: "text", text: "done" }] };
		},
	});
}
"#,
    )
    .unwrap();
    let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    let faux = Faux::new([
        Response::tool_call("call-1", "relay", json!({})),
        Response::text("done"),
    ]);
    let session = session(&faux, &dir, host.for_session());
    session
        .bind_extensions(Arc::new(NoUi), Mode::Print, None, None)
        .await;
    let log = Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = log.clone();
    session.subscribe(Box::new(move |event| {
        let line = match event {
            AgentEvent::ToolExecutionStart { tool_call_id, .. } => format!("start {tool_call_id}"),
            AgentEvent::ToolExecutionUpdate {
                tool_call_id,
                partial_result,
                ..
            } => format!(
                "update {tool_call_id} {}",
                yapi_types::message::blocks_text(&partial_result.content, "|")
            ),
            AgentEvent::ToolExecutionEnd { tool_call_id, .. } => format!("end {tool_call_id}"),
            _ => return,
        };
        sink.lock().unwrap().push((tokio::task::try_id(), line));
    }));
    session.prompt("go", Vec::new()).await.unwrap();

    let log = log.lock().unwrap();
    let lines: Vec<&str> = log.iter().map(|(_, line)| line.as_str()).collect();
    assert_eq!(
        lines,
        [
            "start call-1",
            "start call-1/1",
            "update call-1 before",
            "end call-1/1",
            "start call-1/2",
            "update call-1 between",
            "end call-1/2",
            "update call-1 after",
            "end call-1",
        ]
    );
    assert!(log.iter().all(|(task, _)| *task == log[0].0), "{log:?}");
}

/// A terminal that keeps the editor an extension puts in place, the text
/// it last reported, its last component widget, the last runtime's input
/// listeners and how often it was asked to render.
#[derive(Default)]
struct Screen {
    editor: std::sync::Mutex<Option<RemoteComponent>>,
    text: std::sync::Mutex<String>,
    widget: std::sync::Mutex<Option<RemoteComponent>>,
    listeners: std::sync::Mutex<Option<Arc<dyn ComponentHost>>>,
    renders: std::sync::atomic::AtomicUsize,
    terminal: tokio::sync::watch::Sender<TerminalState>,
}

impl ExtensionUi for Screen {
    fn has_ui(&self) -> bool {
        true
    }

    fn notify(&self, _message: &str, _kind: NotifyKind) {}

    fn shows_components(&self) -> bool {
        true
    }

    fn keybindings(&self) -> Value {
        json!({"bindings": {"tui.editor.cursorLeft": ["left"]}})
    }

    fn set_editor(&self, editor: Option<RemoteComponent>, _embeds_status: bool) {
        *self.editor.lock().unwrap() = editor;
    }

    fn editor_changed(&self, text: &str) {
        text.clone_into(&mut self.text.lock().unwrap());
    }

    fn set_widget(&self, _key: &str, widget: Option<Widget>, _placement: Option<Placement>) {
        if let Some(Widget::Component(component)) = widget {
            *self.widget.lock().unwrap() = Some(component);
        }
    }

    fn request_render(&self) {
        self.renders
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    fn set_terminal_input(&self, _runtime: u64, listeners: Option<Arc<dyn ComponentHost>>) {
        *self.listeners.lock().unwrap() = listeners;
    }

    fn terminal(&self) -> Option<tokio::sync::watch::Receiver<TerminalState>> {
        Some(self.terminal.subscribe())
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn pastes_go_in_at_the_cursor_of_an_extension_editor() {
    let dir = scratch("editor-paste");
    let path = dir.join("editor.ts");
    std::fs::write(
        &path,
        r#"
import { CustomEditor, type ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	pi.on("session_start", (_event, ctx) => {
		ctx.ui.setEditorComponent((tui, theme, keybindings) => new CustomEditor(tui, theme, keybindings));
	});
}
"#,
    )
    .unwrap();
    let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    let session = session(&Faux::new([]), &dir, host.for_session());
    let screen = Arc::new(Screen::default());
    session
        .bind_extensions(screen.clone(), Mode::Tui, None, None)
        .await;
    let editor = screen.editor.lock().unwrap().clone().unwrap();
    // The text and rows once the editor has taken the insertion.
    let insert = async |text: &str, apart: bool| {
        editor.editor_op(&json!({"op": "insertTextAtCursor", "text": text, "apart": apart}));
        let rows = editor.render(80).await;
        (screen.text.lock().unwrap().clone(), rows)
    };

    // A saved image's path goes in as it is. A paste would put a space before it.
    editor.input("see");
    let (text, _) = insert("/tmp/yapi-clipboard-1.png", false).await;
    assert_eq!(text, "see/tmp/yapi-clipboard-1.png");
    // Long text goes in whole, without a paste's marker.
    editor.editor_op(&json!({"op": "setText", "text": ""}));
    let long: Vec<String> = (1..=11).map(|line| line.to_string()).collect();
    let (text, rows) = insert(&long.join("\n"), false).await;
    assert_eq!(text, long.join("\n"));
    assert!(!rows.iter().any(|row| row.contains("[paste")), "{rows:?}");
    // Copied files' paths are set apart from the words at the editor's cursor.
    editor.editor_op(&json!({"op": "setText", "text": "ab"}));
    editor.input("\x1b[D");
    let (text, _) = insert("/tmp/photo.png", true).await;
    assert_eq!(text, "a /tmp/photo.png b");
}

#[tokio::test(flavor = "multi_thread")]
async fn extensions_see_the_terminal_size_as_it_changes() {
    let dir = scratch("terminal-size");
    let path = dir.join("size.ts");
    std::fs::write(
        &path,
        r#"
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
	let resizes = 0;
	process.stdout.on("resize", () => resizes++);
	pi.on("session_start", (_event, ctx) => {
		ctx.ui.setWidget("size", (tui) => ({
			render: () => [
				`tui ${tui.terminal.columns}x${tui.terminal.rows}`,
				`stdout ${process.stdout.columns}x${process.stdout.rows} tty=${process.stdout.isTTY} resizes=${resizes}`,
			],
			invalidate() {},
		}));
	});
}
"#,
    )
    .unwrap();
    let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    let session = session(&Faux::new([]), &dir, host.for_session());
    let screen = Arc::new(Screen::default());
    screen.terminal.send_modify(|state| state.size = (120, 30));
    session
        .bind_extensions(screen.clone(), Mode::Tui, None, None)
        .await;
    let widget = screen.widget.lock().unwrap().clone().unwrap();
    assert_eq!(
        widget.render(80).await,
        ["tui 120x30", "stdout 120x30 tty=true resizes=0"]
    );

    // The runtime hears of the resize after the screen does, without the
    // screen waiting for it, and asks for a render.
    let renders = screen.renders.load(std::sync::atomic::Ordering::SeqCst);
    screen.terminal.send_modify(|state| state.size = (100, 40));
    let resized = ["tui 100x40", "stdout 100x40 tty=true resizes=1"];
    let mut rows = Vec::new();
    for _ in 0..500 {
        rows = widget.render(80).await;
        if rows == resized {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    assert_eq!(rows, resized);
    assert!(screen.renders.load(std::sync::atomic::Ordering::SeqCst) > renders);
}

#[tokio::test(flavor = "multi_thread")]
async fn extensions_see_whether_the_prompt_editor_has_the_keyboard() {
    let dir = scratch("editor-focus");
    let path = dir.join("focus.ts");
    std::fs::write(
        &path,
        r#"
import { CustomEditor, type ExtensionAPI } from "@earendil-works/pi-coding-agent";
import { Editor } from "@mariozechner/pi-tui";

export default function (pi: ExtensionAPI) {
	let editor: CustomEditor | undefined;
	pi.on("session_start", (_event, ctx) => {
		ctx.ui.setWidget("focus", (tui) => ({
			render: () => {
				const focused = (tui as any).focusedComponent;
				return [`editor ${focused instanceof Editor} own ${!!editor && focused === editor}`];
			},
			invalidate() {},
		}));
	});
	pi.registerCommand("own", {
		description: "Put an editor in place",
		handler: async (_args, ctx) => {
			ctx.ui.setEditorComponent((tui, theme, keybindings) => (editor = new CustomEditor(tui, theme, keybindings)));
		},
	});
}
"#,
    )
    .unwrap();
    let js = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    let wasm = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/input-hooks.wasm");
    let native = ExtensionHost::load_native(&engine(), options(&dir), &cli_source(&wasm))
        .await
        .unwrap();
    let mut extensions = js.for_session();
    extensions.extend(native.for_session());
    let session = session(&Faux::new([]), &dir, extensions);
    let screen = Arc::new(Screen::default());
    screen.terminal.send_replace(TerminalState {
        size: (80, 24),
        editor_focused: true,
    });
    session
        .bind_extensions(screen.clone(), Mode::Tui, None, None)
        .await;
    let widget = screen.widget.lock().unwrap().clone().unwrap();
    let listeners = screen.listeners.lock().unwrap().clone().unwrap();
    // The JS runtime hears of a change after the screen, which never waits.
    let shows = async |expected: &str| {
        let mut rows = Vec::new();
        for _ in 0..500 {
            rows = widget.render(80).await;
            if rows == [expected] {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        assert_eq!(rows, [expected]);
    };
    // The native listener takes `?` from the empty prompt only while the
    // prompt editor has the keyboard; a dialog gets it.
    let help = async || listeners.terminal_input(vec!["?".into()]).await;
    let focus = |editor_focused: bool| {
        screen
            .terminal
            .send_modify(|state| state.editor_focused = editor_focused);
    };

    shows("editor true own false").await;
    assert!(help().await.is_empty());
    focus(false);
    assert_eq!(help().await, ["?"]);
    shows("editor false own false").await;
    focus(true);
    assert!(help().await.is_empty());
    shows("editor true own false").await;

    // An editor in the built-in one's place is the focused component.
    session.prompt("/own", Vec::new()).await.unwrap();
    shows("editor true own true").await;
}

/// A UI that counts how often a runtime restarted.
#[derive(Default)]
struct Restarts(std::sync::atomic::AtomicUsize);

impl ExtensionUi for Restarts {
    fn has_ui(&self) -> bool {
        true
    }

    fn notify(&self, _message: &str, _kind: NotifyKind) {}

    fn drop_components(&self, _runtime: u64) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }
}

/// A drawing request that stops the runtime is not sent to it again, so
/// redrawing the transcript after the restart cannot stop it over and over.
/// The restarted runtime draws everything else.
#[tokio::test(flavor = "multi_thread")]
async fn a_drawing_that_stops_the_runtime_is_not_asked_again() {
    let dir = scratch("drawing-crash");
    let path = dir.join("draw.ts");
    std::fs::write(
        &path,
        r#"
const overflow = () => overflow();
export default function (pi) {
	pi.yapi.registerCodeRenderer((code) => (code === "crash" ? overflow() : [code.toUpperCase()]));
	pi.registerMarkdownTransformer((text) => (text === "crash" ? overflow() : text.toUpperCase()));
}
"#,
    )
    .unwrap();
    let host = ExtensionHost::load(&engine(), options(&dir), &[cli_source(&path)])
        .await
        .unwrap();
    let session = session(&Faux::new([]), &dir, host.for_session());
    let ui = Arc::new(Restarts::default());
    session
        .bind_extensions(ui.clone(), Mode::Tui, None, None)
        .await;
    let renderer = &session.code_renderers()[0];
    let transformer = &session.markdown_transformers()[0];
    let code = |code: &str| json!({"code": code, "lang": null, "width": 40});
    let markdown = |text: &str| json!({"markdown": text, "context": {"messageType": "assistant"}});
    for _ in 0..2 {
        let error = renderer.render_code(&code("crash")).await.unwrap_err();
        assert!(error.starts_with("extension runtime stopped: "), "{error}");
        let error = transformer
            .transform_markdown(&markdown("crash"))
            .await
            .unwrap_err();
        assert!(error.starts_with("extension runtime stopped: "), "{error}");
    }
    assert_eq!(
        renderer.render_code(&code("fine")).await.unwrap(),
        Some(vec!["FINE".to_owned()])
    );
    assert_eq!(
        transformer
            .transform_markdown(&markdown("fine"))
            .await
            .unwrap()
            .as_deref(),
        Some("FINE")
    );
    assert_eq!(ui.0.load(std::sync::atomic::Ordering::SeqCst), 2);
}
