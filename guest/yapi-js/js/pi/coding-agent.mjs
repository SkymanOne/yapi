// `@earendil-works/pi-coding-agent` for extensions in yapi: pi's own modules
// for what runs inside the extension, built-in tools backed by yapi's own, and
// stubs for the exports that would run a second agent, so every import links.
// A stub throws when called.
import { Container, Editor, Text, getKeybindings } from "@earendil-works/pi-tui";
import { theme } from "yapi:pi/host";

const yapi = globalThis.__yapi;

/** pi's bundled modules, evaluated on first use: parsing them would cost every extension load. */
const vendored = () => yapi.importSync("yapi:vendor/pi-coding-agent");
/**
 * pi's exports from the bundled modules, each a function or class that loads
 * them when it is first called, constructed or read (statics, prototype).
 */
const lazy = new Proxy(
	{},
	{
		get: (_exports, name) =>
			new Proxy(function () {}, {
				apply: (_target, self, args) => Reflect.apply(vendored()[name], self, args),
				construct: (_target, args, newTarget) => Reflect.construct(vendored()[name], args, newTarget),
				get: (_target, key) => vendored()[name][key],
			}),
	},
);

export { CONFIG_DIR_NAME, VERSION, convertToPng, formatDimensionNote, getAgentDir, getDocsPath, getEditorTheme, getExamplesPath,
	getLanguageFromPath, getMarkdownTheme, getPackageDir, getReadmePath, getSelectListTheme, getSettingsListTheme, highlightCode, initTheme,
	parseFrontmatter, resizeImage, stripFrontmatter, Theme } from "yapi:pi/host";
export const { ArminComponent, AssistantMessageComponent, BashExecutionComponent,
	BranchSummaryMessageComponent, buildContextEntries, buildSessionContext, buildSessionProjection, calculateContextTokens,
	collectEntriesForBranchSummary, compact, CompactionSummaryMessageComponent, createLocalPowerShellOperations, createPowerShellTool,
	createPowerShellToolDefinition, createToolSearchExtension, CustomMessageComponent, detectSupportedImageMimeTypeFromFile, estimateTokens,
	ExtensionEditorComponent, ExtensionInputComponent, ExtensionSelectorComponent, findCutPoint, findTurnStartIndex, FooterComponent,
	formatSize, formatSkillsForPrompt, generateBranchSummary, generateDiffString, generateSummary, generateSummaryWithUsage,
	generateUnifiedPatch, getLastAssistantUsage, getPowerShellConfig, hasTrustRequiringProjectResources, loadSkills, loadSkillsFromDir,
	LoginDialogComponent, migrateSessionEntries, ModelSelectorComponent, OAuthSelectorComponent, parseArgs, parseSessionEntries,
	prepareBranchEntries, ProjectTrustStore, readStoredCredential, renderDiff, resolveCliModel, resolveModelScopeWithDiagnostics,
	SessionManager, SessionSelectorComponent, sessionEntryToContextMessages, SettingsManager, SettingsSelectorComponent, shouldCompact,
	ShowImagesSelectorComponent, SkillInvocationMessageComponent, ThemeSelectorComponent, ThinkingSelectorComponent, ToolExecutionComponent,
	TreeSelectorComponent, truncateHead, truncateLine, truncateTail, UserMessageComponent, UserMessageSelectorComponent, wrapRegisteredTool,
	wrapRegisteredTools } = lazy;

export const CURRENT_SESSION_VERSION = 3;
export const DEFAULT_MAX_BYTES = 50 * 1024;
export const DEFAULT_MAX_LINES = 2000;
export const DEFAULT_COMPACTION_SETTINGS = { enabled: true, reserveTokens: 16384, keepRecentTokens: 20000 };
export const VIRTUAL_MODEL_STATE_ENTRY = "pi.virtual-model-state";

export function getShellConfig() {
	return { shell: process.env.SHELL || "/bin/sh", args: ["-c"] };
}

/** pi's `loadProjectContextFiles`, as yapi's system prompt loads them. */
export function loadProjectContextFiles(options) {
	const { path } = globalThis.__yapi_builtins;
	return yapi.request("context.files", { cwd: path.resolve(options.cwd), agentDir: path.resolve(options.agentDir) });
}

/** pi's `parseSkillBlock`: the skill block that `text` is, else `null`. */
export function parseSkillBlock(text) {
	const match = text.match(/^<skill name="([^"]+)" location="([^"]+)">\n([\s\S]*?)\n<\/skill>(?:\n\n([\s\S]+))?$/);
	if (!match) return null;
	return { name: match[1], location: match[2], content: match[3], userMessage: match[4]?.trim() || undefined };
}

export class CredentialSynchronizationError extends Error {
	constructor(providerId, operation, credential, options) {
		super(`Credential ${operation} committed for ${providerId}, but local synchronization failed`, options);
		this.name = "CredentialSynchronizationError";
		this.providerId = providerId;
		this.operation = operation;
		this.credential = credential;
	}
}

// ----- tool definitions and events -------------------------------------------
export function defineTool(tool) {
	return tool;
}
export function isToolCallEventType(toolName, event) {
	return event.toolName === toolName;
}
const resultOf = (name) => (event) => event.toolName === name;
export const isBashToolResult = resultOf("bash");
export const isPowerShellToolResult = resultOf("powershell");
export const isReadToolResult = resultOf("read");
export const isEditToolResult = resultOf("edit");
export const isWriteToolResult = resultOf("write");
export const isGrepToolResult = resultOf("grep");
export const isFindToolResult = resultOf("find");
export const isLsToolResult = resultOf("ls");

export const createEventBus = yapi.createEventBus;

// ----- built-in tools ----------------------------------------------------------------
/**
 * pi's factory for built-in tool `name`: yapi's tool, which the host runs, or
 * pi's own tool from its factory `piFactory` when the options change how it
 * runs, as custom `operations` (a remote filesystem, say) or bash's spawn
 * hook and command prefix do.
 */
function builtinTool(name, cwd, options, piFactory) {
	if (options?.operations || options?.spawnHook || options?.commandPrefix) return vendored()[piFactory](cwd ?? process.cwd(), options);
	const info = yapi.request("builtin.tool", { name, cwd: cwd ?? process.cwd() });
	return {
		name: info.name,
		label: info.label,
		description: info.description,
		promptSnippet: info.promptSnippet,
		promptGuidelines: info.promptGuidelines,
		parameters: info.parameters,
		// pi's edit tool draws its own shell; yapi draws the built-in renderers.
		...(name === "edit" ? { renderShell: "self" } : {}),
		async execute(toolCallId, params) {
			return yapi.op("builtin.execute", { name, cwd: cwd ?? process.cwd(), toolCallId, params });
		},
	};
}
/** pi's `create<Name>Tool` and `create<Name>ToolDefinition` for built-in tool `name`. */
function factories(name) {
	const factory = `create${name[0].toUpperCase()}${name.slice(1)}Tool`;
	return [factory, `${factory}Definition`].map((piFactory) => (cwd, options) => builtinTool(name, cwd, options, piFactory));
}
export const [createBashTool, createBashToolDefinition] = factories("bash");
export const [createReadTool, createReadToolDefinition] = factories("read");
export const [createEditTool, createEditToolDefinition] = factories("edit");
export const [createWriteTool, createWriteToolDefinition] = factories("write");
export const [createGrepTool, createGrepToolDefinition] = factories("grep");
export const [createFindTool, createFindToolDefinition] = factories("find");
export const [createLsTool, createLsToolDefinition] = factories("ls");
export const createCodingTools = (cwd, options) => [
	createReadTool(cwd, options?.read),
	createBashTool(cwd, options?.bash),
	createEditTool(cwd, options?.edit),
	createWriteTool(cwd, options?.write),
];
export const createReadOnlyTools = (cwd, options) => [
	createReadTool(cwd, options?.read),
	createGrepTool(cwd, options?.grep),
	createFindTool(cwd, options?.find),
	createLsTool(cwd, options?.ls),
];
/** pi's local shell: runs `command` with bash, streaming its output to `onData`. */
export function createLocalBashOperations() {
	const { child_process, fs, os } = globalThis.__yapi_builtins;
	return {
		async exec(command, cwd, { onData, signal, timeout, env }) {
			if (signal?.aborted) throw new Error("aborted");
			if (!fs.existsSync(cwd)) throw new Error(`Working directory does not exist: ${cwd}\nCannot execute bash commands.`);
			const shell = fs.existsSync("/bin/bash") ? "/bin/bash" : "sh";
			const child = child_process.spawn(shell, ["-c", command], { cwd, env, stdio: ["ignore", "pipe", "pipe"] });
			let timedOut = false;
			const kill = () => child.kill("SIGKILL");
			const timer = timeout > 0 ? setTimeout(() => ((timedOut = true), kill()), timeout * 1000) : undefined;
			child.stdout.on("data", onData);
			child.stderr.on("data", onData);
			signal?.addEventListener("abort", kill, { once: true });
			try {
				const exitCode = await yapi.waitForChild(child);
				if (signal?.aborted) throw new Error("aborted");
				if (timedOut) throw new Error(`timeout:${timeout}`);
				// A shell a signal ended reports 128 plus the signal's number, as in pi.
				return { exitCode: exitCode ?? (child.signalCode ? 128 + (os.constants.signals[child.signalCode] ?? 0) : 1) };
			} finally {
				clearTimeout(timer);
				signal?.removeEventListener("abort", kill);
			}
		},
	};
}


// ----- text and session helpers -------------------------------------------------------------
export function getLatestCompactionEntry(entries) {
	for (let i = entries.length - 1; i >= 0; i--) if (entries[i]?.type === "compaction") return entries[i];
	return undefined;
}
export function createSyntheticSourceInfo(path, options = {}) {
	return { path, source: options.source ?? "local", scope: "temporary", origin: "top-level", baseDir: options.baseDir };
}

/** pi's `withFileMutationQueue`, in the host's queue, which yapi's edit and write tools share. */
export async function withFileMutationQueue(path, fn) {
	const id = await yapi.op("file.lock", { path: globalThis.__yapi_builtins.path.resolve(String(path)) });
	try {
		return await fn();
	} finally {
		yapi.request("file.unlock", { id });
	}
}

// ----- conversation helpers, run by yapi ------------------------------------------------------
export function convertToLlm(messages) {
	return yapi.request("util.convertToLlm", { messages });
}
export function serializeConversation(messages) {
	return yapi.request("util.serializeConversation", { messages });
}
export function copyToClipboard(text) {
	return yapi.op("clipboard.copy", { text });
}

// ----- key hints -----------------------------------------------------------------------------------
/** pi's `formatKeyText`: keys as hints show them, with macOS's name for alt. */
function formatKeyText(key) {
	const part = (name) => (process.platform === "darwin" && name.toLowerCase() === "alt" ? "option" : name);
	return key
		.split("/")
		.map((keys) => keys.split("+").map(part).join("+"))
		.join("/");
}
/** The keys bound to `keybinding`, as the session's bindings give them; empty without one. */
export function keyText(keybinding) {
	const keys = getKeybindings().getKeys(keybinding);
	return keys.length === 0 ? "" : formatKeyText(keys.join("/"));
}
export function keyHint(keybinding, description) {
	return theme.fg("dim", keyText(keybinding)) + theme.fg("muted", ` ${description}`);
}
export function rawKeyHint(key, description) {
	return theme.fg("dim", formatKeyText(key)) + theme.fg("muted", ` ${description}`);
}

// ----- components ---------------------------------------------------------------------------------
export class DynamicBorder {
	constructor(color = (text) => theme.fg("border", text)) {
		this.color = color;
	}
	invalidate() {}
	render(width) {
		return [this.color("─".repeat(Math.max(1, width)))];
	}
}
export class BorderedLoader extends Container {
	constructor(_tui, _theme, message) {
		super();
		this.signal = new AbortController().signal;
		this.onAbort = undefined;
		this.addChild(new DynamicBorder());
		this.addChild(new Text(message ?? "", 1, 0));
		this.addChild(new DynamicBorder());
	}
	handleInput() {}
	dispose() {}
}
/**
 * pi's `CustomEditor`: the editor with the app's key bindings. The handlers
 * are set when the editor replaces the built-in one; the host then draws
 * the working status in its top border when `embedWorkingStatus` is set.
 */
export class CustomEditor extends Editor {
	constructor(tui, theme, keybindings, options) {
		super(tui, theme, options);
		this.keybindings = keybindings;
		this.embedWorkingStatus = options?.embedWorkingStatus ?? false;
		this.actionHandlers = new Map();
	}
	onAction(action, handler) {
		this.actionHandlers.set(action, handler);
	}
	handleInput(data) {
		if (this.onExtensionShortcut?.(data)) return;
		if (this.keybindings.matches(data, "app.clipboard.pasteImage")) {
			this.onPasteImage?.();
			return;
		}
		if (this.keybindings.matches(data, "app.interrupt")) {
			if (!this.isShowingAutocomplete()) {
				const handler = this.onEscape ?? this.actionHandlers.get("app.interrupt");
				if (handler) {
					handler();
					return;
				}
			}
			super.handleInput(data);
			return;
		}
		if (this.keybindings.matches(data, "app.exit") && this.getText().length === 0) {
			const handler = this.onCtrlD ?? this.actionHandlers.get("app.exit");
			if (handler) handler();
			return;
		}
		if (this.keybindings.matches(data, "tui.editor.historyPrevious") || this.keybindings.matches(data, "tui.editor.historyNext")) {
			super.handleInput(data);
			return;
		}
		for (const [action, handler] of this.actionHandlers) {
			if (action !== "app.interrupt" && action !== "app.exit" && this.keybindings.matches(data, action)) {
				handler();
				return;
			}
		}
		super.handleInput(data);
	}
}

// ----- RPC client ------------------------------------------------------------------------------
/**
 * pi's `RpcClient`: runs another agent in RPC mode and drives it over its
 * standard input and output. The agent is yapi itself, the binary at
 * `process.execPath`, so `cliPath` is ignored.
 */
export class RpcClient {
	constructor(options = {}) {
		this.options = options;
		this.process = null;
		this.eventListeners = [];
		this.pendingRequests = new Map();
		this.requestId = 0;
		this.stderr = "";
		this.exitError = null;
	}
	async start() {
		if (this.process) throw new Error("Client already started");
		this.exitError = null;
		const args = ["--mode", "rpc"];
		if (this.options.provider) args.push("--provider", this.options.provider);
		if (this.options.model) args.push("--model", this.options.model);
		if (this.options.args) args.push(...this.options.args);
		const child = globalThis.__yapi_builtins.child_process.spawn(process.execPath, args, {
			cwd: this.options.cwd,
			env: { ...process.env, ...this.options.env },
			stdio: ["pipe", "pipe", "pipe"],
		});
		this.process = child;
		const fail = (error) => {
			if (this.process !== child) return;
			this.exitError = error;
			for (const pending of this.pendingRequests.values()) pending.reject(error);
			this.pendingRequests.clear();
		};
		child.stderr.on("data", (data) => {
			this.stderr += data.toString();
			process.stderr.write(data);
		});
		child.once("exit", (code, signal) => fail(this.exitedError(code, signal)));
		child.once("error", (error) => fail(new Error(`Agent process error: ${error.message}. Stderr: ${this.stderr}`)));
		// Strict JSONL: records end at LF only.
		let buffer = "";
		child.stdout.setEncoding("utf8");
		child.stdout.on("data", (chunk) => {
			buffer += chunk;
			for (let end; this.process === child && (end = buffer.indexOf("\n")) !== -1; buffer = buffer.slice(end + 1)) {
				this.handleLine(buffer.slice(0, end).replace(/\r$/, ""));
			}
		});
		await new Promise((resolve) => setTimeout(resolve, 100));
		if (this.exitError) throw this.exitError;
		if (child.exitCode !== null) throw (this.exitError = this.exitedError(child.exitCode, child.signalCode));
	}
	async stop() {
		const child = this.process;
		if (!child) return;
		child.kill("SIGTERM");
		await new Promise((resolve) => {
			const timeout = setTimeout(() => {
				child.kill("SIGKILL");
				resolve();
			}, 1000);
			child.on("exit", () => {
				clearTimeout(timeout);
				resolve();
			});
		});
		this.process = null;
		this.pendingRequests.clear();
	}
	onEvent(listener) {
		this.eventListeners.push(listener);
		return () => {
			const index = this.eventListeners.indexOf(listener);
			if (index !== -1) this.eventListeners.splice(index, 1);
		};
	}
	getStderr() {
		return this.stderr;
	}
	/** Events until the agent settles; `prompt` sends one first. */
	collectEvents(timeout = 60000) {
		return new Promise((resolve, reject) => {
			const events = [];
			const timer = setTimeout(() => {
				unsubscribe();
				reject(new Error(`Timeout collecting events. Stderr: ${this.stderr}`));
			}, timeout);
			const unsubscribe = this.onEvent((event) => {
				events.push(event);
				if (event.type !== "agent_settled") return;
				clearTimeout(timer);
				unsubscribe();
				resolve(events);
			});
		});
	}
	waitForIdle(timeout = 60000) {
		return this.collectEvents(timeout).then(() => undefined, (error) => {
			throw new Error(error.message.replace("collecting events", "waiting for agent to become idle"));
		});
	}
	async promptAndWait(message, images, timeout = 60000) {
		const events = this.collectEvents(timeout);
		await this.prompt(message, images);
		return events;
	}
	handleLine(line) {
		let data;
		try {
			data = JSON.parse(line);
		} catch {
			return;
		}
		const pending = data.type === "response" && data.id ? this.pendingRequests.get(data.id) : undefined;
		if (pending) {
			this.pendingRequests.delete(data.id);
			pending.resolve(data);
			return;
		}
		for (const listener of [...this.eventListeners]) listener(data);
	}
	exitedError(code, signal) {
		return new Error(`Agent process exited (code=${code} signal=${signal}). Stderr: ${this.stderr}`);
	}
	send(command) {
		const child = this.process;
		if (!child?.stdin) return Promise.reject(new Error("Client not started"));
		if (this.exitError) return Promise.reject(this.exitError);
		if (child.exitCode !== null) return Promise.reject((this.exitError = this.exitedError(child.exitCode, child.signalCode)));
		const id = `req_${++this.requestId}`;
		return new Promise((resolve, reject) => {
			const timeout = setTimeout(() => {
				this.pendingRequests.delete(id);
				reject(new Error(`Timeout waiting for response to ${command.type}. Stderr: ${this.stderr}`));
			}, 30000);
			const settle = (finish) => (value) => {
				clearTimeout(timeout);
				finish(value);
			};
			this.pendingRequests.set(id, { resolve: settle(resolve), reject: settle(reject) });
			child.stdin.write(`${JSON.stringify({ ...command, id })}\n`);
		});
	}
}
/**
 * RpcClient's commands, as pi defines them: the method, the RPC command, its
 * arguments in order, and what the method returns: nothing (`undefined`),
 * the response's data (`null`), or one field of it.
 */
const RPC_COMMANDS = [
	["prompt", "prompt", ["message", "images", "streamingBehavior"], "disposition"],
	["steer", "steer", ["message", "images"], "disposition"],
	["followUp", "follow_up", ["message", "images"], "disposition"],
	["abort", "abort", []],
	["clearQueue", "clear_queue", [], null],
	["newSession", "new_session", ["parentSession"], null],
	["getState", "get_state", [], null],
	["setModel", "set_model", ["provider", "modelId"], null],
	["cycleModel", "cycle_model", [], null],
	["getAvailableModels", "get_available_models", [], "models"],
	["setThinkingLevel", "set_thinking_level", ["level"]],
	["cycleThinkingLevel", "cycle_thinking_level", [], null],
	["getAvailableThinkingLevels", "get_available_thinking_levels", [], "levels"],
	["setSteeringMode", "set_steering_mode", ["mode"]],
	["setFollowUpMode", "set_follow_up_mode", ["mode"]],
	["compact", "compact", ["customInstructions"], null],
	["setAutoCompaction", "set_auto_compaction", ["enabled"]],
	["setAutoRetry", "set_auto_retry", ["enabled"]],
	["abortRetry", "abort_retry", []],
	["bash", "bash", ["command"], null],
	["abortBash", "abort_bash", []],
	["getSessionStats", "get_session_stats", [], null],
	["exportHtml", "export_html", ["outputPath"], null],
	["switchSession", "switch_session", ["sessionPath"], null],
	["fork", "fork", ["entryId"], null],
	["clone", "clone", [], null],
	["getForkMessages", "get_fork_messages", [], "messages"],
	["getEntries", "get_entries", ["since"], null],
	["getTree", "get_tree", [], null],
	["getLastAssistantText", "get_last_assistant_text", [], "text"],
	["setSessionName", "set_session_name", ["name"]],
	["getMessages", "get_messages", [], "messages"],
	["getCommands", "get_commands", [], "commands"],
];
for (const [method, type, keys, returns] of RPC_COMMANDS) {
	RpcClient.prototype[method] = async function (...values) {
		const response = await this.send({ type, ...Object.fromEntries(keys.map((key, index) => [key, values[index]])) });
		if (returns === undefined) return;
		if (!response.success) throw new Error(response.error);
		return returns === null ? response.data : response.data?.[returns];
	};
}

// ----- not available in yapi --------------------------------------------------------------------------
// These run a second pi agent inside the extension: a session with its own
// model runtime and credentials, extensions loaded with jiti (which needs
// Node), pi's package manager (which needs npm) and pi's modes (which take over
// the terminal and output yapi owns). They would bring pi's whole coding agent
// into the runtime. pi's MCP extension stays out too, as yapi's MCP client is
// native.
export const { AgentSession, AgentSessionRuntime, DefaultPackageManager, DefaultResourceLoader, ExtensionRunner, InteractiveMode, ModelRegistry,
	ModelRuntime, createAgentSession, createAgentSessionFromServices, createAgentSessionRuntime, createAgentSessionServices, createExtensionRuntime,
	createMcpExtension, discoverAndLoadExtensions, main, runPrintMode, runRpcMode } = yapi.stubs("@earendil-works/pi-coding-agent");
/**
 * pi's codemode extension, for packages that decorate the tool before
 * registering it. yapi runs the scripts; `mode`, `inlineBudget` and `models`
 * options are ignored.
 */
export function createCodemodeExtension() {
	return (pi) => {
		const info = yapi.request("codemode.definition", {});
		pi.registerTool({
			...info,
			exposure: "model-only",
			defaultActive: false,
			async execute(toolCallId, params) {
				return yapi.op("codemode.execute", { toolCallId, params });
			},
		});
	};
}
