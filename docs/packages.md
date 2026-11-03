# Use packages

A package bundles extensions, skills, prompt templates and themes. yapi installs Pi packages from npm, git or a local folder, with no Node.js required.

## Install and manage packages

```sh
yapi install npm:@scope/some-pi-package     # from npm
yapi install git:github.com/user/repo       # from git
yapi install ./my-package -l                 # a local folder, for this project only
yapi install ./shout.wasm                    # a single native extension
yapi list
yapi update --extensions
yapi remove npm:@scope/some-pi-package
```

yapi records installed packages in `settings.json`. `-l` installs into the project's `.yapi` folder and records the package in `.yapi/settings.json`, which loads only in a [trusted project](security.md#project-trust). Local folders and files load in place and are not copied. At startup and on `/reload`, yapi installs configured packages that are missing, and reinstalls npm packages whose installed version is outside their configured range or whose install was interrupted.

Packages can run code and can hold skills that instruct the model to run programs. Review a third-party package before you install it.

`yapi list` shows each package with its install location. A tag after the source shows its extension kinds: `[npm]` for JavaScript or TypeScript, `[wasm]` for native, or `[npm, wasm]` for both. Packages without extensions, and packages not installed yet, have no tag.

```text
User packages:
  npm:pi-mcp-adapter [npm]
    /home/you/.yapi/agent/npm/node_modules/pi-mcp-adapter
  /home/you/extensions/shout.wasm [wasm]
    /home/you/extensions/shout.wasm
```

`yapi update --extensions` moves each npm package to the newest version its source's range allows, unless the source names an exact version, and each git package to the latest commit of its branch or ref. It installs a configured git package that is missing, and fails on a git package folder that is not a checkout of its own, so git never touches an enclosing repository. The `npm` and `git` folders each hold a `.gitignore` that keeps installed packages out of a project's repository. At startup, the terminal interface checks npm packages and git packages without a ref for updates, and lists those that have one.

## Try a package

`-e` loads a package for one run without adding it to settings:

```sh
yapi -e npm:@scope/some-pi-package
yapi -e git:github.com/user/repo
```

yapi installs these into `~/.yapi/agent/tmp/extensions` and reuses them on later runs. An npm package is installed again when its version no longer matches the source's range, and a git source without a ref is updated on each run. With `--offline` or `PI_OFFLINE`, nothing is installed or updated, and a source that is not installed yet is skipped.

## Turn resources on and off

`yapi config` lists every extension, skill, prompt template and theme from your packages, settings and folders. Space toggles one, and yapi saves the choice in `settings.json`. Tab switches between global settings and the current project. `yapi config -l` starts with the project. Before it opens, it installs the packages that are missing or outside their range, those of your global settings first, then those of a trusted project. If an install fails, `yapi config` prints the error and exits. `PI_OFFLINE` skips these installs.

## Project packages with chest

chest keeps a project's skills, subagents and extensions in `chest.toml` and pins them in `chest.lock`, with a content hash for each entry. Use it when a project's packages belong in its repository and should install the same way for everyone, or when an extension should run with only some capabilities. Use `yapi install` for packages you want in every project, as in Pi. [Manage project packages with chest](chest.md) covers the commands, and the [chest.toml reference](chest-manifest.md) the manifest.

At startup and on `/reload`, yapi reads the project's manifest when the project is [trusted](security.md#project-trust), and `~/.chest/chest.toml` in every project. It puts back entries that the lock pins and that are missing, unless `--offline` or `PI_OFFLINE` is set, in which case it warns about them. It loads the installed extensions as packages named `chest:<name>`. Only chest names packages this way, so yapi ignores a `packages` entry in settings that starts with `chest:`. An entry under `[patch]` needs no lock entry. A patched extension loads from its patch, and a patched skill from where `chest install` placed it, or else from its patch. An entry that is not locked, or whose lock no longer matches the manifest, is left out with a warning to run `chest install`. yapi never writes the lock.

Each chest extension runs with the [capabilities](chest-manifest.md#capabilities) its entry grants and with its [event policy](chest-manifest.md#event-bus), while packages from `yapi install` keep every grant. A skill that chest places loads in yapi wherever the manifest puts it, even in a folder yapi does not read on its own, such as `.claude/skills`. `yapi config` lists chest's extensions and skills as a run loads them, marked as managed by chest. They are read-only there, because the manifest decides what loads. [Configure yapi](configuration.md#chest) lists the files.

## Package manifests

yapi reads the `pi` key of `package.json` to find a package's resources. Two more sources can list them, and for each kind of resource the first source that lists it wins:

1. The `[package]` table of a `chest.toml` in the package.
2. The `yapi` key of `package.json`, with the same shape as `pi`.
3. The `pi` key.

This lets one package ship a [native build](native-extensions.md) for yapi and JavaScript for Pi, and lets a package without `package.json` list its resources in `chest.toml`. When a `pi` key exists, or `[package]` or `yapi` lists some kind, a kind that no source lists provides nothing, as in Pi. Otherwise the package uses the conventional folders: `extensions`, `skills`, `prompts` and `themes`. A `yapi` key that holds only `capabilities` therefore hides neither the `pi` lists nor the folders. yapi loads `.wasm` files from `extensions` as native extensions.

Pi's [package documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/packages.md) covers the manifest format.

## npm client

yapi has a built-in npm client. It resolves versions with npm's range syntax, verifies integrity hashes and lays out `node_modules` as npm does. It accepts only the package names npm allows, and no name or scope that starts with a dot. A package's registry metadata may be at most 256 MiB to download. Its tarball may be at most 512 MiB to download and unpack to at most 2 GiB in at most 200,000 entries. A package over any of these fails the install and leaves nothing behind. It does not run lifecycle scripts, which mostly build native addons that the WebAssembly runtime cannot load. Such addons are installed unbuilt and fail only if an extension loads one.

The client reads registries and credentials from `npm_config_*` environment variables and from `~/.npmrc`, or the file `npm_config_userconfig` names. It reads `registry`, `@scope:registry` and each registry's `_authToken`, `_auth`, or `username` and `_password`, and expands `${VAR}` references. Credentials go only to the registry their key names. A request that takes longer than `fetch-timeout` milliseconds, five minutes by default, fails the install. `0` sets no limit.

```ini
@acme:registry=https://npm.acme.dev/
//npm.acme.dev/:_authToken=${ACME_NPM_TOKEN}
```

Pi's `npmCommand` setting replaces the built-in client for installing and updating npm sources. Removals and the dependencies of git packages always use the built-in client.
