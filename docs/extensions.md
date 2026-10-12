# Extensions

Extensions add tools, commands, flags, event handlers and interface components. yapi runs two types on one WebAssembly host:

- Pi extensions, written in TypeScript or JavaScript and usually distributed through npm, run unchanged.
- [Native extensions](native-extensions.md) are written in Rust and compiled to WebAssembly components.

Both types load from the same places, ship in the same [packages](packages.md) and use the same events and interface model.

## Choose an extension type

The two types differ in runtime, isolation and portability:

| | Pi extension | Native extension |
|---|---|---|
| Language | TypeScript or JavaScript | Rust |
| Runs in | Pi and yapi | yapi only |
| Runtime | QuickJS-NG, a JavaScript interpreter without a JIT compiler, compiled to WebAssembly | Machine code that Wasmtime compiles from the extension's WebAssembly |
| Instance | One instance shared by all Pi extensions | An instance of its own |
| Limits | 1 GiB of memory and 60 seconds per call, shared by all Pi extensions | 1 GiB of memory and 60 seconds per call, for this extension alone |
| After a crash | The shared instance restarts and every Pi extension reloads | Only this extension restarts |
| API | Pi's `ExtensionAPI` from Pi 1.0, with the exceptions in [Differences from Pi](compat.md#extensions) | yapi's Rust SDK, unstable until yapi 1.0 |
| Dependencies | npm packages in plain JavaScript | Rust crates that build for `wasm32-wasip2` |
| Build | None. yapi strips TypeScript types on load and caches the result. | `cargo build` for `wasm32-wasip2`. Packages ship the built `.wasm` file. |
| Size | Source files and their npm dependencies | One `.wasm` file, about 170 KB without the widgets feature |

Pi extensions are the better choice when:

- the extension should also run in Pi, or already exists as a Pi package
- it needs parts of Pi's API that the native SDK does not cover yet: providers, MCP servers, model classification and image generation, and session actions such as `ctx.newSession` and `ctx.fork`
- it builds on npm libraries, or benefits from editing without a build step

Native extensions are the better choice when:

- the extension does heavy computation, which compiled code runs much faster than an interpreter
- it should be isolated from other extensions. A native extension's crash does not reload the others, and its instance is the unit that [per-package grants](roadmap.md#capability-based-sandboxing) will apply to.
- memory matters. Native extensions do not load the JavaScript runtime. In the [benchmark](performance.md#results), ten small Pi extensions raise idle memory on Linux from 18.8 MB to 40.0 MB.
- the code base is in Rust, or should use Rust crates

Weigh these limits as well:

- A Pi extension that needs Node features the runtime lacks, such as native addons, `net` and `tls` servers or `worker_threads`, cannot run in yapi. [Differences from Pi](compat.md#extensions) lists them.
- A native extension needs the Rust toolchain to build, and may need to be rebuilt for a new yapi release until 1.0.
- One package can ship both types. A `yapi` key in `package.json` names the native build for yapi, and the `pi` key names the JavaScript build for Pi. See [Package manifests](packages.md#package-manifests).

## Load extensions

yapi loads extensions from:

- `~/.yapi/agent/extensions`, and `.yapi/extensions` in a trusted project
- packages and files added with `yapi install`, which accepts npm and git sources, local folders and single `.ts`, `.js` or `.wasm` files
- `-e <source>` for one run, which accepts a file, a package folder, or an npm or git source

`yapi -ne` starts without extensions, and `yapi config` turns single extensions off.

`/reload` loads every extension again from disk, so edits and rebuilt native extensions take effect. A new, resumed, forked or cloned session picks up added and removed extension files and installs packages added to settings, but keeps the extensions it already loaded until `/reload`. A session in another working folder loads every extension again.

## Write a Pi extension

Extensions use Pi's API. Save this as `~/.yapi/agent/extensions/hello.ts`:

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

yapi runs extensions written for `@earendil-works/pi-coding-agent` 1.0, including ones that import the older `@mariozechner/*` package names. It is tested against Pi, with both programs loading the same code:

| Test | Result |
|---|---|
| Pi's example extensions | 79 of 79 register the same tools, commands, flags, shortcuts and event handlers as in Pi, and fail with Pi's messages where Pi fails |
| The 500 most-downloaded Pi packages on npm | 418 of 500 work without errors |
| Extension UI | Dialogs, widgets, footers, overlays, custom components and tool renderers match Pi's screens row for row at 80×24 and 120×40 in the recorded scenarios, except for the [listed differences](compat.md#extensions) |

A package works without errors when its extensions load without errors and register the same tools, commands, flags, shortcuts and event handlers as in Pi. 43 of the 418 provide only skills, prompts or themes. Of the other 82, 47 fail in Pi with the same error, and most of the rest need native addons, parts of Pi beyond the extension API, or Node features the runtime lacks. [dev/status.md](https://github.com/SkymanOne/yapi/blob/main/dev/status.md#m5-extension-host-headless) lists each one.

## How Pi extensions run

Pi extensions run in `yapi-js`, a QuickJS-NG runtime compiled to WebAssembly, so yapi needs no Node.js.

- `@earendil-works/pi-coding-agent`, `pi-ai`, `pi-agent-core`, `pi-tui` and `typebox` resolve to copies bundled with yapi.
- yapi resolves modules as Node does and strips TypeScript types, with a cache keyed by file content.
- Shims provide Node's `fs`, `path`, `os`, `child_process`, `events`, `stream`, `util`, `crypto` hashes, `buffer`, timers, `fetch`, `dns.lookup` and `Intl`. Native addons, `net` and `tls` servers and `worker_threads` fail when used, and the rest of the extension keeps working.
- Interface components render inside the runtime. yapi paints their last frame, so a slow extension never blocks the interface.

[Differences from Pi](compat.md#extensions) lists where the behavior differs.

## Crashes

[Security model and sandboxing](security.md#sandboxing) describes the limits of each instance and what extensions may access. A crashed instance restarts and its extensions reload. yapi reports the crash as an extension error, closes the instance's dialogs and widgets, and restores the built-in editor, footer and header. Tool calls and messages it drew stay in the transcript and are drawn again by the restarted instance.

## Providers from extensions

`pi.registerProvider()` adds a provider and its models. They appear in `/model` and work with `--model` and RPC's `set_model`.

- A provider configured like a `models.json` entry streams through yapi's implementation of its `api`.
- A provider with its own `streamSimple` can implement an API yapi does not have, or hand requests to pi-ai's built-in APIs, such as `anthropicMessagesApi()`. A stream registered with pi-ai's `registerApiProvider` serves every model of its API, including ones from `models.json`. Provider hooks such as `before_provider_request` run either way.
- A provider with `oauth` adds its sign-in to `/login`. yapi stores its credentials in `auth.json` and refreshes them before they expire.
- A provider with `refreshModels` updates its model list whenever yapi refreshes model catalogs.

Pi's [custom provider documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/custom-provider.md) describes the configuration.
