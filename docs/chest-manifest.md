# chest.toml reference

This page lists every key of `chest.toml`, the rules chest checks, and the format of `chest.lock`. [Manage project packages with chest](chest.md) shows how to use them.

## Which manifest

- chest uses `chest.toml` in the current folder, and only there.
- `--manifest <path>` names another, and `CHEST_MANIFEST` sets the default for it.
- An environment's manifest is a complete manifest of its own, such as `chest.dev.toml`. Its lock is the same name with `.lock` for `.toml`, here `chest.dev.lock`.
- `-g` uses the global manifest, `~/.chest/chest.toml`.

Paths in a manifest are relative to the manifest's folder. A project manifest keeps every folder inside the project. The global manifest may use `~` and absolute paths.

## `capabilities-defaults`

A list of capabilities that every `[extensions]` entry of this manifest gets on top of its own. Default: none.

```toml
capabilities-defaults = ["env"]
```

## `[folders]`

| Key | Type | Meaning |
|---|---|---|
| `skills` | list of folders | Where skills go. The first holds the canonical copies, the others link to them. |
| `agents` | list of folders | Where subagents go, placed as skills are. |
| `extensions` | folder | The extension store, one folder per entry. Default: `.yapi/chest`, or `~/.chest/store` for the global manifest. |

```toml
[folders]
skills = [".agents/skills", ".claude/skills"]
agents = [".yapi/agents", ".claude/agents"]
extensions = ".yapi/chest"
```

Each folder must name a folder, so `.` is refused. No folder may hold another or the store. In a project manifest a folder is a relative path without `..` or `~`. A section with entries needs its folders set.

## `[skills]`, `[agents]` and `[extensions]`

Each entry is a name with an inline table, or a `[section.name]` table. The source is one of these:

| Keys | Source |
|---|---|
| `git`, optional `rev`, optional `path` | A git repository at a branch, tag or commit, the default branch without `rev`. `path` picks a folder or file inside it. |
| `npm`, optional `version`, optional `path` | An npm package at a version range or dist-tag, the latest version without `version`. `path` picks a folder or file inside it. |
| `path` alone | A folder or file, relative to the manifest. |

```toml
[skills]
pdf    = { git = "https://github.com/anthropics/skills", rev = "v1.2.0", path = "skills/pdf" }
review = { npm = "@acme/agent-skills", version = "^2.1", path = "review" }
notes  = { path = "tools/skills/notes" }

[agents]
reviewer = { git = "github.com/acme/agents", rev = "main", path = "reviewer.md" }
```

- A `git` value is a repository that `yapi install` accepts after `git:`, without a revision: put that in `rev`. `rev` must not start with `-`.
- `path` inside a git or npm package stays inside it, without `..`. A path source may use `..` to reach a sibling checkout, and chest notes it on install. In a project manifest it may not be absolute or start with `~`.
- With `path`, a skills or agents entry installs one item under the entry's name. A subagent is a `.md` file, placed as `<name>.md`. Without `path`, it installs every skill or subagent the package provides, each under its own name.

Extension entries take three more kinds of keys:

| Key | Type | Meaning |
|---|---|---|
| `capabilities` | list | What the extension may reach, on top of `capabilities-defaults`. Default: nothing. See [Capabilities](#capabilities). |
| `events` | `"shared"`, `"isolated"` or `{ send, receive }` | Which `pi.events` channels cross to extensions with other grants. Default: `"shared"`. See [Event bus](#event-bus). |
| `skills`, `prompts`, `themes` | lists of patterns | Which of the package's resources load, as the filters of a `settings.json` package. |

```toml
[extensions]
subagent = { npm = "@acme/pi-subagent", version = "^1.4", capabilities = ["all"] }
todo = { git = "https://github.com/acme/pi-todo", rev = "4f1c2e9", capabilities = ["fs"], events = { send = ["todo:*"], receive = [] }, skills = ["!legacy/**"] }
shout = { path = "ext/shout.wasm" }
```

An extension from a path loads where it is. Others install into the store.

## `[patch]`

A patch installs an entry from a local path instead of its source. It takes `path` only, and the lock keeps what it recorded for the entry.

```toml
[patch]
todo = { path = "../pi-todo" }
```

## `[package]`

What a package says about itself. Every key is optional.

| Key | Type | Meaning |
|---|---|---|
| `name`, `version`, `description` | string | Shown by `chest add` and `chest list` |
| `authors` | list | As `Name <email>` |
| `license`, `repository`, `homepage` | string | |
| `keywords` | list | |
| `capabilities` | list | The capabilities its extensions need. A request only: the consumer grants. |
| `events` | as `events` above | The channels its extensions use. A request only. |
| `skills`, `agents`, `extensions`, `prompts`, `themes` | lists of paths and globs | Its resources, relative to the package. `!` patterns exclude. |

## Names and paths

- Entry names match `[a-z0-9][a-z0-9._-]*` and are unique across `[skills]`, `[agents]` and `[extensions]`.
- Unknown keys are errors. Errors name the entry and the key, as in this one for a path entry with a revision: skills.notes: `rev` needs `git`.
- Every item a package provides, and every `path` inside a git or npm package, must lie inside the package once links are followed.
- A skill holding a symbolic link is refused.

## Capabilities

| Capability | Allows |
|---|---|
| `fs` | File access, imports from outside the package's own folder, `context.files`, and the built-in `read`, `write`, `edit` and `ls` tools |
| `net` | `fetch`, streamed fetch and DNS, model calls, reading provider credentials, HTTP MCP servers, and refreshing model catalogs over the network |
| `env` | Reading environment variables, and the environment key fallback when calling a model |
| `session` | Starting, steering and stopping turns, and changing the session, its files or the model's setup |
| `all` | Everything above, plus running programs, stdio MCP servers, the `bash`, `grep` and `find` tools, and registering model providers |
| `none` | Nothing beyond the baseline. It cannot be combined with other values. |

- Only `all` runs programs, because a program runs as you, outside the sandbox. `process` is not a capability.
- Calling a tool that is not built in needs `all`.
- Always allowed: registering tools, commands, flags, shortcuts, renderers and providers, UI dialogs, widgets and notifications, reading session state, the clock, loading the package's own modules, and `pi.events` as its `events` allows.

## Event bus

An event from extension S on channel C reaches extension R when S and R have identical grants, or when S may send C and R may hear C.

| `events` | Sends | Hears |
|---|---|---|
| absent or `"shared"` | every channel | every channel |
| `"isolated"` | none | none |
| `{ send = [...], receive = [...] }` | the channels in `send` | the channels in `receive` |

A pattern is a channel name, a prefix ending in `*` such as `todo:*`, or `*` for every channel. Anything else, such as `*` inside a name or `?`, is an error. Extensions with the same grants and the same `events` run in one instance.

## Package metadata

For each kind of resource, the first of these that lists it wins: the package's `chest.toml` `[package]`, then `package.json`'s `yapi` key, then its `pi` key. When a `pi` key exists, or `[package]` or `yapi` lists some kind, a kind none of them lists provides nothing. Otherwise the conventional folders apply: `skills`, `agents`, `extensions`, `prompts` and `themes`.

Capabilities come from `[package]`, else `yapi.capabilities`, else none. Name, version, description and authors come from `[package]`, else from `package.json`.

## `chest.lock`

chest writes the lock. Do not edit it. It holds `version = 1` and one table per entry, in `[[skill]]`, `[[agent]]` and `[[extension]]`, sorted by name:

| Field | Meaning |
|---|---|
| `name` | The entry |
| `git`, `rev`, `commit` | The repository, the revision asked for, and the 40-character commit it resolved to |
| `npm`, `version` | The package and the version installed |
| `path` | The subpath, or the source of a path entry |
| `tarball`, `integrity` | For an npm skill or subagent, its tarball and sha512 integrity |
| `hash` | The content hash of the installed item, or of the store folder |
| `items` | Without `path`, each item with its name and hash, sorted by name |
| `[[extension.package]]` | For an npm or git extension, every npm package of its tree, sorted by path, with `path`, `name`, `version`, `tarball` and `integrity` |

Path entries record their path only, with no hash, and `[patch]` entries never appear. A pinned tarball must lie on the registry configured for its name, at the standard `<name>/-/<file>.tgz` path. Tarballs pinned on `registry.npmjs.org` come from the configured registry when that is a mirror.

The lock is up to date when every entry has a record with the same section and name, the same source kind, location and subpath, and the same `rev`, and its locked npm version still satisfies the entry's range. Capabilities, events and filters never make it stale.

### Content hash

The hash is `sha256-` followed by lowercase hex:

1. List every regular file under the item's folder, skipping any folder named `.git` at any depth. A symbolic link anywhere fails the hash. An item that is one file is a list of that file with an empty path.
2. Sort the files by the bytes of their paths, relative to the item, with `/` between parts.
3. For each file, feed one SHA-256 with its path in UTF-8, a zero byte, `x` when any execute bit is set or `-` otherwise, and the 32-byte SHA-256 of its contents.
4. The result is that SHA-256, in hex.

## A complete example

```toml
# Every extension may read environment variables.
capabilities-defaults = ["env"]

[folders]
skills = [".agents/skills", ".claude/skills"]   # .agents/skills holds the copies
agents = [".yapi/agents", ".claude/agents"]
extensions = ".yapi/chest"

[skills]
pdf    = { git = "https://github.com/anthropics/skills", rev = "v1.2.0", path = "skills/pdf" }
review = { npm = "@acme/agent-skills", version = "^2.1", path = "review" }
notes  = { path = "tools/skills/notes" }

[agents]
reviewer = { git = "github.com/acme/agents", rev = "main", path = "reviewer.md" }

[extensions]
subagent = { npm = "@acme/pi-subagent", version = "^1.4", capabilities = ["all"] }
todo     = { git = "https://github.com/acme/pi-todo", rev = "4f1c2e9", capabilities = ["fs"], events = { send = ["todo:*"], receive = [] } }
shout    = { path = "ext/shout.wasm" }

# Work on todo from a local checkout.
[patch]
todo = { path = "../pi-todo" }

# What this repository publishes, for those who add it.
[package]
name = "acme-tools"
version = "0.1.0"
capabilities = []
skills = ["tools/skills/*"]
```
