# Extend yapi with extensions

Extensions add tools, commands, flags, event handlers and interface components. This page compares the two extension types, then covers loading extensions, writing a Pi extension, compatibility with Pi, yapi's own APIs, how Pi extensions run, crashes and providers. yapi runs both types on one WebAssembly host:

- **Pi extensions**: written in TypeScript or JavaScript, they run unchanged.
- **[Native extensions](native-extensions.md)**: written in Rust and compiled to WebAssembly components.

Both types load from the same places, ship in the same [packages](packages.md) and use the same events and interface model.

## Choose an extension type

The two types differ in runtime, isolation and portability:

| | Pi extension | Native extension |
|---|---|---|
| Language | TypeScript or JavaScript | Rust |
| Runs in | Pi and yapi | yapi only |
| Runtime | QuickJS-NG, a JavaScript interpreter without a just-in-time (JIT) compiler, compiled to WebAssembly | Machine code that Wasmtime compiles from the extension's WebAssembly |
| Instance | One instance shared by the Pi extensions with the same grants and event policy | An instance of its own |
| Memory | Shared by the Pi extensions in its instance | Its own |
| Compute | While one Pi extension computes, the others in its instance wait | Doesn't block other extensions |
| After a crash | Its instance restarts and the Pi extensions in it reload | Only this extension restarts |
| API | Pi's `ExtensionAPI` from Pi 1.0, with the exceptions in [Differences from Pi](compat.md#extension-runtime-and-api) | yapi's Rust SDK, unstable until yapi 1.0 |
| Dependencies | npm packages in plain JavaScript | Rust crates that build for `wasm32-wasip2` |
| Build | None. yapi strips TypeScript types on load and caches the result. | `cargo build` for `wasm32-wasip2`. Packages ship the built `.wasm` file. |
| Size | Source files and their npm dependencies | One `.wasm` file, about 210 KB without the widgets feature |

Both types run under the same [compute and memory limits](security.md#instances-and-limits).

Pi extensions are the better choice when:

- the extension should also run in Pi, or already exists as a Pi package
- it needs parts of Pi's API that the native SDK doesn't cover yet: providers, Model Context Protocol (MCP) servers, model classification and image generation, and session actions such as `ctx.newSession` and `ctx.fork`
- it builds on npm libraries, or benefits from editing without a build step

Native extensions are the better choice when:

- the extension does heavy computation, which compiled code runs faster than an interpreter
- it should be isolated from other extensions, since a native extension's crash doesn't reload the others
- memory matters. Native extensions don't load the JavaScript runtime, and the [benchmark](performance.md#benchmark-results) measures the memory that Pi extensions add.
- the code base is in Rust, or should use Rust crates

Weigh these limits as well:

- A Pi extension that needs Node features the runtime lacks, such as native addons, `net` and `tls` servers or `worker_threads`, can't run in yapi. [Differences from Pi](compat.md#extension-runtime-and-api) lists them.
- A native extension needs the Rust toolchain to build, and you may need to rebuild it for a new yapi release until 1.0.

One package can ship both types. A `yapi` key in `package.json` names the native build for yapi, and the `pi` key names the JavaScript build for Pi. See [Package manifests](packages.md#package-manifests).

## Where yapi loads extensions from

yapi loads extensions from three places:

- `~/.yapi/agent/extensions`, and `.yapi/extensions` in a trusted project
- packages and files added with `yapi install`, which accepts npm and git sources, local folders and single `.ts`, `.js` or `.wasm` files
- `-e <source>` for one run, which accepts a file, a package folder, or an npm or git source

`yapi -ne` starts without extensions, and `yapi config` turns single extensions off. A [chest](chest-extensions.md) manifest adds a project's extensions too, each with the capabilities its entry grants.

`/reload` loads every extension again from disk, so edits and rebuilt native extensions take effect. A new, resumed, forked or cloned session picks up added and removed extension files and installs packages added to settings, but keeps the extensions it already loaded until `/reload`. A session in another working folder loads every extension again.

### Request capabilities

A package can say which capabilities it needs, from `fs`, `net`, `env`, `session` and `all`, and which `pi.events` channels it sends and hears. List them as `capabilities` and `events` in the `[package]` table of a `chest.toml` in the package, or in the `yapi` key of `package.json`:

```json
{
  "yapi": {
    "capabilities": ["fs"],
    "events": { "send": ["todo:*"], "receive": [] }
  }
}
```

A request grants nothing. chest shows it when a project adds the package, and the project's manifest decides what the package gets. A package from `yapi install` gets every grant whatever it requests.

Test a package under the grants it requests, so yapi doesn't refuse a call it needs once others install it. [What each capability allows](chest-manifest.md#what-each-capability-allows) lists what each capability covers.

## Write a Pi extension

Extensions use Pi's API. Save this as `~/.yapi/agent/extensions/hello.ts`. It registers a `/hello` command that greets the name you pass:

```typescript
import type { ExtensionAPI } from "@earendil-works/pi-coding-agent";

export default function (pi: ExtensionAPI) {
  pi.registerCommand("hello", {
    description: "Show a greeting",
    handler: async (name, ctx) => {
      ctx.ui.notify(`Hello, ${name || "world"}!`, "info");
    },
  });
}
```

Start yapi and run `/hello Ada`. Pi's [extension documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/extensions.md) covers tools, events, UI components and the rest of the API.

## Compatibility with Pi

yapi runs extensions written for `@earendil-works/pi-coding-agent` 1.0, including ones that import the older `@mariozechner/*` package names. The tests run yapi and Pi on the same code and compare them:

| Test | Result |
|---|---|
| Pi's example extensions | 79 of 79 register the same tools, commands, flags, shortcuts and event handlers as in Pi, and fail with Pi's messages where Pi fails |
| The 500 most-downloaded Pi packages on npm | 418 of 500 work without errors |
| Extension UI | Dialogs, widgets, footers, overlays, custom components and tool renderers match Pi's screens row for row at 80×24 and 120×40 in the recorded scenarios, except for the [listed differences](compat.md#extension-runtime-and-api) |

A package works without errors when its extensions load without errors and register the same tools, commands, flags, shortcuts and event handlers as in Pi. 43 of the 418 provide only skills, prompts or themes. Of the other 82, 47 fail in Pi with the same error. The remaining 35 include packages that need native addons, parts of Pi beyond the extension API, or Node features the runtime lacks.

## yapi-only APIs

`pi.yapi` holds APIs that Pi doesn't have. Call them as `pi.yapi?.method()` so the package still loads in Pi.

### Code renderers

yapi shows code in one color. `pi.yapi.registerCodeRenderer(render)` lets an extension color it, or draw it differently, in Markdown code blocks, `read` and `write` tool rows and codemode scripts. This renderer colors three SQL keywords:

```typescript
pi.yapi?.registerCodeRenderer((code, lang, { width }, theme) => {
  if (lang !== "sql") return undefined;
  const keyword = /\b(SELECT|FROM|WHERE)\b/g;
  return code
    .split("\n")
    .map((line) =>
      line.replace(keyword, (word) => theme.fg("syntaxKeyword", word)),
    );
});
```

A renderer follows these rules:

- `lang` is the code block's language or the file's language, and may be `undefined`.
- Return the styled lines, or `undefined` to keep the plain code.
- When more than one extension registers a renderer, yapi uses the first one that returns lines.
- Code shows plain until the renderer returns.

## How Pi extensions run

Pi extensions run in `yapi-js`, a QuickJS-NG runtime compiled to WebAssembly, so yapi needs no Node.js. These details shape how they run:

- `@earendil-works/pi-coding-agent`, `pi-ai`, `pi-agent-core`, `pi-tui` and `typebox` resolve to copies bundled with yapi. `pi-ai` is Pi's own code, with its wire APIs and image generation on yapi's providers.
- `pi-agent-core`, `pi-tui` and the parts of `pi-coding-agent` that run inside an extension, such as session entry helpers, compaction, skills and the interface components, are Pi's own code, loaded the first time an extension uses them. They use yapi's folders.
- Exports that would reach provider credentials, project trust, settings or session files without the host throw, as [Differences from Pi](compat.md#extension-runtime-and-api) lists.
- Built-in tool factories return yapi's tools, which yapi runs, or Pi's own over custom `operations`, as Pi's `ssh` example uses them.
- yapi resolves modules as Node does and strips TypeScript types, with a cache keyed by file content.
- Shims provide Node's `fs`, `path`, `os`, `child_process`, `events`, `stream`, `util`, `crypto` hashes, `buffer`, timers, `fetch`, `dns.lookup` and `Intl`. Native addons, `net` and `tls` servers and `worker_threads` fail when used, and the rest of the extension keeps working.
- Dates use the local time zone, from `TZ` or the system, as in Node.
- Interface components render inside the runtime. yapi paints their last frame, so a slow extension never blocks the interface.
- `pi.events` reaches extensions in other runtimes according to their grants and event policies. By default an extension sends and hears every channel, as in Pi, and a chest entry can narrow that, as [Event bus routing](chest-manifest.md#event-bus-routing) describes.

[Differences from Pi](compat.md#extension-runtime-and-api) lists where the behavior differs.

## What happens when an extension crashes

A crashed instance restarts and its extensions reload. yapi reports the crash as an extension error, closes the instance's dialogs and widgets, and restores the built-in editor, footer and header. Tool calls and messages it drew stay in the transcript, and the restarted instance draws them again. [How the extension sandbox works](security.md#how-the-extension-sandbox-works) describes the limits of each instance and what extensions may access.

## Providers from extensions

`pi.registerProvider()` adds a provider and its models. They appear in `/model` and work with `--model` and the `set_model` command of remote procedure call (RPC) mode. An extension can register a provider at any time, such as from a command or an event handler, and the change applies at once.

A configuration registered again for the same provider keeps the previous configuration's values for the keys it leaves undefined. `pi.unregisterProvider()` removes a provider whichever extension registered it, and restores the built-in or `models.json` provider it replaced. What a provider does depends on what it defines:

- A provider configured like a `models.json` entry streams through yapi's implementation of its `api`.
- A provider with its own `streamSimple` can implement an API yapi doesn't have, or pass requests to pi-ai's built-in APIs, such as `anthropicMessagesApi()`. A stream registered with pi-ai's `registerApiProvider` serves every model of its API, including ones from `models.json`. Provider hooks such as `before_provider_request` run either way.
- A provider with `oauth` adds its sign-in to `/login`. yapi stores its credentials in `auth.json` and refreshes them before they expire.
- A provider with `refreshModels` updates its model list whenever yapi refreshes model catalogs.
- A provider with the legacy `oauth.modifyModels` adjusts its models for the signed-in account whenever yapi refreshes model catalogs.
- Image and classifier models run through the provider's `images` and `classifiers` implementations for their `api`. Codemode scripts and extensions use them with `generateImages` and `classify`.
- A complete pi-ai `Provider`, such as one `createProvider` builds, brings its own models, API key and OAuth sign-in, streams, image and classifier implementations and model refresh. `/login` asks for its API key with the provider's own `login`, or says the key is set up outside yapi when it has none.
- A pi-ai `Provider`'s API key `check`, `filterModels` and `filterAllModels` decide which of its models are available for the stored credential, or for the key `--api-key` gives. They run as the extension loads, whenever yapi refreshes model catalogs, after you sign in or out, and after an extension registers or unregisters a provider. See [Differences from Pi](compat.md#extension-runtime-and-api).

Pi's [custom provider documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/custom-provider.md) describes the configuration.
