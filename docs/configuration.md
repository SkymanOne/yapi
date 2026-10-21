# Configure yapi

yapi keeps global configuration in `~/.yapi/agent` and project configuration in a project's `.yapi` folder. Set `YAPI_CODING_AGENT_DIR` to move the global folder. Every file uses Pi's format, described in Pi's [settings documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/settings.md).

| What | Where |
|---|---|
| Settings | `~/.yapi/agent/settings.json`, overridden by `.yapi/settings.json` |
| Project instructions | `AGENTS.override.md`, `AGENTS.md` or `CLAUDE.md` in the project, its parent folders and `~/.yapi/agent` |
| System prompt | `SYSTEM.md` replaces it and `APPEND_SYSTEM.md` extends it, in `.yapi/` or `~/.yapi/agent/` |
| Skills | `~/.yapi/agent/skills`, `~/.agents/skills`, `.yapi/skills` and `.agents/skills` |
| Prompt templates | `~/.yapi/agent/prompts` and `.yapi/prompts` |
| Themes | `~/.yapi/agent/themes` and `.yapi/themes` |
| Key bindings | `~/.yapi/agent/keybindings.json` |
| Models and providers | `~/.yapi/agent/models.json` |
| Credentials | `~/.yapi/agent/auth.json` |
| MCP servers | `~/.yapi/agent/mcp.json` and `.yapi/mcp.json` |
| Documentation for the model | `~/.yapi/agent/docs`. See [Install yapi](install.md#documentation-for-the-model). |

`/settings` edits common settings in place. Files in a project's `.yapi` folder load only after you [trust the project](security.md#project-trust).

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

## Themes

Choose a theme in `/settings` or with `"theme"` in settings. `--use-theme <name>` applies one for a single run. Theme files use Pi's JSON format, described in Pi's [theme documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/themes.md).

## Key bindings

`~/.yapi/agent/keybindings.json` changes the keys. Pi's [key binding documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/keybindings.md) lists the format and every action. `/hotkeys` shows the bindings in effect.

## Images

yapi prepares images in prompts, `@path` arguments and tool results before they reach the model:

- Images that are not PNG, JPEG, GIF or WebP are converted to PNG when they are BMP, TIFF, ICO, PNM, HDR, DDS or farbfeld. This applies to images that arrive with a MIME type: RPC and SDK prompts, tool results and the clipboard. `@path` arguments and the `read` tool only recognize PNG, JPEG, GIF, WebP and BMP files, so other formats are read as text.
- With `"images": {"autoResize": true}`, the default, larger images are scaled down to fit 2000×2000 pixels and 4.5 MiB as base64, or the model's own limits. A scaled image may be sent as JPEG, with a note that gives the model its original size.
- A prompt or `read` image that cannot be converted or made to fit is left out, with a note that says so. A tool result keeps its original image unchanged.
- A pasted clipboard image that is not PNG, JPEG, WebP or GIF is converted to PNG when pasted, ignoring its EXIF orientation.
- `"images": {"blockImages": true}` keeps all images from the model.

Both settings are also in `/settings`.

## Images in the terminal

In kitty, Ghostty, WezTerm, Warp, iTerm2 and Herdr, tool results show their images below the tool. Kitty's protocol takes PNG only, so other formats are shown as PNG copies. Other terminals, tmux and screen show a note with the image's type and size instead. In fullscreen mode, iTerm2 also shows the note, as in Pi.

`"terminal": {"showImages": false}` turns images off, and `"imageWidthCells"` sets their preferred width in cells, 60 by default. Both are in `/settings` when the terminal shows images. Set `"terminal": {"images": "kitty"}`, `"iterm2"` or `false` to override the detection. The `PI_IMAGE_PROTOCOL` environment variable, `kitty`, `iterm2` or `none`, does the same, and the setting takes precedence.

## Links

In a terminal that supports hyperlinks, Markdown links show only their text, and the paths in `read`, `write`, `edit` and `ls` calls link to their files. In other terminals, a Markdown link shows its URL after its text.

yapi detects hyperlink support in common terminals, such as iTerm2, Ghostty, kitty, WezTerm and VS Code, and asks tmux whether it passes hyperlinks on. Set `"terminal": {"hyperlinks": true}` or `false` to override the detection. The `PI_HYPERLINKS` environment variable, `1` or `0`, does the same, and the setting takes precedence.
