# Contribute to yapi

yapi is a Cargo workspace whose crates mirror Pi's packages. This page covers the tools you need, building and checking a change, where common changes go, comparing with Pi, releasing and building these docs. [AGENTS.md](https://github.com/SkymanOne/yapi/blob/main/AGENTS.md) describes the architecture, the design decisions and the rules for changes.

## Tools you need

A build and its checks need these tools:

- Rust stable. `rust-toolchain.toml` selects it with Clippy and rustfmt, and rustup installs it on the first build.
- [cargo-deny](https://github.com/EmbarkStudios/cargo-deny) for the license and advisory check: `cargo install --locked cargo-deny`.
- `rg`, `fd` and `python3` on `PATH` for the tests.
- [prek](https://prek.j178.dev/) for the git hooks: `cargo install --locked prek` or `brew install prek`. It installs the hooks' own tools when they first run.
- A checkout of Pi `v1.0.0` (commit `a13d35a`) outside this repository. Read the matching Pi source before changing a behavior.
- Node.js 22.19 or newer, only to record goldens from Pi or compare with Pi live.
- A [WebAssembly System Interface (WASI) SDK](https://github.com/WebAssembly/wasi-sdk) and the `wasm32-wasip2` Rust target, only to rebuild the JS runtime with `cargo xtask js-runtime`.

The first build compiles about 420 crates, wasmtime among them, so it takes minutes rather than seconds. Later builds are incremental.

## Build and run from source

Clone the repository and run the `yapi` binary from it:

```sh
git clone https://github.com/SkymanOne/yapi yapi
cd yapi
cargo run -p yapi --
```

## Run the checks

Run these before every commit. CI runs the same checks on Linux and macOS. It skips them when a change touches only `docs/`, `.agents/`, `assets/`, `book.toml`, the license files, root Markdown files other than `CHANGELOG.md`, or workflows other than CI.

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

`cargo test` includes the end-to-end scenarios, which replay recorded provider streams and compare the results with goldens recorded from Pi.

`prek install` sets up the git hooks in `.pre-commit-config.yaml`. On commit they run `cargo fmt` and Clippy, check whitespace, line endings, TOML, YAML, JSON, shell scripts and Markdown, and check the commit message and the branch name. On push they check the branch name.

`prek run --all-files` runs the hooks on the whole repository. [AGENTS.md](https://github.com/SkymanOne/yapi/blob/main/AGENTS.md) gives the rules for commit messages and branch names.

## Where common changes go

Each kind of change has a home in the workspace:

| Change | Where |
|---|---|
| A wire API | A module in `crates/yapi-ai/src/api/`, with a cassette per case in `tests/fixtures/cassettes/` |
| A provider on an existing wire API | `cargo xtask models` regenerates the built-in catalog from pi-ai. Users add their own in `models.json`. |
| A built-in tool | `crates/yapi-core/src/tools/` |
| A slash command | `crates/yapi/src/interactive/commands.rs` |
| An end-to-end scenario | An entry in `tests/fixtures/scenarios/scenarios.json`, then `cargo xtask e2e --record-pi` to record Pi's golden |
| A theme | A JSON file in Pi's theme format, with no Rust changes |

The READMEs in [tests/fixtures/pi](https://github.com/SkymanOne/yapi/blob/main/tests/fixtures/pi/README.md) and [tests/fixtures/cassettes](https://github.com/SkymanOne/yapi/blob/main/tests/fixtures/cassettes/README.md) explain the golden files and cassettes and how to regenerate them.

## Compare yapi with Pi

These commands run Pi and yapi side by side on every scenario, rewrite the goldens from Pi, and time yapi against the Pi binary at `path_to_pi`:

```sh
cargo xtask e2e --differential
cargo xtask e2e --record-pi
cargo xtask bench --pi path_to_pi
```

Pi installs from `tests/fixtures/pi/generator` with `npm ci`, which needs Node.js 22.19 or newer.

## Release a version

A release takes these steps, in order:

1. Set the new version in `[workspace.package]` of both `Cargo.toml` and `guest/Cargo.toml`. The extension SDK takes the guest workspace's version, and the Release workflow fails when the two differ from each other or from the tag.
2. Raise the `version` requirements on `yapi-ai`, `yapi-agent` and `yapi-core` in `Cargo.toml` to the new version, as their API changes in any release. For a new minor or major version, raise those on `yapi-types` and `yapi-tui` in `Cargo.toml` and `guest/yapi-extension-api/Cargo.toml` too.
3. Set chest's version, which is its own, in `crates/chest/Cargo.toml`. Raise it, following semver, when `git diff --stat v<previous> -- crates/chest crates/yapi-core crates/yapi-agent crates/yapi-ai crates/yapi-types` lists changes. Otherwise keep it, and the release ships the same chest and skips publishing `chest-cli`, which crates.io already has.
4. Update the versions that the `yapi new` template, the README and these docs pin. `git grep` for the old version lists them, such as `git grep -n '0\.1\.0'`. They include the tag in `pkg-url` in `crates/chest/Cargo.toml`, which the Release workflow checks, so that each published `chest-cli` downloads the release that built it.
5. Run `cargo metadata --format-version 1 >/dev/null` in `guest` to update `guest/Cargo.lock`, then `cargo xtask js-runtime` with a WASI SDK. The lock file is an input of `yapi-js.wasm`.
6. In `CHANGELOG.md`, move the lines under `## [Unreleased]` into a section for the release, with its date and link.
7. Merge, then wait for CI on `main` to pass for the merge commit itself, which `gh run list --branch main` shows. The pull request's green run doesn't prove main's. A flaky test is a bug to fix, not a run to repeat.
8. Push the tag `v<version>`. The Release workflow builds and publishes the binaries and the documentation archive. Then it publishes `yapi-types`, `yapi-tui`, `yapi-ai`, `yapi-agent`, `yapi-core`, `chest-cli` and `yapi-extension-api` to crates.io with trusted publishing, so the repository stores no token. It skips each crate that crates.io already has at its version. Each crate has a trusted publisher on crates.io for repository `SkymanOne/yapi`, workflow `release.yml` and no environment.

### Publish a new crate for the first time

crates.io accepts a trusted publisher only for a crate that exists, so a maintainer publishes each new crate once with their own token. The first publish of `yapi-ai`, `yapi-agent`, `yapi-core` and `chest-cli` has to come with a new workspace version, because the `yapi-types` 0.3.1 on crates.io is older than the code they build on. For that release, follow these steps, with `v0.4.0` standing for its tag:

1. Release as above. The Release workflow publishes `yapi-types` and `yapi-tui`, then fails on `yapi-ai`, which has no trusted publisher yet.
2. Create a crates.io API token with the `publish-new` scope, then publish the four crates from the tag. Cargo publishes them in dependency order:

   ```sh
   git checkout v0.4.0
   cargo login
   cargo publish -p yapi-ai -p yapi-agent -p yapi-core -p chest-cli
   cargo logout
   ```

3. On crates.io, open each of the four crates' settings and add a trusted publisher for GitHub: owner `SkymanOne`, repository `yapi`, workflow `release.yml` and no environment.
4. Rerun the failed `crates` job of the Release workflow. It skips the four crates and publishes `yapi-extension-api`.
5. Revoke the token on crates.io.

## Build the documentation

This site lives in `docs/`, takes its configuration from `book.toml` and builds with [mdBook](https://rust-lang.github.io/mdBook/). This serves it locally and rebuilds on each change:

```sh
mdbook serve
```

A workflow publishes it to GitHub Pages on every push to `main`. A pull request that changes it gets a preview, linked in a comment on the pull request.
