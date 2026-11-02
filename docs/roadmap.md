# Roadmap

yapi implements Pi's coding agent. This page lists the work planned beyond it. The items have no dates and may change. The [issue tracker](https://github.com/SkymanOne/yapi/issues) tracks individual work.

## Capability-based sandboxing

For now, every package receives every grant: file, process, network, environment and session access. The plan is to run each package with only the access it needs:

- A package declares the grants it needs in its own manifest. You approve them, and yapi runs the package in its own instance with only those grants.
- File grants can be limited to paths and to read-only access, and network grants to hosts.
- The `pi.events` bus stays within packages of the same grants unless your settings allow more. A package's own manifest cannot widen it.
- The `pi.events` bus, which does not check grants today, will check them.

[Security model and sandboxing](security.md) describes the current model.

## Project manifests

A `yapi.toml` file at a project's root will list the packages, extensions and skills the project uses. Anyone who checks out the project then installs the same set at the same versions, as `Cargo.toml` does for a Rust project.

## Package registry

A central registry of yapi packages will let manifests name packages and versions instead of npm or git sources.

## Stable native extension API

The native extension API is unstable until yapi 1.0. The Rust SDK and host requests may change in any release before then, and extensions may need to be rebuilt. yapi 1.0 will stabilize them. Asynchronous host calls will move to WASI Preview 3 as a new version of the WIT world.

## Platforms

Linux and macOS are fully supported, and Windows is best effort. Windows-specific features, such as a PowerShell tool and native console input, will follow once the core is stable.
