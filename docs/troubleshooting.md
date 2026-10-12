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

`yapi -ne` starts without extensions. Then turn the failing one off with `yapi config`.

## Reporting a problem

Run `/debug` to write the screen, the session's messages and recent extension output to `~/.yapi/agent/yapi-debug.log`. Check it for private data, then attach it to a report on the [issue tracker](https://github.com/SkymanOne/yapi/issues) with the output of `yapi --version` and your operating system.
