# yapi-agent

The agent loop of [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages. It streams the model's replies, runs the tools the model calls with progress updates and cancellation, and handles steering and follow-up queues and their events. It ports Pi's pi-agent-core.

yapi publishes this crate as a dependency of `yapi-core`, which [`chest-cli`](https://crates.io/crates/chest-cli) builds on. It's an internal library with no stable API. Any release can change it, and the crates that use it require its exact version. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option.
