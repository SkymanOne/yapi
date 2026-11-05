# Run yapi with project packages only

> **yapi only.**

`--project-only` runs yapi with only the packages, skills and Model Context Protocol (MCP) servers that the project provides, such as the entries of its `chest.toml`. Your global ones stay out, unless you choose to keep some. Use it to check that a project's packages cover what it needs, to keep a project's tools apart from your own, or in CI. It works with or without a `chest.toml`.

The mode leaves out what you installed. Your other settings still apply, and so do global instruction files such as `~/.yapi/agent/AGENTS.md` and `SYSTEM.md`, so two people's project-only runs of one project can differ.

This page covers how to turn the mode on, what loads in it, how to keep chosen global items such as themes, and how to check what loaded.

## Turn it on

Pass `--project-only` for one run:

```sh
yapi --project-only
```

Set `YAPI_PROJECT_ONLY` to turn it on for every run in a shell. yapi accepts `1`, `true` and `yes`:

```sh
export YAPI_PROJECT_ONLY=1
```

`unset YAPI_PROJECT_ONLY` turns it off again. `/reload` keeps the mode.

The mode applies to yapi runs only. Other commands work as usual: `yapi install` without `-l` still adds a package to your global settings, which project-only runs then leave out. Add a project's packages with `chest add`, or with `yapi install -l`.

The mode passes to the processes yapi starts: bash commands, extension processes, stdio MCP servers, and subagents that start another yapi. They get `YAPI_PROJECT_ONLY=1` in their environment. Extensions with the environment capability see it in `process.env` too. A process that an extension starts with an environment of its own, which leaves the variable out, runs without the mode.

## What loads

A project-only run loads these from a trusted project:

- the project's chest entries
- the `packages` and resource paths in `.yapi/settings.json`
- `.yapi/extensions`, `.yapi/skills`, `.yapi/prompts` and `.yapi/themes`
- the servers in `.yapi/mcp.json`
- `.agents/skills` in the project and its parent folders up to the repository root, or up to `/` without a repository

It also loads these, whatever the project:

- the built-in extensions, with your `+builtin:` and `-builtin:` choices
- the paths you pass with `-e`, `--skill`, `--prompt-template` and `--theme`
- the global items that [`projectOnly`](#keep-global-items) keeps

## What's skipped

Every other global item stays out:

- the `packages` and resource paths in `~/.yapi/agent/settings.json`
- `~/.yapi/agent/extensions`, `~/.yapi/agent/skills`, `~/.yapi/agent/prompts` and `~/.yapi/agent/themes`
- `~/.agents/skills`
- the entries of `~/.chest/chest.toml`
- the servers in `~/.yapi/agent/mcp.json`

A trusted project can still load a global file on purpose: by its path in the project's own settings, or with an `autoload: false` entry for a package you installed globally. You approved the project's files when you trusted it, so yapi follows them.

## What stays the same

- your settings, such as the model and the theme name
- your sign-ins in `auth.json` and `mcp-auth.json`, `models.json`, key bindings and project trust
- the `AGENTS.md` and `CLAUDE.md` files in the project, in the folders above it and in `~/.yapi/agent`. Add `--no-context-files` to leave them all out.
- `SYSTEM.md` and `APPEND_SYSTEM.md`: the project's, or else yours in `~/.yapi/agent`

## Keep global items

The `projectOnly` object in your global settings names the global items that still load in the mode. yapi reads it only from `~/.yapi/agent/settings.json`, or from `settings.toml` there, and ignores it in a project's settings. That way a repository you clone can't turn on your global extensions or servers.

In `settings.json`:

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

The same in `settings.toml`:

```toml
[projectOnly]
packages = ["npm:@acme/pi-themes", "chest:notes"]
extensions = ["extensions/notify.ts"]
skills = ["~/.agents/skills/pdf"]
themes = ["*"]
mcpServers = ["github", "linear*"]
```

An item loads when `packages` keeps the package it comes from, or when the list for its kind matches it. Each key is optional, and `prompts` works like `extensions`.

### `packages`

`packages` keeps whole global packages, with every extension, skill, prompt and theme they provide. Write each source the way your global `packages` list writes it, such as `npm:@acme/pi-themes`, `git:github.com/acme/tools` or a local path. yapi compares packages by name, so `npm:@acme/pi-themes` also keeps the package installed as `npm:@acme/pi-themes@^2`. Name an entry of `~/.chest/chest.toml` as `chest:<name>`.

`"packages": ["*"]` keeps every global package and every global chest entry. It doesn't keep files in your global folders, such as `~/.yapi/agent/extensions`, which the kind lists below cover.

### `extensions`, `skills`, `prompts` and `themes`

Each list keeps single global resources of its kind, wherever they come from: your global folders, a path in your settings, a global package or a global chest entry. Entries use Pi's glob patterns, and each one matches any of these:

- the path relative to `~/.yapi/agent`, such as `extensions/notify.ts` or `themes/dark.json`
- the file name, such as `notify.ts`
- the whole path, where `~` stands for your home folder, such as `~/.agents/skills/pdf`

For a skill in a folder, the folder also matches, so `~/.agents/skills/pdf` keeps `~/.agents/skills/pdf/SKILL.md`. `*` matches every resource of the kind. `"themes": ["*"]` keeps every global theme, including the themes inside global packages, without loading those packages' extensions.

Your `theme` setting names a theme but doesn't keep it. When it names a global theme, add that theme to `themes`, or yapi warns and falls back to the system theme.

### `mcpServers`

`mcpServers` keeps servers from `~/.yapi/agent/mcp.json`, by name. Entries can use `*`, such as `linear*`, and `*` alone keeps every server. A kept server keeps its `auth`, so a server that sends the token of a provider you signed in to with `/login` keeps working. The servers in the project's `.yapi/mcp.json` always load.

### What gets installed

A project-only run installs a global package only when `projectOnly` could keep part of it:

- A `packages` entry installs the package it names.
- `*` in `packages`, or a kind list that isn't empty, installs every global package, since a pattern can match a file inside any of them.
- Any of these, or a `chest:` entry, puts back every missing entry of `~/.chest/chest.toml`.

Otherwise the run installs nothing global.

### Examples

Keep every global theme:

```json
{ "projectOnly": { "themes": ["*"] } }
```

Keep one theme package, with everything it provides:

```json
{ "projectOnly": { "packages": ["npm:@acme/pi-themes"] } }
```

Keep one skill from `~/.agents/skills`:

```json
{ "projectOnly": { "skills": ["~/.agents/skills/pdf"] } }
```

Keep one entry of `~/.chest/chest.toml`:

```json
{ "projectOnly": { "packages": ["chest:notes"] } }
```

Keep the GitHub MCP server:

```json
{ "projectOnly": { "mcpServers": ["github"] } }
```

## Choose kept items in yapi config

`yapi config` lists every global extension, skill, prompt template and theme, whether the mode is on or not. In the Global Resources view, where `yapi config` opens, select an item and press `ctrl+o` to keep it in project-only runs. Press it again to stop keeping it. yapi writes the item's path to `projectOnly` in your global settings, and kept items show `project-only` beside them.

- yapi writes a path relative to `~/.yapi/agent` when the item is inside it, else from `~`, else the whole path. A skill in a folder is written as its folder, and a skill that's a single `.md` file as that file.
- `ctrl+o` works on chest's items too, which Space leaves alone.
- `ctrl+o` does nothing on built-in extensions, which project-only runs never leave out, or on an item that a pattern or a `packages` entry keeps. Edit those in the settings file.
- `ctrl+o` does nothing in the project view, which Tab opens, because yapi ignores `projectOnly` in a project's settings.

`yapi config` lists neither packages nor MCP servers, so edit `packages` and `mcpServers` in the settings file.

## Check what loaded

The header at startup lists the loaded context files, skills, prompt templates and extensions, each with its scope. `/mcp` lists the MCP servers. When the `quietStartup` setting hides the header, start yapi with `--verbose` to show it.

## Project trust

An untrusted project's files don't load. When an untrusted project has files that need trust, yapi warns once: `--project-only: the project isn't trusted, so none of its packages load`. The run then loads the built-in extensions, the paths you pass on the command line and the global items `projectOnly` keeps.

Interactive mode asks whether to trust the project first, as it always does. Trust the project, or pass `--approve` for one run. [Project trust](security.md#project-trust) covers trust.
