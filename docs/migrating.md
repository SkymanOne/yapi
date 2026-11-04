# Move from Pi to yapi

yapi reads Pi's formats but keeps its own folders, so the two never write the same files. `yapi import pi` copies an existing Pi setup into yapi's folders. This page covers what the import copies and leaves out, how to move project packages to chest, and which names change.

## Copy a Pi setup

To copy an existing Pi setup, run:

```sh
yapi import pi
```

The command copies Pi's agent folder, `~/.pi/agent` or `PI_CODING_AGENT_DIR`, into yapi's. It also copies the current project's `.pi` folder into `.yapi`. From the agent folder, it copies:

- settings, API keys, models, key bindings and Model Context Protocol (MCP) servers
- trust decisions
- the global `AGENTS.md` or `CLAUDE.md`, `SYSTEM.md` and `APPEND_SYSTEM.md`
- sessions, prompt templates, skills, themes, extensions and packages

The import keeps the files yapi already has, except a `settings.json` that holds only the version yapi recorded on first start. It keeps an installed package that yapi already has whole, without any of Pi's files for it, so the two installs never mix. It leaves out Pi's `lastChangelogVersion`, because that holds Pi's version.

After the import, yapi and Pi work side by side, and each opens the other's session files.

## Sign in again after the import

The import doesn't copy subscription sign-ins or MCP server sign-ins. Providers can replace the refresh token on every refresh and reject the old one, so a copy would make yapi and Pi sign each other out.

The command lists the providers and servers it left out. Sign in to them again in yapi with `/login`, or with `/mcp login` for MCP servers.

## Move project packages to chest

To move a project's packages from `.yapi/settings.json` into a manifest with a lock, run `chest import --settings` in the project. It writes an extension entry for each package with `capabilities = ["all"]`, the access Pi gives packages, and prints the `yapi remove -l` command for each one.

Narrow the capabilities, run `chest install`, then remove the packages from `settings.json`. [Import packages from settings.json](chest-extensions.md#import-packages-from-settingsjson) covers the details.

## Names that change

yapi renames its folders, two environment variables and its own commands:

| Pi | yapi |
|---|---|
| `~/.pi/agent` | `~/.yapi/agent` |
| Project `.pi` folder | Project `.yapi` folder |
| `PI_CODING_AGENT_DIR` | `YAPI_CODING_AGENT_DIR` |
| `PI_CODING_AGENT_SESSION_DIR` | `YAPI_CODING_AGENT_SESSION_DIR` |
| `pi install`, `pi config`, `pi mcp`, `pi auth` | `yapi install`, `yapi config`, `yapi mcp`, `yapi auth` |

yapi renames only the two folder variables. Provider keys and every other `PI_*` variable, such as `PI_OFFLINE` and `PI_CACHE_RETENTION`, keep their names. Command-line flags, slash commands and the JSON and remote procedure call (RPC) protocols are the same as Pi's.

## Behavior that differs by design

Some behaviors differ on purpose, such as the startup header and features tied to Pi's online services. [Differences from Pi](compat.md) lists each one with its reason.
