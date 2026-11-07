# Install yapi

Releases ship two executables: `yapi` and [chest](chest.md), the package manager for skills, subagents and extensions. chest also works with other agents and without yapi, so every method on this page installs both or either one alone. This page covers the install script, cargo-binstall, release archives and building from source, then the documentation yapi reads, upgrades and uninstalling.

Releases include builds for Linux and macOS on x86_64 and arm64, and other platforms can build yapi from source. yapi fully supports Linux and macOS. Windows support is best effort.

## Run the install script

The install script installs both executables, or one of them, with one command:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh \
  | sh
```

The script downloads the latest release for your system and verifies its SHA-256 checksum. In a terminal, it asks what to install:

```text
┌   yapi installer
│
◆  What to install?
│  ● yapi and chest (the coding agent and its package manager)
│  ○ yapi only
│  ○ chest only
└
```

Move with ↑ and ↓ and press Enter. Enter alone installs both, and Ctrl+C cancels. When the terminal can't read single keys, such as with `TERM=dumb`, the script asks for 1, 2 or 3 instead. Set `NO_COLOR` to turn off colors.

The script puts the executables in `~/.local/bin`, and with `yapi` it also installs the [documentation for the model](#documentation-for-the-model). Without a terminal, such as in CI, it installs both without asking.

| Option | Variable | Effect |
|---|---|---|
| `--only <name>` | `YAPI_INSTALL_ONLY` | Install `yapi`, `chest` or `both` without asking |
| `--version <tag>` | `YAPI_VERSION` | Install that release, such as `v0.4.2` |
| `--to <dir>` | `YAPI_INSTALL_DIR` | Install into `<dir>` |
| `--from-source` | `YAPI_FROM_SOURCE=1` | Build the release with Cargo instead of downloading it, as [Build from source](#build-from-source) shows |
| `--no-docs` | `YAPI_NO_DOCS=1` | Skip the documentation for the model |

Pass options after `sh -s --`. This installs `chest` alone, such as for another agent:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh \
  | sh -s -- --only chest
```

This installs both from `v0.4.2` into `/usr/local/bin`, without the menu:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh \
  | sh -s -- --only both --version v0.4.2 --to /usr/local/bin
```

## Install with cargo-binstall

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) installs the release executables into `~/.cargo/bin`. The first command installs `yapi`, and the second installs `chest` from its crate on crates.io, `chest-cli`. Run one or both:

```sh
cargo binstall --git https://github.com/SkymanOne/yapi yapi
cargo binstall chest-cli
```

Keep `--git` for yapi, because the `yapi` crate on crates.io is an unrelated project. cargo-binstall installs the yapi release that the manifest on `main` names. If it reports that this release doesn't exist, use the install script.

## Install from a release archive

Each [release](https://github.com/SkymanOne/yapi/releases) has a `yapi-<target>.tar.gz` archive per platform, with a `.sha256` checksum. The archive holds `yapi`, `chest` and their license files.

| Target | Systems |
|---|---|
| `x86_64-unknown-linux-gnu` | Linux on x86_64 with glibc 2.35 or newer, such as Ubuntu 22.04, Debian 12 and Fedora 36 |
| `aarch64-unknown-linux-gnu` | Linux on arm64 with glibc 2.35 or newer |
| `x86_64-apple-darwin` | macOS on Intel |
| `aarch64-apple-darwin` | macOS on Apple silicon |

Set `target` to your system's target from the table. These commands download its archive, verify it and unpack `yapi` and `chest` into `~/.local/bin`, or any folder on `PATH`:

```sh
target=x86_64-unknown-linux-gnu
url=https://github.com/SkymanOne/yapi/releases/latest/download
curl -fsSLO "$url/yapi-$target.tar.gz"
curl -fsSLO "$url/yapi-$target.tar.gz.sha256"
sha256sum -c "yapi-$target.tar.gz.sha256"
tar xzf "yapi-$target.tar.gz" -C ~/.local/bin yapi chest
```

To adapt the commands:

- To install one executable, end the `tar` command with only `yapi` or only `chest`.
- On macOS, verify with `shasum -a 256 -c`.
- For a specific release, replace `latest/download` with `download/<tag>`.
- With an older glibc, or with musl such as Alpine, [build from source](#build-from-source) instead.

If macOS refuses to open a binary downloaded in a browser, see [macOS blocks a downloaded binary](troubleshooting.md#macos-blocks-a-downloaded-binary).

## Build from source

yapi builds with Rust 1.99 or newer, and chest with Rust 1.91 or newer. Install Rust with [rustup](https://rustup.rs), then build with the install script or with Cargo.

With `--from-source`, the install script builds the latest release's tag with Cargo, or the tag `--version` names, and never the `main` branch. Then it installs the executables in `~/.local/bin`, and the documentation for the model with `yapi`. The first command builds both, the second `yapi` alone and the third `chest` alone. Run one:

```sh
script=https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh
curl -fsSL "$script" | sh -s -- --from-source --only both
curl -fsSL "$script" | sh -s -- --from-source --only yapi
curl -fsSL "$script" | sh -s -- --from-source --only chest
```

Cargo builds chest from its crate on crates.io:

```sh
cargo install --locked chest-cli
```

To build from the repository instead, name a release's tag. The first command below builds `yapi`, and the second builds `chest` from its crate, `chest-cli`. Run one or both:

```sh
repo=https://github.com/SkymanOne/yapi
cargo install --locked --git "$repo" --tag v0.4.2 yapi
cargo install --locked --git "$repo" --tag v0.4.2 chest-cli
```

Cargo installs the executables in `~/.cargo/bin`. Leave out `--tag` to build the latest commit on `main`.

To build from a clone, run `cargo install --locked --path crates/yapi` in it, or `--path crates/chest` for chest. If Cargo reports that rustc is too old, run `rustup update`.

## Check the installation

Each executable prints its version. yapi's version matches the release tag, such as `0.4.2` for `v0.4.2`. chest has a version number of its own. Run the line for each one you installed:

```sh
yapi --version
chest --version
```

## Documentation for the model

yapi reads its own documentation to answer questions about its features. The documentation holds these pages and Pi's docs and examples, which describe the extension API. Each release publishes it as `yapi-docs.tar.gz`, which goes to `~/.yapi/agent/docs`, or `docs` in `YAPI_CODING_AGENT_DIR` when that's set.

The install script installs the documentation with `yapi`, and skips it when it installs `chest` alone. With any other method, yapi downloads it in the background on first start. Either way, yapi verifies the archive against its checksum, and the archive replaces the previous copy. `YAPI_NO_DOCS=1`, `--offline` and `PI_OFFLINE=1` turn the download off.

Until a copy for its own version is in place, yapi points the model to the online documentation. Pi's pages use Pi's names, such as `~/.pi/agent`, and [Names that change](migrating.md#names-that-change) lists yapi's.

To install the documentation by hand, such as on a machine that runs offline:

```sh
url=https://github.com/SkymanOne/yapi/releases/latest/download
mkdir -p ~/.yapi/agent/docs
curl -fsSL "$url/yapi-docs.tar.gz" | tar xzf - -C ~/.yapi/agent/docs
```

For an older yapi, replace `latest/download` with `download/v<version>`, where `<version>` is what `yapi --version` prints. yapi replaces these files on upgrade, so keep your own notes elsewhere.

## Upgrade yapi

yapi doesn't update itself. `yapi update` prints how to upgrade, `yapi update --extensions` updates packages and `yapi update --models` updates model catalogs. Upgrade yapi and chest the same way you installed them:

| Installed with | Upgrade |
|---|---|
| Install script | Run the script again and make the same choice, with `--from-source` if you built from source. From another script, pass the same `--only`, because without a terminal it installs both. |
| cargo-binstall | Run the same `cargo binstall` commands again. |
| Release archive | Unpack the executables you installed from the new archive over the old ones. |
| Cargo | Run `cargo install --locked chest-cli` again for chest from crates.io. Run the `--git` commands again with the new release's tag. |

Upgrades keep your settings, credentials, sessions and packages.

In the terminal, yapi checks at startup for a newer release and for package updates, and shows a notice when it finds one. `PI_SKIP_VERSION_CHECK=1` turns off the release check. `--offline` and `PI_OFFLINE=1` turn off both.

`YAPI_RELEASES_URL` points the install script, the release check and the documentation download at another releases page, such as a mirror. It defaults to `https://github.com/SkymanOne/yapi/releases`.

## Uninstall yapi

How you remove the binaries depends on how you installed them:

- **Install script or release archive**: delete `~/.local/bin/yapi`, `~/.local/bin/chest` or both, or the same files in the folder you chose. This also applies to builds with `--from-source`.
- **cargo-binstall or Cargo**: run `cargo uninstall yapi` to remove yapi, and `cargo uninstall chest-cli` to remove chest, from crates.io or the repository.

To remove settings, credentials, sessions, packages and caches as well, delete `~/.yapi`, or `YAPI_CODING_AGENT_DIR` when set, and each project's `.yapi` folder. chest keeps its global manifest, store and records in `~/.chest`.
