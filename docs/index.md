# yapi

yapi is a coding agent for the terminal. Given a task and a working folder, it inspects files, runs commands, edits code and works through multi-step tasks.

yapi is a Rust reimplementation of [Pi](https://pi.dev) `v1.0.0`. It uses Pi's commands, settings, session files, packages and extension API, so an existing Pi setup works in yapi without changes. It ships as a single native binary and does not require Node.js.

## Get started

The [Quickstart](quickstart.md) covers installation, connecting a model and a first task. Pi users can bring their existing setup, as [Coming from Pi](migrating.md) describes.

- [Use yapi in the terminal](interactive.md) to attach files, run commands and direct ongoing work.
- [Choose a model](models.md) and connect a subscription, API key, cloud provider or local model.
- [Manage sessions](sessions.md) to resume work or try another approach without losing history.
- [Configure yapi](configuration.md) with settings, project instructions, skills and prompt templates.

## Customize yapi

- [Packages](packages.md) bundle extensions, skills, prompt templates and themes, and install from npm, git or a local path.
- [MCP servers](mcp.md) connect external tools.
- [Extensions](extensions.md) add tools, commands, event handlers and interface components. Pi extensions run unchanged, and [native extensions](native-extensions.md) are written in Rust. [Choose an extension type](extensions.md#choose-an-extension-type) compares the two.

## Automate yapi

[Print, JSON and RPC modes](automation.md) serve scripts, editors and other programs. The formats and protocols are Pi's, so Pi's clients work with yapi.

## Security

yapi reads files, edits them and runs commands with the user's permissions, and it does not ask before each tool call. Extensions run in a WebAssembly sandbox with memory and compute limits. Read [Security model and sandboxing](security.md) before using yapi with untrusted repositories, packages or unattended automation.

## Project status

[Differences from Pi](compat.md) lists every known difference. The [Roadmap](roadmap.md) lists planned work, and [Performance](performance.md) compares startup time, memory and install size with Pi. Report problems in the [issue tracker](https://github.com/SkymanOne/yapi/issues).
