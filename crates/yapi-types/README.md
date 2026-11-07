# yapi-types

The JSON types of [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages. They describe Pi's messages, events, session entries, settings, models and extension payloads with serde. Session files, RPC mode and the calls between yapi and its extensions all use them. yapi keeps each of Pi's files as a document in its own key order, so a file it writes back matches the one it read byte for byte.

`yapi-tui` builds on this crate, and so does the `widgets` feature of [`yapi-extension-api`](https://crates.io/crates/yapi-extension-api) through it. Its API follows yapi's needs, so any release before 1.0 can change it. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option.
