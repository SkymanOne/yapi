# Agent skills

The repository ships two [Agent Skills](https://agentskills.io/specification) that teach a coding agent to work with yapi. yapi and Pi load them, and so do other agents that read Agent Skills, such as Claude Code.

| Skill | Use it to |
|---|---|
| `yapi` | Install and run yapi, hand it tasks from scripts or other agents in print, JSON or RPC mode, choose models and credentials, and manage sessions, packages and settings. `scripts/ask.py` sends one prompt over RPC and prints the answer. |
| `yapi-extension` | Write, test and package Pi extensions in TypeScript and native extensions in Rust. It includes a template for Pi extensions and notes on where yapi's runtime differs from Pi. `scripts/check-extension.py` loads extensions without a model and reports their commands and load errors. |

## Install the skills

In yapi or Pi, install the repository at a release tag as a package:

```sh
yapi install git:github.com/SkymanOne/yapi@v0.1.0
```

yapi clones the repository but loads only its `skills` folder. A package installed at a tag stays at that tag through `yapi update --extensions`, so install a newer tag to update the skills. Installed without a tag, the package follows the `main` branch.

To use the skills without the rest of the repository, copy their folders into a skills directory, such as `~/.yapi/agent/skills/`, `~/.agents/skills/` or, for Claude Code, `~/.claude/skills/`:

```sh
cp -r skills/yapi skills/yapi-extension ~/.agents/skills/
```

## Use the skills

The agent loads a skill when a task matches its description. To load one explicitly in yapi or Pi, run `/skill:yapi` or `/skill:yapi-extension` followed by the request:

```text
/skill:yapi-extension add a /standup command that summarizes today's git commits
```

The helper scripts need Python 3 and `yapi` on `PATH`, or its path in `--yapi`.
