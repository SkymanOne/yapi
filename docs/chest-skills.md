# Share skills and subagents

> **Any agent.** chest places skills and subagents in the folders each agent reads. You don't need yapi. Only the last section is about yapi.

A skill is a folder with a `SKILL.md`, and a subagent is a `.md` file. This page covers adding them, choosing the folders chest fills for each agent, how chest places and tracks items, and importing from skills.sh.

List skills and subagents under `[skills]` and `[agents]` in `chest.toml`, or add them with `chest add`:

```sh
chest add git:github.com/anthropics/skills --path skills/pdf
chest add git:github.com/acme/agents --path reviewer.md
```

chest tells a skill from a subagent by looking at the package. Pass `--skill` or `--agent` when it can't tell. With `--path`, an entry installs one item. Without it, the entry installs every skill or subagent the package provides. A local folder that isn't a skill itself works the same way.

chest installs skills that are folders with a `SKILL.md`. yapi and Pi also load a skill that's a single `.md` file, but chest doesn't install one. An entry for a whole package skips such files with a warning. An entry that names one fails, and `chest add` refuses a `.md` file in a `skills` folder. A `.md` file anywhere else is a subagent, and `--agent` adds any `.md` file as one.

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

At a terminal, `chest init` lists the folders by number, with the ones it found selected. Without a terminal, or with `--yes`, it takes those. When it finds no agent, skills go to `.agents/skills` and subagents to `.yapi/agents`. `--skills`, `--agents` and `--store` set the folders directly.

A folder can be a link to another one, such as `.claude/skills` linking to `../.agents/skills`. chest places each item once, in the folder the link resolves to.

## How chest copies and links items

The first folder of each kind holds the canonical copy of every item. The other folders get relative links to it, or a copy where chest can't make a link.

## The `.gitignore` block

chest lists every installed path in a block of the project's `.gitignore`, from `# BEGIN chest (chest.toml)` to `# END chest`. It rebuilds the block on each install and leaves every other line alone. chest escapes each line so that it matches only its own path, whatever characters a folder name holds.

The block also ignores two kinds of folder that an interrupted install can leave where chest copies:

- **`.*-staging`**: a copy that chest hadn't finished
- **`.*-old`**: an item's previous copy, when chest stopped while replacing it

The next install removes both.

## Keep hand-written skills in the same folders

Hand-written skills can live in the same folders. chest replaces or removes an item only when it installed the item itself on this machine and the item still holds what chest installed. It also takes over a copy it didn't record when that copy already holds exactly what the lock pins. chest keeps its record in `~/.chest/state`, outside the project.

When someone edits an item after chest installed it, chest leaves the item as it is, even when the lock now pins another version, and installs the other entries as usual. `chest install`, `chest update` and `chest list` show the entry as `edited`, and `chest verify` reports the item. To get the locked copy back, delete the item and run `chest install`. `chest install --frozen` fails on an edited item instead.

## Import from skills.sh

`chest import --skills-lock` turns the skills of a skills.sh `skills-lock.json` into skill entries. It reads the `skills-lock.json` beside the manifest, or the file you name. chest computes its own hashes, so it drops the file's. Run `chest install` after importing.

## What yapi adds

> **yapi only.**

yapi does more with the same entries. When it starts, it puts back missing skills and subagents: those of `~/.chest/chest.toml` in every project, and those of the project's manifest once you trust the project. It also loads skills from folders it doesn't read on its own. [How yapi loads a manifest](chest-extensions.md#how-yapi-loads-a-manifest) describes it.
