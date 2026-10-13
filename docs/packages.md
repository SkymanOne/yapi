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

yapi records installed packages in `settings.json`. `-l` installs into the project's `.yapi` folder and records the package in `.yapi/settings.json`, which loads only in a [trusted project](security.md#project-trust). Local folders and files load in place and are not copied. At startup and on `/reload`, yapi installs configured packages that are missing, and reinstalls npm packages whose installed version is outside their configured range.

Packages can run code and can hold skills that instruct the model to run programs. Review a third-party package before you install it.

`yapi list` shows each package with its install location. A tag after the source shows its extension kinds: `[npm]` for JavaScript or TypeScript, `[wasm]` for native, or `[npm, wasm]` for both. Packages without extensions, and packages not installed yet, have no tag.

```
User packages:
  npm:pi-mcp-adapter [npm]
    /home/you/.yapi/agent/npm/node_modules/pi-mcp-adapter
  /home/you/extensions/shout.wasm [wasm]
    /home/you/extensions/shout.wasm
```

## Try a package

`-e` loads a package for one run without adding it to settings:

```sh
yapi -e npm:@scope/some-pi-package
yapi -e git:github.com/user/repo
```

yapi installs these into `~/.yapi/agent/tmp/extensions` and reuses them on later runs. An npm package is installed again when its version no longer matches the source's range, and a git source without a ref is updated on each run. With `--offline` or `PI_OFFLINE`, nothing is installed or updated, and a source that is not installed yet is skipped.

## Turn resources on and off

`yapi config` lists every extension, skill, prompt template and theme from your packages, settings and folders. Space toggles one, and yapi saves the choice in `settings.json`. Tab switches between global settings and the current project. `yapi config -l` starts with the project. Before it opens, it installs the packages that are missing or outside their range, those of your global settings first, then those of a trusted project. If an install fails, `yapi config` prints the error and exits. `PI_OFFLINE` skips these installs.

## Package manifests

yapi reads the `pi` key of `package.json` to find a package's resources. An optional `yapi` key with the same shape takes precedence, so one package can ship a [native build](native-extensions.md) for yapi and JavaScript for Pi. Packages without a manifest use the conventional folders: `extensions`, `skills`, `prompts` and `themes`. yapi loads `.wasm` files from `extensions` as native extensions.

Pi's [package documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/packages.md) covers the manifest format.

## npm client

yapi has a built-in npm client. It resolves versions with npm's range syntax, verifies integrity hashes and lays out `node_modules` as npm does. It does not run lifecycle scripts, which mostly build native addons that the WebAssembly runtime cannot load. Such addons are installed unbuilt and fail only if an extension loads one.

The client reads registries and credentials from `npm_config_*` environment variables and from `~/.npmrc`, or the file `npm_config_userconfig` names. It reads `registry`, `@scope:registry` and each registry's `_authToken`, `_auth`, or `username` and `_password`, and expands `${VAR}` references. Credentials go only to the registry their key names.

```ini
@acme:registry=https://npm.acme.dev/
//npm.acme.dev/:_authToken=${ACME_NPM_TOKEN}
```

Pi's `npmCommand` setting replaces the built-in client for installing and updating npm sources. Removals and the dependencies of git packages always use the built-in client.

## Planned features

Packages that declare and receive only the grants they need, project manifests and a package registry are planned. See the [Roadmap](roadmap.md).
