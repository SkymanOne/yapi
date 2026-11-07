# yapi-agent

The agent loop of [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages. It sends the conversation to the model, runs the tools the model calls, and repeats until the model stops calling them. Tools report progress as they run and stop when the run is cancelled. A message the user sends in the meantime waits in a queue: a steering message joins at the next turn, and a follow-up when the agent would otherwise finish. It ports Pi's pi-agent-core.

`yapi-core` builds on this crate and requires its exact version. It's an internal library with no stable API, so any release can change it. To write a yapi extension, use [`yapi-extension-api`](https://crates.io/crates/yapi-extension-api) instead. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option.
