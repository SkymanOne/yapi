---
name: yapi-scenarios
description: Add, run, record or debug yapi's end-to-end scenarios and provider cassettes. Use when a change needs a test that compares yapi with pi end to end, when a scenario in crates/yapi/tests/scenarios.rs fails, or when a task touches tests/fixtures/scenarios or tests/fixtures/cassettes.
---

# End-to-end scenarios and cassettes

A scenario runs pi or yapi as a black box against a cassette, in a fresh working directory and agent directory with a cleared environment. Scenarios are listed in `tests/fixtures/scenarios/scenarios.json`, and `Scenario` in `crates/yapi-mock/src/scenario.rs` documents their fields. The golden `tests/fixtures/scenarios/<name>.json` is pi's normalized result. Never edit a golden by hand.

## Run and record

```sh
cargo test -p yapi --test scenarios           # yapi against every golden
cargo xtask e2e --only <text>                 # scenarios whose name contains <text>
cargo xtask e2e --differential                # live pi against live yapi
cargo xtask e2e --record-pi --only <text>     # rewrite goldens from pi
```

pi runs from the fixture generator's install. Run `npm ci` in `tests/fixtures/pi/generator` with Node 22.19 or newer. [tests/fixtures/pi/README.md](../../../tests/fixtures/pi/README.md) covers that install and pi's other goldens.

## Requirements and timing

- The suite needs `rg` and `fd` on `PATH` for the search tool scenarios, and `python3` for the MCP test server in `tests/fixtures/mcp`.
- TUI scenarios (`tty`) type into the program in a pseudo-terminal and compare the final screen text, without each product's startup header. `YAPI_SETTLE_MS` lengthens the quiet time that ends each step on slow machines.
- RPC scenarios (`rpc`) send each command once the previous one's response, or the event its `until` names, has arrived. A step that sets `requests` also waits until the mock has received that many requests, so an abort lands while a request is in flight.
- Client scenarios (`client`) run a Node script from the fixture generator against the program, such as `rpc-client.mjs`, pi's `RpcClient` example. `cargo test` skips them when the generator's packages are not installed.
- Goldens are recorded on Linux. A scenario whose screen shows text pi chooses by platform, such as macOS's Option key name, sets `"os": "linux"` and runs only there.

## Cassettes

A cassette is an HTTP exchange with a provider that `yapi-mock` replays, kept in `tests/fixtures/cassettes/<wire-api>/<case>.json`. Write one by hand, or record one through a proxy to a real provider:

```sh
cargo xtask mock-sse --record <upstream-url> --out tests/fixtures/cassettes/<wire-api>/<case>.json
```

Recordings redact credentials. `cargo xtask mock-sse --cassette <file>` serves a cassette to pi or another out-of-process client and reports unmatched requests on exit. [tests/fixtures/cassettes/README.md](../../../tests/fixtures/cassettes/README.md) covers the format, WebSocket exchanges and serving a cassette to pi.
