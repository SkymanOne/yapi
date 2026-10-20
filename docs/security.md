# Security model and sandboxing

yapi reads files, edits them and runs commands on your machine. This page describes what yapi protects, what it leaves to the operating system, and how extensions are isolated.

## Security model

yapi runs with the permissions of the user who starts it. The built-in tools `read`, `bash`, `edit` and `write` are not sandboxed, and yapi does not ask for approval before each tool call. A command the model runs can read or change any file, and reach any service, that the user can.

Treat the commands and code the model generates as untrusted. Files, comments and command output that the model reads can steer it through prompt injection. Reviewing the transcript and granting project trust do not create a security boundary.

| Component | Runs as | Access |
|---|---|---|
| Built-in tools and `!` commands | yapi's process and its child processes | Everything the user can access |
| Extensions | WebAssembly instances | What their grants allow. In v0.1, Pi's defaults: files, processes, network and environment. |
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

Each call into an instance may compute for 60 seconds, not counting time spent waiting on yapi. Each extension instance may use 1 GiB of memory, and each codemode instance 384 MiB. An instance runs one call at a time, so a long computation in one Pi extension delays the other Pi extensions. An instance that traps, for example by exceeding a limit, is restarted and its extensions are reloaded.

### Grants

The host checks every file, process, network and environment access against the extension's grants. In v0.1, every package receives Pi's default grants, which allow all four, so an extension can do what it could do in Pi. Per-package grants that you approve are planned, as the [Roadmap](roadmap.md#capability-based-sandboxing) describes.

A process grant amounts to all four in practice. The host checks each grant separately, but a process an extension starts runs as the user, outside the sandbox, and inherits yapi's environment, including API keys. A subagent is such a process: a full yapi with the user's installed packages.

An extension can register a stdio MCP server only with the process grant, and an HTTP server only with the network grant. It reads a provider's credentials with `ctx.modelRegistry.getProviderAuth` only with the network grant.

### Agent sessions and events

yapi never runs an agent session inside an extension's instance, because that session would run tools and use credentials outside the extension's grants. Subagents run as separate yapi processes instead.

Extensions share Pi's `pi.events` bus, so every extension receives the events of every other extension.

## Project trust

Files in a project's `.yapi` folder can change how yapi behaves and can run code: `settings.json`, `mcp.json`, `extensions`, `skills`, `prompts`, `themes`, `SYSTEM.md` and `APPEND_SYSTEM.md`, as well as an `.agents/skills` folder in the project or a parent folder. Until the project is trusted, yapi loads none of them, and no project extension runs.

The first time interactive mode starts in a project that has them, yapi asks whether to trust it. Print, JSON and RPC modes never ask, and treat the project as untrusted unless a decision applies.

- yapi saves the decision in `~/.yapi/agent/trust.json`. `/trust` changes it later.
- `--approve` and `--no-approve` decide for one run.
- `"defaultProjectTrust"` in global settings set to `"always"` or `"never"` decides without asking.

`AGENTS.md` and `CLAUDE.md` load whether or not the project is trusted. Treat their contents as untrusted input as well.

### Use the project of another folder

`--project-dir <dir>` takes the project from another folder, as if yapi had started there. yapi loads that folder's `.yapi` files and the `AGENTS.md` or `CLAUDE.md` of that folder and its parents, and applies trust to that folder, including `--approve` and `--no-approve`. The working folder's own `.yapi` folder does not load. A relative path is resolved against the working folder.

Tools, the session and extensions' `ctx.cwd` stay in the working folder. A native extension finds the project folder with [`project_dir()`](native-extensions.md#where-the-project-is). Subagents use the flag to work in one folder with their parent's project.

## Report a vulnerability

Report vulnerabilities privately, as [SECURITY.md](https://github.com/SkymanOne/yapi/blob/main/SECURITY.md) describes.
