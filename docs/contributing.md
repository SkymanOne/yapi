# Contributing

yapi is a Cargo workspace whose crates mirror Pi's packages. [AGENTS.md](https://github.com/SkymanOne/yapi/blob/main/AGENTS.md) describes the architecture, the design decisions and the rules for changes. [dev/status.md](https://github.com/SkymanOne/yapi/blob/main/dev/status.md) tracks progress and deferred work.

## Prerequisites

- Rust stable. `rust-toolchain.toml` selects it with Clippy and rustfmt, and rustup installs it on the first build.
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) for the license and advisory check: `cargo install --locked cargo-deny`.
- `rg`, `fd` and `python3` on `PATH` for the tests.
- A checkout of Pi `v1.0.0` (commit `a13d35a`) outside this repository. Read the matching Pi source before changing a behavior.
- Node.js 22.19 or newer, only to record goldens from Pi or compare with Pi live.
- A [WASI SDK](https://github.com/WebAssembly/wasi-sdk) and the `wasm32-wasip2` Rust target, only to rebuild the JS runtime with `cargo xtask js-runtime`.

The first build compiles about 420 crates, wasmtime among them, and takes several minutes. Later builds are incremental.

## Build and run

```sh
git clone https://github.com/SkymanOne/yapi yapi
cd yapi
cargo run -p yapi --
```

## Checks

Run these before every commit. CI runs the same checks on Linux and macOS. It skips them when a change touches only documentation, internal notes, skills or other workflows.

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

`cargo test` includes the end-to-end scenarios, which replay recorded provider streams and compare the results with goldens recorded from Pi.

## Common changes

| Change | Where |
|---|---|
| A wire API | A module in `crates/yapi-ai/src/api/`, with a cassette per case in `tests/fixtures/cassettes/` |
| A provider on an existing wire API | `cargo xtask models` regenerates the built-in catalog from pi-ai. Users add their own in `models.json`. |
| A built-in tool | `crates/yapi-core/src/tools/` |
| A slash command | `crates/yapi/src/interactive/commands.rs` |
| An end-to-end scenario | An entry in `tests/fixtures/scenarios/scenarios.json`, then `cargo xtask e2e --record-pi` to record Pi's golden |
| A theme | A JSON file in Pi's theme format, with no Rust changes |

The READMEs in [tests/fixtures/pi](https://github.com/SkymanOne/yapi/blob/main/tests/fixtures/pi/README.md) and [tests/fixtures/cassettes](https://github.com/SkymanOne/yapi/blob/main/tests/fixtures/cassettes/README.md) explain the golden files and cassettes and how to regenerate them.

## Comparing with Pi

```sh
cargo xtask e2e --differential         # run Pi and yapi side by side on every scenario
cargo xtask e2e --record-pi            # rewrite the goldens from Pi
cargo xtask bench --pi <path-to-pi>    # time and memory against Pi
```

Pi installs from `tests/fixtures/pi/generator` with `npm ci`, which needs Node.js 22.19 or newer.

## Releasing

1. Set the new version in `[workspace.package]` of both `Cargo.toml` and `guest/Cargo.toml`. The extension SDK takes the guest workspace's version, and the Release workflow fails when the two differ from each other or from the tag.
2. For a new minor or major version, raise the `version` requirements on `yapi-types` and `yapi-tui` in `Cargo.toml` and `guest/yapi-extension-api/Cargo.toml`.
3. Update the versions that the `yapi new` template, the README and these docs pin. `git grep` for the old version lists them, such as `git grep -n '0\.1\.0'`.
4. Run `cargo metadata --format-version 1 >/dev/null` in `guest` to update `guest/Cargo.lock`, then `cargo xtask js-runtime` with a WASI SDK. The lock file is an input of `yapi-js.wasm`.
5. Add the release's section to `CHANGELOG.md`.
6. Merge, then push the tag `v<version>`. The Release workflow builds and publishes the binaries and the documentation archive, then publishes `yapi-types`, `yapi-tui` and `yapi-extension-api` to crates.io with the `CARGO_REGISTRY_TOKEN` secret.

## Documentation

This site lives in `docs/`, is configured in `book.toml` and builds with [mdBook](https://rust-lang.github.io/mdBook/):

```sh
mdbook serve
```

A workflow publishes it to GitHub Pages on every push to `main`. A pull request that changes it gets a preview, linked in a comment on the pull request.
