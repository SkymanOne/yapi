---
name: yapi
description: Run, script and configure yapi, the Rust reimplementation of the Pi coding agent. Use when a task asks you to install yapi, hand work to it from a script or another agent (print, JSON or RPC mode), choose its model or credentials, manage its sessions, packages, skills or settings, move a setup over from Pi, or find out why yapi misbehaves.
license: MIT OR Apache-2.0
---

# Working with yapi

yapi (Yet Another Pi) is a terminal coding agent: give it a goal and a working folder, and it reads files, runs commands and edits code. It reimplements Pi `v1.0.0` in Rust, so it uses Pi's commands, flags, settings, session files, packages and extension API, from one native binary that needs no Node.js. This skill covers where yapi keeps its files, installing it, handing it tasks, models, sessions, packages, chest and troubleshooting.

When you know Pi, assume yapi behaves the same and check [Differences from Pi](https://skymanone.github.io/yapi/compat.html) for the exceptions.

## Where yapi keeps its files

yapi keeps global state in its agent folder and project state in `.yapi/`:

| What | Where |
|---|---|
| Global state | `~/.yapi/agent` (move it with `YAPI_CODING_AGENT_DIR`) |
| Project state | `.yapi/` in the project, read only after you trust the project |
| Settings | `~/.yapi/agent/settings.json`, overridden by `.yapi/settings.json`. Either can be `settings.toml` instead, with the same settings, but not both in one folder. |
| Credentials | `~/.yapi/agent/auth.json` |
| Custom providers and models | `~/.yapi/agent/models.json` |
| Sessions | `~/.yapi/agent/sessions/`, one JSON Lines file per session, grouped by project (move them with `YAPI_CODING_AGENT_SESSION_DIR` or `--session-dir`) |
| Context files | `AGENTS.md` or `CLAUDE.md` in the project, its parents and `~/.yapi/agent` |
| Skills | `~/.yapi/agent/skills`, `.yapi/skills`, `~/.agents/skills` and `.agents/skills` up to the repository root |

Every file uses Pi's format. Provider variables such as `ANTHROPIC_API_KEY` and `PI_OFFLINE` keep Pi's names.

## Install yapi and check the version

The install script puts the release binaries, `yapi` and `chest`, in `~/.local/bin`. In a terminal it asks which to install, so pass `--only` with `yapi`, `chest` or `both` to skip the question:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh \
  | sh -s -- --only both
yapi --version
```

Add `--from-source` to build the release with Cargo instead of downloading it. With Cargo directly, the first command installs the release binary through cargo-binstall, and the second builds from source with Rust 1.99 or newer:

```sh
cargo binstall --git https://github.com/SkymanOne/yapi yapi
cargo install --locked --git https://github.com/SkymanOne/yapi yapi
```

Keep `--git` for yapi: on crates.io, `yapi` is an unrelated crate. chest is on crates.io as `chest-cli`, so `cargo binstall chest-cli` and `cargo install --locked chest-cli` install it. To bring an existing Pi setup across, run `yapi import pi`. It copies settings, credentials, sessions and packages, and keeps files yapi already has.

## Hand a task to yapi

The interactive interface needs a terminal. From a script or another agent, use one of the headless modes.

**Print mode** answers once and exits. Piped input becomes part of the message, and `@path` attaches files and images:

```sh
yapi -p --no-session "Summarize the open TODOs in src/"
git diff | yapi -p --no-session "Write a commit message for this diff"
```

**JSON mode** prints one JSON event per line. To get the final answer, take the last assistant `message_end`:

```sh
yapi --mode json -p --no-session "List the crates in this workspace" \
  | jq -rs '[.[] | select(.type == "message_end"
                    and .message.role == "assistant")]
            | last | .message.content[] | select(.type == "text") | .text'
```

A run emits `session`, `agent_start`, then per turn `turn_start`, `message_start`, `message_update`, `message_end`, `tool_execution_start` and `tool_execution_end`, `turn_end`, and finally `agent_end` with every message.

**Remote procedure call (RPC) mode** keeps yapi running for more than one prompt. Write one JSON command per line to stdin, such as `{"id":"1","type":"prompt","message":"Run the tests"}`, and read responses (`"type": "response"`, with your `id`) and events from stdout. yapi exits when stdin closes, so keep it open until the run's `agent_end` event arrives. [references/rpc.md](references/rpc.md) has a client and every command.

These flags matter when another program drives yapi:

| Flag | Effect |
|---|---|
| `--no-session` | Don't save the run |
| `--model <provider/id[:thinking]>` | Pick the model, such as `anthropic/claude-sonnet-4-5:high` |
| `--tools read,grep,find,ls` | Allow only these tools, here read-only ones. `--no-tools` disables all. |
| `--exclude-tools <names>` | Disable some tools and keep the rest |
| `-ne`, `-ns`, `-nc` | Start without extensions, skills or context files |
| `--offline` | Skip startup network work, as `PI_OFFLINE=1` does |
| `--approve` or `--no-approve` | Trust or ignore project-local files for this run without prompting |
| `--project-dir <dir>` | Take project files, trust and context files from another folder while tools run in the working folder |
| `--append-system-prompt <text or file>` | Add instructions for this run |

Exit code 0 means the run finished. A provider error ends print mode with a non-zero code and the message on stderr.

## Choose models and credentials

yapi finds credentials in the environment, in `auth.json` and through sign-ins:

- API keys come from the environment (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `GEMINI_API_KEY`, `OPENROUTER_API_KEY` and the others `yapi --help` lists), from `auth.json`, or from `--api-key`.
- Subscriptions and accounts (Claude Pro or Max, ChatGPT, GitHub Copilot, Kimi, Meta, xAI, OpenRouter, Radius) sign in with `/login` in the interactive interface. Amazon Bedrock, Google Vertex AI and Cloudflare also read their platform's usual credentials. A local llama.cpp router signs in with `/login llama.cpp` or `LLAMA_BASE_URL`.
- `yapi --list-models [search]` lists what's available with the current credentials.
- `yapi auth check --provider <id>` reports whether a provider is ready, and `--json` makes it machine-readable. `yapi auth print-api-key` and `print-bearer-token` print a credential for another tool.

Never print or log credentials yourself. Prefer `yapi auth check` over reading `auth.json`.

## Resume and branch sessions

Sessions resume from the command line and branch in the interface:

- `yapi -c` continues the latest session in this folder, `yapi -r` picks one, `--session <path or id>` opens one, and `--fork <path or id>` copies one into a new session.
- In the interface, `/tree` moves to any earlier point and continues from there, `/fork` and `/clone` branch, and `/compact` summarizes old context.
- `yapi --export <session.jsonl> [out.html]` writes a session as HTML.

## Install packages, extensions and skills

These commands manage what yapi loads:

| Command | Effect |
|---|---|
| `yapi install npm:package_name` | Install a Pi package from npm |
| `yapi install git:github.com/your_user/your_repo` | Install a package from git |
| `yapi install ./folder -l` | Install a local package for this project only |
| `yapi list` | List packages, tagged `[npm]` or `[wasm]` by their extensions |
| `yapi update --extensions` | Update packages |
| `yapi remove npm:package_name` | Remove a package by its source |
| `yapi config` | Turn single resources on and off |
| `yapi -e ./ext.ts` | Load an extension for one run |
| `yapi -e npm:package_name` | Try a package for one run, without adding it to settings |
| `yapi --skill ./my-skill` | Load a skill for one run |

Pi packages run unchanged. Their extensions run in a WebAssembly sandbox, so code that needs native addons, sockets, worker threads or SQLite fails when it reaches them. To write an extension, use the `yapi-extension` skill.

## Pin project packages with chest

`chest` is a separate binary for a project's skills, subagents and extensions: one `chest.toml`, a `chest.lock` that pins commits, npm trees and content hashes, and capabilities granted per extension. Use it when a project needs reproducible installs or narrow grants, or skills shared with other agents. Use `yapi install` for personal packages that behave as in Pi.

| Command | Effect |
|---|---|
| `chest init` | Write `chest.toml` with folders for the agents found |
| `chest add git:github.com/acme/skills --path skills/pdf` | Add a skill from git |
| `chest add npm:@acme/ext --grant fs` | Add an extension and grant only what it needs |
| `chest install` | Install as `chest.lock` pins |
| `chest install --frozen` | In CI, fail on drift and write nothing |
| `chest verify` | Check installed copies against the lock |

Skills and subagents work with any agent. Extensions, capabilities and events work only in yapi. `all` is the only capability that runs programs, and it's full access. See [Get started with chest](https://skymanone.github.io/yapi/chest.html) and [Grant capabilities](https://skymanone.github.io/yapi/chest-extensions.html#grant-capabilities).

## Find why yapi misbehaves

Work through these checks in order:

1. Reproduce with a clean state: `YAPI_CODING_AGENT_DIR=$(mktemp -d) yapi -p --no-session "hi"` rules out settings and packages. `-ne` rules out extensions.
2. `yapi auth check --provider <id>` and `yapi --list-models` confirm the model and credentials.
3. Startup warnings name settings, skills, themes or extensions that failed to load. The interactive header lists the loaded resources, and `--verbose` shows it even when `quietStartup` is set.
4. In the interface, `/debug` writes what yapi rendered and sent to `~/.yapi/agent/yapi-debug.log`.
5. When behavior differs from Pi, check [Differences from Pi](https://skymanone.github.io/yapi/compat.html) before treating it as a bug. Report bugs in the [issue tracker](https://github.com/SkymanOne/yapi/issues).
