# yapi-core

The core of [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages. It holds sessions and compaction, settings, resource loading, the built-in tools, the system prompt, packages, the extension runner and its grants, MCP, and the install engine of chest. It ports the core of Pi's pi-coding-agent and pi-mcp.

yapi publishes this crate as a dependency of [`chest-cli`](https://crates.io/crates/chest-cli). It's an internal library with no stable API. Any release can change it, and the crates that use it require its exact version. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option.
