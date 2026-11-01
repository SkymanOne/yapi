---
name: pi-upgrade
description: Move yapi to a new pinned pi release. Use when a task asks to bump, upgrade or follow a new version of pi or its npm packages (@earendil-works/pi-coding-agent, pi-ai, pi-agent-core, pi-tui).
---

# Upgrade the pinned pi release

The pi release pinned in `tests/fixtures/pi/generator/package.json` is yapi's behavioral specification. Read `AGENTS.md` first. An upgrade lands as one change that moves the pin and everything derived from it.

1. Clone pi outside this repository and check out the old and the new tag.
2. Diff the extension API and the docs between the tags: `packages/coding-agent/src/core/extensions/types.ts` and `packages/coding-agent/docs`. Read the source changes of every behavior yapi ports, which the `Port of ... in pi vX.Y.Z` doc comments name.
3. Set the new version in `tests/fixtures/pi/generator/package.json`, then run `npm install --ignore-scripts` in that directory to update its lock file.
4. Update `yapi-types`, the JS facade in `guest/yapi-js/js/pi/` and the ported Rust code for the changes in the diff.
5. `cargo xtask vendor-pi` bundles the new pi packages into `guest/yapi-js/js/vendor`. `cargo xtask js-runtime` then rebuilds `yapi-js.wasm` and its inputs hash.
6. Set `PI_AI_VERSION` in `xtask/src/models.rs`, then `cargo xtask models` regenerates the model catalog from the new pi-ai.
7. Regenerate pi's goldens as `tests/fixtures/pi/README.md` lists, and the scenario goldens with `cargo xtask e2e --record-pi`. Review every changed golden: each change is a pi change that yapi must match or list in `docs/compat.md`.
8. Find the remaining mentions of the old version with `git grep`, such as the README, `docs/compat.md`, `docs/contributing.md`, the skills' links to pi's docs and the port comments, and update them.
9. Run the checks in `AGENTS.md`, and diff the extension `types.ts` against yapi's facade once more for exports that were added or removed.
