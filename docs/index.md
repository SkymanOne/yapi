# yapi

yapi is a coding agent for the terminal. Given a task and a working folder, it inspects files, runs commands, edits code and works through multi-step tasks. This book covers installing and running yapi, connecting models, customizing it with packages and extensions, and pinning a project's packages with chest.

## How yapi relates to Pi

yapi is a Rust reimplementation of [Pi](https://pi.dev) `v1.0.0`. It uses Pi's commands, settings, session files, packages and extension API, so `yapi import pi` brings an existing Pi setup over without conversion. It ships as a single native binary and doesn't require Node.js.

## Get started with yapi

[Run your first task](quickstart.md) covers installation, connecting a model and a first task. Pi users can copy their setup, as [Move from Pi to yapi](migrating.md) describes. After that:

- [Use yapi in the terminal](interactive.md) to attach files, run commands and direct ongoing work.
- [Choose a model](models.md) and connect a subscription, API key, cloud provider or local model.
- [Manage sessions](sessions.md) to resume work or try another approach without losing history.
- [Configure yapi](configuration.md) with settings, project instructions, skills and prompt templates.

## Customize yapi

Four mechanisms add to what yapi can do:

- [Packages](packages.md) bundle extensions, skills, prompt templates and themes, and install from npm, git or a local path.
- [chest](chest.md) pins a project's skills, subagents and extensions in a manifest and lock. Skills and subagents work with any agent. Extensions load in yapi, each with the capabilities its entry grants.
- [Model Context Protocol (MCP) servers](mcp.md) connect external tools.
- [Extensions](extensions.md) add tools, commands, event handlers and interface components. Pi extensions run unchanged, and [native extensions](native-extensions.md) are written in Rust. [Choose an extension type](extensions.md#choose-an-extension-type) compares the two.

## Automate yapi from other programs

[Print mode, JSON mode and remote procedure call (RPC) mode](automation.md) serve scripts, editors and other programs. The formats and protocols are Pi's, so Pi's clients work with yapi.

## Understand what yapi can access

yapi reads files, edits them and runs commands with your permissions, and it doesn't ask before each tool call. Extensions run in a WebAssembly sandbox with memory and compute limits. Read [Security model and sandboxing](security.md) before you use yapi with untrusted repositories, packages or unattended automation.

## Check compatibility and project status

[Differences from Pi](compat.md) lists every known difference. [Planned work](roadmap.md) lists what comes next, and [Performance compared with Pi](performance.md) compares startup time, memory and install size. Report problems in the [issue tracker](https://github.com/SkymanOne/yapi/issues).
