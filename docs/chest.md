# Get started with chest

chest is a package manager for coding agents. It keeps a project's skills, subagents and yapi extensions in one manifest, `chest.toml`, and pins what each one resolved to in a lock, `chest.lock`. Commit both files, and everyone who clones the project installs the same bytes with `chest install`. This page covers which features need yapi, how to install chest and how to set up a first manifest.

## Any agent or yapi only

chest is a separate binary. Its skill and subagent features work with any agent, and its extension features work only in yapi. Other agents never read `chest.toml`. They read the skills and subagents that chest places in their folders, and yapi reads the manifest as well, so it can load extensions with only the capabilities you grant them.

| Feature | Works with |
|---|---|
| [The `chest` command](chest-commands.md) | Any agent |
| [Skills and subagents](chest-skills.md) | Any agent |
| [The lock and its content hashes](chest-lock.md#what-the-lock-records) | Any agent |
| [Frozen installs in CI](chest-lock.md#run-chest-in-ci) | Any agent |
| [`chest verify`, `outdated` and `update`](chest-lock.md#verify-installed-copies) | Any agent |
| [The `.gitignore` block](chest-skills.md#the-gitignore-block) | Any agent |
| [Environment manifests](chest-environments.md#keep-a-manifest-per-environment) | Any agent |
| [Global packages](chest-environments.md#install-global-packages) | Any agent |
| [Local patches](chest-environments.md#develop-a-package-locally) | Any agent |
| [Import from skills.sh](chest-skills.md#import-from-skillssh) | Any agent |
| [Publishing skills and subagents](chest-publish.md) | Any agent |
| [Extensions](chest-extensions.md) | yapi only |
| [Capabilities and events](chest-extensions.md#grant-capabilities) | yapi only |
| [Putting back missing entries](chest-extensions.md#how-yapi-loads-a-manifest) | yapi only |
| [Skills from other agents' folders](chest-extensions.md#how-yapi-loads-a-manifest) | yapi only |
| [`CHEST_MANIFEST` in yapi](chest-environments.md#how-yapi-reads-chest_manifest) | yapi only |
| [`chest check`](chest-publish.md#check-a-native-extension) | yapi only |
| [Import from `settings.json`](chest-extensions.md#import-packages-from-settingsjson) | yapi only |

With another agent, run `chest install` after you clone or pull. yapi puts back what the lock pins by itself, as [How yapi loads a manifest](chest-extensions.md#how-yapi-loads-a-manifest) describes.

Each page in this part starts with a note that says whether it applies to any agent or to yapi only. Where a page mixes both, each section has its own note.

## Choose between chest and `yapi install`

> **yapi only.** It compares chest with yapi's own package commands.

`yapi install` and the `packages` list in `settings.json` work as in Pi. They have no lock, and every package can reach everything. Use chest when a project needs reproducible installs, narrow grants, or skills shared with other agents. You can use both side by side.

## Install chest

> **Any agent.** chest doesn't need yapi to run.

yapi's [install script](install.md#run-the-install-script) installs `chest` with `yapi`, or alone. To use chest with another agent and without yapi, install `chest` alone:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh \
  | sh -s -- --only chest
```

Without `--only`, the script asks in a terminal whether to install `yapi`, `chest` or both. cargo-binstall installs the release's `chest` alone as well, and Cargo builds it from source:

```sh
cargo binstall --git https://github.com/SkymanOne/yapi chest-cli
cargo install --locked --git https://github.com/SkymanOne/yapi chest-cli
```

[Install yapi](install.md) covers release archives, upgrades and uninstalling for both executables.

chest has its own npm client, so it needs neither Node.js nor npm. It reads registry settings from `~/.npmrc` and `npm_config_*` variables. Git sources need `git`.

## Set up a project's packages

> **Any agent**, except the last `chest add`, which adds an extension that only yapi loads.

In the project folder, run these commands with your own sources in place of the `acme` examples:

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

## More chest guides

The other pages in this part each cover one task:

- [Share skills and subagents](chest-skills.md) covers the folders chest fills for each agent.
- [Run yapi extensions](chest-extensions.md) covers capabilities, events and what yapi loads.
- [Lock, verify and update packages](chest-lock.md) covers reproducible installs and CI.
- [Environments, global packages and patches](chest-environments.md) covers more than one manifest and local work on a package.
- [Publish a package](chest-publish.md) covers the `[package]` table and `chest check`.
- [chest commands](chest-commands.md) and the [chest.toml reference](chest-manifest.md) list every command and key.
