# Roadmap

yapi implements Pi's coding agent. This page lists the work planned beyond it. The items have no dates and may change. The [issue tracker](https://github.com/SkymanOne/yapi/issues) tracks individual work.

## Capability-based sandboxing

Extensions installed with [chest](chest.md) run with the capabilities their manifest grants, and packages from `yapi install` keep Pi's grants. The capabilities are coarse for now, so the plan is to narrow them:

- File grants limited to paths and to read-only access, such as `fs:read` or a list of folders.
- Network grants limited to hosts.
- Process grants limited to programs, such as `exec:git`, so an extension can run one tool without `all`.
- Extensions with identical grants always share `pi.events`. Once file and network grants are scoped, they will share only when their paths and hosts match too.

[Security model and sandboxing](security.md) describes the current model.

## Package registry

A central registry of yapi packages will let `chest.toml` name packages and versions instead of npm or git sources.

## Stable native extension API

The native extension API is unstable until yapi 1.0. The Rust SDK and host requests may change in any release before then, and extensions may need to be rebuilt. yapi 1.0 will stabilize them. Asynchronous host calls will move to WASI Preview 3 as a new version of the WIT world.

## Platforms

Linux and macOS are fully supported, and Windows is best effort. Windows-specific features, such as a PowerShell tool and native console input, will follow once the core is stable.
