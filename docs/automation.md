# Automate yapi

yapi runs without its interface for scripts, editors and other programs. The output formats and protocols are Pi's, so Pi's clients work with yapi.

## Print mode

`-p` sends one message, prints the answer and exits:

```sh
yapi -p "Summarize the open TODOs in src/"
git diff | yapi -p "Write a commit message for this diff"
yapi -p @report.md @chart.png "What does the chart show?"
```

Piped input becomes part of the first message, and `@path` attaches files and images.

## JSON mode

`--mode json` prints one JSON event per line for the whole run: message updates, tool calls, tool results and the final state. Pi's [JSON documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/json.md) describes the events.

```sh
yapi --mode json -p "List the crates in this workspace"
```

## RPC mode

`--mode rpc` keeps yapi running and reads commands from stdin, one JSON object per line. Responses and events go to stdout. Clients can prompt, steer, abort, switch models and sessions, and answer extension dialogs. Pi's [RPC documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/rpc.md) lists the commands, and Pi's `RpcClient` works with yapi.

```sh
yapi --mode rpc --no-session
```

## Useful flags

| Flag | Effect |
|---|---|
| `--no-session` | Do not save the run |
| `--model <pattern>` | Choose the model |
| `--tools read,grep,find,ls` | Allow only these tools, here read-only ones |
| `--no-tools` | Disable all tools |
| `-ne` | Start without extensions |
| `--offline` | Skip startup network work |
| `--project-dir <dir>` | Take the project from another folder. See [Project trust](security.md#project-trust). |

`yapi --help` lists every flag.
