---
name: validate-live
description: Try a yapi branch against a real model before merging it. Use when asked to validate, smoke-test or check a branch or pull request live, beyond the mocked tests, or to see a change working in yapi's print, JSON, RPC or interactive mode.
---

# Validate a branch against a real model

`cargo test` uses mocks only. A live check runs the branch's binary against a real provider, with credentials kept out of the repository and away from the user's own agent directory.

## Set up

1. Build the branch: `cargo build -p yapi`. The binary is `target/debug/yapi`.
2. Make an isolated agent directory in a scratch location and point `YAPI_CODING_AGENT_DIR` at it.
3. Copy into it only the needed provider's entry of `~/.yapi/agent/auth.json`, as an `auth.json` holding that one key. When the model comes from the fetched catalog rather than the built-in one, copy `models-store.json` too. Add only the settings and packages the check needs.
4. Delete the directory, and the key copy with it, when the check is done.

## Run

- Print and JSON modes: close stdin, or yapi waits for piped input, as pi does.

  ```sh
  YAPI_CODING_AGENT_DIR="$dir" target/debug/yapi --model <provider>/<model> -p "<prompt>" < /dev/null
  YAPI_CODING_AGENT_DIR="$dir" target/debug/yapi --model <provider>/<model> --mode json -p "<prompt>" < /dev/null
  ```

- Interactive mode: run yapi in a tmux session, type with `tmux send-keys` and read the screen with `tmux capture-pane -p`. Wait for the screen to settle between steps.
- `/bug`: put a fake `gh` first on `PATH`, such as a script that prints its arguments, so no issue is posted to the public repository.

Report what was run, the model used and what was observed, and say which behaviors were not exercised.
