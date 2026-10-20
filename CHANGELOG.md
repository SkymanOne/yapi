# Changelog

Notable changes to yapi, in the format of [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). yapi follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- Update wasmtime to 49.0.4 for RUSTSEC-2026-0378 and RUSTSEC-2026-0379: a host panic on mismatched component streams, and wrong jumps in aarch64 code for large `br_table` instructions ([#186](https://github.com/SkymanOne/yapi/pull/186)).

## [0.2.0] - 2026-10-14

- Add Mermaid diagrams and LaTeX math to messages. Extensions get Pi's entry renderers and Markdown transformers, and code renderers (`pi.yapi.registerCodeRenderer`, `Api::register_code_renderer`) that can highlight code ([#167](https://github.com/SkymanOne/yapi/pull/167), [#168](https://github.com/SkymanOne/yapi/pull/168), [#169](https://github.com/SkymanOne/yapi/pull/169), [#170](https://github.com/SkymanOne/yapi/pull/170)).
- Add Codex streaming over a WebSocket, as Pi does, which continues from the previous response and falls back to SSE ([#163](https://github.com/SkymanOne/yapi/pull/163), [#165](https://github.com/SkymanOne/yapi/pull/165)).
- Add prompt cache warming with the `cacheWarming` setting, cache statistics in `/session` and the "Cache miss notices" setting ([#150](https://github.com/SkymanOne/yapi/pull/150), [#158](https://github.com/SkymanOne/yapi/pull/158)).
- Add startup notices for new yapi releases and package updates. `--offline`, `PI_OFFLINE` and `PI_SKIP_VERSION_CHECK` turn them off ([#151](https://github.com/SkymanOne/yapi/pull/151)).
- Add grammar-constrained custom tools for the OpenAI APIs, which codemode uses ([#152](https://github.com/SkymanOne/yapi/pull/152)).
- Add TIFF, ICO, PNM, HDR, DDS and farbfeld images, converted to PNG before they reach the model ([#162](https://github.com/SkymanOne/yapi/pull/162)).
- Show local times in `/tree` labels, extensions and codemode, and honor `timeZone` in extensions' date formatting ([#161](https://github.com/SkymanOne/yapi/pull/161), [#179](https://github.com/SkymanOne/yapi/pull/179)).
- Let codemode scripts wait for the MCP servers they call to finish connecting ([#153](https://github.com/SkymanOne/yapi/pull/153)).
- Change `yapi-tui`'s `MarkdownTheme`, which gains a `highlight_code` field. Code that builds it field by field must set it ([#167](https://github.com/SkymanOne/yapi/pull/167)).
- Fix `yapi config` not installing configured packages that are missing ([#156](https://github.com/SkymanOne/yapi/pull/156)).
- Fix `sendUserMessage` expanding prompt templates. As in Pi, it expands them only with `expandPromptTemplates` ([#157](https://github.com/SkymanOne/yapi/pull/157)).
- Fix custom messages sent with `triggerTurn: false` missing from the next turn's context ([#171](https://github.com/SkymanOne/yapi/pull/171)).
- Fix a prompt typed while an extension command runs being dropped ([#172](https://github.com/SkymanOne/yapi/pull/172)).
- Fix `PI_CACHE_RETENTION` in a provider's environment being ignored ([#180](https://github.com/SkymanOne/yapi/pull/180)).
- Fix yapi aborting on macOS with "mach_msg failed" when a signal reached the extension runtime's exception thread ([#183](https://github.com/SkymanOne/yapi/pull/183)).
- Update wasmtime to 49.0.3, which fixes host panics in five security advisories, and rustls to 0.23.46 for RUSTSEC-2026-0346 ([#183](https://github.com/SkymanOne/yapi/pull/183), [#175](https://github.com/SkymanOne/yapi/pull/175)).

### Known issues

- Sign in with ChatGPT can fail with `invalid_grant` after the browser step, as it does in Pi ([#181](https://github.com/SkymanOne/yapi/issues/181)). The OpenAI Codex login works.

### Install

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh
```

[Install](https://skymanone.github.io/yapi/install.html) covers cargo-binstall, direct downloads and building from source.

## [0.1.0] - 2026-10-07

Yet Another Pi (in Rust) - Fast, Portable, Compatible.

The first release of yapi, a Rust reimplementation of the [Pi](https://github.com/earendil-works/pi) coding agent. It follows Pi `v1.0.0`, reads and writes Pi's files and runs Pi packages unchanged, as a single binary that needs no Node.js.

### Added

- Interactive, print, JSON and RPC modes, with Pi's flags, slash commands and protocols.
- Sessions in Pi's JSONL format, with branching, `/tree`, `/fork`, `/clone` and compaction.
- The terminal interface: Pi's editor, keybindings, themes and selectors.
- Every wire API and provider in Pi's model catalog. Sign-in with API keys or with subscriptions such as Claude Pro and Max, ChatGPT and GitHub Copilot. See [Models and sign-in](https://skymanone.github.io/yapi/models.html).
- Pi packages from npm, git and local paths. Pi extensions run unchanged in a WebAssembly sandbox, including their interface components, every extension event, session actions, custom editors, terminal input listeners, autocomplete providers and providers with their own streams and sign-in.
- Images are resized and converted as Pi does before they reach the model.
- Native extensions in Rust, built as WebAssembly components, with `async` handlers, dialogs, HTTP, processes and background tasks. `yapi new` creates one.
- Subagents and other long-running processes in extensions. `child_process.spawn` streams output and takes input and signals, so Pi's `subagent` example runs unchanged. Pi's `RpcClient` runs and drives another yapi, and a native `subagent` example works in the foreground or the background. Extension tools stop when the user aborts the run.
- Context files, skills, prompt templates and project trust.
- MCP servers over stdio and streamable HTTP, with OAuth sign-in for HTTP servers (`yapi mcp login`), and codemode.
- `yapi import pi`, which copies Pi's settings, credentials, sessions and packages into yapi's directories.
- Documentation for the model. yapi's pages and Pi's docs and examples install with yapi, or download on its first start, so the model can explain yapi and the extension API it implements.

### Known issues

- Requests to Amazon Bedrock, Google Vertex AI and Cloudflare, and the llama.cpp router, are tested against mock servers only. They are not yet checked against the live services.
- CI runs the `x86_64-apple-darwin` archive only under Rosetta on Apple silicon, not on an Intel Mac.
- Native WebAssembly extensions are unstable until yapi 1.0. The WIT world, the Rust SDK and the host requests they use may change in any release before then, and extensions may need to be rebuilt. Pi extensions from npm use Pi's extension API and are not affected.
- [Differences from Pi](https://skymanone.github.io/yapi/compat.html) lists every intentional difference, and its Open Gaps table links an issue for each Pi feature not yet ported.

### Install

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh
```

[Install](https://skymanone.github.io/yapi/install.html) covers cargo-binstall, direct downloads and building from source.

[Unreleased]: https://github.com/SkymanOne/yapi/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/SkymanOne/yapi/releases/tag/v0.2.0
[0.1.0]: https://github.com/SkymanOne/yapi/releases/tag/v0.1.0
