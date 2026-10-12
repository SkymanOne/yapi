# Quickstart

yapi runs in your terminal and works with the files on your machine. It needs a model, through a subscription, an API key or a local server.

## 1. Install yapi

On Linux or macOS, run the install script:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh | sh
```

It installs `yapi` in `~/.local/bin`. [Install yapi](install.md) covers the other methods. Check the installation:

```sh
yapi --version
```

## 2. Start yapi

Change to the folder you want yapi to work in, then start it:

```sh
cd ~/code/my-project
yapi
```

yapi uses the working folder to find files, project instructions and configuration, and to group saved sessions.

## 3. Choose a model

Run `/login`, choose a provider and follow the prompts to sign in with a subscription or save an API key. Run `/model` to choose another model.

yapi also reads API keys from the environment, such as `ANTHROPIC_API_KEY`. [Choose a model](models.md) lists every provider.

## 4. Give yapi a task

Type a task and press Enter. yapi shows each file it reads, each command it runs and each edit it makes. It does not ask before each tool call.

```text
Explain how requests flow through src/server.rs
Find why `cargo test` fails and fix it
@Cargo.toml Which dependencies are outdated?
!git status
```

`@path` attaches a file. `!command` runs a shell command and sends its output to the model, and `!!command` runs it without sending the output. Press Escape to stop the agent and Ctrl+C twice to quit.

Review the changes when yapi finishes. Use version control for important work, and a container for untrusted or unattended work. See [Run yapi safely](security.md).

## Continue later

yapi saves sessions automatically. To resume the most recent session in a folder, run:

```sh
yapi -c
```

Use `/resume` to choose another session. See [Manage sessions](sessions.md).

## Next steps

- [Use yapi in the terminal](interactive.md) covers keys, commands and queued messages.
- [Configure yapi](configuration.md) covers settings and project instructions in `AGENTS.md`.
- [Coming from Pi](migrating.md) shows how to copy a Pi setup with `yapi import pi`.

### Choose how to customize yapi

Start with the simplest mechanism that meets your need:

| Need | Start with |
|---|---|
| Instructions yapi follows in a project | [`AGENTS.md`](configuration.md#project-instructions) |
| A reusable prompt in the `/` menu | [Prompt template](configuration.md#skills-and-prompt-templates) |
| Task-specific instructions and files | [Skill](configuration.md#skills-and-prompt-templates) |
| Tools from an external server | [MCP server](mcp.md) |
| New tools, commands or event handlers | [Extension](extensions.md) |
| Several resources to share | [Package](packages.md) |
