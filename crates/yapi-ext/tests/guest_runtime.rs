//! Regression tests for the JS runtime component's Node shims and dispatch.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers; a panic is a test failure"
)]

mod common;

use std::sync::Arc;

use serde_json::{Value, json};
use yapi_ext::{Instance, NoBridge, Options};

/// Loads extension `main` (TypeScript) in a fresh instance and returns it
/// with what loading reported.
async fn load(name: &str, main: &str) -> (Instance, Value) {
    let dir = common::scratch(&format!("guest-{name}"));
    let path = dir.join("main.ts");
    std::fs::write(&path, main).unwrap();
    let instance = Instance::start(
        &common::engine(),
        Options::new(dir.clone()),
        Arc::new(NoBridge),
    )
    .await
    .unwrap();
    let loaded = instance
        .call(
            "load",
            &json!({"cwd": dir, "extensions": [{"id": 1, "path": path}]}),
        )
        .await
        .unwrap();
    (instance, loaded["extensions"][0].clone())
}

/// The description of the `probe` command extension `main` registers.
async fn probe(name: &str, main: &str) -> String {
    let (_instance, extension) = load(name, main).await;
    assert_eq!(extension.get("error"), None, "{extension}");
    extension["commands"][0]["description"]
        .as_str()
        .unwrap()
        .to_owned()
}

/// Every Node built-in imports by its bare name, as in Node, except the ones
/// Node loads only with the `node:` prefix.
#[tokio::test(flavor = "multi_thread")]
async fn imports_bare_node_builtins() {
    let main = r#"
import qs from "querystring";
import { ReadableStream } from "stream/web";
import { text } from "stream/consumers";
import { isPromise } from "util/types";
import { lookup } from "dns/promises";
import win32 from "path/win32";
import inspector from "inspector/promises";
import constants from "constants";
import sys from "sys";
import wasi from "wasi";
import nodeConsole from "console";
import { createRequire, isBuiltin } from "node:module";
export default async function (pi) {
	const require = createRequire(import.meta.url);
	const failure = (load) => {
		try {
			load();
			return "loaded";
		} catch (error) {
			return error.message.split("\n")[0];
		}
	};
	const sqlite = await import("sqlite").then(() => "loaded", (error) => error.message.split("\n")[0]);
	pi.registerCommand("probe", {
		description: [
			qs.stringify({ a: [1, 2] }),
			typeof ReadableStream,
			typeof text,
			isPromise(Promise.resolve()),
			typeof lookup,
			typeof win32.join,
			typeof inspector.Session,
			constants.F_OK,
			typeof sys.format,
			typeof wasi.WASI,
			typeof nodeConsole.log,
			isBuiltin("querystring"),
			isBuiltin("test"),
			isBuiltin("node:test"),
			failure(() => require("node:test")),
			failure(() => require("test")),
			sqlite,
		].join("|"),
		handler: async () => {},
	});
}
"#;
    assert_eq!(
        probe("builtins", main).await,
        "a=1&a=2|function|function|true|function|function|function|0|function|function|function|true|false|true|loaded|Cannot find module 'test'|Cannot find module 'sqlite'"
    );
}

/// UTF-8 split across chunks decodes whole, inputs too large for a call's
/// arguments convert, and base64 and lone surrogates follow Node.
#[tokio::test(flavor = "multi_thread")]
async fn decodes_streams_large_inputs() {
    let main = r#"
import { StringDecoder } from "node:string_decoder";
export default function (pi) {
	const smile = new TextEncoder().encode("😀");
	const decoder = new TextDecoder();
	const streamed = decoder.decode(smile.subarray(0, 2), { stream: true }) + decoder.decode(smile.subarray(2, 3), { stream: true }) + decoder.decode(smile.subarray(3));
	const cut = new TextDecoder();
	const flushed = cut.decode(smile.subarray(0, 2), { stream: true }) + cut.decode();
	const utf8 = new StringDecoder("utf8");
	const euro = utf8.write(Buffer.from([0xe2, 0x82])) + utf8.write(Buffer.from([0xac]));
	const large = "\xe9".repeat(200000);
	pi.registerCommand("probe", {
		description: JSON.stringify([
			streamed,
			flushed,
			euro,
			new StringDecoder("hex").write(Buffer.from([0xc3])),
			Buffer.from(large, "latin1").toString("latin1") === large,
			atob(btoa(large)) === large,
			new TextDecoder("latin1").decode(Buffer.alloc(200000, 0xe9)) === large,
			[...Buffer.from("a\ud800b")],
			new TextDecoder().decode(Buffer.from([0xef, 0xbb, 0xbf, 0x41])),
			Buffer.from([0xef, 0xbb, 0xbf, 0x41]).toString(),
			new TextDecoder().decode(new Uint8Array([0xed, 0xa0, 0x80])),
			[...Buffer.from("QQ==QQ==", "base64")],
			[...Buffer.from(" Q Q\nQ", "base64")],
			Buffer.from([255, 239]).toString("base64url"),
			[...Buffer.from("_-8", "base64")],
		]),
		handler: async () => {},
	});
}
"#;
    assert_eq!(
        probe("encodings", main).await,
        r#"["😀","�","€","c3",true,true,true,[97,239,191,189,98],"A","﻿A","���",[65],[65,4],"_-8",[255,239]]"#
    );
}

/// `process` is an EventEmitter: `once` listeners run once.
#[tokio::test(flavor = "multi_thread")]
async fn process_emits_events() {
    let main = r#"
import EventEmitter from "node:events";
export default function (pi) {
	let calls = 0;
	const listener = () => calls++;
	const chained = process.once("probe", listener) === process;
	const first = process.emit("probe");
	const second = process.emit("probe");
	const left = process.listenerCount("probe");
	process.on("probe", listener);
	pi.registerCommand("probe", {
		description: [process instanceof EventEmitter, chained, first, second, calls, left, process.listenerCount("probe")].join(","),
		handler: async () => {},
	});
}
"#;
    assert_eq!(probe("process", main).await, "true,true,true,false,1,0,1");
}

/// `removeListener` drops only the most recent registration of a listener,
/// as Node does, and finds `once` listeners by the function they wrap.
#[tokio::test(flavor = "multi_thread")]
async fn remove_listener_drops_one() {
    let main = r#"
import { EventEmitter } from "node:events";
export default function (pi) {
	const emitter = new EventEmitter();
	const calls = [];
	const a = () => calls.push("a");
	const b = () => calls.push("b");
	emitter.on("x", a).on("x", b).on("x", a);
	emitter.removeListener("x", a);
	emitter.emit("x");
	const once = () => calls.push("once");
	emitter.once("y", once).once("y", once);
	emitter.off("y", once);
	emitter.emit("y");
	emitter.emit("y");
	pi.registerCommand("probe", {
		description: [calls.join(""), emitter.listenerCount("x"), emitter.listenerCount("y")].join(","),
		handler: async () => {},
	});
}
"#;
    assert_eq!(probe("remove-listener", main).await, "abonce,2,0");
}

/// The host cannot have the runtime evaluate source text.
#[tokio::test(flavor = "multi_thread")]
async fn evaluates_no_source_text() {
    let (instance, _) = load("eval", "export default function () {}\n").await;
    let error = instance
        .call("eval", &json!({"source": "globalThis.evaluated = true"}))
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Unknown dispatch kind: eval");
}

/// pi exports yapi does not provide link, can be extended, and throw when
/// called or constructed.
#[tokio::test(flavor = "multi_thread")]
async fn used_stubs_throw() {
    let main = r#"
import { AgentSession, createAgentSession } from "@earendil-works/pi-coding-agent";
import { PiMessagesResponseError } from "@earendil-works/pi-ai/providers/pi-messages";
class Session extends AgentSession {}
const failure = (run) => {
	try {
		run();
		return "ran";
	} catch (error) {
		return `${error.code}: ${error.message}`;
	}
};
export default function (pi) {
	pi.registerCommand("probe", {
		description: [
			failure(() => new Session()),
			failure(() => createAgentSession()),
			failure(() => new PiMessagesResponseError()),
			AgentSession.name,
		].join("|"),
		handler: async () => {},
	});
}
"#;
    assert_eq!(
        probe("stubs", main).await,
        "ERR_NOT_SUPPORTED: AgentSession from @earendil-works/pi-coding-agent is not available in yapi extensions|\
         ERR_NOT_SUPPORTED: createAgentSession from @earendil-works/pi-coding-agent is not available in yapi extensions|\
         ERR_NOT_SUPPORTED: PiMessagesResponseError from @earendil-works/pi-ai is not available in yapi extensions|\
         AgentSession"
    );
}

/// pi's exports that run inside the extension behave as in pi: session
/// entries, compaction, settings saved to their file, skills, diffs,
/// arguments, model scopes, images, components, pi-agent-core's agent loop
/// over the extension's own stream and tools, and built-in tools over custom
/// operations. The expected results were recorded from pi's installed
/// packages with Node.
#[tokio::test(flavor = "multi_thread")]
async fn exports_match_pi() {
    let main = r#"
import { Agent } from "@earendil-works/pi-agent-core";
import { createAssistantMessageEventStream } from "@earendil-works/pi-ai";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Type } from "typebox";
import { BranchSummaryMessageComponent, createBashTool, createEditTool, createReadTool, buildSessionContext, convertToPng, CredentialSynchronizationError, estimateTokens, findCutPoint, formatDimensionNote, formatSkillsForPrompt, generateDiffString, generateUnifiedPatch, initTheme, parseArgs, parseSessionEntries, parseSkillBlock, renderDiff, resizeImage, resolveCliModel, resolveModelScopeWithDiagnostics, SessionManager, SettingsManager, shouldCompact, truncateHead } from "@earendil-works/pi-coding-agent";

const PNG = "iVBORw0KGgoAAAANSUhEUgAAAAQAAAAECAYAAACp8Z5+AAAAEklEQVR4nGP4z8DwHxkzkC4AADxAH+HggXe0AAAAAElFTkSuQmCC";
const plain = (lines) => lines.map((line) => line.replace(/\x1b\[[0-9;]*m/g, "").trimEnd());
const usage = { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } };
/** An agent whose stream calls `echo` once, then answers with the tool's output. */
async function runAgent() {
	const model = { id: "faux", name: "Faux", api: "faux", provider: "faux", baseUrl: "", reasoning: false, input: ["text"], cost: usage.cost, contextWindow: 1000, maxTokens: 100 };
	const echo = {
		name: "echo", label: "Echo", description: "Echoes", parameters: Type.Object({ text: Type.String() }),
		execute: async (_id, params) => ({ content: [{ type: "text", text: params.text.toUpperCase() }], details: {} }),
	};
	const streamFn = (model, context) => {
		const stream = createAssistantMessageEventStream();
		const last = context.messages.at(-1);
		const content = last.role === "toolResult" ? [{ type: "text", text: `echo said ${last.content[0].text}` }] : [{ type: "toolCall", id: "t1", name: "echo", arguments: { text: "hi" } }];
		const message = { role: "assistant", content, api: model.api, provider: model.provider, model: model.id, usage, stopReason: last.role === "toolResult" ? "stop" : "toolUse", timestamp: 1 };
		setTimeout(() => {
			stream.push({ type: "start", partial: message });
			stream.push({ type: "done", reason: message.stopReason, message });
		});
		return stream;
	};
	const agent = new Agent({ initialState: { model, systemPrompt: "Be brief", tools: [echo] }, streamFn });
	const events = [];
	agent.subscribe((event) => events.push(event.type));
	await agent.prompt("go");
	return { events, messages: agent.state.messages.map(({ role, content, toolName, isError }) => ({ role, content, toolName, isError })) };
}
/** pi's read, edit and bash tools over operations on an in-memory "remote" filesystem. */
async function runOperations() {
	const files = new Map([["/remote/a.txt", "one\ntwo\nthree\n"]]);
	const fileOps = {
		readFile: async (path) => Buffer.from(files.get(path)),
		writeFile: async (path, content) => void files.set(path, content),
		access: async (path) => {
			if (!files.has(path)) throw new Error(`ENOENT: ${path}`);
		},
		detectImageMimeType: async () => null,
	};
	const bashOps = {
		exec: async (command, cwd, { onData }) => {
			onData(Buffer.from(`ran ${command} in ${cwd}\n`));
			return { exitCode: 2 };
		},
	};
	const text = (result) => result.content.map((block) => block.text).join("");
	const read = await createReadTool("/remote", { operations: fileOps }).execute("r", { path: "a.txt", offset: 2 });
	const edit = await createEditTool("/remote", { operations: fileOps }).execute("e", { path: "a.txt", edits: [{ oldText: "two", newText: "TWO" }] });
	const bash = await createBashTool("/remote", { operations: bashOps }).execute("b", { command: "make" }).then(text, (error) => `error: ${error.message}`);
	return { read: text(read), edit: [text(edit), edit.details?.diff], file: files.get("/remote/a.txt"), bash };
}
/** pi's settings file after a setting changes: its keys keep their order and the user's own stay. */
async function saveSettings() {
	const dir = mkdtempSync(join(tmpdir(), "settings-"));
	writeFileSync(join(dir, "settings.json"), '{\n  "theme": "dark",\n  "httpIdleTimeoutMs": 1000,\n  "custom": [1, 2]\n}\n');
	const settings = SettingsManager.create(dir, dir);
	settings.setHttpIdleTimeoutMs(60_000);
	await settings.flush();
	return [settings.getHttpIdleTimeoutMs(), readFileSync(join(dir, "settings.json"), "utf8")];
}
export async function probe() {
	// pi renders only after `initTheme`; yapi draws with the session's theme.
	initTheme("dark");
	const at = "2026-01-01T00:00:00.000Z";
	const message = (id, parentId, body) => JSON.stringify({ type: "message", id, parentId, timestamp: at, message: { timestamp: 1, ...body } });
	const entries = parseSessionEntries([
		JSON.stringify({ type: "session", version: 3, id: "s", timestamp: at, cwd: "/w" }),
		message("a", null, { role: "user", content: "hello there" }),
		message("b", "a", { role: "assistant", content: [{ type: "text", text: "hi" }], api: "x", provider: "p", model: "m", usage: { input: 1, output: 2, cacheRead: 0, cacheWrite: 0, totalTokens: 3, cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 } }, stopReason: "stop" }),
		JSON.stringify({ type: "thinking_level_change", id: "c", parentId: "b", timestamp: at, thinkingLevel: "high" }),
	].join("\n"));
	const models = [
		{ id: "claude-opus-4-5", name: "Claude Opus 4.5", provider: "anthropic", reasoning: true },
		{ id: "gpt-5", name: "GPT-5", provider: "openai", reasoning: true },
	];
	const runtime = { getModels: () => models, getAvailable: async () => models };
	const resized = await resizeImage(Buffer.from(PNG, "base64"), "image/png", { maxWidth: 2, maxHeight: 2 });
	const sessions = SessionManager.inMemory("/w");
	sessions.appendMessage({ role: "user", content: "hi", timestamp: 1 });
	sessions.appendThinkingLevelChange("low");
	const settings = SettingsManager.inMemory({ defaultThinkingLevel: "high", compaction: { reserveTokens: 1 } });
	return {
		context: buildSessionContext(entries.slice(1)),
		sessionManager: [sessions.getEntries().map((entry) => entry.type), sessions.buildSessionContext()],
		settingsManager: [settings.getDefaultThinkingLevel(), settings.getCompactionSettings(), await saveSettings()],
		tokens: estimateTokens(entries[2].message),
		shouldCompact: [shouldCompact(90, 100, { enabled: true, reserveTokens: 20, keepRecentTokens: 5 }), shouldCompact(50, 100, { enabled: true, reserveTokens: 20, keepRecentTokens: 5 })],
		cutPoint: findCutPoint(entries.slice(1), 0, 3, 1),
		skills: formatSkillsForPrompt([{ name: "pdf", description: "Read <PDF> files", filePath: "/s/pdf/SKILL.md", baseDir: "/s/pdf", disableModelInvocation: false }]),
		diff: generateDiffString("a\nb\nc\nd\n", "a\nB\nc\nd\ne\n"),
		patch: generateUnifiedPatch("f.txt", "one\ntwo\n", "one\n2\n"),
		args: parseArgs(["--model", "anthropic/claude-opus-4-5:high", "-p", "--no-tools", "hello", "@file.md"]),
		skillBlock: parseSkillBlock('<skill name="pdf" location="/s/pdf/SKILL.md">\nBody\n</skill>\n\n  Do it  '),
		truncated: truncateHead("one\ntwo\nthree\n", { maxLines: 2 }),
		scope: await resolveModelScopeWithDiagnostics(["anthropic/*:low", "nope"], runtime),
		cli: [resolveCliModel({ cliModel: "opus", modelRuntime: runtime }), resolveCliModel({ cliProvider: "x", cliModel: "y", modelRuntime: runtime })],
		image: resized && { mimeType: resized.mimeType, width: resized.width, height: resized.height, wasResized: resized.wasResized, note: formatDimensionNote(resized) },
		png: (await convertToPng(PNG, "image/png")).mimeType,
		error: new CredentialSynchronizationError("anthropic", "refresh", { type: "oauth" }).message,
		summary: plain(new BranchSummaryMessageComponent({ role: "branchSummary", summary: "Did **work**", fromId: "x", timestamp: 1 }).render(30)),
		agent: await runAgent(),
		operations: await runOperations(),
		renderDiff: plain(renderDiff("-1 old\n+1 new\n 2 same").split("\n")),
	};
}
export default async function (pi) {
	pi.registerCommand("probe", { description: JSON.stringify(await probe()), handler: async () => {} });
}
"#;
    let result: Value = serde_json::from_str(&probe("exports", main).await).unwrap();
    let expected = json!({
        "context": {"messages": [{"timestamp": 1, "role": "user", "content": "hello there"}, {"timestamp": 1, "role": "assistant", "content": [{"type": "text", "text": "hi"}], "api": "x", "provider": "p", "model": "m", "usage": {"input": 1, "output": 2, "cacheRead": 0, "cacheWrite": 0, "totalTokens": 3, "cost": {"input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0, "total": 0}}, "stopReason": "stop"}], "thinkingLevel": "high", "model": {"provider": "p", "modelId": "m"}},
        "sessionManager": [["message", "thinking_level_change"], {"messages": [{"role": "user", "content": "hi", "timestamp": 1}], "thinkingLevel": "low", "model": null}],
        "settingsManager": ["high", {"enabled": true, "reserveTokens": 1, "keepRecentTokens": 20000}, [60000, "{\n  \"theme\": \"dark\",\n  \"httpIdleTimeoutMs\": 60000,\n  \"custom\": [\n    1,\n    2\n  ]\n}"]],
        "tokens": 1,
        "shouldCompact": [true, false],
        "cutPoint": {"firstKeptEntryIndex": 1, "turnStartIndex": 0, "isSplitTurn": true},
        "skills": "\n\nThe following skills provide specialized instructions for specific tasks.\nUse the read tool to load a skill's file when the task matches its description.\nWhen a skill file references a relative path, resolve it against the skill directory (parent of SKILL.md / dirname of the path) and use that absolute path in tool commands.\n\n<available_skills>\n  <skill>\n    <name>pdf</name>\n    <description>Read &lt;PDF&gt; files</description>\n    <location>/s/pdf/SKILL.md</location>\n  </skill>\n</available_skills>",
        "diff": {"diff": " 1 a\n-2 b\n+2 B\n 3 c\n 4 d\n+5 e", "firstChangedLine": 2},
        "patch": "--- f.txt\n+++ f.txt\n@@ -1,2 +1,2 @@\n one\n-two\n+2\n",
        "args": {"messages": ["hello"], "fileArgs": ["file.md"], "unknownFlags": {}, "diagnostics": [], "model": "anthropic/claude-opus-4-5:high", "print": true, "noTools": true},
        "skillBlock": {"name": "pdf", "location": "/s/pdf/SKILL.md", "content": "Body", "userMessage": "Do it"},
        "truncated": {"content": "one\ntwo", "truncated": true, "truncatedBy": "lines", "totalLines": 3, "totalBytes": 14, "outputLines": 2, "outputBytes": 7, "lastLinePartial": false, "firstLineExceedsLimit": false, "maxLines": 2, "maxBytes": 51200},
        "scope": {"scopedModels": [{"model": {"id": "claude-opus-4-5", "name": "Claude Opus 4.5", "provider": "anthropic", "reasoning": true}, "thinkingLevel": "low"}], "diagnostics": [{"type": "warning", "code": "no-match", "message": "No models match pattern \"nope\"", "pattern": "nope"}]},
        "cli": [{"model": {"id": "claude-opus-4-5", "name": "Claude Opus 4.5", "provider": "anthropic", "reasoning": true}}, {"error": "Unknown provider \"x\". Use --list-models to see available providers/models."}],
        "image": {"mimeType": "image/png", "width": 2, "height": 2, "wasResized": true, "note": "[Image: original 4x4, displayed at 2x2. Multiply coordinates by 2.00 to map to original image.]"},
        "png": "image/png",
        "error": "Credential refresh committed for anthropic, but local synchronization failed",
        "summary": ["", " [branch]", "", " Branch summary ( to expand)", ""],
        "agent": {"events": ["agent_start", "turn_start", "message_start", "message_end", "message_start", "message_end", "tool_execution_start", "tool_execution_end", "message_start", "message_end", "turn_end", "turn_start", "message_start", "message_end", "turn_end", "agent_end"], "messages": [{"role": "system", "content": "Be brief"}, {"role": "user", "content": [{"type": "text", "text": "go"}]}, {"role": "assistant", "content": [{"type": "toolCall", "id": "t1", "name": "echo", "arguments": {"text": "hi"}}]}, {"role": "toolResult", "content": [{"type": "text", "text": "HI"}], "toolName": "echo", "isError": false}, {"role": "assistant", "content": [{"type": "text", "text": "echo said HI"}]}]},
        "operations": {"read": "two\nthree\n", "edit": ["Successfully replaced 1 block(s) in a.txt.", " 1 one\n-2 two\n+2 TWO\n 3 three"], "file": "one\nTWO\nthree\n", "bash": "ran make in /remote\n\n\nCommand exited with code 2"},
        "renderDiff": ["-1 old", "+1 new", " 2 same"]
    });
    assert_eq!(result, expected);
}

/// The API of an extension that failed to load throws, as in pi.
#[tokio::test(flavor = "multi_thread")]
async fn failed_extension_api_throws() {
    let dir = common::scratch("guest-failed");
    let broken = dir.join("broken.ts");
    std::fs::write(
        &broken,
        "export default function (pi) {\n\tglobalThis.stale = pi;\n\tthrow new Error(\"broken\");\n}\n",
    )
    .unwrap();
    let probe = dir.join("probe.ts");
    std::fs::write(
        &probe,
        r#"
const failure = (run) => {
	try {
		run();
		return "ran";
	} catch (error) {
		return error.message;
	}
};
export default function (pi) {
	const { stale } = globalThis;
	pi.registerCommand("probe", {
		description: [failure(() => stale.getFlag("x")), failure(() => stale.events.on("x", () => {}))].join("|"),
		handler: async () => {},
	});
}
"#,
    )
    .unwrap();
    let instance = Instance::start(
        &common::engine(),
        Options::new(dir.clone()),
        Arc::new(NoBridge),
    )
    .await
    .unwrap();
    let loaded = instance
        .call(
            "load",
            &json!({"cwd": dir, "extensions": [{"id": 1, "path": broken}, {"id": 2, "path": probe}]}),
        )
        .await
        .unwrap();
    assert_eq!(
        loaded["extensions"][0]["error"],
        "Failed to load extension: broken"
    );
    let inactive = format!(
        "Extension \"{}\" failed to load and its API is no longer active.",
        broken.display()
    );
    assert_eq!(
        loaded["extensions"][1]["commands"][0]["description"],
        format!("{inactive}|{inactive}")
    );
}

/// Descriptor reads and writes reach their range of the file: chunked reads,
/// writes inside and past the end, and truncation.
#[tokio::test(flavor = "multi_thread")]
async fn descriptor_range_io() {
    let main = r#"
import fs from "node:fs";
import path from "node:path";
export default function (pi) {
	const file = path.join(import.meta.dirname, "data.txt");
	fs.writeFileSync(file, "abcdefgh");
	const fd = fs.openSync(file, "r+");
	const chunks = [];
	const buffer = Buffer.alloc(3);
	let count;
	while ((count = fs.readSync(fd, buffer, 0, 3, null)) > 0) chunks.push(buffer.toString("utf8", 0, count));
	fs.writeSync(fd, "XY", 2);
	fs.writeSync(fd, Buffer.from("Z"), 0, 1, 10);
	const written = [...fs.readFileSync(file)].map((byte) => (byte === 0 ? "0" : String.fromCharCode(byte))).join("");
	fs.ftruncateSync(fd, 4);
	fs.closeSync(fd);
	pi.registerCommand("probe", {
		description: [chunks.join("/"), written, fs.readFileSync(file, "utf8")].join(","),
		handler: async () => {},
	});
}
"#;
    assert_eq!(
        probe("descriptors", main).await,
        "abc/def/gh,abXYefgh00Z,abXY"
    );
}

/// `createEventBus` runs handlers as pi's does: as soon as an event is
/// emitted.
#[tokio::test(flavor = "multi_thread")]
async fn creates_event_buses() {
    let main = r#"
import { createEventBus } from "@earendil-works/pi-coding-agent";
export default function (pi) {
	const bus = createEventBus();
	const seen = [];
	const off = bus.on("x", (data) => seen.push(data));
	bus.emit("x", 1);
	const immediate = seen.join();
	off();
	bus.emit("x", 2);
	bus.on("x", (data) => seen.push(data));
	bus.clear();
	bus.emit("x", 3);
	pi.registerCommand("probe", { description: [immediate, seen.join()].join("|"), handler: async () => {} });
}
"#;
    assert_eq!(probe("event-bus", main).await, "1|1");
}
