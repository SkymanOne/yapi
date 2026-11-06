<p align="center">
  <a href="https://skymanone.github.io/yapi/">
    <img alt="yapi logo" src="assets/logo.svg" width="242">
  </a>
</p>
<p align="center">
  <strong>Yet Another Pi (in Rust): Fast, Portable, Compatible</strong>
</p>
<p align="center">
  <a href="https://github.com/SkymanOne/yapi/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/SkymanOne/yapi/ci.yml?branch=main&style=flat-square&label=CI" /></a>
  <a href="https://github.com/SkymanOne/yapi/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/SkymanOne/yapi?style=flat-square" /></a>
  <a href="https://skymanone.github.io/yapi/"><img alt="Docs" src="https://img.shields.io/badge/docs-online-blue?style=flat-square" /></a>
  <a href="#license-and-notices"><img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=flat-square" /></a>
</p>

# yapi

yapi is a coding agent for the terminal, written in Rust. It reimplements [Pi](https://pi.dev) `v1.0.0`, reads Pi's settings, sessions, credentials and packages without conversion, and ships as a single native binary with no Node.js dependency. It also ships [`chest`](#manage-packages-with-chest), a package manager for skills, subagents and extensions that works with other agents too.

![yapi in a terminal, answering a question about this repository](assets/screenshot.png)

## Get started

Install the latest release on Linux or macOS. The script asks whether to install `yapi`, its package manager `chest`, or both, and installs both when you press Enter:

```bash
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh \
  | sh
```

Options go after `sh -s --`: `--only yapi` or `--only chest` installs one without the question, and `--from-source` builds the release with Cargo instead of downloading it. chest is also on crates.io as `chest-cli`. [Install yapi](https://skymanone.github.io/yapi/install.html) covers cargo-binstall, building from source, upgrading and uninstalling, for either executable or both.

Start yapi in the folder where you want it to work:

```bash
cd /path/to/project
yapi
```

Run `/login` to connect a subscription or API key, then give yapi a task. Existing Pi users can copy their settings, credentials, sessions and packages with `yapi import pi`.

The [documentation](https://skymanone.github.io/yapi/) covers setup and usage.

## How yapi compares with Pi

yapi keeps Pi's command-line flags, slash commands, file formats, JSON and remote procedure call (RPC) protocols and extension API. The main differences are in how it runs:

| | Pi | yapi |
|---|---|---|
| Runtime | Node.js 22.19 or newer | One native binary |
| Startup to first paint | 324 ms | 10 ms |
| Memory, idle | 117 MB | 19 MB |
| Install size | 247 MB with Node.js | 35 MB |
| Extensions | Run in Pi's process | Run in WebAssembly. Pi extensions run unchanged, and native extensions are written in Rust. |
| Project packages | `settings.json`, without a lock | `chest.toml` and `chest.lock`, with capabilities per extension |

Figures are medians on GitHub's hosted Linux runner. [Performance compared with Pi](https://skymanone.github.io/yapi/performance.html) describes the method and has the macOS results.

yapi's tests use Pi itself as the reference. All 79 of Pi's example extensions, and 418 of the 500 most-downloaded Pi packages on npm, load and register the same tools, commands and event handlers as in Pi. [Differences from Pi](https://skymanone.github.io/yapi/compat.html) lists every known difference.

## Run Pi and native extensions

Pi extensions in TypeScript or JavaScript run unchanged in a bundled QuickJS-NG runtime, so existing Pi packages work without Node.js. Native extensions are written in Rust and compiled to WebAssembly components. They run in an instance of their own as compiled code, at the cost of a build step and an API that's unstable until yapi 1.0.

`yapi new` creates a native extension. [Choose an extension type](https://skymanone.github.io/yapi/extensions.html#choose-an-extension-type) compares the two.

## Manage packages with chest

`chest` is a package manager for coding agents. It installs with yapi as a separate binary, or alone, and you can use it without yapi.

List a project's skills, subagents and extensions in `chest.toml`. chest pins what they resolved to in `chest.lock`, with a content hash for every installed item. Commit both files, and everyone who clones the project installs the same set.

chest splits its features by who reads them:

- **Any agent**: chest places skills and subagents in the folders each agent reads, such as `.agents/skills` and `.claude/skills`. One manifest serves Claude Code, Codex, Cursor, GitHub Copilot, OpenCode and yapi. The lock, `chest install --frozen` in CI, `chest verify` and `chest update` work the same for all of them.
- **yapi only**: extensions, both Pi packages and native WebAssembly extensions, load only in yapi. Each one runs with only the capabilities its entry grants. yapi also puts back missing locked entries when it starts in a trusted project.

`chest init` writes `chest.toml` with folders for the agents it finds. The first `chest add` adds a skill for any agent, the second an extension for yapi with file access, and `chest install --frozen` installs exactly what the lock pins, in CI or after cloning:

```sh
chest init
chest add git:github.com/anthropics/skills --path skills/pdf
chest add git:github.com/acme/pi-todo@4f1c2e9 --grant fs
chest install --frozen
```

A manifest with the same skill, pinned to a tag, and the same extension looks like this:

```toml
[folders]
skills = [".agents/skills", ".claude/skills"]

[skills.pdf]
git = "https://github.com/anthropics/skills"
rev = "v1.2.0"
path = "skills/pdf"

[extensions.todo]
git = "https://github.com/acme/pi-todo"
rev = "4f1c2e9"
capabilities = ["fs"]
```

[Get started with chest](https://skymanone.github.io/yapi/chest.html) covers the rest.

## Security model and sandboxing

yapi's built-in tools run with your permissions, and yapi doesn't ask before each tool call. For untrusted repositories or unattended work, run yapi in a container or virtual machine.

Extensions run in WebAssembly instances with memory and compute limits, and the host checks their file, process, network, environment and session access against grants. chest extensions run with the capabilities their manifest grants. Packages from `yapi install` keep Pi's grants, so they can do what they could do in Pi.

Codemode scripts, which the model writes to call tools, run with no file, process or network access. [Security model and sandboxing](https://skymanone.github.io/yapi/security.html) has the details.

## What's planned

The next pieces of work are:

- File grants scoped to paths and read-only access, and network grants scoped to hosts
- A registry for yapi packages
- A stable native extension API in yapi 1.0

[Planned work](https://skymanone.github.io/yapi/roadmap.html) has the details.

## Skills for coding agents

The repository ships two [Agent Skills](https://agentskills.io): `yapi`, for running and scripting yapi, and `yapi-extension`, for writing extensions. Install them with:

```sh
yapi install git:github.com/SkymanOne/yapi@v0.4.1
```

To pin them in a project for yapi and other agents, add them with chest instead:

```sh
chest add git:github.com/SkymanOne/yapi@v0.4.1 --skill --name yapi-skills
```

## Contribute to yapi

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to build yapi and run its checks. Report problems in the [issue tracker](https://github.com/SkymanOne/yapi/issues).

## License and notices

MIT OR Apache-2.0, at your option. Vendored Pi code keeps its MIT notices.
