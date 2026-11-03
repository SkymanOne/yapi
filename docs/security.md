# Security model and sandboxing

yapi reads files, edits them and runs commands on your machine. This page describes what yapi protects, what it leaves to the operating system, and how extensions are isolated.

## Security model

yapi runs with the permissions of the user who starts it. The built-in tools `read`, `bash`, `edit` and `write` are not sandboxed, and yapi does not ask for approval before each tool call. A command the model runs can read or change any file, and reach any service, that the user can.

Treat the commands and code the model generates as untrusted. Files, comments and command output that the model reads can steer it through prompt injection. Reviewing the transcript and granting project trust do not create a security boundary.

| Component | Runs as | Access |
|---|---|---|
| Built-in tools and `!` commands | yapi's process and its child processes | Everything the user can access |
| Extensions | WebAssembly instances | What their grants allow. For now, every grant, so as much as in Pi: files, processes, network, environment and session actions. |
| Processes that extensions start, including subagents | Child processes, outside the sandbox | Everything the user can access |
| MCP servers over stdio | Child processes | Everything the user can access |
| MCP servers over HTTP | Remote services | What the server allows |
| Codemode scripts | Fresh WebAssembly instances with no grants | Only the tools they call, and the clock and time zone as in Pi |

For untrusted repositories, unfamiliar packages or unattended runs, isolate yapi at the operating-system level:

- Run yapi in a container or virtual machine that holds only the files the task needs.
- Provide only the credentials the task needs, preferably short-lived and narrowly scoped.
- Restrict network access when the task does not need it.
- Keep important work under version control.

## Sandboxing

Every extension runs in a WebAssembly instance hosted by Wasmtime, separate from yapi's own memory. The sandbox limits what extension code can do directly. It does not restrict the built-in tools, child processes or MCP servers.

### Instances and limits

An instance is a trust domain. All Pi extensions share one QuickJS-NG instance, because grants cannot be enforced between packages that share a JavaScript realm. Each native extension gets an instance of its own, and codemode gets a fresh instance with no grants for every script.

Each call into an instance may compute for 60 seconds, not counting time spent waiting on yapi. Each extension instance may use 1 GiB of memory, and each codemode instance 384 MiB. An instance runs one call at a time, so a long computation in one Pi extension delays the other Pi extensions. An instance that traps, for example by exceeding a limit, is restarted and its extensions are reloaded. If reloading them traps again, the instance stays stopped, and each later call to it fails after one more attempt to restart it.

### Grants

The host checks an extension's file, process, network, environment and session access against its grants. Two paths do not check grants yet: the `pi.events` bus, and what event handlers return, such as a `tool_call` handler that rewrites a tool's input. Without the file grant, an extension cannot import modules from files either. For now, every package receives all five grants, so an extension can do what it could do in Pi. Per-package grants that you approve are planned, as the [Roadmap](roadmap.md#capability-based-sandboxing) describes.

A process grant amounts to all the others in practice. The host checks each grant separately, but a process an extension starts runs as the user, outside the sandbox, and inherits yapi's environment, including API keys. A subagent is such a process: a full yapi with the user's installed packages.

An extension can register a stdio MCP server only with the process grant, and an HTTP server only with the network grant. It reads a provider's credentials with `ctx.modelRegistry.getProviderAuth`, `getApiKey`, `getApiKeyForProvider` or `getApiKeyAndHeaders`, and calls a model with `complete`, `stream`, `streamSimple`, `classify` or `generateImages`, only with the network grant. Without the network grant, `ctx.modelRegistry.refresh` refreshes model lists offline.

Running a tool needs grants too, whether an extension creates a built-in tool such as `createBashTool` or calls a tool with `ctx.executeTool`:

- The built-in `read`, `write`, `edit` and `ls` tools need the file grant, and `bash` needs the process grant.
- The built-in `grep` and `find` tools need the file, process and network grants, because they run ripgrep and fd and download them when they are missing.
- Any other tool needs every grant, because yapi cannot tell what it reaches. This covers another extension's tool, a tool that replaces a built-in one, and an MCP tool.

A codemode script that an extension runs sees only the tools that the extension's grants allow. It calls models only with the network grant, and keeps what it stores only with the session grant.

The session grant covers actions that start, steer or stop turns, or change the session, its files or the model's setup:

- `pi.sendMessage`, `sendUserMessage`, `appendEntry`, `setSessionName`, `setLabel`, `setModel`, `setThinkingLevel` and `setActiveTools`
- writes through `ctx.sessionManager`, and the system prompt options that a command edits
- `ctx.compact`, `abort` and `shutdown`, and a command's `newSession`, `fork`, `navigateTree`, `switchSession` and `reload`
- text that a custom editor submits
- the editor actions that interrupt, exit or suspend yapi, clear the editor, cycle the model or the thinking level, show or hide thinking, restore queued messages, or start a new session

Without the file grant, `switchSession` and the `parentSession` of `newSession` accept only session files in the session directory. Holding a file with `withFileMutationQueue` needs the file grant, because the model's `edit` and `write` tools wait while an extension holds it. Pi has no session grant.

Some requests need no grant:

- reading the session, such as its entries, name and model
- dialogs, widgets and notifications
- the editor actions that expand tool output, copy the last message, or open the model, tree, fork or resume selector, where you make the choice
- `copyToClipboard`, which writes the system clipboard. A person may paste what it copies into a terminal or another program, so treat text an extension copies as untrusted, as you would text from a web page.

Some actions need every grant, because they reach outside the sandbox or act with yapi's own access:

- registering a provider or a wire API, because a provider's replies drive the agent's tools and its requests carry your credentials
- custom editor text that starts with `!` or `/`, after any leading spaces, because it runs a shell or slash command
- listening to raw terminal input with `ctx.ui.onTerminalInput`, or adding an autocomplete provider with `ctx.ui.addAutocompleteProvider`, because either can turn what you type into a shell or slash command. Without every grant, both calls throw in a Pi extension and do nothing in a native one.
- the editor actions that open the external editor, paste from the clipboard, or submit the editor's text as a follow-up
- any request or editor action that yapi does not know, so a new one stays denied until yapi classifies it

### Agent sessions and events

yapi never runs an agent session inside an extension's instance, because that session would run tools and use credentials outside the extension's grants. Subagents run as separate yapi processes instead.

Pi's exports that read `auth.json`, write `trust.json` or `settings.json`, or write session files directly throw in extensions, so extensions reach credentials, project trust, settings and sessions through the host. Until file grants can be limited to paths, an extension with the file grant can still open these files itself, as in Pi.

Extensions share Pi's `pi.events` bus, so every extension receives the events of every other extension.

## Project trust

Files in a project's `.yapi` folder can change how yapi behaves and can run code: `settings.json`, `mcp.json`, `extensions`, `skills`, `prompts`, `themes`, `SYSTEM.md` and `APPEND_SYSTEM.md`, as well as an `.agents/skills` folder in the project or a parent folder. Until the project is trusted, yapi loads none of them, and no project extension runs.

The first time interactive mode starts in a project that has them, yapi asks whether to trust it. Print, JSON and RPC modes never ask, and treat the project as untrusted unless a decision applies.

- yapi saves the decision in `~/.yapi/agent/trust.json`. `/trust` changes it later.
- If `trust.json` cannot be read or is not a JSON object, yapi reports the error and does not start in a project that needs trust, as Pi does, unless `--approve` or `--no-approve` decides. It never overwrites that file.
- `--approve` and `--no-approve` decide for one run.
- `"defaultProjectTrust"` in global settings set to `"always"` or `"never"` decides without asking.

`AGENTS.md` and `CLAUDE.md` load whether or not the project is trusted. Treat their contents as untrusted input as well.

### Use the project of another folder

`--project-dir <dir>` takes the project from another folder, as if yapi had started there. yapi loads that folder's `.yapi` files and the `AGENTS.md` or `CLAUDE.md` of that folder and its parents, and applies trust to that folder, including `--approve` and `--no-approve`. The working folder's own `.yapi` folder does not load. A relative path is resolved against the working folder.

Tools, the session and extensions' `ctx.cwd` stay in the working folder. A native extension finds the project folder with [`project_dir()`](native-extensions.md#where-the-project-is). Subagents use the flag to work in one folder with their parent's project.

## Report a vulnerability

Report vulnerabilities privately, as [SECURITY.md](https://github.com/SkymanOne/yapi/blob/main/SECURITY.md) describes.
