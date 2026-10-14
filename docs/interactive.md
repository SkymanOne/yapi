# Use yapi in the terminal

Run `yapi` from the folder to work in. To install yapi and choose a model first, follow the [Quickstart](quickstart.md).

The transcript shows your prompts, the model's responses, tool calls and their results. Prompts and commands go in the editor at the bottom. The footer shows the folder, the model, context usage and cost.

## Enter a prompt

Type a request and press Enter to send it. Shift+Enter adds a line, and Ctrl+G opens the prompt in `$EDITOR`.

- `@path` attaches a file. Tab completes the path.
- Ctrl+V pastes text, an image, or on macOS the paths of copied files. A pasted image is saved to a temporary file and its path goes in the editor.
- Up recalls earlier prompts.

A prompt or command sent while extensions are still starting, or while an extension command runs, waits its turn. yapi sends it once they finish, in the order you sent it.

## Follow the work

yapi shows each tool call as it runs. Ctrl+O expands or collapses tool output, and Ctrl+T shows or hides thinking.

Messages show Markdown with LaTeX math and Mermaid diagrams. "Mermaid diagrams" in `/settings` draws diagrams while a message streams, only once it is complete, or never.

## Change direction

Input sent while yapi works changes what it does next:

| What you want | Action |
|---|---|
| Adjust the current task | Type a message and press Enter |
| Add work after the current task | Type a message and press Alt+Enter |
| Edit queued messages | Press Alt+Up |
| Stop the current task | Press Escape |

## Keys

These are the defaults. `/hotkeys` shows the bindings in effect, and `~/.yapi/agent/keybindings.json` changes them.

| Key | Action |
|---|---|
| Enter | Send the message, or steer the current turn while yapi works |
| Alt+Enter | Queue a follow-up for when yapi finishes |
| Shift+Enter | Insert a new line |
| Escape | Stop the agent. Twice on an empty editor opens the session tree, or what the `doubleEscapeAction` setting chooses. |
| Ctrl+C | Clear the editor. Twice quits. |
| Ctrl+D | Quit when the editor is empty |
| Ctrl+O | Expand or collapse tool output |
| Ctrl+T | Show or hide thinking |
| Ctrl+G | Write the message in `$EDITOR` |
| Ctrl+V | Paste from the clipboard |
| Ctrl+L | Open the model selector |
| Ctrl+P | Cycle models |
| Shift+Tab | Cycle the thinking level |
| Tab | Complete paths and commands |
| Up | Recall earlier messages |

## Slash commands

Type `/` to list commands. Prompt templates, skills (`/skill:name`) and extension commands appear in the same list.

| Command | Action |
|---|---|
| `/model`, `/thinking` | Switch the model or thinking level |
| `/settings` | Change settings such as the theme, auto-compaction or fullscreen mode |
| `/scoped-models` | Choose which models Ctrl+P cycles through |
| `/new`, `/resume` | Start a session or open an earlier one |
| `/tree` | Browse the session's branches and jump to an earlier point |
| `/fork`, `/clone` | Branch from an earlier message or duplicate the session |
| `/compact` | Summarize older context to free up the context window |
| `/name`, `/session` | Name the session or show its file, tokens and cost |
| `/copy`, `/export` | Copy the last answer or export the session to HTML |
| `/login`, `/logout` | Manage provider credentials |
| `/import` | Copy a session file into the session folder and resume it |
| `/llama` | Load, unload and download models on a llama.cpp server. Built-in extension. |
| `/mcp` | Manage MCP servers. Built-in extension. |
| `/trust` | Change whether the project's `.yapi` folder is trusted |
| `/reload` | Reload settings, keybindings, skills, themes and extensions from disk |
| `/hotkeys` | Show every key binding |
| `/debug` | Write the screen, the session's messages and recent extension output to `~/.yapi/agent/yapi-debug.log` |
| `/quit` | Quit yapi |

`/llama` and `/mcp` come from built-in extensions, so `-ne` removes them. The `/debug` log can hold private data from messages and extension output. Read it before you attach it to a bug report.

## Run a shell command

Start a message with `!` to run a shell command and add its output to the conversation:

```text
!git status
```

Use `!!` to run a command without sending its output to the model.

## Display modes

yapi draws in fullscreen mode by default. Regular mode keeps the transcript in the terminal's scrollback instead. Switch with `/settings` or `--tui-mode regular`.

In fullscreen mode, yapi handles the mouse:

- The wheel scrolls the transcript. Hold Alt to scroll five times as far. "Fullscreen wheel scrolling" in `/settings` sets a fixed number of lines per step.
- Drag to select text. A double click selects a word or path, and a triple click selects a line. Releasing the button copies the selection. With "Fullscreen copy on select" off, press Ctrl+X to copy it.
- A click on a tool call, thinking, a skill or a summary expands or collapses it. Ctrl+O and Ctrl+T still set every item at once.
- A click on a web or mail link opens it, and a click on a file link shows the file in its folder. Other links do not open, so a click never starts a program. See [Links](configuration.md#links).
- A click in the editor, a list or a dialog moves the cursor or chooses the item.
- Drag the scrollbar's thumb, or click its track to jump. With "Fullscreen scrollbar" set to `auto`, pointing at the scrollbar shows it, except inside tmux, Zellij and GNU Screen.
