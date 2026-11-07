# yapi-ai

The model layer of [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages. It streams replies over each provider's wire API, such as Anthropic Messages, OpenAI Responses and the Gemini API. It also keeps the model catalog and registry and works out what each request costs. It stores each provider's credentials, and signs in with OAuth for subscriptions such as GitHub Copilot. It ports Pi's pi-ai.

`yapi-agent`, `yapi-core` and [`chest-cli`](https://crates.io/crates/chest-cli) build on this crate and require its exact version. It's an internal library with no stable API, so any release can change it. To write a yapi extension, use [`yapi-extension-api`](https://crates.io/crates/yapi-extension-api) instead. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option.
