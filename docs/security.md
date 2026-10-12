# Run yapi safely

yapi reads, edits and runs files with the permissions of the user who started it, and it does not ask before each tool call. Treat the commands and code it generates as untrusted. Files and command output that the model reads can steer it through prompt injection.

To limit what a wrong or hostile action can reach, run yapi in a container or virtual machine that holds only the files and credentials the task needs. Project trust and reviewing the transcript do not create a security boundary.

## Project trust

Files in a project's `.yapi` folder can change how yapi behaves and can run code. The first time yapi starts in a project that has them, it asks whether to trust the folder. Until you trust it, yapi loads nothing from `.yapi` or `.agents/skills`, and no project extension runs.

- yapi saves the answer in `~/.yapi/agent/trust.json`. `/trust` changes it later.
- `--approve` and `--no-approve` decide for one run.
- `"defaultProjectTrust": "always"` in global settings skips the question.

`AGENTS.md` and `CLAUDE.md` load whether or not the project is trusted, so treat them as untrusted input too.

### Use the project of another folder

`--project-dir <dir>` takes the project from another folder, as if yapi had started there. yapi loads that folder's `.yapi` files and the `AGENTS.md` or `CLAUDE.md` of that folder and its parents, and applies trust to that folder, including `--approve` and `--no-approve`. The working folder's own `.yapi` folder does not load. A relative path is relative to the working folder.

Tools, the session and extensions' `ctx.cwd` stay in the working folder. A native extension finds the project folder with [`project_dir()`](native-extensions.md#where-the-project-is). Subagents use the flag to work in one folder with their parent's project.

## Extensions

Extensions run in WebAssembly instances with memory and compute limits. yapi checks every file, process, network and environment access against the extension's grants. In v0.1 every package gets Pi's default grants, which allow all four, so an extension can do what it could do in Pi. Package manifests that declare a smaller set of grants are [planned](packages.md#planned-manifests-and-registry).

- The process grant implies the other three. A process an extension starts runs as your user, outside the sandbox, and inherits yapi's environment, API keys included. A subagent is such a process: a full yapi with your installed packages.
- yapi never runs an agent session inside an extension's instance, because that session would run tools and use credentials outside the extension's grants.
- Extensions share Pi's `pi.events` bus, so every extension receives every other extension's events.
- Codemode scripts run in a fresh instance with no file, process or network access. They act only through the tools they call.

## MCP servers

MCP servers started over stdio run as normal processes with your permissions. An extension can register a stdio server only with the process grant, and an HTTP server only with the network grant.

## Report a vulnerability

Report vulnerabilities privately, as [SECURITY.md](https://github.com/SkymanOne/yapi/blob/main/SECURITY.md) describes.
