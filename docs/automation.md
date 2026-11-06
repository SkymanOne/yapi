# Automate yapi

yapi runs without its interface for scripts, editors and other programs. This page covers print mode, JSON mode and remote procedure call (RPC) mode, and the flags that matter when another program drives yapi. The output formats and protocols are Pi's, so Pi's clients work with yapi.

## Answer once with print mode

`-p` sends one message, prints the answer and exits. These three runs take a prompt, piped input and attached files:

```sh
yapi -p "Summarize the open TODOs in src/"
git diff | yapi -p "Write a commit message for this diff"
yapi -p @report.md @chart.png "What does the chart show?"
```

Piped input becomes part of the first message, and `@path` attaches files and images.

## Stream events with JSON mode

`--mode json` prints one JSON event per line for the whole run: message updates, tool calls, tool results and the final state. Pi's [JSON documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/json.md) describes the events.

```sh
yapi --mode json -p "List the crates in this workspace"
```

## Drive yapi with RPC mode

`--mode rpc` keeps yapi running and reads commands from stdin, one JSON object per line. Responses and events go to stdout. Clients can prompt, steer, abort, switch models and sessions, and answer extension dialogs.

Pi's [RPC documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/rpc.md) lists the commands, and Pi's `RpcClient` works with yapi. This starts an RPC session that saves nothing:

```sh
yapi --mode rpc --no-session
```

## Flags for automation

These flags change how yapi behaves when another program runs it:

| Flag | Effect |
|---|---|
| `--no-session` | Don't save the run |
| `--model <pattern>` | Choose the model |
| `--tools read,grep,find,ls` | Allow only these tools, here read-only ones |
| `--no-tools` | Disable all tools |
| `-ne` | Start without extensions, including the built-in Model Context Protocol (MCP) and llama.cpp extensions |
| `--offline` | Skip startup network work |
| `--enable-logging` | Append a log to `~/.yapi/agent/yapi.log`. See [Read yapi's log](troubleshooting.md#read-yapis-log). |
| `--project-dir <dir>` | Take the project from another folder. See [Project trust](security.md#project-trust). |
| `--project-only` | Load only the project's packages, skills and MCP servers. `YAPI_PROJECT_ONLY=1` does the same. See [Run yapi with only the project's packages](project-only.md). |

`yapi --help` lists every flag.
