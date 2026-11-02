<p align="center">
  <a href="https://skymanone.github.io/yapi/">
    <img alt="yapi logo" src="assets/logo.svg" width="242">
  </a>
</p>
<p align="center">
  <strong>Yet Another Pi (in Rust) - Fast, Portable, Compatible</strong>
</p>
<p align="center">
  <a href="https://github.com/SkymanOne/yapi/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/SkymanOne/yapi/ci.yml?branch=main&style=flat-square&label=CI" /></a>
  <a href="https://github.com/SkymanOne/yapi/releases/latest"><img alt="Release" src="https://img.shields.io/github/v/release/SkymanOne/yapi?style=flat-square" /></a>
  <a href="https://skymanone.github.io/yapi/"><img alt="Docs" src="https://img.shields.io/badge/docs-online-blue?style=flat-square" /></a>
  <a href="#license"><img alt="License" src="https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue?style=flat-square" /></a>
</p>

# yapi

yapi is a coding agent for the terminal, written in Rust. It reimplements [Pi](https://pi.dev) `v1.0.0`, reads Pi's settings, sessions, credentials and packages without conversion, and ships as a single native binary with no Node.js dependency. It also ships `chest`, a package manager for skills, subagents and extensions.

![yapi in a terminal, answering a question about this repository](assets/screenshot.png)

## Getting started

Install the latest release on Linux or macOS. The script installs `yapi` and `chest`, its package manager:

```bash
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh
```

[Install yapi](https://skymanone.github.io/yapi/install.html) covers cargo-binstall, building from source, upgrading and uninstalling.

Start yapi in the directory where you want it to work:

```bash
cd /path/to/project
yapi
```

Run `/login` to connect a subscription or API key, then give yapi a task. Existing Pi users can copy their settings, credentials, sessions and packages with `yapi import pi`.

The [documentation](https://skymanone.github.io/yapi/) covers setup and usage.

## yapi and Pi

yapi keeps Pi's command-line flags, slash commands, file formats, JSON and RPC protocols and extension API. The main differences are in how it runs:

| | Pi | yapi |
|---|---|---|
| Runtime | Node.js 22.19 or newer | One native binary |
| Startup to first paint | 324 ms | **10 ms** |
| Memory, idle | 117 MB | **19 MB** |
| Install size | 247 MB with Node.js | **35 MB** |
| Extensions | Run in Pi's process | Run in WebAssembly. Pi extensions run unchanged, and native extensions are written in Rust. |
| Project packages | `settings.json`, without a lock | `chest.toml` and `chest.lock`, with capabilities per extension |

Figures are medians on GitHub's hosted Linux runner. [Performance](https://skymanone.github.io/yapi/performance.html) describes the method and has the macOS results.

Compatibility is tested against Pi itself. All 79 of Pi's example extensions, and 418 of the 500 most-downloaded Pi packages on npm, load and register the same tools, commands and event handlers as in Pi. [Differences from Pi](https://skymanone.github.io/yapi/compat.html) lists every known difference.

## Extensions

Pi extensions in TypeScript or JavaScript run unchanged in a bundled QuickJS-NG runtime, so existing Pi packages work without Node.js. Native extensions are written in Rust and compiled to WebAssembly components. They run in an instance of their own as compiled code, at the cost of a build step and an API that is unstable until yapi 1.0. `yapi new` creates one. [Choose an extension type](https://skymanone.github.io/yapi/extensions.html#choose-an-extension-type) compares the two.

## Project packages with chest

`chest` keeps a project's skills, subagents and extensions in one manifest, `chest.toml`, and pins what they resolved to in a lock, `chest.lock`, with a content hash of every installed item. Each extension runs with only the capabilities its entry grants. Skills and subagents go into the folders each agent reads, so one manifest serves yapi, Claude Code, Codex and other agents.

```sh
chest init
chest add git:github.com/anthropics/skills --path skills/pdf
chest add npm:@acme/pi-subagent --grant all
chest install --frozen    # in CI, or after cloning
```

```toml
[folders]
skills = [".agents/skills", ".claude/skills"]

[skills]
pdf = { git = "https://github.com/anthropics/skills", rev = "v1.2.0", path = "skills/pdf" }

[extensions]
todo = { git = "https://github.com/acme/pi-todo", rev = "4f1c2e9", capabilities = ["fs"] }
```

[Manage project packages with chest](https://skymanone.github.io/yapi/chest.html) is the guide.

## Security model and sandboxing

yapi's built-in tools run with the permissions of the user who starts it, and yapi does not ask before each tool call. For untrusted repositories or unattended work, run yapi in a container or virtual machine.

Extensions run in WebAssembly instances with memory and compute limits, and the host checks their file, process, network, environment and session access against grants. chest extensions run with the capabilities their manifest grants. Packages from `yapi install` keep Pi's grants, so they can do what they could do in Pi. Codemode scripts run with no file, process or network access. [Security model and sandboxing](https://skymanone.github.io/yapi/security.html) has the details.

## Roadmap

- File grants scoped to paths and read-only access, and network grants scoped to hosts
- A registry for yapi packages
- A stable native extension API in yapi 1.0

The [Roadmap](https://skymanone.github.io/yapi/roadmap.html) has the details.

## Agent skills

The repository ships two [Agent Skills](https://agentskills.io): `yapi`, for running and scripting yapi, and `yapi-extension`, for writing extensions. Install them with:

```sh
yapi install git:github.com/SkymanOne/yapi@v0.3.1
```

To pin them in a project for yapi and other agents, add them with chest instead:

```sh
chest add git:github.com/SkymanOne/yapi@v0.3.1 --skill --name yapi-skills
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to build yapi and run its checks. Report problems in the [issue tracker](https://github.com/SkymanOne/yapi/issues).

## License

MIT OR Apache-2.0, at your option. Vendored Pi code keeps its MIT notices.
