# Connect MCP servers

yapi connects to [Model Context Protocol](https://modelcontextprotocol.io) (MCP) servers over stdio and streamable HTTP. This page covers describing and managing servers, signing in, servers that extensions register, and codemode, which lets the model call MCP tools from a script.

## Describe servers in `mcp.json`

Describe servers in `~/.yapi/agent/mcp.json`, or in `.yapi/mcp.json` for one project. This file adds a stdio server that `npx` starts and an HTTP server with a bearer token from the environment:

```json
{
  "mcpServers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
    },
    "docs": {
      "url": "https://example.com/mcp",
      "headers": { "Authorization": "Bearer ${DOCS_TOKEN}" }
    }
  }
}
```

Pi's [MCP documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/mcp.md) describes every option.

A [project-only run](project-only.md) connects the project's servers, and of `~/.yapi/agent/mcp.json` only those that the `mcpServers` list of your `projectOnly` setting names.

## Manage servers

`yapi mcp` edits the file and checks servers without starting a session. These commands add the two servers above, list them and remove one:

```sh
yapi mcp add filesystem -- npx -y @modelcontextprotocol/server-filesystem .
yapi mcp add docs --url https://example.com/mcp \
  --bearer-token-env-var DOCS_TOKEN
yapi mcp list
yapi mcp remove docs
```

`yapi mcp list` shows each server's state, tools and errors, and exits with 1 if a server fails.

Inside yapi, `/mcp` opens a manager that lists each server with its state, tool count, exposure and configuration file. Select a server to see its tools, reconnect, sign in or out, change its exposure, or enable or disable it. yapi saves exposure and enabled changes to the `mcp.json` that defines the server. Outside interactive mode, `/mcp` prints the status, and `/mcp login`, `/mcp logout` and `/mcp reconnect` act on a server directly.

## Sign in to a server

HTTP servers without an `Authorization` header sign in with OAuth. A server that needs a sign-in shows it in `/mcp` and `yapi mcp list`. Select it in `/mcp`, or run `/mcp login <server>` inside yapi or `yapi mcp login <server>` in a terminal. `logout` deletes the stored credentials.

yapi opens the authorization page in your browser and waits for the redirect. It opens only `http` and `https` pages and fails the sign-in when a server names another scheme. If the browser runs on another machine, paste the URL it was redirected to.

yapi registers itself with servers that allow it. For other servers, set a pre-registered client in the server's `oauth` object, or pass `--oauth-client-id` and the related options to `yapi mcp add`. The `oauth` object takes `clientId`, `clientSecret`, `callbackPort` or `callbackUrl`, `scope`, `clientName` and `authServerMetadataUrl`.

yapi stores credentials in `~/.yapi/agent/mcp-auth.json`, in Pi's format, and refreshes access tokens before they expire. A running session picks up a sign-in made with `yapi mcp login` on its next turn.

To send the token of a provider you signed in to with `/login` instead, add `"auth": { "provider": "your_provider_id" }` to a server in the global `mcp.json`.

## Servers that extensions register

An extension adds a server for the session with `pi.registerMcpServer(name, config)`, where `config` takes the same fields as an `mcp.json` entry, and removes it with `pi.unregisterMcpServer(name)`. A server of the same name in `mcp.json` takes precedence. Changes to a registered server in `/mcp` last for the session only.

A stdio server needs the extension's process grant, and an HTTP server its network grant. Packages from `yapi install` have both. A chest entry needs `all` for a stdio server and `net` for an HTTP one.

As in Pi, `registerMcpServer` throws for a name that another extension registered, or one whose tool prefix would clash with it, such as `my_docs` after `my-docs`. During loading, the error fails the extension's load, and the extension that loaded first keeps the name.

## Call MCP tools through codemode

By default, yapi offers MCP tools through codemode. Instead of one tool call per step, the model writes a short JavaScript program that calls more than one tool and combines their results. Programs run in a fresh WebAssembly sandbox with no file, process or network access of their own. Three settings change what codemode covers:

- `"exposure": "direct"` on a server offers its tools to the model directly.
- `--tools read,bash,edit,write,codemode` routes the built-in tools through codemode as well.
- `"codemode": { "mode": "only" }` in `settings.json` offers the tools that scripts can call only through codemode.

The first prompt doesn't wait for servers whose tools only codemode reaches. A script that names a server's tools, such as `tools.mcp__docs__search`, waits until that server has connected or failed to connect. A script that searches, lists or describes tools waits for every server that's still connecting. Aborting the turn ends the wait.

A package that registers its own `codemode` tool replaces the built-in one, and yapi warns about it at startup.

Scripts can also use the `models` global to list the catalog and run classifier and image models with the session's credentials. Pi's [codemode documentation](https://github.com/earendil-works/pi/blob/v1.0.0/packages/coding-agent/docs/codemode.md) describes the scripting API.
