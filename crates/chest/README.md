# chest-cli

[chest](https://skymanone.github.io/yapi/chest.html) is a package manager for agent skills, subagents and [yapi](https://github.com/SkymanOne/yapi) extensions. A project's `chest.toml` lists them, `chest.lock` pins their commits and versions, and chest installs them into the folders each agent reads, such as `.agents/skills` and `.claude/skills`. It works with yapi, Claude Code and other agents. In yapi, each extension gets only the capabilities its entry grants.

```sh
cargo install --locked chest-cli
```

The crate installs the `chest` executable and needs Rust 1.91 or newer. `cargo binstall chest-cli` installs the release build instead. The [chest guide](https://skymanone.github.io/yapi/chest.html) covers the manifest, the lock and every command.

## License

MIT OR Apache-2.0, at your option.
