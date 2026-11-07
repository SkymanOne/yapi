# chest-cli

[chest](https://skymanone.github.io/yapi/chest.html) is a package manager for agent skills, subagents and [yapi](https://github.com/SkymanOne/yapi) extensions. A project's `chest.toml` lists them, `chest.lock` pins their commits and versions, and chest installs them into the folders each agent reads, such as `.agents/skills` for yapi and `.claude/skills` for Claude Code. It works without yapi too. In yapi, each extension gets only the capabilities its entry grants, such as reading files or using the network.

```sh
cargo install --locked chest-cli
chest init
chest add git:github.com/SkymanOne/yapi --skill --name yapi-skills
```

`chest init` writes a manifest with a folder for each agent it finds, and `chest add` adds the skills from yapi's repository and installs them. To install a release build instead of compiling chest, run `cargo binstall chest-cli`. The crate installs the `chest` executable and needs Rust 1.91 or newer. The [chest guide](https://skymanone.github.io/yapi/chest.html) covers the manifest, the lock and every command.

## License

MIT OR Apache-2.0, at your option.
