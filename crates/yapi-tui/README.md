# yapi-tui

Terminal UI components for [yapi](https://github.com/SkymanOne/yapi), on `ratatui-core`. It ports Pi's pi-tui: widgets such as `SelectList`, `Editor` and `SettingsList`, raw input decoding and an ANSI bridge. Components render styled lines for a width, as in pi-tui.

yapi publishes this crate for the `widgets` feature of [`yapi-extension-api`](https://crates.io/crates/yapi-extension-api). Its API follows yapi's needs and may change in any release before 1.0. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option. `themes/dark.json` and `themes/light.json` are Pi's built-in themes, under the MIT License, as `themes/README.md` states.
