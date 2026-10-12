# yapi-types

Serde types for the JSON formats of the [Pi](https://pi.dev) coding agent, as [yapi](https://github.com/SkymanOne/yapi) uses them: messages, events, session entries, settings, models and extension payloads. Pi's files are handled as order-preserving documents, so they round-trip byte for byte.

yapi publishes this crate as a dependency of `yapi-tui` and [`yapi-extension-api`](https://crates.io/crates/yapi-extension-api). Its API follows yapi's needs and may change in any release before 1.0. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option.
