# Coming from Pi

yapi reads Pi's formats but keeps its own folders, so the two never write the same files. To copy an existing Pi setup, run:

```sh
yapi import pi
```

The command copies Pi's agent folder, `~/.pi/agent` or `PI_CODING_AGENT_DIR`, into yapi's: settings, credentials, models, key bindings, MCP servers and their sign-ins, trust decisions, the global `AGENTS.md` or `CLAUDE.md`, `SYSTEM.md` and `APPEND_SYSTEM.md`, sessions, prompt templates, skills, themes, extensions and packages. It also copies the current project's `.pi` folder into `.yapi`.

Files yapi already has are kept, except a `settings.json` that holds only the version yapi recorded on first start. Pi's `lastChangelogVersion` is left out, because it holds Pi's version.

After the import, yapi and Pi work side by side, and each opens the other's session files.

## Names that change

| Pi | yapi |
|---|---|
| `~/.pi/agent` | `~/.yapi/agent` |
| Project `.pi` folder | Project `.yapi` folder |
| `PI_CODING_AGENT_DIR` | `YAPI_CODING_AGENT_DIR` |
| `PI_CODING_AGENT_SESSION_DIR` | `YAPI_CODING_AGENT_SESSION_DIR` |
| `pi install`, `pi config`, `pi mcp`, `pi auth` | `yapi install`, `yapi config`, `yapi mcp`, `yapi auth` |

Only the two directory variables are renamed. Provider keys and every other `PI_*` variable, such as `PI_OFFLINE` and `PI_CACHE_RETENTION`, keep their names. Command-line flags, slash commands and the JSON and RPC protocols are the same as Pi's.

## What differs

Some behaviors differ by design, such as the startup header and features tied to Pi's online services. [Differences from Pi](compat.md) lists each one with its reason.
