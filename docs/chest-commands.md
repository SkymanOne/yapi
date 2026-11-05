# chest commands

> **Any agent**, except the parts that only matter in yapi: `chest check`, `chest import --settings`, and the extension options `--extension` and `--grant` of `chest add` and `--store` of `chest init`.

This page lists every chest command with its flags, then how chest uses a proxy, and its exit codes. Every command takes `-g`, for the global manifest, and `--manifest <path>`. Without either, `CHEST_MANIFEST` names the manifest, else `chest.toml` in the current folder.

| Command | Flags | What it does |
|---|---|---|
| `chest init` | `--skills a,b`, `--agents a,b`, `--store <dir>`, `--package`, `--yes` | Writes a manifest with folders for the agents found. Refuses an existing manifest unless `--package`, which adds `[package]`. |
| `chest add <source>` | `--skill`, `--agent`, `--extension`, `--name <name>`, `--path <path>`, `--rev <rev>`, `--version <range>`, `--grant a,b`, `--yes` | Adds an entry and installs it. The source is `npm:<name>[@<range>]`, `git:<repo>[@<rev>]`, a git URL or a path. A git URL starts with `https://`, `http://`, `ssh://` or `git://`, or takes the form `user@host:path`, where the path is relative to the remote home. An absolute path takes `ssh://`, as in `ssh://user@host/srv/repo`. Without a section flag, chest infers the section from the package. The same name with the same source updates its version or rev and keeps its grants. The same name with another source, or in another section, is an error. |
| `chest remove <name>` | | Removes the entry and what chest installed for it. |
| `chest install` | `--frozen` | Installs what the manifest lists, as the lock pins it, and writes the lock. With `--frozen`, installs exactly what the lock pins and fails on any difference between the manifest and the lock, writing neither. |
| `chest update [names]` | | Moves the entries, or all, to the newest version or commit their source allows. Shows an entry whose version, commit or path source didn't change as `unchanged`. A path extension shows as `unchanged` even after its files change, since yapi loads it from its source. |
| `chest list` | | Lists each entry with its locked version or commit, its state, its capabilities and description. `installed`: in place as the lock pins it. `edited`: a copy changed since chest installed it, whatever the lock pins now. `outdated`: in place as an older lock pinned it. `stale`: not locked, or the lock no longer matches the manifest. `patched`: under `[patch]`. `missing`: locked but not in place. `no source`: a path entry whose source is gone. `chest install` then skips it with a warning and installs the other entries. When the folder held several items, chest can't tell their copies from dropped ones, so it keeps every copy in that section, and lists them, until the folder is back. `chest install --frozen` fails on it. Restore the folder or remove the entry. yapi acts on the same states at startup. It doesn't hash copies, so it treats an `edited` entry as `installed`, or as `outdated` when the lock pins another version. |
| `chest outdated` | | Lists entries whose source has something newer. |
| `chest verify` | | Checks installed copies against the lock, and copies of path and `[patch]` entries against what chest installed. Reports a path entry whose source is gone. |
| `chest import` | `--settings` or `--skills-lock`, `[path]` | Turns settings.json packages or a skills-lock.json into entries. |
| `chest check [path]` | | Compares the capabilities that the native extensions of the package at `path`, or the `.wasm` file, use with the ones the package requests in its `chest.toml` or `package.json`. `path` is the current folder by default. A `.wasm` file belongs to the nearest folder above it with either file. Fails when the package's `[package]` doesn't parse. |

## Proxy

chest reads proxy variables such as `HTTPS_PROXY`, as yapi does. Without them, it uses the `httpProxy` setting in yapi's global `settings.json`, like yapi.

## Exit codes and prompts

chest exits with 0 on success, 1 when a command fails or `verify` or `check` finds a mismatch, and 2 on a usage error. Questions go to stderr, and chest asks them only at a terminal.
