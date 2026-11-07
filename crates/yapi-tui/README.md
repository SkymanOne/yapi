# yapi-tui

The terminal UI of [yapi](https://github.com/SkymanOne/yapi), a Rust coding agent that runs [Pi](https://pi.dev) packages, built on `ratatui-core`. It ports Pi's pi-tui: widgets such as `SelectList`, `Editor` and `SettingsList`, Markdown with LaTeX and Mermaid, and the decoder for raw terminal input. As in pi-tui, a component renders styled lines for a given width, and an ANSI bridge turns its text into those lines.

The `widgets` feature of [`yapi-extension-api`](https://crates.io/crates/yapi-extension-api) builds on this crate, so native extensions can draw the same widgets as yapi. Its API follows yapi's needs, so any release before 1.0 can change it. The crate needs Rust 1.91 or newer.

## License

MIT OR Apache-2.0, at your option. `themes/dark.json` and `themes/light.json` are Pi's built-in themes, under the MIT License, as `themes/README.md` states. `src/mermaid` ports the grok-mermaid renderer, under the Apache License 2.0, as `src/mermaid/LICENSES.md` states.
