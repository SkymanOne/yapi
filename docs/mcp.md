# Connect MCP servers

yapi connects to [Model Context Protocol](https://modelcontextprotocol.io) servers over stdio and streamable HTTP. Describe them in `~/.yapi/agent/mcp.json`, or in `.yapi/mcp.json` for one project:

```json
{
  "mcpServers": {
    "filesystem": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "."] },
    "docs": { "url": "https://example.com/mcp", "headers": { "Authorization": "Bearer ${DOCS_TOKEN}" } }
  }
}
```

Pi's [MCP documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/mcp.md) describes every option.

## Manage servers

`yapi mcp` edits the file and checks servers without starting a session:

```sh
yapi mcp add filesystem -- npx -y @modelcontextprotocol/server-filesystem .
yapi mcp add docs --url https://example.com/mcp --bearer-token-env-var DOCS_TOKEN
yapi mcp list                 # state, tools and errors, exits 1 if a server fails
yapi mcp remove docs
```

Inside yapi, `/mcp` opens a manager that lists each server with its state, tool count, exposure and configuration file. Select a server to see its tools, reconnect, sign in or out, change its exposure, or enable or disable it. Exposure and enabled changes are saved to the `mcp.json` that defines the server. Outside interactive mode, `/mcp` prints the status, and `/mcp login`, `/mcp logout` and `/mcp reconnect` act on a server directly.

## Sign in

HTTP servers without an `Authorization` header sign in with OAuth. A server that needs a sign-in shows it in `/mcp` and `yapi mcp list`. Select it in `/mcp`, or run `/mcp login <server>` inside yapi or `yapi mcp login <server>` in a terminal. `logout` deletes the stored credentials.

yapi opens the authorization page in your browser and waits for the redirect. If the browser runs on another machine, paste the URL it was redirected to. yapi registers itself with servers that allow it. For other servers, set a pre-registered client in the server's `oauth` object (`clientId`, `clientSecret`, `callbackPort` or `callbackUrl`, `scope`, `clientName`, `authServerMetadataUrl`), or pass `--oauth-client-id` and the related options to `yapi mcp add`.

yapi stores credentials in `~/.yapi/agent/mcp-auth.json`, in Pi's format, and refreshes access tokens before they expire. A running session picks up a sign-in made with `yapi mcp login` on its next turn.

To send the token of a provider you signed in to with `/login` instead, add `"auth": { "provider": "<provider>" }` to a server in the global `mcp.json`.

## Servers from extensions

An extension adds a server for the session with `pi.registerMcpServer(name, config)`, where `config` takes the same fields as an `mcp.json` entry, and removes it with `pi.unregisterMcpServer(name)`. A server of the same name in `mcp.json` takes precedence. Changes to a registered server in `/mcp` last for the session only.

A stdio server needs the extension's process grant, and an HTTP server its network grant. Every package has both by default. As in Pi, `registerMcpServer` throws for a name that another extension registered, or one whose tool prefix would clash with it, such as `my_docs` after `my-docs`. During loading, the error fails the extension's load, and the extension that loaded first keeps the name.

## Codemode

By default, yapi offers MCP tools through codemode. Instead of one tool call per step, the model writes a short JavaScript program that calls several tools and combines their results. Programs run in a fresh WebAssembly sandbox with no file, process or network access of their own.

- `"exposure": "direct"` on a server offers its tools to the model directly.
- `--tools read,bash,edit,write,codemode` routes the built-in tools through codemode as well.
- `"codemode": { "mode": "only" }` in `settings.json` offers the tools that scripts can call only through codemode.

A package that registers its own `codemode` tool replaces the built-in one, and yapi warns about it at startup.

Scripts can also use the `models` global to list the catalog and run classifier and image models with the session's credentials. Pi's [codemode documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/codemode.md) describes the scripting API.
