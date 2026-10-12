# yapi

yapi is a coding agent for the terminal. Give it a goal and a working folder, and it can inspect files, run commands, edit code and work through multi-step tasks.

yapi is a Rust reimplementation of [Pi](https://pi.dev) `v1.0.0`. It uses Pi's commands, settings, session files, packages and extension API, so a Pi setup works in yapi unchanged. It ships as one native binary and needs no Node.js.

## Start using yapi

New to yapi? Follow the [Quickstart](quickstart.md) to install it, connect a model and complete your first task. If you use Pi already, see [Coming from Pi](migrating.md).

- [Use yapi in the terminal](interactive.md) to add files, run commands and direct ongoing work.
- [Choose a model](models.md) or connect a subscription, API key, cloud provider or local model.
- [Manage sessions](sessions.md) to resume work or try another approach without losing history.
- [Configure yapi](configuration.md) with settings, project instructions, skills and prompt templates.

## Customize yapi

- [Packages](packages.md) bundle extensions, skills, prompt templates and themes, and install from npm, git or a local path.
- [MCP servers](mcp.md) connect external tools.
- [Extensions](extensions.md) add tools, commands, event handlers and interface components. Pi extensions run unchanged, and [native extensions](native-extensions.md) are written in Rust.

## Automate yapi

Use [print, JSON and RPC modes](automation.md) from scripts, editors and other programs. The formats and protocols are Pi's, so Pi's clients work with yapi.

## Work safely

yapi reads files, edits them and runs commands with your permissions, and it does not ask before each tool call. Read [Run yapi safely](security.md) before you use it with untrusted repositories, packages or unattended automation.

## Compare with Pi

[Differences from Pi](compat.md) lists every known difference, including Pi features not ported yet. [Performance](performance.md) compares startup time, memory and install size. Report problems in the [issue tracker](https://github.com/SkymanOne/yapi/issues).
