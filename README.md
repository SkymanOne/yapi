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

yapi is a coding agent for the terminal, written in Rust. It is a reimplementation of [Pi](https://pi.dev) `v1.0.0` that reads Pi's settings, sessions, credentials and packages unchanged, and ships as one native binary with no Node.js dependency.

![yapi in a terminal, answering a question about this repository](assets/screenshot.png)

## Getting started

Install the latest release on Linux or macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh
```

[Install yapi](https://skymanone.github.io/yapi/install.html) covers cargo-binstall, building from source, upgrading and uninstalling.

Start yapi in the directory where you want it to work:

```bash
cd /path/to/project
yapi
```

Run `/login` to connect a subscription or API key, then give yapi a task. If you already use Pi, `yapi import pi` copies your settings, credentials, sessions and packages.

See the [documentation](https://skymanone.github.io/yapi/) for setup and usage.

## yapi and Pi

yapi keeps Pi's command-line flags, slash commands, file formats, JSON and RPC protocols and extension API. It differs in how it runs:

| | Pi | yapi |
|---|---|---|
| Runtime | Node.js 22.19 or newer | One native binary |
| Startup to first paint | 324 ms | 10 ms |
| Memory, idle | 117 MB | 19 MB |
| Install size | 247 MB with Node.js | 35 MB |
| Extensions | Run in Pi's process | Run in WebAssembly. Pi extensions run unchanged, and native extensions are written in Rust. |

Figures are medians on GitHub's hosted Linux runner. [Performance](https://skymanone.github.io/yapi/performance.html) describes the method and has the macOS results.

Compatibility is tested against Pi itself. All 79 of Pi's example extensions, and 418 of the 500 most-downloaded Pi packages on npm, load and register the same tools, commands and event handlers as in Pi. [Differences from Pi](https://skymanone.github.io/yapi/compat.html) lists every known difference.

## Agent skills

The repository ships two [Agent Skills](https://agentskills.io): `yapi`, for running and scripting yapi, and `yapi-extension`, for writing extensions. Install them with:

```sh
yapi install git:github.com/SkymanOne/yapi@v0.1.0
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for how to build yapi and run its checks. Report problems in the [issue tracker](https://github.com/SkymanOne/yapi/issues).

## License

MIT OR Apache-2.0, at your option. Vendored Pi code keeps its MIT notices.
