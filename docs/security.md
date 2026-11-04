# Security model and sandboxing

yapi reads files, edits them and runs commands on your machine. This page describes what yapi protects, what it leaves to the operating system, how yapi isolates extensions and which grants they need, and how project trust works.

## What runs with your permissions

yapi runs with your permissions. It doesn't sandbox its built-in tools `read`, `bash`, `edit` and `write`, and it doesn't ask for approval before each tool call. A command the model runs can read or change any file, and reach any service, that you can.

Treat the commands and code the model generates as untrusted. Files, comments and command output that the model reads can steer it through prompt injection. Reviewing the transcript and granting project trust don't create a security boundary.

Each part of yapi runs with its own access:

| Component | Runs as | Access |
|---|---|---|
| Built-in tools and `!` commands | yapi's process and its child processes | Everything you can access |
| Extensions | WebAssembly instances | What their grants allow. Extensions from `yapi install`, settings and `-e` get every grant, so as much as in Pi: files, processes, network, environment and session actions. [chest](chest.md) extensions get the capabilities their manifest entry grants. |
| Processes that extensions start, including subagents | Child processes, outside the sandbox | Everything you can access |
| Model Context Protocol (MCP) servers over stdio | Child processes | Everything you can access |
| MCP servers over HTTP | Remote services | What the server allows |
| [Codemode](mcp.md#call-mcp-tools-through-codemode) scripts | Fresh WebAssembly instances with no grants | Only the tools they call, and the clock and time zone as in Pi |

For untrusted repositories, unfamiliar packages or unattended runs, isolate yapi at the operating-system level:

- Run yapi in a container or virtual machine that holds only the files the task needs.
- Provide only the credentials the task needs, preferably short-lived and narrowly scoped.
- Restrict network access when the task doesn't need it.
- Keep important work under version control.

## How the extension sandbox works

Every extension runs in a WebAssembly instance hosted by Wasmtime, separate from yapi's own memory. The sandbox limits what extension code can do directly. It doesn't restrict the built-in tools, child processes or MCP servers.

### Instances and limits

An instance is a trust domain. Pi extensions with the same grants and event policy share one QuickJS-NG instance, because the host can't enforce grants between packages that share a JavaScript realm. Pi extensions whose grants or event policy differ run in separate instances, and each extra instance costs a thread and its own memory limit of 1 GiB.

Packages from `yapi install` share one instance, and [chest](packages.md#project-packages-with-chest) extensions with other grants or another event policy run in instances of their own. Each native extension gets an instance of its own, and codemode gets a fresh instance with no grants for every script.

Each instance runs under these limits:

- **Compute per call**: 60s, not counting time spent waiting on yapi
- **Compute while loading**: 10s per call while an instance starts and while it loads or reloads its extensions, so yapi stops an extension that never yields as it loads rather than waiting for it
- **Memory**: 1 GiB for each extension instance, and 384 MiB for each codemode instance

An instance runs one call at a time, so a long computation in one Pi extension delays the other Pi extensions. When an instance traps, for example by exceeding a limit, yapi restarts it and reloads its extensions. If reloading them traps again, the instance stays stopped, and each later call to it fails after one more attempt to restart it.

### How yapi enforces grants

The host checks an extension's file, process, network, environment and session access against its grants, and routes `pi.events` by them, as [Agent sessions and events](#agent-sessions-and-events) describes. Packages from `yapi install` and `-e` receive all five grants, so an extension can do what it could do in Pi. A chest extension receives only the capabilities its manifest entry grants, and the [capability table](chest-manifest.md#what-each-capability-allows) lists what each one allows.

Without the file grant, an extension loads the files of the packages that share its instance, and their dependencies, and nothing else. yapi refuses a path that leads out of a package through a symlink, and a path with `.` or `..` in it. For a chest extension, the other packages in its instance are the ones with the same grants and event policy.

Every extension file yapi finds in a chest package, including an `index.ts` or `index.js` that no manifest lists, runs with its entry's capabilities. The file keeps them when `-e` or a project folder loads it too, even through a symlink, and yapi warns about it. A file that yapi loads for a chest package but can't match to an entry runs with no capabilities, with a warning.

chest refuses to install an extension whose folder is or holds the project, the home folder or the agent folder, because it could load any file there. yapi doesn't load such an entry, locked or not, nor a global entry whose folder holds the project yapi runs in. In two more cases, files run with no capabilities:

- **Extension files of a refused entry**: they run with no capabilities when Pi's discovery, settings or `-e` load them. Other files in its folder keep the grants of their own source.
- **Files of an entry with a stale or missing lock**: every one of them runs with no capabilities when something else loads it.

A file that runs with no capabilities in either case loads no other file as a module. When two manifests give the same file, it runs with only what both allow.

A process grant amounts to all the others in practice. The host checks each grant separately, but a process an extension starts runs as you, outside the sandbox, and inherits yapi's environment, including API keys. A subagent is such a process: a full yapi with your installed packages.

### Grants for servers, models and tools

Registering servers, reading credentials and calling models each need a specific grant. An extension can register a stdio MCP server only with the process grant, and an HTTP server only with the network grant. It reads a provider's credentials with `ctx.modelRegistry.getProviderAuth`, `getApiKey`, `getApiKeyForProvider` or `getApiKeyAndHeaders`, and calls a model with `complete`, `stream`, `streamSimple`, `classify` or `generateImages`, only with the network grant. Without the network grant, `ctx.modelRegistry.refresh` refreshes model lists offline.

Running a tool needs grants too, whether an extension creates a built-in tool such as `createBashTool` or calls a tool with `ctx.executeTool`:

- The built-in `read`, `write`, `edit` and `ls` tools need the file grant, and `bash` needs the process grant.
- The built-in `grep` and `find` tools need the file, process and network grants, because they run ripgrep and fd and download them when they're missing.
- Any other tool needs every grant, because yapi can't tell what it reaches. This covers another extension's tool, a tool that replaces a built-in one, and an MCP tool.

A codemode script that an extension runs sees only the tools that the extension's grants allow. It calls models only with the network grant, and keeps what it stores only with the session grant.

### What the session grant covers

The session grant covers actions that start, steer or stop turns, or change the session, its files or the model's setup:

- `pi.sendMessage`, `sendUserMessage`, `appendEntry`, `setSessionName`, `setLabel`, `setModel`, `setThinkingLevel` and `setActiveTools`
- writes through `ctx.sessionManager`, and the system prompt options that a command edits
- `ctx.compact`, `abort` and `shutdown`, and a command's `newSession`, `fork`, `navigateTree`, `switchSession` and `reload`
- text that a custom editor submits
- the editor actions that interrupt, exit or suspend yapi, clear the editor, cycle the model or the thinking level, show or hide thinking, restore queued messages, or start a new session

Without the file grant, `switchSession` and the `parentSession` of `newSession` accept only session files in the session folder. Holding a file with `withFileMutationQueue` needs the file grant, because the model's `edit` and `write` tools wait while an extension holds it. Pi has no session grant.

### Requests that need no grant or every grant

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
- any request or editor action that yapi doesn't know, so a new one stays denied until yapi classifies it

A provider or wire API that an extension without every grant registers stays unused, and yapi logs a warning for each one in the extension output.

A custom editor that passes a key on to extension shortcuts runs only extension shortcuts, never yapi's own key actions, which go through the editor actions above. Each shortcut runs under the grants of the extension that registered it, as a `pi.events` listener does.

### Event handler results

yapi applies what an extension's event handlers return only within the extension's grants. Any change to what the model sees needs the session grant:

- A `tool_call` handler can always block a call, but the reason it gives reaches the model only with the session grant. Without it, the model reads "Tool execution was blocked".
- A `tool_call` handler changes the call's input only with the session grant and the grants that would let it run that tool itself.
- Results that add or change messages, context, user input, the system prompt or its options, provider requests, tool results, tool descriptions, compaction or tree summaries, or entries staged at the end of a turn need the session grant. So do results that start another turn or keep warming the prompt cache.
- Results that only cancel, block or stop need no grant, such as cancelling a compaction, a session switch or a fork, or stopping a run or cache warming. An `input` handler that handles the input itself and a `user_bash` handler that fails a `!` command need no grant either, because they stop work and show text only to you, not to the model.
- A `user_bash` handler that runs a `!` command through its own operations, or answers it with its own result, needs every grant, because the command then runs and shows what the extension decides where you expect your shell.
- The skill, prompt and theme paths that a `resources_discover` handler names need the session and file grants.

A result that yapi ignores counts as no result, so it never replaces what an extension with the grant returned for the same event. yapi logs a warning in the extension output for each ignored result, once for each extension and event.

The `before_provider_request` and `before_provider_headers` events carry a request's body and headers, which can hold your credentials, so they reach only extensions with the network grant.

### Agent sessions and events

yapi never runs an agent session inside an extension's instance, because that session would run tools and use credentials outside the extension's grants. Subagents run as separate yapi processes instead.

Pi's exports that read `auth.json`, write `trust.json` or `settings.json`, or write session files directly throw in extensions, so extensions reach credentials, project trust, settings and sessions through the host. Until yapi can limit file grants to paths, an extension with the file grant can still open these files itself, as in Pi.

Extensions share Pi's `pi.events` bus. Between extensions with different grants, sharing leaks access both ways: an extension that acts on events lets the sender borrow its grants, and an extension that listens learns what the sender emits.

yapi routes each event by the grants and the event policy of the extensions it runs in. An event reaches another extension when both have identical grants, or when the sender's policy lets it send the channel and the receiver's policy lets it hear it. A policy lists channels as exact names, prefixes ending in `*`, or `*` for every channel.

Packages from `yapi install`, and chest entries without an `events` key, get the shared policy, `*` both ways, so they hear every other extension's events, as in Pi. A chest entry's `events` key narrows its own policy, as the [event bus reference](chest-manifest.md#event-bus-routing) describes.

When routing keeps an event from an extension that listens on its channel, yapi logs a warning in the extension output once per channel and session. The warning starts with the policy lists that would let the event across, then names both runtimes by their first extension, with their grants. After 8 such warnings between two runtimes, yapi logs that it suppresses the rest.

## Project trust

Some files in a project can change how yapi behaves and can run code. Until you trust the project, yapi loads none of them, and no project extension runs:

- `settings.json`, `mcp.json`, `extensions`, `skills`, `prompts`, `themes`, `SYSTEM.md` and `APPEND_SYSTEM.md` in the project's `.yapi` folder
- an `.agents/skills` folder in the project or a parent folder
- a [chest](chest-extensions.md#how-yapi-loads-a-manifest) manifest that installs anything

The first time interactive mode starts in a project that has them, yapi asks whether to trust it. Print, JSON and remote procedure call (RPC) modes never ask, and treat the project as untrusted unless a decision applies:

- yapi saves the decision in `~/.yapi/agent/trust.json`. `/trust` changes it later.
- If yapi can't read `trust.json`, or it isn't a JSON object, yapi reports the error and doesn't start in a project that needs trust, as Pi does, unless `--approve` or `--no-approve` decides. It never overwrites that file.
- `--approve` and `--no-approve` decide for one run.
- `"defaultProjectTrust"` in global settings set to `"always"` or `"never"` decides without asking.

`AGENTS.md` and `CLAUDE.md` load whether or not you trust the project. Treat their contents as untrusted input as well.

### Use the project of another folder

`--project-dir <dir>` takes the project from another folder, as if yapi had started there. yapi loads that folder's `.yapi` files and the `AGENTS.md` or `CLAUDE.md` of that folder and its parents, and applies trust to that folder, including `--approve` and `--no-approve`. The working folder's own `.yapi` folder doesn't load. yapi resolves a relative path against the working folder.

Tools, the session and extensions' `ctx.cwd` stay in the working folder. A native extension finds the project folder with [`project_dir()`](native-extensions.md#where-the-project-is). Subagents use the flag to work in one folder with their parent's project.

## Report a vulnerability

Report vulnerabilities privately, as [SECURITY.md](https://github.com/SkymanOne/yapi/blob/main/SECURITY.md) describes.
