# chest commands

> **Any agent**, except the parts that only matter in yapi: `chest check`, `chest import --settings`, and the extension options `--extension` and `--grant` of `chest add` and `--store` of `chest init`.

This page lists every chest command with its flags, the states of an entry, how chest uses a proxy, and its exit codes. Every command takes `-g`, for the global manifest, or `--manifest <path>`, but not both. Without either, `CHEST_MANIFEST` names the manifest, else `chest.toml` in the current folder.

| Command | Flags | What it does |
|---|---|---|
| `chest init` | `--skills a,b`, `--agents a,b`, `--store <dir>`, `--package`, `--yes` | Writes a manifest with folders for the agents found. Refuses an existing manifest unless `--package`, which adds `[package]`. |
| `chest add <source>` | `--skill [name]`, `--agent`, `--extension`, `--name <name>`, `--path <path>`, `--rev <rev>`, `--version <range>`, `--grant a,b`, `--yes` | Adds an entry and installs it. The source is `npm:<name>[@<range>]`, `git:<repo>[@<rev>]`, a git URL or a path. A git URL starts with `https://`, `http://`, `ssh://` or `git://`, or takes the form `user@host:path`. Without a section flag, chest infers the section from the package. `--skill <name>` adds the skill of that name from the source's `skills` folder, as `--path skills/<name>` does, and repeats for more skills, such as `chest add git:github.com/apollographql/skills --skill rust-best-practices`. `--rev` is for git sources and `--version` for npm ones, and neither may repeat a revision or version the source names. The same name with the same source updates its version or rev and keeps its grants, so `--grant` then fails unless it lists the grants the entry has. The same name with another source or section is an error. So is a second name for a source and `path` that an entry of the section already installs, at any version or rev. |
| `chest remove <name>` | | Removes the entry and what chest installed for it. It also removes the entry's `[patch]`, but not the patch's folder, which isn't chest's. |
| `chest install` | `--frozen` | Installs what the manifest lists, as the lock pins it, and writes the lock. With `--frozen`, installs exactly what the lock pins and fails on any difference between the manifest and the lock, writing neither. |
| `chest update [names]` | | Moves the named entries, or all of them, to the newest version or commit their source allows, and writes the lock. See [Update entries](chest-lock.md#update-entries). |
| `chest list` | | Lists each entry with its locked version or commit, its [state](#entry-states), its capabilities and its description. The description is an extension package's, or the `description` in the frontmatter of a skill or subagent. |
| `chest outdated` | | Lists entries whose source has something newer. |
| `chest verify` | | Checks installed copies against the lock, and copies of path and `[patch]` entries against what chest installed. Reports a path entry whose source is gone. |
| `chest import` | `--settings` or `--skills-lock`, `[path]` | Turns the packages of settings.json or settings.toml, or a skills-lock.json, into entries. |
| `chest check [path]` | | Compares the capabilities that the native extensions of the package at `path`, or the `.wasm` file, use with the ones the package requests in its `chest.toml` or `package.json`. `path` is the current folder by default. See [Check a native extension](chest-publish.md#check-a-native-extension). |

## Entry states

`chest list` shows each entry in one of these states:

- **`installed`**: in place as the lock pins it
- **`edited`**: a copy changed since chest installed it, whatever the lock pins now
- **`outdated`**: in place as an older lock pinned it
- **`stale`**: not locked, or the lock no longer matches the manifest
- **`patched`**: under `[patch]`
- **`missing`**: locked but not in place
- **`no source`**: a path entry whose source is gone. `chest install` skips it with a warning and installs the other entries, and `chest install --frozen` fails on it. Restore the folder or remove the entry.

yapi acts on the same states at startup. It doesn't hash copies, so it treats an `edited` entry as `installed`, or as `outdated` when the lock pins another version.

## Use a proxy

chest reads proxy variables such as `HTTPS_PROXY`, as yapi does. Without them, it uses the `httpProxy` setting in yapi's global `settings.json`, like yapi.

## Exit codes and prompts

chest exits with 0 on success, 1 when a command fails or `verify` or `check` finds a mismatch, and 2 on a usage error.

At a terminal, `chest init` and `chest add` ask their questions as lists you move through with the arrow keys, tick with Space and answer with Enter. Each shows a summary and asks you to confirm before it writes anything. Esc or Ctrl+C at a question cancels the command, which writes nothing and exits with 1. `chest install`, `chest update`, `chest remove` and `chest import` show a spinner and a summary of each entry's outcome.

To skip the questions:

- Pass the flag that answers one, such as `--skills` for `chest init` or `--path` for `chest add`.
- Pass `--yes` to take the defaults and skip the confirmation.
- Run chest without a terminal, or with `CI` set. chest then asks nothing and prints plain lines, with each entry's outcome on stdout.

chest draws on stderr, in color unless `NO_COLOR` is set.
