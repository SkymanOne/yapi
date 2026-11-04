# {{project-name}}

A native extension for [yapi](https://github.com/SkymanOne/yapi), written in Rust with the `yapi-extension-api` crate. `src/lib.rs` starts as yapi's `hello` example: a `shout` tool, a `/hello` command, a `--shout-suffix` flag and two event handlers. Replace them with your own.

The project is a yapi package. `package.json` names the built extension, `extensions/{{crate_name}}.wasm`, and `chest.toml` lists the capabilities it asks for. This README covers building, declaring capabilities, trying, installing and sharing the extension.

Native WebAssembly extensions are unstable until yapi 1.0. The Wasm Interface Type (WIT) world, the Rust SDK and the host requests they use may change in any release before then, and you may need to rebuild extensions. Pi extensions from npm use Pi's extension API and aren't affected.

## Build the extension

Add the `wasm32-wasip2` target once, then build and copy the component into `extensions/`:

```sh
rustup target add wasm32-wasip2
cargo build --release
cp target/wasm32-wasip2/release/{{crate_name}}.wasm extensions/
```

`.cargo/config.toml` makes `wasm32-wasip2` the default target, so `cargo build` builds the WebAssembly component.

## Declare capabilities

The extension starts with the `session` capability, which it needs to add a session entry when a session starts. Each feature of `yapi-extension-api` in `Cargo.toml` declares one capability: `fs`, `net`, `env`, `session`, or `all` for everything, running processes included. A request that needs a capability the build leaves out fails with a message that names the feature.

Use the version the template pins or a newer one. Older releases may lack the capability features, and cargo then reports that the feature doesn't exist.

List the same capabilities in `chest.toml`. After each build, [chest](https://skymanone.github.io/yapi/chest-publish.html#check-a-native-extension) checks that they match:

```sh
cargo build --release
cp target/wasm32-wasip2/release/{{crate_name}}.wasm extensions/
chest check
```

`chest check` fails when the extension uses a capability that `chest.toml` doesn't list, and warns about one listed that it doesn't use.

## Try the extension

Load the package for one run without installing it:

```sh
yapi -e .
```

## Install the extension

Install it for every project, or with `-l` for the current one:

```sh
yapi install .
yapi install . -l
```

yapi loads the `.wasm` file from this folder, so a rebuild takes effect after `/reload` or the next time yapi starts.

## Share the extension

Commit `extensions/{{crate_name}}.wasm` along with the sources, because yapi installs prebuilt files and never compiles code. Then push the repository, and others install it by its URL:

```sh
yapi install git:github.com/your_github_user/{{project-name}}
```

To publish on npm, run `npm publish`, and others install it with `yapi install npm:{{project-name}}`.

## Read the API documentation

The [native extension guide](https://skymanone.github.io/yapi/native-extensions.html) covers the API, and the SDK's reference is on [docs.rs](https://docs.rs/yapi-extension-api). To build the reference locally, such as to read it offline:

```sh
cargo doc --open -p yapi-extension-api
```
