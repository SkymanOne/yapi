# Environments, global packages and patches

> **Any agent**, except the sections marked yapi only.

One project can keep more than one manifest, and one manifest in your home folder serves every project. This page covers manifests per environment, how yapi chooses a manifest, global packages and installing an entry from a local checkout.

## Keep a manifest per environment

> **Any agent.**

A manifest's lock sits beside it, with `.lock` in place of `.toml`. Keep an environment's entries in a separate, complete manifest, such as `chest.dev.toml`, which locks in `chest.dev.lock`. Select it with `--manifest`:

```sh
chest --manifest chest.dev.toml install
```

`CHEST_MANIFEST` sets the default for `--manifest`, as a file name or a path inside the current folder. A value that's absolute, starts with `~` or uses `..` is an error. `-g` ignores the variable. This sets the same manifest for every command in the shell:

```sh
export CHEST_MANIFEST=chest.dev.toml
chest install
```

The manifests of one project, such as `chest.toml` and `chest.dev.toml`, can share folders. A path that one of them drops stays while another still lists it. Each manifest keeps its own block in `.gitignore`, named after it, such as `# BEGIN chest (chest.dev.toml)`. Messages about a manifest name the commands for it, such as `chest --manifest chest.dev.toml install`.

## How yapi reads `CHEST_MANIFEST`

> **yapi only.** Other agents read only the folders chest fills, so the variable means nothing to them.

yapi reads the same variable, relative to the project folder, and loads only a manifest inside the project. For a value that's absolute, starts with `~` or uses `..`, yapi warns and loads no project manifest. The global manifest still loads.

chest refuses the same values, so chest and yapi always agree on the manifest. [Project packages from chest](configuration.md#project-packages-from-chest) lists the files yapi reads.

## Install global packages

> **Any agent.** Only yapi loads global extensions, and it reads the global manifest in every project, unless a [project-only run](project-only.md#global-packages-yapi-still-installs) can keep none of its entries.

`-g` uses `~/.chest/chest.toml` instead of the project's manifest. Its folders can be anywhere under your home folder, such as `~/.claude/skills`. Its extensions go to `~/.chest/store`, and it writes no `.gitignore`. Skills and subagents installed this way reach every project, for yapi and for the other agents that read those folders.

With `-g`, `chest init` looks for each agent's folder in your home folder:

| Agent | Found by | Skills | Subagents |
|---|---|---|---|
| Agent Skills (Codex, Cursor, Copilot, OpenCode, Pi) | `~/.agents` | `~/.agents/skills` | |
| Claude Code | `~/.claude` | `~/.claude/skills` | `~/.claude/agents` |
| Codex | `~/.codex` | `~/.agents/skills` | `~/.codex/agents` |
| Cursor | `~/.cursor` | `~/.cursor/skills` | `~/.cursor/agents` |
| GitHub Copilot | `~/.copilot` | `~/.copilot/skills` | `~/.copilot/agents` |
| OpenCode | `~/.config/opencode` | `~/.config/opencode/skills` | `~/.config/opencode/agent` |
| yapi | `~/.yapi` | `~/.agents/skills` | `~/.yapi/agent/agents` |

When it finds no agent, skills go to `~/.agents/skills` and subagents to `~/.yapi/agent/agents`. With `YAPI_CODING_AGENT_DIR` set, yapi's subagents go to the `agents` folder inside it instead, which is where yapi reads them. When that folder is outside your home folder, `chest init` leaves it out and says why, because the global manifest installs only inside your home folder.

For both the project and the global manifest, chest keeps its download cache and its record of what it placed in `~/.chest`.

## Develop a package locally

> **Any agent.**

`[patch]` installs an entry from a local folder instead of its source, without changing the lock:

```toml
[patch]
todo = { path = "../pi-todo" }
```

A path that points outside the project gets a notice on each install. chest refuses a path source or patch whose folder holds one of its install folders or the store, because chest would copy it into itself.

chest also refuses an extension's path source or patch whose folder is or holds the home folder, the agent folder, the filesystem's root or the project, because the extension could load any file there.

`chest list` shows a patched entry as `patched`, as `edited` when someone changed a copy that chest installed from the patch, and as `missing` when that copy is gone. `chest verify` reports both. `chest install` puts a missing copy back from the patch, also with `--frozen`, which fails on an edited copy instead.

yapi loads a patched entry without a lock entry, as [How yapi loads a manifest](chest-extensions.md#how-yapi-loads-a-manifest) describes.
