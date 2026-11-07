# yapi-core

The core of [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages. It runs a coding session: the session file and its compaction, the settings, the system prompt, and the built-in tools such as `read`, `edit` and `bash`. It also loads packages and what they provide, runs extensions within their grants, connects to MCP servers, and holds the install engine behind [chest](https://crates.io/crates/chest-cli). It ports the core of Pi's pi-coding-agent and pi-mcp.

[`chest-cli`](https://crates.io/crates/chest-cli) builds on this crate and requires its exact version. It's an internal library with no stable API, so any release can change it. To write a yapi extension, use [`yapi-extension-api`](https://crates.io/crates/yapi-extension-api) instead. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option.
