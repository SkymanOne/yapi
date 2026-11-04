# chest commands

> **Any agent**, except the parts that only matter in yapi: `chest check`, `chest import --settings`, and the extension options `--extension` and `--grant` of `chest add` and `--store` of `chest init`.

Every command takes `-g`, for the global manifest, and `--manifest <path>`. Without either, `CHEST_MANIFEST` names the manifest, else `chest.toml` in the current folder.

| Command | Flags | What it does |
|---|---|---|
| `chest init` | `--skills a,b`, `--agents a,b`, `--store <dir>`, `--package`, `--yes` | Writes a manifest with folders for the agents found. Refuses an existing manifest unless `--package`, which adds `[package]`. |
| `chest add <source>` | `--skill`, `--agent`, `--extension`, `--name <name>`, `--path <path>`, `--rev <rev>`, `--version <range>`, `--grant a,b`, `--yes` | Adds an entry and installs it. The source is `npm:<name>[@<range>]`, `git:<repo>[@<rev>]`, a git URL or a path. Without a section flag, chest infers the section from the package. The same name with the same source updates its version or rev and keeps its grants. The same name with another source, or in another section, is an error. |
| `chest remove <name>` | | Removes the entry and what chest installed for it. |
| `chest install` | `--frozen` | Installs what the manifest lists, as the lock pins it, and writes the lock. With `--frozen`, installs exactly what the lock pins and fails on any difference between the manifest and the lock, writing neither. |
| `chest update [names]` | | Moves the entries, or all, to the newest version or commit their source allows. |
| `chest list` | | Lists each entry with its locked version or commit, whether it is installed, its capabilities and description. |
| `chest outdated` | | Lists entries whose source has something newer. |
| `chest verify` | | Checks installed copies against the lock. |
| `chest import` | `--settings` or `--skills-lock`, `[path]` | Turns settings.json packages or a skills-lock.json into entries. |
| `chest check [path]` | | Compares the capabilities that the native extensions of the package at `path`, or the `.wasm` file, use with the ones the package requests in its `chest.toml` or `package.json`. `path` is the current folder by default. A `.wasm` file belongs to the nearest folder above it with either file. Fails when the package's `[package]` does not parse. |

chest exits with 0 on success, 1 when a command fails or `verify` or `check` finds a mismatch, and 2 on a usage error. Questions go to stderr, and chest asks them only at a terminal.
