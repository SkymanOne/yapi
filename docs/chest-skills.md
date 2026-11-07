# Share skills and subagents

> **Any agent.** chest places skills and subagents in the folders each agent reads. You don't need yapi. Only the last section is about yapi.

A skill is a folder with a `SKILL.md`, and a subagent is a `.md` file. This page covers adding them, choosing the folders chest fills for each agent, how chest places and tracks items, and importing from skills.sh.

List skills and subagents under `[skills]` and `[agents]` in `chest.toml`, or add them with `chest add`:

```sh
chest add git:github.com/anthropics/skills --path skills/pdf
chest add git:github.com/acme/agents --path reviewer.md
```

chest tells a skill from a subagent by looking at the package. Pass `--skill` or `--agent` when it can't tell.

With `--path`, an entry installs one item. Without it, the entry installs every skill or subagent the package provides. A local folder that isn't a skill itself works the same way.

To add skills from a repository by name, pass `--skill <name>` once for each. chest looks in the `skills` folder at the root of the source, by folder name or by the `name` in a skill's `SKILL.md`, and writes the entry that `--path skills/<name>` writes:

```sh
chest add git:github.com/apollographql/skills --skill rust-best-practices
```

A bare `--skill`, after the source, only tells chest that the source is a skill.

## Pick skills at a terminal

When a package provides several skills or subagents and you pass neither `--path` nor `--skill <name>`, `chest add` lists them with their descriptions, all ticked:

- **Keep them all**: chest writes one entry for the whole package, which gets the skills the package adds when you run `chest update`. A script or `--yes` writes the same entry.
- **Clear some**: chest writes one entry for each skill you keep, each with its `path`.

The summary before you confirm says which of the two chest writes.

chest installs only skills that are folders with a `SKILL.md`. It skips or refuses a skill that's a single `.md` file, which yapi and Pi also load. A `.md` file outside a `skills` folder is a subagent.

## Choose folders for your agents

`chest init` looks for each agent's folder in the project and offers its skill and subagent folders:

| Agent | Found by | Skills | Subagents |
|---|---|---|---|
| Agent Skills (Codex, Cursor, Copilot, OpenCode, Pi) | `.agents` | `.agents/skills` | |
| Claude Code | `.claude` | `.claude/skills` | `.claude/agents` |
| Codex | `.codex` | `.agents/skills` | `.codex/agents` |
| Cursor | `.cursor` | `.agents/skills` | `.cursor/agents` |
| GitHub Copilot | `.github/agents` or `.github/copilot-instructions.md` | `.agents/skills` | `.github/agents` |
| OpenCode | `.opencode` | `.agents/skills` | `.opencode/agent` |
| yapi | `.yapi` | `.agents/skills` | `.yapi/agents` |

At a terminal, `chest init` lists the folders of each kind with the agents that read them, and ticks the ones it found. When you tick two or more folders of a kind, it asks which one holds the copy and lists that one first. Then it shows a summary and asks before it writes `chest.toml`. With both `--skills` and `--agents`, it asks nothing and writes the file at once. Without a terminal, or with `--yes`, it takes the ticked folders. When it finds no agent, skills go to `.agents/skills` and subagents to `.yapi/agents`. `--skills`, `--agents` and `--store` set the folders directly.

A folder can be a link to another one, such as `.claude/skills` linking to `../.agents/skills`. chest places each item once, in the folder the link resolves to.

## How chest copies and links items

The first folder of each kind holds the canonical copy of every item. The other folders get relative links to it, or a copy where chest can't make a link.

## The `.gitignore` block

chest lists every installed path in a block of the project's `.gitignore`, from `# BEGIN chest (chest.toml)` to `# END chest`. It rebuilds the block on each install and leaves every other line alone. The block also ignores the `.*-staging` and `.*-old` folders that an interrupted install can leave behind, which the next install removes.

## Keep hand-written skills in the same folders

Hand-written skills can live in the same folders. chest replaces or removes an item only when it installed the item itself on this machine and the item still holds what chest installed. It also takes over a copy it didn't record when that copy already holds exactly what the lock pins. chest keeps its record in `~/.chest/state`, outside the project.

When someone edits an item after chest installed it, chest leaves the item as it is, even when the lock now pins another version, and installs the other entries as usual. `chest install`, `chest update` and `chest list` show the entry as `edited`, and `chest verify` reports the item. To get the locked copy back, delete the item and run `chest install`. `chest install --frozen` fails on an edited item instead.

## Import from skills.sh

`chest import --skills-lock` turns the skills of a skills.sh `skills-lock.json` into skill entries. It reads the `skills-lock.json` beside the manifest, or the file you name. Run `chest install` after importing.

Import takes skills from GitHub, GitLab, other git hosts, local folders and `node_modules`. It skips, with a warning, a skill of another `sourceType`, a skill the manifest already has, and a different skill whose name the manifest already uses.

## What yapi adds

> **yapi only.**

yapi does more with the same entries. When it starts, it puts back missing skills and subagents: those of `~/.chest/chest.toml` in every project, and those of the project's manifest once you trust the project. It also loads skills from folders it doesn't read on its own. [How yapi loads a manifest](chest-extensions.md#how-yapi-loads-a-manifest) describes it.
