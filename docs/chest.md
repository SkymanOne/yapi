# Manage project packages with chest

chest keeps a project's agent skills, subagents and yapi extensions in one manifest, `chest.toml`, and pins exactly what they resolved to in a lock, `chest.lock`. Commit both, and everyone who clones the project installs the same bytes with `chest install`.

## What chest is

- One manifest and one lock for three kinds of entries: skills (folders with a `SKILL.md`), subagents (`.md` files) and yapi extensions (Pi packages and native `.wasm` extensions).
- It works for other agents too. Skills and subagents land in the folders that Claude Code, Codex, Cursor, GitHub Copilot, OpenCode and yapi read, so a team can share one manifest whatever agent each person runs.
- The lock pins git commits, npm versions with their full dependency trees, and a content hash of everything installed.
- Each extension gets only the capabilities its entry grants.

`yapi install` and the `packages` list in `settings.json` work as in Pi: no lock, and every package can reach everything. Use chest when a project needs reproducible installs, narrow grants, or skills shared with other agents. Both can be used side by side.

## Install

chest is a separate binary, `chest`, built from the yapi repository:

```sh
cargo install --git https://github.com/SkymanOne/yapi chest-cli
```

## Quickstart

In the project folder:

```sh
chest init
chest add git:github.com/anthropics/skills --path skills/pdf
chest add git:github.com/acme/agents --path reviewer.md
chest add npm:@acme/pi-subagent --grant all
git add chest.toml chest.lock .gitignore
git commit -m "Add project packages"
```

`chest init` writes `chest.toml` with folders for the agents it finds. Each `chest add` looks at the package, asks what to grant an extension, writes the entry, installs it and updates the lock and the `.gitignore` block.

Someone who clones the project runs:

```sh
chest install
```

## Choose folders for your agents

`chest init` looks for each agent's folder in the project and offers its skill and subagent folders:

| Agent | Found by | Skills | Subagents |
|---|---|---|---|
| Agent Skills (Codex, Cursor, Copilot, OpenCode, Pi) | `.agents` | `.agents/skills` | |
| Claude Code | `.claude` | `.claude/skills` | `.claude/agents` |
| Codex | `.codex` | `.agents/skills` | `.codex/agents` |
| Cursor | `.cursor` | `.agents/skills` | `.cursor/agents` |
| GitHub Copilot | `.github/agents` or `.github/copilot-instructions.md` | `.agents/skills` | `.github/agents` |
| OpenCode | `.opencode` | `.agents/skills` | `.opencode/agent` |
| yapi | `.yapi` | `.agents/skills` | `.yapi/agents` |

At a terminal, `init` lists the folders by number with the ones it found selected. Without a terminal, or with `--yes`, it takes those. `--skills`, `--agents` and `--store` set the folders directly.

The first folder of each kind holds the canonical copy of every item. The other folders get relative links to it, or a copy where links cannot be made. Every installed path is listed in a block of the project's `.gitignore`, from `# BEGIN chest (chest.toml)` to `# END chest`, which chest rebuilds on each install and leaves every other line alone.

Hand-written skills can live in the same folders. chest replaces or removes only what it installed itself on this machine and that still holds what it installed. It keeps that record in `~/.chest/state`, outside the project. An item that someone edited after chest installed it stays as it is, and chest asks you to remove or restore it.

## Grant capabilities

An extension entry lists what it may reach:

| Capability | Allows |
|---|---|
| `fs` | Reading and writing files, and the `read`, `write`, `edit` and `ls` tools |
| `net` | Network requests, model calls and provider credentials |
| `env` | Environment variables |
| `session` | Starting turns and changing the session and its files |
| `all` | Everything above, plus running programs |
| `none` | Nothing beyond the baseline. The default. |

`chest add` shows the capabilities a package requests and asks about each one. For a native extension it also shows what the build uses, from the features it was built with and the WASI interfaces it imports, and asks about the ones the package does not request. A bare `.wasm` file requests nothing, so chest offers what it uses. A request never grants anything by itself. Without a terminal, it grants only what `--grant` lists, which is nothing by default. `capabilities-defaults` in the manifest adds capabilities to every extension entry of that manifest.

`all` is the only way to run programs. A program runs as you, outside the sandbox, with yapi's environment and API keys, so `all` is full access, and `chest add` says so before granting it. [The reference](chest-manifest.md#capabilities) lists exactly what each capability allows.

## Let extensions talk

Extensions share `pi.events` by default, as in Pi, so cooperating Pi extensions keep working whatever their grants. Sharing has a cost between extensions with different grants: one that acts on events lets the sender borrow its access, and one that listens learns what others send.

An entry's `events` narrows that:

```toml
[extensions]
todo    = { git = "https://github.com/acme/pi-todo", capabilities = ["fs"], events = { send = ["todo:*"], receive = ["subagent:done"] } }
scraper = { npm = "@acme/scraper", capabilities = ["net"], events = "isolated" }
```

Extensions with identical grants always hear each other. Across different grants, an event reaches an extension only when the sender may send its channel and the receiver may hear it. `chest add` offers the shared default, the channels the package declares, or `isolated`.

## Lock and reproduce

`chest.lock` records each entry's commit or npm version, the tarball and integrity of every npm package in its tree, and a content hash of each installed item. Do not edit it.

A pinned tarball must come from the registry configured for its package, at the standard `<name>/-/<file>.tgz` path. Tarballs pinned on `registry.npmjs.org` come from your registry when it is a mirror. Registries that keep tarballs elsewhere, such as GitHub Packages, cannot be replayed from a lock.

- `chest install` keeps what is locked and present, installs what is locked but missing exactly as locked, and resolves entries that changed in the manifest. Then it writes the lock.
- `chest install --frozen` installs nothing. It fails when the lock is missing or out of date, or when an installed copy no longer matches its hash. It is the default when `CI` is set to anything but an empty value, `0` or `false`.
- `chest verify` checks every installed copy, copy fallback and link against the lock and exits with 1 when one does not match.

## Update

`chest outdated` lists entries whose source has a newer commit for its `rev`, or a newer npm version in range, and npm's `latest`. `chest update` moves the named entries, or all of them, to those, installs them and writes the lock. When an updated extension now requests capabilities or event channels beyond its grants, `chest update` warns and grants nothing more.

## Environments

A manifest's lock sits beside it with `.lock` for `.toml`. Keep an environment's entries in a separate complete manifest, such as `chest.dev.toml`, which locks in `chest.dev.lock`, and select it with `--manifest`:

```sh
chest --manifest chest.dev.toml install
```

`CHEST_MANIFEST` sets the default for `--manifest`, as a file name or a path relative to the current folder:

```sh
export CHEST_MANIFEST=chest.dev.toml
chest install
```

## Global packages

`-g` uses `~/.chest/chest.toml` instead of the project's manifest. Its folders can be anywhere under your home folder, such as `~/.claude/skills`, its extensions go to `~/.chest/store`, and it writes no `.gitignore`. Skills and subagents installed this way reach every project, for yapi and other agents.

## Develop a package locally

`[patch]` installs an entry from a local folder instead of its source, without changing the lock:

```toml
[patch]
todo = { path = "../pi-todo" }
```

A path that points outside the project gets a notice on each install.

## Move from settings.json or skills.sh

`chest import --settings` turns the packages of `.yapi/settings.json` into extension entries with `capabilities = ["all"]`, which is what `yapi install` gives them, and keeps their filters. It leaves `settings.json` alone and prints the `yapi remove -l` command for each package, to run once chest installs it. `-g` imports the agent folder's `settings.json`.

`chest import --skills-lock` turns the skills of a skills.sh `skills-lock.json` into skill entries. chest computes its own hashes, so the file's are dropped.

Run `chest install` after importing.

## Publish skills, subagents and extensions

A package describes itself in the `[package]` table of its own `chest.toml`. `chest init --package` adds one, listing the conventional folders it finds:

```toml
[package]
name = "acme-review"
version = "1.3.0"
description = "Code review skills and a reviewer subagent"
capabilities = ["fs", "session"]
events = { send = ["review:*"], receive = ["subagent:done"] }
skills = ["skills/*"]
agents = ["agents/*.md"]
extensions = ["extensions/index.ts"]
```

`capabilities` and `events` are requests. `chest add` shows them to the consumer, who decides what to grant. A package can also request them in `package.json` under `yapi.capabilities` and `yapi.events`.

Before you publish a native extension, run `chest check` in the package folder. It compares the capabilities each `.wasm` extension uses with the ones the package requests:

```sh
chest check
```

It reads the [capability features](native-extensions.md#declare-capabilities) the extension was built with, and the WASI interfaces it imports, where `wasi:filesystem` means `fs`. A capability the extension uses that the package does not request is an error, and chest exits with 1. So is an import of `wasi:sockets` or `wasi:http`, which yapi does not support, because extensions reach the network through the SDK's `fetch`. `chest add` warns about such an import. A capability the package requests that no extension uses is a warning. An extension built without yapi-extension-api has no record of its features, so chest checks only its imports. JS and TS extensions have nothing to read before they run, so yapi checks them only at runtime. Entries in the package's own `[skills]`, `[agents]` or `[extensions]` sections are not installed for its consumers. `chest add` lists them so the consumer can add the ones they want.

## Command reference

Every command takes `-g` (the global manifest) and `--manifest <path>`. Without either, `CHEST_MANIFEST` names the manifest, else `chest.toml` in the current folder.

| Command | Flags | What it does |
|---|---|---|
| `chest init` | `--skills a,b`, `--agents a,b`, `--store <dir>`, `--package`, `--yes` | Writes a manifest with folders for the agents found. Refuses an existing manifest unless `--package`, which adds `[package]`. |
| `chest add <source>` | `--skill`, `--agent`, `--extension`, `--name <name>`, `--path <path>`, `--rev <rev>`, `--version <range>`, `--grant a,b`, `--yes` | Adds an entry and installs it. The source is `npm:<name>[@<range>]`, `git:<repo>[@<rev>]`, a git URL or a path. Without a section flag, chest infers it from the package. The same name with the same source updates its version or rev and keeps its grants. The same name with another source is an error. |
| `chest remove <name>` | | Removes the entry and what chest installed for it. |
| `chest install` | `--frozen` | Installs what the manifest lists, as the lock pins it, and writes the lock. |
| `chest update [names]` | | Moves the entries, or all, to the newest version or commit their source allows. |
| `chest list` | | Lists each entry with its locked version or commit, whether it is installed, its capabilities and description. |
| `chest outdated` | | Lists entries whose source has something newer. |
| `chest verify` | | Checks installed copies against the lock. |
| `chest import` | `--settings` or `--skills-lock`, `[path]` | Turns settings.json packages or a skills-lock.json into entries. |
| `chest check [path]` | | Compares the capabilities the native extensions of the package at `path`, or the `.wasm` file, use with the ones the package requests in its `chest.toml` or `package.json`. `path` is the current folder by default. A `.wasm` file belongs to the nearest folder above it with either file. Fails when the package's `[package]` does not parse. |

chest exits with 0 on success, 1 when a command fails or `verify` or `check` finds a mismatch, and 2 on a usage error. Questions go to stderr and are asked only at a terminal.
