# yapi-ai

LLM wire APIs for [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages: the provider APIs, the model catalog and registry, cost tracking, credentials and OAuth sign-in. It ports Pi's pi-ai.

yapi publishes this crate as a dependency of `yapi-agent`, `yapi-core` and [`chest-cli`](https://crates.io/crates/chest-cli). It's an internal library with no stable API. Any release can change it, and the crates that use it require its exact version. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option.
