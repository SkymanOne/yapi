# Write native extensions in Rust

Native extensions are WebAssembly components written in Rust with the [`yapi-extension-api`](https://docs.rs/yapi-extension-api) crate. Like Pi extensions, they register tools, commands, flags, event handlers and interface components, and they use Pi's JSON shapes for events and results. Each one runs in a WebAssembly instance of its own, with no JavaScript runtime. This page covers creating, building, testing and sharing an extension, then the SDK's API.

The native extension API is unstable until yapi 1.0. The Wasm Interface Type (WIT) world, the Rust SDK and the host requests may change in any release before then, and you may need to rebuild extensions. Pi extensions aren't affected.

## Create an extension

The SDK needs Rust 1.88 or newer, or 1.91 with the `widgets` feature. Add the `wasm32-wasip2` target to your Rust toolchain:

```sh
rustup target add wasm32-wasip2
```

Create a project:

```sh
yapi new shout
```

`yapi new` writes a Cargo project that's also a yapi package:

```text
shout/
├── .cargo/config.toml   makes wasm32-wasip2 the default target
├── .gitignore
├── Cargo.toml           a cdylib crate that depends on yapi-extension-api 0.4.2
├── README.md
├── chest.toml           the capabilities the extension asks for
├── extensions/          where the built extension goes
├── package.json         names extensions/shout.wasm for yapi
└── src/lib.rs           the hello example, to replace with your own
```

The package takes the folder's name, or the one `--name` gives. Names use lowercase ASCII letters, digits, `-` and `_`, and start with a letter. The same template works with [cargo-generate](https://github.com/cargo-generate/cargo-generate):

```sh
cargo generate --git https://github.com/SkymanOne/yapi --tag v0.4.2 \
  crates/yapi/templates/extension
```

To start without the template, create a library crate with `crate-type = ["cdylib"]` and depend on the SDK version of the yapi release you target:

```toml
[dependencies]
yapi-extension-api = "0.4.2"
```

An extension registers its features in an init function and exports it with `extension!`. Tools, commands and event handlers are `async` closures. This `src/lib.rs` registers a `shout` tool and a `/hello` command:

```rust
use yapi_extension_api::{Api, Tool, ToolResult, json, notify};

fn init(api: &mut Api) {
    api.register_tool(Tool::new(
        "shout",
        "Repeats the text in capitals",
        json!({
            "type": "object",
            "properties": {"text": {"type": "string"}},
            "required": ["text"]
        }),
        |params, _ctx| async move {
            let text = params["text"].as_str().unwrap_or_default();
            Ok(ToolResult::text(text.to_uppercase()))
        },
    ));
    api.register_command("hello", "Says hello", |args, _ctx| async move {
        let name = if args.is_empty() { "world" } else { &args };
        notify(&format!("Hello, {name}!"), "info");
        Ok(())
    });
}

yapi_extension_api::extension!(init);
```

Build the extension, then copy `shout.wasm` from Cargo's target folder into the package's `extensions` folder:

```sh
cargo build --release
cp target/wasm32-wasip2/release/shout.wasm extensions/
```

A crate without the template's `.cargo/config.toml` builds with `cargo build --release --target wasm32-wasip2`.

## Declare capabilities

The features of `yapi-extension-api` declare the capabilities an extension uses, the same ones a package requests in [`capabilities`](chest-manifest.md#what-each-capability-allows). Each feature enables the requests that need its grant:

| Feature | Enables |
|---|---|
| `fs` | `context.files` |
| `net` | `fetch`, `fetch.stream`, `dns.lookup`, and the model and credential operations such as `ai.stream` and `models.apiKey` |
| `env` | `env`, which reads environment variables |
| `session` | `send_message`, `append_entry`, and the requests that change the session, such as `session.sendMessage`, `session.reload` and `model.set` |
| `all` | Everything above, plus `exec`, `Process`, registering providers, terminal input listeners and autocomplete providers |

The template starts with `session`, which its `hello` code needs:

```toml
[dependencies]
yapi-extension-api = { version = "0.4.2", features = ["session"] }
```

Use the version shown here or a newer one. Older releases may lack the capability features, and cargo then reports that the feature doesn't exist.

A request whose feature the build leaves out fails without reaching yapi, with a message such as ``request `session.appendEntry` needs the `session` feature of yapi-extension-api``. `exec`, `Process`, `send_message` and `append_entry` exist only with their features. Requests that need a grant only for some payloads, such as `tool.execute` and `ui.editorSubmit`, need no feature, and yapi checks them as they run.

### Check the build's capabilities

The build records its features in a `yapi:capabilities` custom section. After each build, run `chest check`. It compares the features, and the WebAssembly System Interface (WASI) interfaces the component imports, with the capabilities in the package's `chest.toml` or `package.json`:

```sh
cargo build --release
cp target/wasm32-wasip2/release/shout.wasm extensions/
chest check
```

[Check a native extension](chest-publish.md#check-a-native-extension) lists what `chest check` reports, and `chest add` shows the same comparison to the person who adds the package. yapi doesn't provide `wasi:http`, so a component that imports it doesn't load, and yapi refuses every `wasi:sockets` call. Reach the network through the SDK's `fetch` with the `net` feature.

Features declare what an extension uses. They grant nothing: yapi grants capabilities from the manifest that installs the package, and refuses requests beyond them. At startup, yapi warns about a native extension from a chest manifest that uses a capability its entry doesn't grant, because it will deny those calls.

### Move to capability features

Earlier versions of the SDK have no capability features and send every request. An extension that moves to this version stops building where it calls `exec`, `Process`, `send_message` or `append_entry`, and its other requests that need a grant fail at runtime. To move it:

1. Enable the features of the requests it sends, from the table in [Declare capabilities](#declare-capabilities).
2. List the same capabilities in `capabilities` under `[package]` in `chest.toml`, or `yapi.capabilities` in `package.json`.
3. Build it and run `chest check`.

## Try an extension without installing it

Load the package for one run:

```sh
yapi -e .
```

`-e` also takes the `.wasm` file itself.

To check that the package loads without a model or network access, run [`check-extension.py`](https://github.com/SkymanOne/yapi/blob/main/skills/yapi-extension/scripts/check-extension.py) from the [agent skills](agent-skills.md). It prints the registered commands and any load errors, and exits with status 1 on an error:

```sh
python3 check-extension.py .
```

## Install the extension

`yapi install` takes the package folder or the `.wasm` file, for every project or with `-l` for the current one:

```sh
yapi install .
yapi install . -l
```

yapi records the path in `settings.json` and loads the file in place, so a rebuild takes effect after `/reload` or a restart. `yapi list` shows the extension, `yapi remove <path>` uninstalls it and `yapi config` turns it off.

## Share the extension

Share a native extension as a package that contains the built `.wasm` file, not only the Rust sources. yapi never compiles code during installation, because a build runs `build.rs` scripts and procedural macros outside the sandbox. A project from `yapi new` is already such a package. Commit `extensions/shout.wasm` with the sources and push the repository, and others can install it by URL:

```sh
yapi install git:github.com/your_github_user/shout
```

To publish on npm, run `npm publish`. Others then install it with `yapi install npm:shout`. The template's `package.json` publishes the `extensions` folder and `chest.toml`, which records the capabilities the extension asks for.

yapi loads every `.wasm` file in a package's `extensions` folder, so a package needs no manifest. A `yapi` key in `package.json` can name the files instead, and lets one package ship a native build for yapi and a JavaScript build for Pi, as [Package manifests](packages.md#package-manifests) describes:

```json
{
  "name": "shout",
  "version": "0.1.0",
  "pi": { "extensions": ["./dist/shout.js"] },
  "yapi": { "extensions": ["./dist/shout.wasm"] }
}
```

A package requests capabilities and `pi.events` channels as a Pi package does, as [Request capabilities](extensions.md#request-capabilities) shows.

## What the SDK provides

The SDK's items cover registration, host actions, components and rendering:

| Item | Purpose |
|---|---|
| `Api::register_tool` | Offer a tool to the model. `Tool` sets a label, prompt snippet and prompt guidelines, and `Tool::constrained_sampling` takes Pi's `constrainedSampling`, such as a grammar the tool's input must follow on OpenAI models. |
| `Api::register_command` | Handle `/name args` |
| `Api::register_flag` | Accept `--name` on the command line, read later with `get_flag` |
| `Api::register_shortcut` | Run a handler when the user presses a key such as `alt+k`, as Pi's `pi.registerShortcut` does. An empty description lists the shortcut by the extension's path. |
| `Api::on` | Handle a Pi event such as `session_start` or `tool_call`. The return value is the handler's result in Pi, for example `{"block": true, "reason": "..."}`. A handler gets a copy of the event, so a `tool_call` handler changes the call's arguments by returning them as `input` instead of editing them. |
| `events::on`, `events::emit` | Subscribe to and emit events that every extension shares, Pi's `pi.events`. See [Events between extensions](#events-between-extensions). |
| `notify` | Show a notification |
| `send_message`, `append_entry` | Add a custom message or entry to the session, with the `session` feature |
| `exec` | Run a process and wait for it, with the `all` feature |
| `Process` | Run a process in the background, reading its output as it arrives, with the `all` feature |
| `sleep` | Wait a number of milliseconds |
| `spawn` | Run a future in the background, after the handler that started it has returned |
| `request` | Call any host action by name with a JSON payload and get its answer at once |
| `extension_path` | The extension's `.wasm` file, and the root of the package it was installed from |
| `project_dir` | The folder the session's project files come from, also while the extension loads. See [Where the project is](#where-the-project-is). |
| `op` | Start a host operation by name, such as a dialog or an HTTP request, and await its answer |
| `Context` | The mode, the working folder and the rest of Pi's `ctx`. In a tool, `Context::update` shows progress, `Context::tool_call_id` is the call's id, and `Context::aborted` resolves when the user aborts the run. See [Background work and cancellation](#background-work-and-cancellation). |
| `Component` | A piece of interface that renders lines for a width and handles keys and mouse events. See [Interface components](#interface-components). |
| `Context::custom`, `Context::set_widget`, `Context::set_footer`, `Context::set_header`, `Context::set_editor_component` | Show components, as Pi's `ctx.ui` does |
| `Tool::render_call`, `Tool::render_result`, `Api::register_message_renderer`, `Api::register_entry_renderer` | Draw tool calls, tool results, custom messages and custom entries in the transcript |
| `Api::register_markdown_transformer` | Change a message's Markdown before it shows |
| `Api::register_code_renderer` | Draw code in messages and tool rows. See [Code renderers](#code-renderers). |
| `Context::on_terminal_input`, `Context::add_autocomplete_provider`, `editor_text` | See raw keys before the editor does, complete text in the editor, and read the editor's text. See [Terminal input and completion](#terminal-input-and-completion). |
| `theme`, `request_render`, `parse_key`, `terminal_size`, `editor_focused` | Style text in the session's theme, render components again, name keys, read the terminal's size, and tell whether the prompt editor has the keyboard |
| `widgets` | pi-tui's widgets, such as `SelectList` and `Editor`, with the `widgets` feature |

[Native extension examples](native-examples.md) walks through 19 complete extensions.

### Host requests

`request(kind, payload)` sends a request to yapi and returns its JSON answer. yapi supports these kinds for native extensions:

| Kind | Payload | Answer |
|---|---|---|
| `log` | `{"level", "message"}`, with `level` one of `debug`, `info`, `warn` and `error` | `null`. yapi treats the message as console output from a Pi extension, which interactive mode keeps off the screen for `/debug`. |
| `cwd` | `{}` | The working folder extensions see, as a string |
| `exec.sync` | `{"command", "args", "cwd", "env", "envAdd", "input", "timeout"}`, all optional except `command` | `{"stdout", "stderr", "code", "signal", "killed"}` once the process exits |
| `execPath` | `{}` | The path of the running yapi binary, to start another yapi |
| `ui.notify` | `{"message", "type"}`, with `type` one of `info`, `warning` and `error`, or left out | `null`, as Pi's `ctx.ui.notify` |
| `session.sendMessage` | `{"message": {"customType", "content", "display", "details"}, "options": {"triggerTurn", "deliverAs"}}` | `null`, as Pi's `pi.sendMessage` |
| `session.appendEntry` | `{"customType", "data"}` | `null`, as Pi's `pi.appendEntry` |
| `session.read` | `{"method", "args"}` | The result of Pi's `ctx.sessionManager.<method>(...args)` |

`exec.sync` runs `command` with `args` and waits for it:

- Without `cwd` the process starts in yapi's working folder. `env` replaces the whole environment, as in Node, and `envAdd` sets variables on top of the inherited environment or of `env`.
- yapi writes `input` to standard input. `timeout` kills the process after that number of milliseconds.
- `code` is `null` when a signal ended the process, and `killed` is `true` when the timeout did.
- A process that can't start answers `{"stdout": "", "stderr": "", "code": null, "error"}`, where `error` is Node's spawn error, such as `spawn git ENOENT`. The request fails when the extension may not run processes.

`session.read` takes the method's arguments as the array `args`, such as `["<entry id>"]` for `getEntry`. The methods are `getCwd`, `getSessionDir`, `getSessionId`, `getSessionFile`, `getSessionName`, `isPersisted`, `getHeader`, `getEntries`, `getEntry`, `getChildren`, `getLabel`, `getLeafId`, `getLeafEntry`, `getBranch`, `buildContextEntries`, `buildSessionProjection` and `buildSessionContext`. Entries have the shapes of Pi's session files.

`exec.sync` needs the `all` feature, and `session.sendMessage` and `session.appendEntry` need the `session` feature. See [Declare capabilities](#declare-capabilities).

`ui.notify` and the `session` requests fail while the init function runs, before yapi binds the extension to a session. Every other kind is internal to yapi and may change or disappear in any release.

### Host operations

`op(kind, payload)` starts an operation and returns a future of its JSON answer. Other handlers and background tasks run while it waits. yapi supports these kinds for native extensions:

| Kind | Payload | Answer |
|---|---|---|
| `timer` | `{"ms"}` | `null` after `ms` milliseconds. `sleep` wraps it. |
| `fetch` | `{"url", "method", "headers", "body"}`, with `bodyBase64` in place of `body` for bytes | `{"status", "statusText", "headers", "bodyBase64"}` once the whole response has arrived |
| `ui.select` | `{"title", "options"}` | The option picked, or `null` when cancelled, as Pi's `ctx.ui.select` |
| `ui.confirm` | `{"title", "message"}` | `true` or `false`, as Pi's `ctx.ui.confirm` |
| `ui.input` | `{"title", "placeholder"}` | The text entered, or `null` when cancelled, as Pi's `ctx.ui.input` |

`fetch` needs the `net` feature, and fails when the extension may not use the network. The dialogs answer `null` or `false` at once in print and JSON modes, which have no interface, so check `Context::has_ui` first. Every other kind is internal to yapi and may change or disappear in any release.

### Background processes

`Process::spawn` starts a process that runs while the extension does other work. It takes `{"command", "args", "cwd", "env", "envAdd", "stdin"}`, all optional except `command`:

- `cwd`, `env` and `envAdd` work as in `exec.sync`. `"stdin": "ignore"` gives the process empty standard input instead of a pipe.
- `Process::next` waits for the next `ProcessEvent`: output on standard output or standard error, then the exit. A dropped `next` future loses no output.
- `write` waits while the process hasn't taken the previous write. `close_stdin` and `kill` act at once.

Dropping a `Process` kills it, and so does stopping the extension. `Process` needs the `all` feature, and starting one needs the process grant, which a chest entry gets only with `all`.

A process inherits yapi's environment, so `envAdd` is how an extension gives it a variable without the environment grant. yapi adds the variables itself, and the extension never sees the values the process inherits. This runs a build script with one added variable:

```rust
let output = request("exec.sync", &json!({
    "command": "sh",
    "args": ["scripts/build.sh"],
    "envAdd": {"BUILD_MODE": "release"},
}))?;
```

### Where the extension is

`extension_path()` returns the extension's `.wasm` file as `file`, and the root of the package it was installed from as `package_root`. `package_root` is `None` for a file given with `-e` or found in an `extensions` folder. An extension that starts another yapi with itself loaded passes `file` to `-e`, and one that ships scripts or data finds them next to `file` or under `package_root`.

### Where the project is

`project_dir()` returns the folder the session's project files come from, the same folder as `projectDir` in `Context::data`. It's the working folder unless yapi runs with `--project-dir`. The init function can call it before any `Context` exists, such as to list project files in a tool's description.

yapi runs the init function again for every new, resumed, forked or reloaded session, and each run sees the project of the session it loads for. Older versions of yapi answer `None`, and an extension then falls back to the working folder from the `cwd` request.

## Events between extensions

`events::emit(channel, &data)` and `events::on(channel, handler)` are Pi's `pi.events`, a bus that every extension shares, native or Pi, whichever runtime it runs in. This extension notifies on `deploy:done` and emits it from a `/deploy` command:

```rust
use yapi_extension_api::events;

fn init(api: &mut Api) {
    events::on("deploy:done", |data| {
        let version = data["version"].as_str().unwrap_or_default();
        notify(&format!("Deployed {version}"), "info");
    });
    api.register_command("deploy", "Deploys", |_args, _ctx| async move {
        events::emit("deploy:done", &json!({"version": "1.2.0"}));
        Ok(())
    });
}
```

`emit` runs the extension's own handlers for the channel before it returns, in the order they subscribed, as Pi does. Other extensions receive a JSON copy of the data after `emit` returns, in the order the extension emitted it.

`on` returns a `Subscription`, and `Subscription::unsubscribe` stops the handler. Dropping the `Subscription` keeps the handler. A handler runs to completion before the next one starts, so a handler that waits for host work starts it with `spawn`. When the extension loads again for another session, yapi drops every handler and the init function subscribes again.

A busy runtime holds at most 64 events from other runtimes. yapi drops later ones until it catches up and logs a warning, so a stuck extension never holds up the others or the interface.

Events cross between extensions according to their grants and event policies, as [Agent sessions and events](security.md#agent-sessions-and-events) describes. `on` tells yapi which channels have handlers, so that yapi can warn when its routing keeps an event from one.

## Interface components

A component implements the `Component` trait. `render` returns the lines to show at a width, as text with ANSI escape sequences. The optional `handle_input` receives raw terminal input while the component has focus, and `handle_mouse` receives mouse events in fullscreen mode and returns whether it took them. This component asks a yes or no question:

```rust
use yapi_extension_api::{Component, CustomOptions, Done, parse_key, theme};

struct Confirm {
    done: Done<bool>,
}

impl Component for Confirm {
    fn render(&mut self, _width: usize) -> Vec<String> {
        vec![theme().fg("accent", "Deploy now? (y/n)")]
    }

    fn handle_input(&mut self, data: &str) {
        if data == "y" || data == "n" {
            self.done.finish(data == "y");
        } else if parse_key(data).as_deref() == Some("escape") {
            self.done.finish(false);
        }
    }
}
```

A command shows it with `custom` and waits for the answer:

```rust
api.register_command("deploy", "Deploys after asking", |_args, ctx| async move {
    let confirmed = ctx
        .custom(|done| Confirm { done }, CustomOptions::default())
        .await;
    if confirmed == Some(true) {
        // ...
    }
    Ok(())
});
```

These `Context` methods show components:

| Method | Pi counterpart | Shows |
|---|---|---|
| `custom(build, options)` | `ctx.ui.custom` | The component `build` returns, with keyboard focus, in the editor's place or as an overlay. `CustomOptions` sets `overlay` and Pi's `overlayOptions`, such as `{"width": 70}`. The future resolves to the value the component passes to `Done::finish`, which also closes it. |
| `set_widget(key, widget, placement)` | `ctx.ui.setWidget` | `Widget::Lines` or `Widget::Component` as widget `key`, above or below the editor. `None` removes the widget. |
| `set_footer(component)`, `set_header(component)` | `ctx.ui.setFooter`, `ctx.ui.setHeader` | A component in place of the footer or the startup header. `None` restores the built-in one. |

yapi renders a component after each key or mouse event it handles, and paints the lines it rendered last in the meantime, so the interface never waits for the extension. A component that changes on its own, from a timer or a background task, calls `request_render` afterwards. yapi cuts lines wider than the width.

`theme()` returns the session's theme. `Theme::fg` and `Theme::bg` color text with a token such as `accent`, `muted` or `success`, and `bold`, `italic`, `underline`, `strikethrough` and `inverse` style it. Text stays plain without a theme, such as in print mode, and for tokens the theme lacks.

`terminal_size()` returns the terminal's columns and rows, as Pi's `tui.terminal.columns` and `tui.terminal.rows` give them, and follows resizes. It returns `None` without a terminal, as in print, JSON and remote procedure call (RPC) modes.

`editor_focused()` tells whether the prompt editor, or an editor component in its place, has the keyboard, as Pi's `tui.focusedComponent` does. It's `false` while a selector, a dialog or a component shown with `custom` has it. A terminal input listener that takes keys from an empty prompt checks it first, so dialogs still get those keys. It returns `true` without a terminal.

`parse_key` names common keys, such as `up`, `enter`, `escape` and `ctrl+c`. It reads both legacy and Kitty keyboard protocol encodings, which differ for Escape and Ctrl keys, so compare keys through it rather than with raw bytes such as `"\x1b"`.

Only the interactive mode shows components. In RPC, print and JSON modes, `custom` resolves to `None` at once, and yapi leaves out component widgets, footers and headers. When the extension loads again for another session, yapi drops every component it showed.

`CustomOptions::on_handle` receives the shown component's `OverlayHandle`, Pi's `onHandle`. `hide` closes the component without finishing it, so prefer `Done::finish`. `set_hidden`, `focus` and `unfocus` only change what `is_hidden` and `is_focused` report, as for Pi extensions in yapi, and the overlay stays on screen. An overlay without `overlay_options` takes the width its component's `Component::width` returns, as Pi does with a component's `width`.

### Replace the editor

`Context::set_editor_component` puts an `EditorComponent` in place of the built-in editor, as Pi's `setEditorComponent` does, or restores the built-in one with `None`. The editor starts with the built-in editor's text. yapi calls its `set_text`, `add_to_history`, `insert_text_at_cursor` and `configure` methods, and the editor reports back with these:

- `editor_changed` when its text changes
- `editor_submit` when the user submits
- `editor_action` for app keys such as `app.interrupt`
- `editor_shortcut` for extension shortcuts

`widgets::CustomEditor` is Pi's `CustomEditor`: the built-in editor's keys, history, completions and app keys. Wrap it to change how it handles keys or renders, as Pi's examples extend it. The [`modal-editor`](native-examples.md#modal-editor) example adds vim-like modes this way.

### Tool and message renderers

Renderers draw tool calls, custom messages and custom entries in the transcript:

- `Tool::render_call` and `Tool::render_result` draw a tool's call and result, as Pi's `renderCall` and `renderResult` do. `Tool::render_shell("self")` says they draw the tool's whole box.
- `Api::register_message_renderer` and `Api::register_entry_renderer` draw custom messages and entries of one type, as Pi's `registerMessageRenderer` and `registerEntryRenderer` do.
- A renderer returns a component, or `None` to keep yapi's own rendering. An entry without a component shows nothing.

The [`entry-renderer`](native-examples.md#entry-renderer) example draws status cards.

`Api::register_markdown_transformer` is Pi's `registerMarkdownTransformer`. It gets a message's Markdown and Pi's context, and returns new Markdown or `None`. A message shows as written until the transformer returns.

A tool renderer gets a `RenderContext`, with Pi's context fields such as `toolCallId`, `expanded` and `isPartial` in `data`, and a `state` value that the call's and the result's renderers share for one tool call. Pi's `lastComponent` has no counterpart, so a renderer builds a new component each time yapi asks. The new component takes the old one's place, and yapi shows the old one's lines until the new one has rendered.

### Code renderers

`Api::register_code_renderer` is the Rust form of [`pi.yapi.registerCodeRenderer`](extensions.md#code-renderers). It gets the code, its language and the width, and returns styled lines or `None`. The [`code-renderer`](native-examples.md#code-renderer) example colors Rust keywords.

### pi-tui widgets

The `widgets` feature adds yapi's ports of pi-tui's widgets, which match pi-tui's output. Turn it on in the SDK dependency:

```toml
[dependencies]
yapi-extension-api = { version = "0.4.2", features = ["widgets"] }
```

`widgets::tui` holds the widgets:

- `select_list::SelectList`, `editor::Editor`, `text_input::TextInput` and `settings_list::SettingsList`, plus the `Text`, `Box` and border layouts in `lines`, render styled lines.
- `widgets::to_ansi` turns those lines into a component's lines, with pi-tui's cursor marker where a cursor belongs.
- `widgets::style` and `widgets::bg_style` give a theme token's style. `widgets::keybindings` gives the key bindings that `handle_input` takes.
- The module also has `Text`, a component with pi-tui's `Text` layout, and pi-tui's `visible_width`, `truncate_to_width` and `wrap_text_with_ansi` for text with escape sequences.

A built extension without the feature is about 210 KB. The six examples that use widgets are 280 to 490 KB.

### Terminal input and completion

Terminal input listeners and autocomplete providers need the `all` feature. What they return reaches yapi's own input, where `!` runs a shell command and `/` a slash command.

`Context::on_terminal_input` adds a listener for raw terminal input, as Pi's `ctx.ui.onTerminalInput` does:

- A listener sees each key before the editor, in the order listeners were added.
- It returns `TerminalInput::Pass` to let the key through, `Consume` to stop it, or `Replace(data)` to pass on `data` instead, as Pi's `undefined`, `{consume: true}` and `{data}`.
- It stops when you drop the returned `Subscription` or call `unsubscribe`, so keep the `Subscription` as long as the listener should run.
- `editor_text()` returns the editor's text, as Pi's `ctx.ui.getEditorText` does, so a listener can take keys only while the editor is empty.

This listener takes the Down key while the editor is empty:

```rust
use std::cell::RefCell;
use yapi_extension_api::{Subscription, TerminalInput, editor_text, parse_key};

thread_local! {
    static LISTENER: RefCell<Option<Subscription>> =
        const { RefCell::new(None) };
}

api.on("session_start", |_event, ctx| async move {
    let listener = ctx.on_terminal_input(|data| {
        if parse_key(data).as_deref() == Some("down")
            && editor_text().is_empty()
        {
            // ...
            return TerminalInput::Consume;
        }
        TerminalInput::Pass
    });
    LISTENER.set(Some(listener));
    Ok(None)
});
```

`Context::add_autocomplete_provider` adds an `AutocompleteProvider` over the editor's completion, as Pi's `ctx.ui.addAutocompleteProvider` does:

- `suggestions` answers the `Suggestions` for an `EditorState`, with Pi's `AutocompleteItem` shapes as items. `apply` answers the editor after the user chooses an item and defaults to `Current::apply`.
- Each gets a `Current`, Pi's `current`, which answers for the providers added before, over the built-in completion of commands and paths.
- `trigger_characters` names characters that open completion, as `@` does.
- The cursor's column in `EditorState` counts bytes, so it slices the line directly.

This provider offers a ticket after `#` and leaves other text to the providers before it. `ctx.add_autocomplete_provider(Tickets)` adds it:

```rust
use yapi_extension_api::{
    AutocompleteProvider, Current, EditorState, Suggestions, json,
};

struct Tickets;

impl AutocompleteProvider for Tickets {
    async fn suggestions(
        &self,
        state: &EditorState,
        force: bool,
        current: &Current,
    ) -> Option<Suggestions> {
        let line = &state.lines[state.cursor_line];
        if !line[..state.cursor_col].ends_with('#') {
            return current.suggestions(state, force).await;
        }
        let item = json!({
            "value": "#42",
            "label": "#42",
            "description": "Fix the build",
        });
        Some(Suggestions { items: vec![item], prefix: "#".into() })
    }
}
```

As for Pi extensions in yapi, listeners run while yapi keeps drawing, so keys that arrive together pass through them together. yapi asks providers for suggestions in the background and applies each suggestion as it asks.

Each native extension runs in an instance of its own, and the listeners of every instance see input. Completion goes through the providers of one instance, the one that added a provider last. Only the interactive mode runs listeners and providers.

## Background work and cancellation

Handlers are `async`. While one waits for a host operation, other handlers, tool calls and background tasks run. `spawn` runs a future in the background after the handler that started it returns, like a promise a Pi extension doesn't await. A background task can report its result with the `session.sendMessage` request, with `"triggerTurn": true` to start a turn when the agent is idle.

When the user aborts a run, yapi drops the futures of the extension tools still running in it, which kills the processes they hold. Background tasks keep running until they finish or the extension stops.

A tool that calls `Context::aborted` gets the abort instead, as a Pi tool's `signal` does. The future resolves when the user aborts the run. The tool keeps running to stop its work, and the result it returns becomes the tool's result, such as the output so far.

yapi drops a tool that hasn't returned 10s after the abort. That leaves time to stop a child process with SIGTERM, wait for it and report. Race the future against the work with `futures::select!` or a small helper, such as `race` in the [countdown example](native-examples.md#countdown):

```rust
let mut process =
    Process::spawn(&json!({"command": "make", "args": ["test"]}))?;
let mut output = Vec::new();
loop {
    match race(process.next(), ctx.aborted()).await {
        Ok(Some(ProcessEvent::Stdout(bytes))) => output.extend(bytes),
        Ok(Some(_)) => {}
        Ok(None) => break,
        Err(()) => {
            process.kill("SIGTERM");
            while process.next().await.is_some() {}
            output.extend(b"\n[stopped]");
            break;
        }
    }
}
Ok(ToolResult::text(String::from_utf8_lossy(&output)))
```

## Resource limits

Each native extension runs in an instance of its own, under the limits that [Instances and limits](security.md#instances-and-limits) describes.
