# Get started with chest

chest is a package manager for coding agents. It keeps a project's skills, subagents and yapi extensions in one manifest, `chest.toml`, and pins what each one resolved to in a lock, `chest.lock`. Commit both files. Everyone who clones the project then installs the same bytes with `chest install`.

## Any agent or yapi only

chest is a separate binary, and most of it works without yapi. Other agents never read `chest.toml`. They read the skills and subagents that chest places in their folders. yapi reads the manifest as well, so it can load extensions with only the capabilities you grant them.

| Feature | Works with |
|---|---|
| The `chest` command | Any agent |
| Skills and subagents, in `[skills]` and `[agents]`, placed in the folders each agent reads | Any agent |
| The lock and its content hashes, `chest install --frozen` in CI, `chest verify`, `chest outdated` and `chest update` | Any agent |
| The `.gitignore` block that lists installed files | Any agent |
| Environment manifests, global packages with `-g`, and `[patch]` | Any agent |
| `chest import --skills-lock`, from a skills.sh `skills-lock.json` | Any agent |
| Publishing skills and subagents with `[package]` | Any agent |
| Extensions, in `[extensions]`: Pi packages and native `.wasm` extensions | yapi only |
| `capabilities`, `capabilities-defaults` and `events` | yapi only |
| Putting back missing locked entries at startup, in trusted projects | yapi only |
| Loading placed skills from a folder the agent does not read on its own, such as `.claude/skills` | yapi only |
| `CHEST_MANIFEST` choosing the manifest the agent loads | yapi only |
| `chest check`, which compares a native extension's capabilities with its requests | yapi only |
| `chest import --settings`, from yapi's `settings.json` | yapi only |

With another agent, run `chest install` after you clone or pull. yapi puts back what the lock pins by itself, as [How yapi loads a manifest](chest-extensions.md#how-yapi-loads-a-manifest) describes.

Each page in this part starts with a note that says whether it applies to any agent or to yapi only. Where a page mixes both, each section has its own note.

## chest and `yapi install`

> **yapi only.** It compares chest with yapi's own package commands.

`yapi install` and the `packages` list in `settings.json` work as in Pi. They have no lock, and every package can reach everything. Use chest when a project needs reproducible installs, narrow grants, or skills shared with other agents. You can use both side by side.

## Install

> **Any agent.** chest does not need yapi to run.

yapi's [install script](install.md#install-script) and release archives install `chest` next to `yapi`:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh
```

To build it from source instead:

```sh
cargo install --locked --git https://github.com/SkymanOne/yapi chest-cli
```

chest has its own npm client, so it needs neither Node.js nor npm. It reads registry settings from `~/.npmrc` and `npm_config_*` variables. Git sources need `git`.

## Quickstart

> **Any agent**, except the last `chest add`, which adds an extension that only yapi loads.

In the project folder, with your own sources in place of the `acme` examples:

```sh
chest init
chest add git:github.com/anthropics/skills --path skills/pdf
chest add git:github.com/acme/agents --path reviewer.md
chest add npm:@acme/pi-subagent --grant all
git add chest.toml chest.lock .gitignore
git commit -m "Add project packages"
```

`chest init` writes `chest.toml` with folders for the agents it finds. Each `chest add` looks at the package and asks what to grant an extension. Then it writes the entry, installs it, and updates the lock and the `.gitignore` block.

Anyone who clones the project runs:

```sh
chest install
```

## Next steps

- [Share skills and subagents](chest-skills.md) covers the folders chest fills for each agent.
- [Run yapi extensions](chest-extensions.md) covers capabilities, events and what yapi loads.
- [Lock, verify and update](chest-lock.md) covers reproducible installs and CI.
- [Environments, global packages and patches](chest-environments.md) covers more than one manifest and local work on a package.
- [Publish a package](chest-publish.md) covers the `[package]` table and `chest check`.
- [chest commands](chest-commands.md) and the [chest.toml reference](chest-manifest.md) list every command and key.
