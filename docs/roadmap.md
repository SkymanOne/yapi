# Planned work

yapi implements Pi's coding agent. This page lists the work planned beyond it: narrower capabilities, a package registry, a stable native extension API and Windows-specific features. The items have no dates and may change. The [issue tracker](https://github.com/SkymanOne/yapi/issues) tracks individual work.

## Capability-based sandboxing

Extensions installed with [chest](chest.md) run with the capabilities their manifest grants, and packages from `yapi install` keep Pi's grants. The capabilities are coarse for now, so the plan is to narrow them:

- File grants limited to paths and to read-only access, such as `fs:read` or a list of folders.
- Network grants limited to hosts.
- Process grants limited to programs, such as `exec:git`, so an extension can run one tool without `all`.
- Extensions with identical grants always share `pi.events`. Once yapi scopes file and network grants, they'll share only when their paths and hosts match too.

[Security model and sandboxing](security.md) describes the current model.

## A package registry

A central registry of yapi packages will let `chest.toml` name packages and versions instead of npm or git sources.

## Stable native extension API

The native extension API is unstable until yapi 1.0. The Rust SDK and host requests may change in any release before then, and you may need to rebuild extensions. yapi 1.0 will stabilize them. Asynchronous host calls will move to WebAssembly System Interface (WASI) Preview 3 as a new version of the Wasm Interface Type (WIT) world.

## Platform support

yapi fully supports Linux and macOS, and Windows support is best effort. Windows-specific features, such as a PowerShell tool and native console input, will follow once the core is stable.
