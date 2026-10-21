# Troubleshooting

## Network

yapi honors `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` and `NO_PROXY`, and the `httpProxy` setting. Certificates are checked against the operating system's trust store.

`--offline`, or `PI_OFFLINE=1`, skips startup network work such as downloading `fd` and `rg`.

## macOS blocks a downloaded binary

macOS marks an archive downloaded in a browser as quarantined, and Gatekeeper then refuses to open the `yapi` binary inside it, which Apple has not notarized. The install script, `curl` and Cargo do not set the mark. Remove it from the unpacked binary:

```sh
xattr -d com.apple.quarantine ~/.local/bin/yapi
```

## No models available

yapi found no credentials. Set a provider's API key or run `/login`. See [Choose a model](models.md).

## An extension breaks startup

`yapi -ne` starts without extensions, including the built-in MCP and llama.cpp extensions. `yapi config` then turns the failing extension off.

## Reporting a problem

Run `/bug`, or `/bug <description>`, to report a problem:

1. Describe what went wrong, or leave the description empty.
2. Choose whether to include the session transcript. It holds your messages, model output, tool calls and their results, including file contents and command output.
3. Without the transcript, the current model can write a summary instead. yapi sends the transcript to the model's provider with your credentials and keeps only the summary.

yapi writes `yapi-bug-report-<id>.zip` to the working folder. The report can include your yapi version, operating system, the current model and provider configuration without API keys, loaded extensions, settings, and provider error diagnostics from the session. Its `report.json` leaves out API keys, tokens, passwords and other credentials. The transcript and the summary are kept as written.

Unless yapi runs with `--offline` or `PI_OFFLINE`, and when the [GitHub CLI](https://cli.github.com/) is signed in, `/bug` then offers to open an issue on the [issue tracker](https://github.com/SkymanOne/yapi/issues). A GitHub issue is public, unlike Pi's private upload to radius.pi.dev. The issue holds your description, the summary and `report.json`, with the names of your settings but not their values, and with your home folder written as `~`. It never holds the transcript. Attach the zip file to the issue yourself if you want to share it, after you check it for private data. Otherwise, open an issue by hand and attach the zip file.

Run `/debug` to write the screen, the session's messages and recent extension output to `~/.yapi/agent/yapi-debug.log`. Check it for private data, then attach it to a report on the [issue tracker](https://github.com/SkymanOne/yapi/issues) with the output of `yapi --version` and your operating system.

## Logs

Start yapi with `--enable-logging` to append its log to `~/.yapi/agent/yapi.log`, or `yapi.log` in `YAPI_CODING_AGENT_DIR`. Each line holds the time, the process ID, the level, where the message comes from, and the message. The log has extension output, extension runtime restarts and crashes, and warnings from yapi's libraries. Only you can read the file, as extension output can hold private data. yapi never shortens or removes it, so delete it when you no longer need it.
