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

An instance is a trust domain. Pi extensions with the same grants and event policy share one QuickJS-NG instance, because grants cannot be enforced between packages that share a JavaScript realm. Pi extensions whose grants or event policy differ run in separate instances, and each extra instance costs a thread and its own memory limit of 1 GiB. Packages from `yapi install` share one instance, and [chest](packages.md#project-packages-with-chest) extensions with other grants or another event policy run in instances of their own. Each native extension gets an instance of its own, and codemode gets a fresh instance with no grants for every script.

Each call into an instance may compute for 60 seconds, not counting time spent waiting on yapi. While an instance starts and while it loads or reloads its extensions, each call may compute for 10 seconds, so an extension that never yields as it loads stops startup within 10 seconds. Each extension instance may use 1 GiB of memory, and each codemode instance 384 MiB. An instance runs one call at a time, so a long computation in one Pi extension delays the other Pi extensions. An instance that traps, for example by exceeding a limit, is restarted and its extensions are reloaded. If reloading them traps again, the instance stays stopped, and each later call to it fails after one more attempt to restart it.

### Grants

The host checks an extension's file, process, network, environment and session access against its grants, and routes `pi.events` by them, as [Agent sessions and events](#agent-sessions-and-events) describes. Without the file grant, an extension loads the files of the packages that share its instance, and their dependencies, and nothing else. A path that leads out of a package through a symlink is refused, and so is a path with `.` or `..` in it. Packages from `yapi install` and `-e` receive all five grants, so an extension can do what it could do in Pi. A chest extension receives only the capabilities its manifest entry grants. Without the file grant it still loads the files of the other packages in its instance, which have the same grants and event policy. Every extension file yapi finds in a chest package, including an `index.ts` or `index.js` that no manifest lists, runs with its entry's capabilities. The file keeps them when `-e` or a project folder loads it too, even through a symlink, and yapi warns about it. A file that yapi loads for a chest package but cannot match to an entry runs with no capabilities, with a warning. chest refuses to install an extension whose folder is or holds the project, the home folder or the agent folder, because it could load any file there. yapi does not load such an entry, locked or not, nor a global entry whose folder holds the project yapi runs in. Only the extension files of such an entry run with no capabilities when pi's discovery, settings or `-e` loads them, and other files in its folder keep the grants of their own source. Every file of an entry whose lock is stale or missing runs with no capabilities when something else loads it. A file that runs with no capabilities for either reason loads no other file as a module. When two manifests give the same file, it runs with only what both allow. The [capability table](chest-manifest.md#capabilities) lists what each one allows.

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

Pi's exports that read `auth.json`, write `trust.json` or `settings.json`, or write session files directly throw in extensions, so extensions reach credentials, project trust, settings and sessions through the host. Until file grants can be limited to paths, an extension with the file grant can still open these files itself, as in Pi.

Extensions share Pi's `pi.events` bus. Between extensions with different grants, sharing leaks access both ways: an extension that acts on events lets the sender borrow its grants, and an extension that listens learns what the sender emits.

yapi routes each event by the grants and the event policy of the extensions it runs in. An event reaches another extension when both have identical grants, or when the sender's policy lets it send the channel and the receiver's policy lets it hear it. A policy lists channels as exact names, prefixes ending in `*`, or `*` for every channel. Packages from `yapi install`, and chest entries without an `events` key, get the shared policy, `*` both ways, so they hear every other extension's events, as in Pi. A chest entry's `events` key narrows its own policy, as the [event bus reference](chest-manifest.md#event-bus) describes.

When routing keeps an event from an extension that listens on its channel, yapi logs a warning in the extension output once per channel and session. It starts with the policy lists that would let the event across, then names both runtimes by their first extension, with their grants. After 8 such warnings between two runtimes, yapi logs that it suppresses the rest.

## Project trust

Files in a project's `.yapi` folder can change how yapi behaves and can run code: `settings.json`, `mcp.json`, `extensions`, `skills`, `prompts`, `themes`, `SYSTEM.md` and `APPEND_SYSTEM.md`, as well as an `.agents/skills` folder in the project or a parent folder, and a [chest](packages.md#project-packages-with-chest) manifest that installs anything. Until the project is trusted, yapi loads none of them, and no project extension runs.

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
