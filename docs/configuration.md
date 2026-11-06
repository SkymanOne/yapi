# Configure yapi

yapi keeps global configuration in `~/.yapi/agent` and project configuration in a project's `.yapi` folder. This page lists where each file lives, then covers chest manifests, project instructions, skills and prompt templates, themes, key bindings, and how yapi handles images, links and colors. Every file uses Pi's format, described in Pi's [settings documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/settings.md). Settings can also be [written in TOML](#write-settings-in-toml), which only yapi reads.

Set `YAPI_CODING_AGENT_DIR` to move the global folder. The table lists where yapi reads each kind of configuration:

| What | Where |
|---|---|
| Settings | `~/.yapi/agent/settings.json`, overridden by `.yapi/settings.json`, or `settings.toml` in either folder |
| Project instructions | `AGENTS.override.md`, `AGENTS.md` or `CLAUDE.md` in the project, its parent folders and `~/.yapi/agent` |
| System prompt | `SYSTEM.md` replaces it and `APPEND_SYSTEM.md` extends it, in `.yapi/` or `~/.yapi/agent/`. `--append-system-prompt` extends it in place of `APPEND_SYSTEM.md`. |
| Skills | `~/.yapi/agent/skills`, `~/.agents/skills`, `.yapi/skills` and `.agents/skills` |
| Prompt templates | `~/.yapi/agent/prompts` and `.yapi/prompts` |
| Themes | `~/.yapi/agent/themes` and `.yapi/themes` |
| Key bindings | `~/.yapi/agent/keybindings.json` |
| Models and providers | `~/.yapi/agent/models.json` |
| Credentials | `~/.yapi/agent/auth.json` |
| Model Context Protocol (MCP) servers | `~/.yapi/agent/mcp.json` and `.yapi/mcp.json` |
| Project packages | `chest.toml` and `chest.lock` in the project, and `~/.chest/chest.toml`. See [Project packages from chest](#project-packages-from-chest). |
| Global items for project-only runs | `projectOnly` in `~/.yapi/agent/settings.json`. See [Run yapi with only the project's packages](project-only.md). |
| Documentation for the model | `~/.yapi/agent/docs`. See [Documentation for the model](install.md#documentation-for-the-model). |

`/settings` edits common settings in place. Files in a project's `.yapi` folder load only after you [trust the project](security.md#project-trust). [`--project-only`](project-only.md) runs yapi with the project's packages and leaves out your global ones.

## Write settings in TOML

> **yapi and chest only.** Pi and other agents read only `settings.json`.

yapi also reads its settings from `settings.toml`, in `~/.yapi/agent` or a project's `.yapi` folder. It holds the same settings as `settings.json`, in the same structure, and TOML lets you add comments. Choose the format per folder: yapi reads and writes whichever file a folder has. Wherever these docs say `settings.json`, `settings.toml` works the same.

When one folder holds both files, every yapi command that reads that folder stops with this error, and so does `chest import --settings`:

```text
Both settings.json and settings.toml are in /home/you/.yapi/agent. Keep one of them.
```

In a project, the check waits until you trust the project, because yapi doesn't read its `.yapi` folder before then.

### How the formats map

| JSON | TOML |
|---|---|
| object | table |
| array | array |
| string, boolean | string, boolean |
| integer, float | integer, float |
| `null` | none: leave the key out |

TOML has no `null`, so a setting yapi sets to `null` is removed from the file, and a `null` inside an array can't be written. No setting takes a date or time, so a TOML date makes yapi report the file as invalid and skip it, as it does for broken JSON. An integer above 9223372036854775807, the largest TOML holds, can't be written either.

Here are the same settings in both formats. In `settings.json`:

```json
{
  "defaultProvider": "anthropic",
  "defaultModel": "claude-sonnet-4-5",
  "compaction": { "enabled": true, "reserveTokens": 16384 },
  "packages": [
    "npm:@acme/pi-tools",
    { "source": "git:github.com/acme/skills@v2", "skills": ["!legacy/**"] }
  ],
  "projectOnly": { "themes": ["*"] }
}
```

In `settings.toml`:

```toml
# The model for new sessions.
defaultProvider = "anthropic"
defaultModel = "claude-sonnet-4-5"
packages = [
  "npm:@acme/pi-tools",
  { source = "git:github.com/acme/skills@v2", skills = ["!legacy/**"] },
]

[compaction]
enabled = true
reserveTokens = 16384

[projectOnly]
themes = ["*"]
```

Keys outside a table, such as `packages`, go before the first `[table]` header, as TOML requires. When every `packages` entry is an object, the list can also be an array of tables. The example above can't use one, because `npm:@acme/pi-tools` is a string:

```toml
[[packages]]
source = "git:github.com/acme/skills@v2"
skills = ["!legacy/**"]
```

### How yapi writes the file

`/settings`, `yapi config`, `yapi install` and the other commands that change settings keep your comments and layout. They rewrite only keys whose values differ, add new keys and delete removed ones. In a list such as `packages`, each entry that stays keeps its comments, a removed entry takes its own comments along, and a new entry follows the layout of the last one, on a line of its own in a multi-line list. If `packages` is an array of tables, it stays one until an entry is a plain source string, such as after `yapi install npm:@acme/pi-tools`. yapi then writes it as a multi-line inline array. The comments above the first `[[packages]]` header go above `packages`, and those above each other header go above its entry. Comments inside a table, on its header line, between its keys, at the end of a line or inside a list, go above its entry too. yapi writes a new array inline, and a new object as a `[table]` at the top level or as an inline table inside another table. A file keeps its CRLF line endings and its byte order mark.

### Switch to TOML

With yapi closed, write `settings.toml` with your settings, then delete `settings.json`. A yapi that's still running keeps writing the file it loaded, which can bring `settings.json` back beside the new file.

`yapi import pi` copies Pi's `settings.json`. When yapi's folder already holds `settings.toml`, the import skips Pi's settings and says so. Copy what you need into `settings.toml` by hand.

## Project packages from chest

yapi loads the [project packages](chest-extensions.md#how-yapi-loads-a-manifest) that chest manifests list. It reads `chest.toml` in the project folder, or the file that `CHEST_MANIFEST` names relative to the project folder, such as `chest.dev.toml`. The lock is the file beside the manifest named with `.lock`, such as `chest.dev.lock`.

For a `CHEST_MANIFEST` value that's absolute, starts with `~` or uses `..`, yapi warns and reads no project manifest, and the chest command stops with an error. The chest command reads `CHEST_MANIFEST` too, as [Which manifest chest uses](chest-manifest.md#which-manifest-chest-uses) describes.

`~/.chest/chest.toml` lists packages for every project. chest keeps its download cache and its record of what it placed in `~/.chest`.

A project manifest that installs anything loads only in a trusted project, and asks for trust as `.yapi` files do. A manifest with only a `[package]` table describes the project as a package and needs no trust.

## Project instructions

yapi reads one instruction file from the working folder and from each parent folder, and adds them to the system prompt. In each folder it takes the first of `AGENTS.override.md`, `AGENTS.md` and `CLAUDE.md` that exists. In a git worktree inside its main checkout, such as `.worktrees/feature`, the worktree's own file replaces the main checkout's file of the same name. Use them for build commands, conventions and anything else the agent should know about the project.

## Skills and prompt templates

Skills follow the [Agent Skills specification](https://agentskills.io/specification). Each skill is a folder with a `SKILL.md` file. yapi lists every skill's name and description in the system prompt, and the model reads the full instructions when a task calls for them. `/skill:name` loads one directly.

Prompt templates are Markdown files that become slash commands. Save this as `~/.yapi/agent/prompts/review.md`:

```markdown
---
description: Review staged git changes
argument-hint: "[focus]"
---
Review the staged changes. Focus on ${1:-correctness and error handling}.
```

`/review` then sends the prompt, and `/review concurrency` fills in the focus. Pi's [prompt template documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/prompt-templates.md) lists every argument form.

## Choose a theme

Choose a theme in `/settings` or with `"theme"` in settings. `--use-theme <name>` applies one for a single run. Theme files use Pi's JSON format, described in Pi's [theme documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/themes.md).

## Change key bindings

`~/.yapi/agent/keybindings.json` changes the keys. Pi's [key binding documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/keybindings.md) lists the format and every action. `/hotkeys` shows the bindings in effect.

## How yapi prepares images for the model

yapi prepares images in prompts, `@path` arguments and tool results before they reach the model:

- yapi converts BMP, TIFF, ICO, PNM, HDR, DDS and farbfeld images to PNG. This applies to images that arrive with a MIME type: remote procedure call (RPC) and SDK prompts, tool results and the clipboard. `@path` arguments and the `read` tool only recognize PNG, JPEG, GIF, WebP and BMP files, so they read other formats as text.
- With `"images": {"autoResize": true}`, the default, yapi scales larger images down to fit 2000×2000 pixels and 4.5 MiB as base64, or the model's own limits. It may send a scaled image as JPEG, with a note that gives the model its original size.
- yapi leaves out a prompt or `read` image that it can't convert or make fit, with a note that says so. A tool result keeps its original image unchanged.
- yapi converts a pasted clipboard image that isn't PNG, JPEG, WebP or GIF to PNG when you paste it, ignoring its EXIF orientation.
- `"images": {"blockImages": true}` keeps all images from the model.

Both settings are also in `/settings`.

## Show images in the terminal

In kitty, Ghostty, WezTerm, Warp, iTerm2 and Herdr, tool results show their images below the tool. Kitty's protocol takes PNG only, so yapi shows other formats as PNG copies. Other terminals, tmux and screen show a note with the image's type and size instead. In fullscreen mode, iTerm2 also shows the note, as in Pi.

`"terminal": {"showImages": false}` turns images off, and `"imageWidthCells"` sets their preferred width in cells, 60 by default. Both are in `/settings` when the terminal shows images. Set `"terminal": {"images": "kitty"}`, `"iterm2"` or `false` to override the detection. The `PI_IMAGE_PROTOCOL` environment variable, `kitty`, `iterm2` or `none`, does the same, and the setting takes precedence.

## Hyperlinks in the terminal

In a terminal that supports hyperlinks, Markdown links show only their text, and the paths in `read`, `write`, `edit` and `ls` calls link to their files. In other terminals, a Markdown link shows its URL after its text.

yapi detects hyperlink support in common terminals, such as iTerm2, Ghostty, kitty, WezTerm and VS Code, and asks tmux whether it passes hyperlinks on. Set `"terminal": {"hyperlinks": true}` or `false` to override the detection. The `PI_HYPERLINKS` environment variable, `1` or `0`, does the same, and the setting takes precedence.

## Terminal colors

Themes use 24-bit color in terminals that support it, such as iTerm2, Ghostty, kitty, WezTerm, VS Code and JetBrains IDEs, or when `COLORTERM` is `truecolor` or `24bit`. Elsewhere, and in tmux or screen unless `COLORTERM` says so, they use 256 colors. Set `"terminal": {"trueColor": true}` or `false` to override the detection. The `PI_TRUE_COLOR` environment variable, `1` or `0`, does the same, and the setting takes precedence.
