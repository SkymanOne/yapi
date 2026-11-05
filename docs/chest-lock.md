# Lock, verify and update packages

> **Any agent.** The lock pins skills and subagents for every agent, and extensions for yapi.

`chest.lock` makes installs reproducible: every clone installs exactly what the lock pins. This page covers what the lock records, how installs use it, running chest in CI, verifying installed copies and updating entries.

## What the lock records

`chest.lock` records each entry's commit or npm version, the tarball and integrity of every npm package in its tree, and a content hash of each installed item. chest writes it, so don't edit it. [The `chest.lock` reference](chest-manifest.md#chestlock) describes the format and the hash.

A pinned tarball must come from the registry configured for its package, at the standard `<name>/-/<file>.tgz` path. Tarballs pinned on `registry.npmjs.org` come from your registry when it's a mirror. chest refuses registries that keep tarballs elsewhere, such as GitHub Packages. It does so when `chest add` or `chest install` resolves the package, before it writes a lock.

## Install from the lock

`chest install` has two modes:

- **`chest install`**: keeps what's locked and present, installs what's locked but missing exactly as locked, and resolves entries that changed in the manifest. Then it writes the lock.
- **`chest install --frozen`**: installs exactly what the lock pins, including what's missing, as `npm ci` does. It fails when the lock is missing or out of date, or when an installed copy no longer matches its hash. It never writes `chest.toml` or `chest.lock`.

## Run chest in CI

`chest install` runs frozen by default when `CI` holds anything but an empty value, `0` or `false`. A fresh clone in CI then gets every skill, subagent and extension exactly as locked. The job fails when someone changed the manifest without updating the lock.

## Verify installed copies

`chest verify` checks every installed copy, copy fallback and link against the lock. It exits with 1 when one doesn't match.

## Update entries

> **Any agent**, except the warning about extension requests, which matters only in yapi.

`chest outdated` lists entries whose source has a newer commit for its `rev`, or a newer npm version in range. It also shows npm's `latest`. `chest update` moves the named entries, or all of them, to those versions, installs them and writes the lock.

When an updated extension now requests capabilities or event channels beyond its grants, `chest update` warns and grants nothing more.

## One command at a time

One chest command at a time changes a manifest or installs from it. Another one waits for as long as the first runs, and says once that it's waiting. A frozen install fails instead when another command changes the manifest while it runs. A command that stopped without letting go of its lock holds it for 30s at most.
