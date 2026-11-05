# Run yapi extensions

> **yapi only.** chest installs extensions in any project, but only yapi loads them. Other agents ignore `[extensions]`, `capabilities` and `events`.

An extension entry installs a Pi package or a native `.wasm` extension, and yapi loads each one with only the capabilities its entry grants. This page covers granting capabilities, letting extensions share events, how yapi loads a manifest and importing packages from `settings.json`.

Git and npm extensions go into the extension store, `.yapi/chest` by default, and a path extension loads where it is. These entries grant one extension everything and another file access:

```sh
chest add npm:@acme/pi-subagent --grant all
chest add git:github.com/acme/pi-todo --grant fs
```

`chest add` treats a source as an extension when it's a `.wasm`, `.ts` or `.js` file, or a package that provides extensions or has an `index.ts` or `index.js`. Pass `--extension` when chest can't tell. chest refuses a source that holds no extension, even with `--extension`.

## Grant capabilities

An extension entry lists what it may reach:

| Capability | Allows |
|---|---|
| `fs` | Reading and writing files, and the `read`, `write`, `edit` and `ls` tools |
| `net` | Network requests, model calls and provider credentials |
| `env` | Environment variables |
| `session` | Starting turns and changing the session and its files |
| `all` | Everything above, plus running programs |
| `none` | Nothing beyond the baseline. The default. |

`chest add` shows the capabilities a package requests and asks about each one. For a native extension it also shows what the build uses, from the features it was built with and the WebAssembly System Interface (WASI) interfaces it imports. It then asks about the ones the package doesn't request. A bare `.wasm` file requests nothing, so chest offers what it uses.

A request never grants anything by itself. Without a terminal, or with `--yes`, chest grants only what `--grant` lists, which is nothing by default. `--grant` applies to extensions only. `capabilities-defaults` in the manifest adds capabilities to every extension entry of that manifest.

`all` is the only way to run programs. A program runs as you, outside the sandbox, with yapi's environment and API keys. So `all` is full access, and `chest add` says so before it grants it. [What each capability allows](chest-manifest.md#what-each-capability-allows) gives the exact list.

## Let extensions share events

Extensions share `pi.events` by default, as in Pi, so cooperating Pi extensions keep working whatever their grants. Sharing has a cost between extensions with different grants. One that acts on events lets the sender borrow its access, and one that listens learns what others send.

An entry's `events` narrows that. Here `todo` sends only `todo:` channels and hears only `subagent:done`, and `scraper` neither sends nor hears:

```toml
[extensions.todo]
git = "https://github.com/acme/pi-todo"
capabilities = ["fs"]
events = { send = ["todo:*"], receive = ["subagent:done"] }

[extensions.scraper]
npm = "@acme/scraper"
capabilities = ["net"]
events = "isolated"
```

Extensions with identical grants always hear each other. Across different grants, an event reaches an extension only when the sender may send its channel and the receiver may hear it.

`chest add` offers the shared default, the channels the package declares, or `isolated`. Without a terminal, or with `--yes`, it keeps the shared default. [Event bus routing](chest-manifest.md#event-bus-routing) has the rules.

## How yapi loads a manifest

At startup and on `/reload`, yapi reads two manifests:

- the project's manifest, when the project is [trusted](security.md#project-trust)
- `~/.chest/chest.toml`, in every project, except for the entries a [project-only run](project-only.md) leaves out

A project manifest that installs anything asks for trust, as `.yapi` files do. A manifest with only a `[package]` table describes the project as a package and needs no trust. `CHEST_MANIFEST` can name another project manifest, as [How yapi reads CHEST_MANIFEST](chest-environments.md#how-yapi-reads-chest_manifest) describes.

yapi puts back every skill, subagent and extension that the lock pins and that's missing. It also puts back an entry whose copy holds what an older lock pinned, as after you pull a lock that a teammate's `chest update` wrote. chest's record of what it placed tells yapi which copy it has, so startup reads no file contents. With `--offline` or `PI_OFFLINE` set, yapi warns about these entries and leaves them out instead. So an extension never loads from a copy the lock no longer pins, and neither does a skill in a folder yapi doesn't read on its own, such as `.claude/skills`. A skill's copy in a folder yapi reads on its own, such as `.agents/skills`, keeps loading, and the warning says so.

yapi leaves out an entry that isn't locked, or whose lock no longer matches the manifest, and warns you to run `chest install`, or `chest -g install` for `~/.chest/chest.toml`. A manifest without a lock installs nothing, with the same advice. yapi never writes the lock or the `.gitignore` block.

An entry under `[patch]` needs no lock entry. A patched extension loads from its patch. A patched skill loads from where `chest install` placed it, or else from its patch. For an entry without `path`, which takes every skill of a package, that holds for each skill the patch's folder provides.

yapi loads the installed extensions as packages named `chest:<name>`. Only chest names packages this way, so yapi ignores a `packages` entry in settings that starts with `chest:`. Each chest extension runs with the [capabilities](chest-manifest.md#what-each-capability-allows) its entry grants and with its [event policy](chest-manifest.md#event-bus-routing). Packages from `yapi install` keep every grant.

When a native extension uses a capability its entry doesn't grant, yapi warns at startup, because it will deny those calls. [How yapi enforces grants](security.md#how-yapi-enforces-grants) covers the checks.

yapi loads each placed skill from its canonical copy, in the first skills folder. That works even in a folder yapi doesn't read on its own, such as `.claude/skills`. A skill in a folder yapi also reads on its own, such as `.agents/skills`, loads once, as chest's.

When two skills share a name, the first one yapi loads keeps it, as in Pi. Load order also decides which extension a tool conflict names. yapi loads resources in this order:

1. The paths that the project's settings list under `skills`, `extensions`, `prompts` and `themes`
2. The project's chest entries
3. The project's folders that yapi reads on its own, such as `.yapi/skills` and `.agents/skills`
4. The paths that your global settings list under those keys
5. The entries of `~/.chest/chest.toml`
6. The global folders, such as `~/.agents/skills` and the agent folder's `skills` and `extensions`
7. The packages from settings, in Pi's order

So a skill that the project's settings list, such as with `"skills": ["../hand-skills"]`, keeps its name over a project chest skill.

yapi never removes placed copies. After you remove an entry from the manifest, or while its lock is stale, its copies in folders yapi reads on its own, such as `.agents/skills`, keep loading until `chest install` removes them.

Before it opens, `yapi config` puts back missing entries as a run does. It lists chest's extensions and skills as a run loads them, marked as managed by chest. Space leaves them alone, because the manifest decides what loads. In the Global Resources view, `ctrl+o` can still keep a global chest item in [project-only runs](project-only.md). [Project packages from chest](configuration.md#project-packages-from-chest) lists the files.

## When a package is installed both ways

A package can be a chest entry and also a `packages` entry in settings, such as after `chest import --settings` or a `yapi install` of a package the project's manifest lists. Two copies would register the same tools, and a tool registered twice stops yapi, as in Pi. So yapi loads an npm or git package once, from the first of these that installs it:

1. The project's manifest
2. The project's settings
3. `~/.chest/chest.toml`
4. Your global settings

Two entries of one manifest that install the same package count too, and the first one loads. An entry with `path` takes one folder of its package. It's the same package as a copy of the whole package, and as an entry with the same source and `path`, however the `path` is spelled, such as `ext-a`, `./ext-a` or `ext-a/`. Two entries that take different folders of one repository both load. An entry with a [`[patch]`](chest-manifest.md#patch) counts as the package it names, so when it loads, its patch loads in place of the other copies.

In a [project-only run](project-only.md), a global copy counts only when the run keeps something of it: `projectOnly.packages` keeps it, or a pattern under `extensions`, `skills`, `prompts` or `themes` matches one of its files. The project's copies and the global copies that `packages` keeps come first, in the order above. A global copy that only a pattern keeps comes after them. So a package that `packages` keeps loads whole, and a pattern that names one copy's file keeps that copy.

yapi leaves out the other copies and warns about each one at startup, with the command that removes it:

- `yapi remove -l <source>` for a copy in the project's settings, with `--approve` while your trust in the project isn't saved
- `yapi remove <source>` for a copy in your global settings
- `chest remove <name>` for an entry of the project's manifest
- `chest -g remove <name>` for an entry of `~/.chest/chest.toml`

Between the project's settings and yours, Pi's rule applies, and the project's entry wins. A path package installed both ways names the same files, so they load once, with the chest entry's capabilities.

## Import packages from settings.json

`chest import --settings` turns the packages of `.yapi/settings.json` into extension entries with `capabilities = ["all"]`, which is what `yapi install` gives them. It keeps their `skills`, `prompts` and `themes` filters. chest has no `extensions` filter and no `autoload: false`. chest leaves out a package that sets either, with a warning, and the package keeps loading from `settings.json`.

Import leaves `settings.json` alone. For each package it imports, it prints the `yapi remove -l` command to run once chest installs it. Run it from the project folder, since `-l` edits the `.yapi/settings.json` of the folder you run it in. With `-g`, it imports the agent folder's `settings.json` and prints `yapi remove` without `-l`, which works from any folder.

After importing, narrow each entry's capabilities and run `chest install` right away. Then run the `yapi remove` commands that import printed. Until `chest install` locks an entry, yapi leaves it out:

- An npm or git package loads from the copy in settings, with every capability.
- A path package's files belong to the chest entry, which doesn't load yet, so they run with no capabilities. That's on purpose: a file that chest manages never runs with more than its entry grants.

Once chest installs an entry, yapi loads chest's copy and leaves out the one in settings, with a warning, until you remove it.
