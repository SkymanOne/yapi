# Install yapi

yapi is one executable. Releases include builds for Linux and macOS on x86_64 and arm64, and other platforms can build it from source. Linux and macOS are fully supported. Windows support is best effort.

## Install script

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh
```

The script downloads the latest release for your system, verifies its SHA-256 checksum and installs it as `~/.local/bin/yapi`. It also installs the [documentation for the model](#documentation-for-the-model).

| Option | Variable | Effect |
|---|---|---|
| `--version <tag>` | `YAPI_VERSION` | Install that release, such as `v0.2.0` |
| `--to <dir>` | `YAPI_INSTALL_DIR` | Install into `<dir>` |
| `--no-docs` | `YAPI_NO_DOCS=1` | Skip the documentation for the model |

Pass options after `sh -s --`:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh -s -- --version v0.2.0 --to /usr/local/bin
```

## cargo-binstall

[cargo-binstall](https://github.com/cargo-bins/cargo-binstall) installs the same release binary into `~/.cargo/bin`:

```sh
cargo binstall --git https://github.com/SkymanOne/yapi yapi
```

Keep `--git`, because the `yapi` crate on crates.io is an unrelated project. cargo-binstall installs the release that the manifest on `main` names. If it reports that the release does not exist, use the install script.

## Release archives

Each [release](https://github.com/SkymanOne/yapi/releases) has a `yapi-<target>.tar.gz` archive per platform, with a `.sha256` checksum. The archive holds `yapi` and its license files.

| Target | Systems |
|---|---|
| `x86_64-unknown-linux-gnu` | Linux on x86_64 with glibc 2.35 or newer, such as Ubuntu 22.04, Debian 12 and Fedora 36 |
| `aarch64-unknown-linux-gnu` | Linux on arm64 with glibc 2.35 or newer |
| `x86_64-apple-darwin` | macOS on Intel |
| `aarch64-apple-darwin` | macOS on Apple silicon |

Download an archive, verify it and unpack `yapi` into a directory on `PATH`:

```sh
curl -fsSLO https://github.com/SkymanOne/yapi/releases/latest/download/yapi-x86_64-unknown-linux-gnu.tar.gz
curl -fsSLO https://github.com/SkymanOne/yapi/releases/latest/download/yapi-x86_64-unknown-linux-gnu.tar.gz.sha256
sha256sum -c yapi-x86_64-unknown-linux-gnu.tar.gz.sha256
tar xzf yapi-x86_64-unknown-linux-gnu.tar.gz -C ~/.local/bin yapi
```

On macOS, verify with `shasum -a 256 -c`. For a specific release, replace `latest/download` with `download/<tag>`. Systems with an older glibc, or with musl such as Alpine, need to build from source. If macOS refuses to open a binary downloaded in a browser, see [Troubleshooting](troubleshooting.md#macos-blocks-a-downloaded-binary).

## Build from source

yapi builds with Rust 1.99 or newer. Install Rust with [rustup](https://rustup.rs), then run:

```sh
cargo install --locked --git https://github.com/SkymanOne/yapi --tag v0.2.0 yapi
```

Cargo installs `yapi` in `~/.cargo/bin`. Leave out `--tag` to build the latest commit on `main`. To build from a clone, run `cargo install --locked --path crates/yapi` in it. If Cargo reports that rustc is too old, run `rustup update`.

## Check the installation

```sh
yapi --version
```

## Documentation for the model

yapi reads its own documentation to answer questions about its features. The documentation holds these pages and Pi's docs and examples, which describe the extension API. Each release publishes it as `yapi-docs.tar.gz`, which goes to `~/.yapi/agent/docs`, or `docs` in `YAPI_CODING_AGENT_DIR` when that is set.

The install script installs the documentation with the binary. With any other method, yapi downloads it in the background on first start. Either way, the archive is verified against its checksum and replaces the previous copy. `YAPI_NO_DOCS=1`, `--offline` and `PI_OFFLINE=1` turn the download off. Until a copy for its own version is in place, yapi points the model to the online documentation. Pi's pages use Pi's names, such as `~/.pi/agent`, and [Coming from Pi](migrating.md#names-that-change) lists yapi's.

To install it by hand, such as on a machine that runs offline:

```sh
mkdir -p ~/.yapi/agent/docs && curl -fsSL https://github.com/SkymanOne/yapi/releases/latest/download/yapi-docs.tar.gz | tar xzf - -C ~/.yapi/agent/docs
```

For an older yapi, replace `latest/download` with `download/v<version>`, where `<version>` is what `yapi --version` prints. yapi replaces these files on upgrade, so keep your own notes elsewhere.

## Upgrade

yapi does not update itself. `yapi update` prints how to upgrade, `yapi update --extensions` updates packages and `yapi update --models` updates model catalogs. Upgrade yapi the same way you installed it:

| Installed with | Upgrade |
|---|---|
| Install script | Run the script again. |
| cargo-binstall | Run the same `cargo binstall` command again. |
| Release archive | Unpack the new archive over the old binary. |
| Cargo | Run `cargo install` again with the new release's tag. |

Settings, credentials, sessions and packages are kept.

In the terminal, yapi checks at startup for a newer release and for package updates, and shows a notice when it finds one. `PI_SKIP_VERSION_CHECK=1` turns off the release check. `--offline` and `PI_OFFLINE=1` turn off both.

`YAPI_RELEASES_URL` points the install script, the release check and the documentation download at another releases page, such as a mirror. It defaults to `https://github.com/SkymanOne/yapi/releases`.

## Uninstall

- Install script or release archive: delete `~/.local/bin/yapi`, or the file in the directory you chose.
- cargo-binstall or Cargo: run `cargo uninstall yapi`.

To remove settings, credentials, sessions, packages and caches as well, delete `~/.yapi`, or `YAPI_CODING_AGENT_DIR` when set, and each project's `.yapi` folder.
