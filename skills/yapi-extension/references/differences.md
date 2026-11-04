# Where Pi extensions behave differently in yapi

yapi runs Pi extensions in a QuickJS-NG runtime compiled to WebAssembly, not in Node.js. Extensions behave as in Pi except where this page says otherwise. It lists what changes how you write an extension: the runtime, the API, the interface and packages. [Differences from Pi](https://skymanone.github.io/yapi/compat.html) lists every difference with its reason, and an installed yapi keeps a copy at `~/.yapi/agent/docs/compat.md`.

## Runtime limits and Node shims

The runtime has compute and memory limits, and Node's built-in modules are shims:

- Each call into the extension may compute for 60s, or 10s while the extension loads, and the runtime may use 1 GiB of memory. A trapped runtime restarts and its extensions load again.
- `fs`, `path`, `os`, `child_process`, `fetch` and `dns.lookup` work.
- `require()` of an ES module fails. Use `import`.
- yapi installs native addons unbuilt, and they fail when an extension loads them. Load them lazily, so the rest of the extension still works.
- `Intl` covers Unicode segmentation and number and plural formatting in `en-US`. `Intl.DateTimeFormat` and `Date` locale formatting honor `timeZone` but no other option.
- `process.arch` is `wasm32`. Console output goes to stderr in print, JSON and remote procedure call (RPC) modes. Interactive mode keeps it off the screen, and `/debug` writes its last 200 lines to `~/.yapi/agent/yapi-debug.log`.
- Error stacks show only frames in the extension's own files.

These import but throw when called:

- `http` requests, `net` sockets and servers, `dns` queries other than `lookup`, `tls`, `http2`, `dgram` and `cluster`
- `worker_threads` workers, `node:sqlite`, `zlib` compression, `vm`, `v8` and `inspector`
- `child_process.fork`, `util.parseArgs`, `stream.pipeline`, `stream.finished`, `events.on` and `process.chdir`

## Extension API differences

Pi's API works as in Pi, with these exceptions:

- Pi's internal classes, such as `SettingsManager`, `ModelRuntime` and `ExtensionRunner`, import but throw when called.
- Built-in tool factories return yapi's tools, or Pi's own when given custom `operations`, a `spawnHook` or a `commandPrefix`. The `grep` and `find` factories refuse those options.
- `createAgentSession` and the rest of Pi's SDK throw, by design. Run a subagent as another yapi instead. Pi's `RpcClient` starts yapi in RPC mode and can prompt, steer, follow up and abort it. Pi's `subagent` example runs `child_process.spawn(process.execPath, ["--mode", "json", "-p", "--no-session", task])` and reads its events as they arrive.
- `newSession`'s `setup` writes to the new session only with `appendMessage`, `appendCustomEntry` and `appendSessionInfo`. Other `SessionManager` writes throw.
- `pi.events` listeners in the emitter's runtime instance run during `emit`, as in Pi. Native extensions and other instances hear the event right after `emit` returns, with a JSON copy of the data.
- `onTerminalInput` handlers run while yapi keeps drawing, so keys that arrive together pass through them before yapi handles the first.
- In yapi's own editor, an autocomplete provider's `applyCompletion` runs for every suggestion as they arrive, and its `signal` never aborts. An editor from `setEditorComponent` behaves as in Pi.
- A message shows as written until its `registerMarkdownTransformer` transformers return.
- yapi shows code in one color. Extensions can color it with `pi.yapi?.registerCodeRenderer`, which Pi doesn't have.
- `registerMcpServer` needs the extension's `process` grant for a stdio server and its `network` grant for an HTTP server. With the default grants it behaves as in Pi.

## Interface components

Components render inside the runtime, which changes what they can show:

- yapi paints a component's last frame, so a slow component never blocks the interface.
- pi-tui reports a terminal without hyperlinks or images, so links and images use pi-tui's text fallbacks. A hyperlink a component writes itself still works, unless its URL holds a control character.
- In fullscreen mode, components get presses, clicks and the wheel through `handleMouse`, but no pointer moves, drags or releases, and a click doesn't give them the keyboard focus. Headers, message renderers and tool renderers get no mouse events.
- A theme change reaches extensions when their session next starts.

## Packages and grants

Packages install and load with yapi's own rules:

- yapi reads a `yapi` key in `package.json` before the `pi` key.
- A package from a chest manifest runs with only the capabilities its entry grants. Without `fs` it loads only its own files and dependencies, and those of the packages that share its runtime instance because they have the same grants and event policy. Calls outside its grants fail with "… is not permitted for this extension". Without `session`, yapi ignores the results of its event handlers that would change the conversation. A package can request capabilities with `capabilities` and `events` in the `yapi` key.
- Dependencies install with yapi's own npm client, without lifecycle scripts. It reads registries and credentials from `~/.npmrc` and `npm_config_*` variables, not from a project's `.npmrc` or npm's global configuration.
