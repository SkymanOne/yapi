# Use packages

A package bundles extensions, skills, prompt templates and themes. yapi installs Pi packages from npm, git or a local folder, with no Node.js required. This page covers installing, trying and turning off packages, pinning a project's packages with chest, package manifests and yapi's built-in npm client.

## Install and manage packages

`yapi install` takes an npm or git source, a local folder or a single native extension, and `-l` installs it for the current project only. `yapi list`, `yapi update --extensions` and `yapi remove` list, update and remove what's installed:

```sh
yapi install npm:@your_scope/your_package
yapi install git:github.com/your_user/your_repo
yapi install ./your_package -l
yapi install ./shout.wasm
yapi list
yapi update --extensions
yapi remove npm:@your_scope/your_package
```

yapi records installed packages in `settings.json`. `-l` installs into the project's `.yapi` folder and records the package in `.yapi/settings.json`, which loads only in a [trusted project](security.md#project-trust). Local folders and files load in place, without a copy.

At startup and on `/reload`, yapi installs configured packages that are missing. It also reinstalls npm packages whose installed version is outside their configured range or whose install was interrupted.

Packages can run code and can hold skills that instruct the model to run programs. Review a third-party package before you install it.

`yapi list` shows each package with its install location. A tag after the source shows its extension kinds: `[npm]` for JavaScript or TypeScript, `[wasm]` for native, or `[npm, wasm]` for both. Packages without extensions, and packages not installed yet, have no tag.

```text
User packages:
  npm:pi-mcp-adapter [npm]
    /home/you/.yapi/agent/npm/node_modules/pi-mcp-adapter
  /home/you/extensions/shout.wasm [wasm]
    /home/you/extensions/shout.wasm
```

`yapi update --extensions` moves each npm package to the newest version its source's range allows, unless the source names an exact version. It moves each git package to the latest commit of its branch or ref.

The `npm` and `git` folders each hold a `.gitignore` that keeps installed packages out of a project's repository. At startup, the terminal interface checks npm packages and git packages without a ref for updates, and lists those that have one.

## Try a package for one run

`-e` loads a package for one run without adding it to settings:

```sh
yapi -e npm:@your_scope/your_package
yapi -e git:github.com/your_user/your_repo
```

yapi installs these into `~/.yapi/agent/tmp/extensions` and reuses them on later runs. It installs an npm package again when its version no longer matches the source's range, and updates a git source without a ref on each run. With `--offline` or `PI_OFFLINE`, yapi installs and updates nothing, and skips a source that isn't installed yet.

## Turn resources on and off

`yapi config` lists every extension, skill, prompt template and theme from your packages, settings and folders. Space toggles one, and yapi saves the choice in `settings.json`, or in `settings.toml` when that's the folder's file. Tab switches between global settings and the current project, and `yapi config -l` starts with the project. In the Global Resources view, `ctrl+o` keeps the selected item in [project-only runs](project-only.md#keep-items-from-yapi-config).

Before it opens, `yapi config` installs the packages that are missing or outside their range, unless `PI_OFFLINE` is set. If an install fails, it prints the error and exits.

To run with only a project's packages and none of your global ones, start yapi with `--project-only`. [Run yapi with only the project's packages](project-only.md) covers the mode.

## Project packages with chest

chest keeps a project's skills, subagents and extensions in `chest.toml` and pins them in `chest.lock`. yapi runs each chest extension with only the capabilities its entry grants. Use it for packages that belong in a project's repository, and `yapi install` for packages you want in every project, as in Pi. [Choose between chest and `yapi install`](chest.md#choose-between-chest-and-yapi-install) compares the two.

## Package manifests

yapi reads the `pi` key of `package.json` to find a package's resources. Two more sources can list them, and for each kind of resource the first source that lists it wins:

1. The `[package]` table of a `chest.toml` in the package.
2. The `yapi` key of `package.json`, with the same shape as `pi`.
3. The `pi` key.

This lets one package ship a [native build](native-extensions.md) for yapi and JavaScript for Pi, and lets a package without `package.json` list its resources in `chest.toml`.

When a `pi` key exists, or `[package]` or `yapi` lists some kind, a kind that no source lists provides nothing, as in Pi. Otherwise the package uses the conventional folders: `extensions`, `skills`, `prompts` and `themes`. A `yapi` key that holds only `capabilities` therefore hides neither the `pi` lists nor the folders. yapi loads `.wasm` files from `extensions` as native extensions.

Pi's [package documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/packages.md) covers the manifest format.

## The built-in npm client

yapi has a built-in npm client. It resolves versions with npm's range syntax, verifies integrity hashes and lays out `node_modules` as npm does.

A package over any of these limits fails the install and leaves nothing behind:

- **Registry metadata**: 256 MiB to download
- **Tarball**: 512 MiB to download, unpacking to at most 2 GiB in at most 200,000 entries

The client doesn't run lifecycle scripts. Packages that build native addons do it in those scripts, and the WebAssembly runtime can't load native addons. Such addons install unbuilt and fail only if an extension loads one.

The client reads registries and credentials from `npm_config_*` environment variables and from `~/.npmrc`, or the file `npm_config_userconfig` names. It reads `registry`, `@scope:registry` and each registry's `_authToken`, `_auth`, or `username` and `_password`, and expands `${VAR}` references. Credentials go only to the registry their key names.

A request that takes longer than `fetch-timeout` milliseconds, 5 minutes by default, fails the install. `0` sets no limit. This `.npmrc` sends a token from the environment to a scoped registry:

```ini
@acme:registry=https://npm.acme.dev/
//npm.acme.dev/:_authToken=${ACME_NPM_TOKEN}
```

Pi's `npmCommand` setting replaces the built-in client for installing and updating npm sources. Removals and the dependencies of git packages always use the built-in client.
