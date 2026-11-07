# Run yapi with only the project's packages

> **yapi only.**

`yapi --project-only` loads the project's extensions, skills and Model Context Protocol (MCP) servers, and leaves out the ones you installed globally. Use it to check that a project's `chest.toml` covers what the project needs, or to keep your personal extensions out of a CI run. It works in projects without a `chest.toml` too.

Only what you installed stays out. Your settings, sign-ins and instruction files such as `~/.yapi/agent/AGENTS.md` still apply, so your project-only run can still differ from a teammate's.

This page covers turning the mode on, what loads, keeping global items such as a theme, and checking what loaded.

## Turn on project-only mode

Pass the flag for one run, or set `YAPI_PROJECT_ONLY` for every run in a shell:

```sh
yapi --project-only
export YAPI_PROJECT_ONLY=1
```

`YAPI_PROJECT_ONLY` accepts `1`, `true` or `yes`, and `unset YAPI_PROJECT_ONLY` turns it off. `/reload` keeps the mode on.

Programs that yapi starts inherit the variable: bash commands, extension processes, stdio MCP servers and subagents that start another yapi.

Only yapi runs follow the mode. `yapi install` without `-l` still adds a package to your global settings, and a project-only run then leaves it out. To add a package to the project, use `chest add` or `yapi install -l`.

## What a project-only run loads

From a trusted project, yapi loads:

- the project's chest entries
- the `packages` and resource paths in `.yapi/settings.json`
- `.yapi/extensions`, `.yapi/skills`, `.yapi/prompts` and `.yapi/themes`
- the servers in `.yapi/mcp.json`
- `.agents/skills` in the project and each parent folder up to the repository root, or up to `/` outside a repository

In every project, it also loads the built-in extensions, following your `+builtin:` and `-builtin:` choices. So do the paths you pass with `-e`, `--skill`, `--prompt-template` and `--theme`.

It leaves out the rest of what you installed globally, unless you [keep some of it](#keep-some-global-items):

- the `packages` and resource paths in `~/.yapi/agent/settings.json`
- `~/.yapi/agent/extensions`, `~/.yapi/agent/skills`, `~/.yapi/agent/prompts` and `~/.yapi/agent/themes`
- `~/.agents/skills`
- the entries of `~/.chest/chest.toml`
- the servers in `~/.yapi/agent/mcp.json`

Your settings, sign-ins, models, key bindings and trust decisions don't change. `AGENTS.md` and `CLAUDE.md` still load from the project, its parent folders and `~/.yapi/agent`, and `--no-context-files` turns them off. `SYSTEM.md` and `APPEND_SYSTEM.md` load from the project, or else from `~/.yapi/agent`.

A trusted project can still load one of your global files on purpose. It can name the file's path in its own settings, or add an `autoload: false` entry for a package you installed. You approved those files when you trusted the project.

## Keep some global items

To keep a global theme, skill or MCP server in project-only runs, list it under `projectOnly` in your global settings:

```json
{
  "projectOnly": {
    "packages": ["npm:@acme/pi-themes", "chest:notes"],
    "extensions": ["extensions/notify.ts"],
    "skills": ["~/.agents/skills/pdf"],
    "themes": ["*"],
    "mcpServers": ["github", "linear*"]
  }
}
```

yapi reads `projectOnly` only from `~/.yapi/agent`, never from a project's settings, so a repository you clone can't turn your global extensions back on. Every key is optional, and `prompts` works like `extensions`.

### Keep whole packages

`packages` keeps every extension, skill, prompt template and theme a global package provides. Write each source the way your global `packages` list writes it, such as `npm:@acme/pi-themes` or `git:github.com/acme/tools`. Versions don't count, so `npm:@acme/pi-themes` also keeps a package installed as `npm:@acme/pi-themes@^2`.

Write `chest:<name>` for an entry of `~/.chest/chest.toml`, and `*` to keep every global package and chest entry. `*` doesn't cover loose files in your global folders, such as `~/.yapi/agent/extensions`, which the lists below do.

### Keep single extensions, skills, prompts or themes

`extensions`, `skills`, `prompts` and `themes` each keep the resources of their kind that a pattern matches. They work wherever the resource comes from: a global folder, a path in your settings, a global package or a chest entry. Patterns use Pi's glob syntax, and each one is matched against:

- the path inside `~/.yapi/agent`, such as `extensions/notify.ts` or `themes/dark.json`
- the file name, such as `notify.ts`
- the whole path, with `~` for your home folder, such as `~/.agents/skills/pdf`

A skill's folder matches too, so `~/.agents/skills/pdf` keeps `~/.agents/skills/pdf/SKILL.md`. `"themes": ["*"]` keeps every global theme, including the ones inside packages, without loading those packages' extensions.

Your `theme` setting doesn't keep a theme by itself. If it names a global theme that `themes` doesn't keep, yapi uses the system theme. Interactive mode shows Pi's error, `Failed to load theme "<name>": Theme not found: <name>`.

### Keep MCP servers

`mcpServers` keeps servers from `~/.yapi/agent/mcp.json` by name, and accepts patterns such as `linear*`. A kept server keeps its `auth`, so a server that sends a token from `/login` still works. The project's own servers in `.yapi/mcp.json` always connect.

### Global packages yapi still installs

A project-only run installs or restores a global package only when `projectOnly` could keep part of it:

- a package that `packages` names
- every global package, when `packages` holds `*` or another list has an entry, because a pattern can match a file inside any package
- the missing entries of `~/.chest/chest.toml`, in either of those cases or when `packages` names a `chest:` entry

With no `projectOnly` list, a project-only run downloads nothing for your global packages.

## Keep items from yapi config

You can also build the list in `yapi config`, which opens on the Global Resources view. Select an extension, skill, prompt template or theme and press `ctrl+o` to keep it. Kept items show `project-only`, and `ctrl+o` again stops keeping the item.

`ctrl+o` works on chest items too, which Space leaves alone. It does nothing in the project view that Tab opens, because yapi ignores a project's `projectOnly`. `yapi config` doesn't list packages or MCP servers, so edit `packages` and `mcpServers` in the settings file.

## Check what loaded

The header at startup lists the loaded skills, prompt templates and extensions with their scope, and `/mcp` lists the MCP servers. If the `quietStartup` setting hides the header, start yapi with `--verbose`.

## Untrusted projects

yapi never loads an untrusted project's files, in project-only runs or otherwise. When an untrusted project has files that need trust, print, JSON and RPC modes warn once:

```text
--project-only: the project isn't trusted, so none of its packages load
```

Interactive mode asks whether to trust the project before it starts instead. If you choose not to trust it, yapi shows Pi's notice that the project's resources and packages are ignored.

Either way, the run then has only the built-in extensions, the paths you passed on the command line and the global items you keep. Trust the project, or pass `--approve` for one run. [Project trust](security.md#project-trust) covers the details.
