# Run your first task

This guide installs yapi, connects a model and gives yapi its first task in one of your projects. You reach a model through a subscription, an API key or a local server.

## Install yapi

On Linux or macOS, run the install script:

```sh
curl -fsSL https://raw.githubusercontent.com/SkymanOne/yapi/main/install.sh \
  | sh
```

The script asks what to install. Press Enter to install `yapi` and its package manager, `chest`, in `~/.local/bin`. [Install yapi](install.md) covers the other methods. Check the installation:

```sh
yapi --version
```

## Start yapi in a project

Start yapi in the folder it should work in:

```sh
cd ~/code/my-project
yapi
```

yapi uses the working folder to find files, project instructions and configuration, and to group saved sessions.

## Choose a model

Run `/login`, choose a provider and follow the prompts to sign in with a subscription or save an API key. Run `/model` to choose another model.

yapi also reads API keys from the environment, such as `ANTHROPIC_API_KEY`. [Choose a model](models.md) lists every provider.

## Give yapi a task

Type a task and press Enter. yapi shows each file it reads, each command it runs and each edit it makes. It doesn't ask before each tool call. For example:

```text
Explain how requests flow through src/server.rs
Find why `cargo test` fails and fix it
@Cargo.toml Which dependencies are outdated?
!git status
```

`@path` attaches a file. `!command` runs a shell command and sends its output to the model, and `!!command` runs it without sending the output. Press Escape to stop the agent and Ctrl+C twice to quit.

Review the changes when yapi finishes. Keep important work under version control, and use a container for untrusted or unattended work, as [Security model and sandboxing](security.md) explains.

## Continue a session later

yapi saves sessions automatically. To resume the most recent session in a folder, run:

```sh
yapi -c
```

Use `/resume` to choose another session. [Manage sessions](sessions.md) covers the rest.

## Go further with yapi

These pages pick up where this guide ends:

- [Use yapi in the terminal](interactive.md) covers keys, commands and queued messages.
- [Configure yapi](configuration.md) covers settings and project instructions in `AGENTS.md`.
- [Move from Pi to yapi](migrating.md) shows how to copy a Pi setup with `yapi import pi`.
- [Get started with chest](chest.md) shows how to pin a project's skills, subagents and extensions, so everyone who clones the project gets the same set.

### Choose how to customize yapi

The table pairs each need with the mechanism to start with:

| Need | Start with |
|---|---|
| Instructions yapi follows in a project | [`AGENTS.md`](configuration.md#project-instructions) |
| A reusable prompt in the `/` menu | [Prompt template](configuration.md#skills-and-prompt-templates) |
| Task-specific instructions and files | [Skill](configuration.md#skills-and-prompt-templates) |
| Tools from an external server | [Model Context Protocol (MCP) server](mcp.md) |
| New tools, commands or event handlers | [Extension](extensions.md) |
| More than one resource to share | [Package](packages.md) |
| The same skills, subagents and extensions for everyone on a project | [chest](chest.md) |
