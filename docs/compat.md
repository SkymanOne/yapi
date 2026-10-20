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
| Failing session actions | A throwing `setup` or `withSession` callback ends interactive mode | yapi reports the error and keeps the current session. |
| Console output | stdout and stderr | stderr only in print, JSON and RPC modes, because stdout carries their output. Interactive mode keeps it off the screen, and `/debug` writes its last 200 lines. |
| Project trust | Extensions loaded before trust can answer `project_trust` | No extension loads before you trust the project. |
| Codemode scripts | May compute without limit | Stop after 60 seconds of computing without awaiting a call. |
| Native extensions | None | WebAssembly components written in Rust. See [Native extensions](native-extensions.md). Unstable until 1.0. |

## Packages and installation

| Area | Pi | yapi |
|---|---|---|
| Directories | `~/.pi/agent`, project `.pi/`, `PI_CODING_AGENT_DIR` | `~/.yapi/agent`, project `.yapi/`, `YAPI_CODING_AGENT_DIR`, and `YAPI_` in place of `PI_` for the session directory variable. Same formats, and `yapi import pi` copies Pi's state. |
| Package installs | `npm install`, with lifecycle scripts | A built-in npm client that skips lifecycle scripts. `npmCommand` applies to npm sources only. |
| Packages that fail to install | Pi exits at startup, and `/reload` or the session change fails | yapi warns at startup and lists the package under `[Extension issues]` after `/reload` or a session change, which try it again. It runs without the package, so one unreachable source does not stop the others. |
| npm configuration | npm reads all its configuration files and settings | Registries and credentials from `~/.npmrc`, or the file `npm_config_userconfig` names, and from `npm_config_*` variables. Other settings, such as `cafile`, are ignored. |
| Project folder | The working folder | `--project-dir <dir>`, a yapi-only flag, takes the project's files, trust and `AGENTS.md` from another folder while tools run in the working folder. Subagents need it to work in one folder with their parent's project. See [Project trust](security.md#project-trust). |
| Package manifests | The `pi` key | A `yapi` key of the same shape takes precedence, so one package can ship a native build for yapi. |
| Self-update | `pi update` updates Pi | `yapi update` prints how to upgrade instead. `--extensions` and `--models` update packages and model catalogs. The update notice takes the version from yapi's latest GitHub release and links the releases page as the changelog. Pi's release note has no counterpart. See [Install](install.md#upgrade). |
| `new` | `pi new ...` is a prompt | `yapi new` creates a native extension. Quote a prompt that starts with `new`. |

## Interface and providers

| Area | Pi | yapi |
|---|---|---|
| `/share` and `/bug` | Use Pi's services | Not available. |
| Clipboard paste | Copied files, images and text on every platform | Nothing on Windows, and images copied in Windows do not reach WSL. |
| Word motions in Chinese, Japanese and Thai | By dictionary word | By character, to keep ICU's dictionaries out of the binary. |
| Time zones | Node's own time zone data | The system's time zone database. With `TZ` set to a zone name on a system without one, times show in UTC. On Windows they show in UTC unless `TZ` holds a POSIX rule such as `EST5EDT`. |
| Links | A click in fullscreen mode opens any link in its default app. Link URLs reach the terminal as written. | A click opens only `http`, `https` and `mailto` links, and shows a linked file in its folder. Opening a link to a script or an app, or with another scheme, could run a program. A link whose URL holds a control character shows as plain text, because the character could end the link and reach the terminal as a command. |
| Syntax highlighting | Highlights code in messages and tool rows | Shows code in one color, to keep memory use and install size low. Extensions can add highlighting with [`pi.yapi.registerCodeRenderer`](extensions.md#code-renderers). |
| Markdown transformers | Change a message's Markdown before it shows | A message shows as written in its first frame, and each change while it streams shows transformed one frame late. |
| Images in the terminal | A Herdr pane shows images when the outer terminal's variables reach it, such as Ghostty's. Image data reaches the terminal as written. | A Herdr pane shows Kitty images in any terminal, as Herdr draws them itself. An image whose data is not base64 shows as a note, because such data could reach the terminal as a command. |
| Mouse in fullscreen mode | Extension editors and components get mouse events. | Extension headers, message renderers and tool renderers get no mouse events. Other extension components get presses, clicks and the wheel, but no pointer moves, drags or releases, and a click does not give them the keyboard focus. |

## Open gaps

Pi features yapi does not have yet, or has only in part. Each row links its issue.

| Area | Pi | yapi today | Issue |
|---|---|---|---|
| Complete pi-ai providers | A `Provider` passed to `registerProvider` can limit its models per account with `filterModels`, and checks for an API key on every availability refresh. `/login` asks for its key with its own prompt. | `filterModels` and `filterAllModels` are ignored. The API key check runs once as the extension loads, and `/login` asks for the key with yapi's prompt. | [#54](https://github.com/SkymanOne/yapi/issues/54) |
| `newSession` setup | `setup` gets the new session's writable session manager before its extensions start | `setup` runs after the new session's `session_start` and gets the read-only session manager. | [#67](https://github.com/SkymanOne/yapi/issues/67) |
| Provider credentials in extensions | `ctx.modelRegistry.getProviderAuth` returns a provider's API key and headers | Rejects. | [#176](https://github.com/SkymanOne/yapi/issues/176) |
