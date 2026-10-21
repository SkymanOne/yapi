# Differences from Pi

yapi follows Pi `v1.0.0` and behaves like Pi except as listed here. Any other difference is a bug. Please [report it](https://github.com/SkymanOne/yapi/issues).

yapi is tested against Pi itself. End-to-end scenarios compare yapi with output recorded from Pi: terminal screens, JSON event streams, print mode runs and RPC sessions. Settings, credentials and session files round-trip byte for byte. [Extensions](extensions.md#compatibility-with-pi) has the extension and package results.

## Extensions

Pi extensions run unchanged, in WebAssembly rather than Node.js.

| Area | Pi | yapi |
|---|---|---|
| Runtime | Pi's Node.js process | QuickJS-NG in WebAssembly: 60 seconds of computing per call, 1 GiB per instance, restart after a crash. See [Sandboxing](security.md#sandboxing). |
| Node APIs | All of Node | [Shims](extensions.md#how-pi-extensions-run). Sockets, servers, `http` requests, `worker_threads`, `node:sqlite`, `zlib`, `vm` and `child_process.fork` throw when used. `require()` of an ES module fails. |
| Native addons | Loaded | Installed, never built. Loading one fails. |
| Pi internals | Every export of Pi's packages | The extension API and its helpers. Other exports throw when called, and built-in tool factories reject custom `operations`. |
| Agent sessions in an extension | `createAgentSession` and Pi's SDK | Not supported, by design: such a session would run tools and use credentials outside the extension's grants. Run a subagent as another yapi, with Pi's `RpcClient` or as Pi's `subagent` example does. |
| Extension UI | Components detect links, images and theme changes | Links and images fall back to text. Theme changes reach extensions at their next session. |
| `pi.events` | Every listener runs during `emit`, in the order of subscription | Listeners in the emitter's runtime instance run during `emit`. Native extensions and other instances get the event after `emit` returns, as a JSON copy, and data JSON cannot carry stays in the emitter's instance. An instance that is busy holds at most 64 events from others and drops later ones until it catches up. |
| Terminal input listeners | Pi handles each key before the next one reaches them | They run while yapi keeps drawing, so keys that arrive together pass through them together. |
| Focused component | `tui.focusedComponent` is the component that has the keyboard | The extension's own editor when it replaced yapi's. Otherwise a stand-in, as yapi's editor, selectors and dialogs are not objects in the runtime: an `Editor` with no state while the prompt editor has the keyboard, and an empty object while a selector, a dialog or a component has it. |
| Autocomplete providers | The editor waits for providers | yapi's editor never waits: `applyCompletion` runs for every suggestion, and `signal` never aborts. Editors from `setEditorComponent` behave as in Pi. |
| `newSession` setup | `setup` gets the new session's whole `SessionManager` | It reads the session and writes with `appendMessage`, `appendCustomEntry` and `appendSessionInfo`. Other writes throw. |
| Failing session actions | A throwing `setup` or `withSession` callback ends interactive mode | yapi reports the error and keeps the current session. |
| Console output | stdout and stderr | stderr only in print, JSON and RPC modes, because stdout carries their output. Interactive mode keeps it off the screen, and `/debug` writes its last 200 lines. |
| Project trust | Extensions loaded before trust can answer `project_trust` | No extension loads before you trust the project. |
| Codemode scripts | May compute without limit | Stop after 60 seconds of computing without awaiting a call. |
| Model availability of extension providers | `getAvailableOfType` runs a pi-ai `Provider`'s API key `check`, `filterModels` and `filterAllModels` on each call | `getAvailableOfType` returns what they kept at the last refresh. They run in the extension's runtime instance as extensions load, on `/login`, `/logout` and `--api-key`, when the model selector opens, on `/reload`, on `ctx.modelRegistry.refresh` and after an extension registers or unregisters a provider. The host answers model queries from codemode and extensions without calling into a runtime instance, so a busy instance never blocks them. |
| Native extensions | None | WebAssembly components written in Rust. See [Native extensions](native-extensions.md). Unstable until 1.0. |

## Packages and installation

| Area | Pi | yapi |
|---|---|---|
| Directories | `~/.pi/agent`, project `.pi/`, `PI_CODING_AGENT_DIR` | `~/.yapi/agent`, project `.yapi/`, `YAPI_CODING_AGENT_DIR`, and `YAPI_` in place of `PI_` for the session directory variable. Same formats, and `yapi import pi` copies Pi's state. |
| Package installs | `npm install`, with lifecycle scripts | A built-in npm client that skips lifecycle scripts. `npmCommand` applies to npm sources only. Its output always goes to stderr and it reads no input, where Pi does this only when stdout carries JSON or RPC output. |
| Git sources | A host or ref may start with `-` | A host or ref that starts with `-` is refused, because git would read it as an option. Clones run as `git clone -- <repo>`. |
| Sign-ins after `yapi import pi` | Pi keeps its sign-ins in its own `auth.json` and `mcp-auth.json` | `yapi import pi` copies API keys but not subscription or MCP sign-ins, and lists the ones it left out. A provider can replace its refresh token on each use, so one shared copy would sign the other tool out. Sign in again with `/login` or `/mcp login`. |
| Packages that fail to install | Pi exits at startup, and `/reload` or the session change fails | yapi warns at startup and lists the package under `[Extension issues]` after `/reload` or a session change, which try it again. It runs without the package, so one unreachable source does not stop the others. |
| npm configuration | npm reads all its configuration files and settings | Registries and credentials from `~/.npmrc`, or the file `npm_config_userconfig` names, and from `npm_config_*` variables. Other settings, such as `cafile`, are ignored. |
| Log file | `/debug` writes `pi-debug.log` | `--enable-logging`, a yapi-only flag, also appends a running log to `yapi.log` in the agent folder. See [Logs](troubleshooting.md#logs). |
| Project folder | The working folder | `--project-dir <dir>`, a yapi-only flag, takes the project's files, trust and `AGENTS.md` from another folder while tools run in the working folder. Subagents need it to work in one folder with their parent's project. See [Project trust](security.md#project-trust). |
| Package manifests | The `pi` key | A `yapi` key of the same shape takes precedence, so one package can ship a native build for yapi. |
| Self-update | `pi update` updates Pi | `yapi update` prints how to upgrade instead. `--extensions` and `--models` update packages and model catalogs. The update notice takes the version from yapi's latest GitHub release and links the releases page as the changelog. Pi's release note has no counterpart. See [Install](install.md#upgrade). |
| `new` | `pi new ...` is a prompt | `yapi new` creates a native extension. Quote a prompt that starts with `new`. |

## Interface and providers

| Area | Pi | yapi |
|---|---|---|
| `/share` | Shares a session through Pi's services | Not available. |
| `/bug` | Uploads a private report to Pi's service at radius.pi.dev, or exports it as a zip file | Always writes the zip file to the working folder, then offers to post a public issue on [SkymanOne/yapi](https://github.com/SkymanOne/yapi/issues) with the GitHub CLI. The report has no crash log. Its diagnostics come from the current branch, and its `session.jsonl` holds the branch without Pi's system prompt and tool list. The session records the report as a `yapi.bug-report` entry. |
| Clipboard paste | Copied files, images and text on every platform | Nothing on Windows, and images copied in Windows do not reach WSL. |
| Word motions in Chinese, Japanese and Thai | By dictionary word | By character, to keep ICU's dictionaries out of the binary. |
| Time zones | Node's own time zone data | The system's time zone database. With `TZ` set to a zone name on a system without one, times show in UTC. On Windows they show in UTC unless `TZ` holds a POSIX rule such as `EST5EDT`. |
| Links | A click in fullscreen mode opens any link in its default app. Link URLs reach the terminal as written. | A click opens only `http`, `https` and `mailto` links, and shows a linked file in its folder. Opening a link to a script or an app, or with another scheme, could run a program. A link whose URL holds a control character shows as plain text, because the character could end the link and reach the terminal as a command. |
| Syntax highlighting | Highlights code in messages and tool rows | Shows code in one color, to keep memory use and install size low. Extensions can add highlighting with [`pi.yapi.registerCodeRenderer`](extensions.md#code-renderers). |
| Markdown transformers | Change a message's Markdown before it shows | A message shows as written in its first frame, and each change while it streams shows transformed one frame late. |
| Images in the terminal | A Herdr pane shows images when the outer terminal's variables reach it, such as Ghostty's. Image data reaches the terminal as written. | A Herdr pane shows Kitty images in any terminal, as Herdr draws them itself. An image whose data is not base64 shows as a note, because such data could reach the terminal as a command. |
| Mouse in fullscreen mode | Extension editors and components get mouse events. | Extension headers, message renderers and tool renderers get no mouse events. Other extension components get presses, clicks and the wheel, but no pointer moves, drags or releases, and a click does not give them the keyboard focus. |

## Tools and sessions

| Area | Pi | yapi |
|---|---|---|
| `edit` on a file that is not UTF-8 | Reads it as UTF-8 and writes every invalid byte back as U+FFFD | Refuses the edit with "File is not valid UTF-8.", so bytes outside the edit never change. |
| Line endings after `edit` | The whole file takes the line ending of its first line break | Lines the edit leaves unchanged keep their own line endings. Changed lines take the ending of the first line break. |
| Session lines with an unpaired surrogate | Text cut inside an emoji is written as an unpaired UTF-16 escape, such as `\ud83d` | The line loads with U+FFFD in place of the unpaired half, as Rust strings cannot hold one. |
| Failed session writes | Later entries can point to an entry that is missing from the file | yapi reports the first failed write and saves the missed entries with the next write, so the history stays whole. |
