# Publish a package

> **Any agent**, except the sections marked yapi only.

A package that others add with chest describes itself in its own `chest.toml`. This page covers the `[package]` table, requesting capabilities and events, and checking a native extension before you publish it.

## Describe the package

> **Any agent**, except `capabilities`, `events`, and the `extensions`, `prompts` and `themes` lists, which only yapi uses.

A package describes itself in the `[package]` table of its own `chest.toml`. `chest init --package` adds one. It names the package after its folder, at version `0.1.0`, requests no capabilities, and lists the conventional folders it finds. A finished table looks like this:

```toml
[package]
name = "acme-review"
version = "1.3.0"
description = "Code review skills and a reviewer subagent"
capabilities = ["fs", "session"]
events = { send = ["review:*"], receive = ["subagent:done"] }
skills = ["skills/*"]
agents = ["agents/*.md"]
extensions = ["extensions/index.ts"]
```

`chest add` and `chest list` show the name, version and description. `skills` and `agents` list what the package provides to any agent. Publish the package to npm or push it to a git repository, and others add it with `chest add`.

The [`[package]` reference](chest-manifest.md#package) lists every key, and [Package metadata](chest-manifest.md#package-metadata) explains how chest falls back to `package.json`.

chest doesn't install the entries in the package's own `[skills]`, `[agents]` or `[extensions]` sections for its consumers. `chest add` lists them so the consumer can add the ones they want.

## Request capabilities and events

> **yapi only.**

`capabilities` and `events` are requests. `chest add` shows them to the consumer, who decides what to grant. A package can also request them in `package.json` under `yapi.capabilities` and `yapi.events`.

## Check a native extension

> **yapi only.**

Before you publish a native extension, run `chest check` in the package folder. It compares the capabilities each `.wasm` extension uses with the ones the package requests:

```sh
chest check
```

It reads the [capability features](native-extensions.md#declare-capabilities) the extension was built with, and the WebAssembly System Interface (WASI) interfaces it imports, where `wasi:filesystem` means `fs`. These are errors, and make chest exit with 1:

- a capability the extension uses that the package doesn't request
- an import of `wasi:sockets` or `wasi:http`, which yapi doesn't support, because extensions reach the network through the SDK's `fetch`

`chest add` warns about such an import. These are warnings:

- a capability the package requests that no extension uses
- a capability the build records that this version of chest doesn't know, such as one a newer SDK adds, which chest doesn't check

An extension built without yapi-extension-api has no record of its features, so chest checks only its imports. JavaScript and TypeScript extensions have nothing to read before they run, so yapi checks them only at runtime.
