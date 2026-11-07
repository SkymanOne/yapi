# Manage sessions

yapi saves every conversation as a JSON Lines (JSONL) file under `~/.yapi/agent/sessions/`, grouped by project. Pi and yapi open each other's sessions. This page covers resuming, branching, compacting, keeping the prompt cache warm, and exporting and importing sessions.

## Continue earlier work

These flags choose which session a run uses, or whether it saves one:

```sh
yapi -c                      # continue the last session in this folder
yapi -r                      # pick a session to resume
yapi --session 3f2a          # open a session by file or id prefix
yapi --fork 3f2a             # copy a session into a new one
yapi --no-session            # don't save this run
```

Inside yapi, `/resume` opens the session picker, `/new` starts a fresh session and `/name` names the current one.

## Branch a session

A session is a tree. When you go back to an earlier message, yapi keeps the abandoned branch. Three commands work with branches:

- `/tree` shows every branch and jumps to any earlier point. By default, pressing Escape twice on an empty editor opens it too. The `doubleEscapeAction` setting changes this to `fork` or `none`.
- `/fork` starts a new session from an earlier message.
- `/clone` duplicates the current session.

## Compact a long session

Long sessions fill the model's context window. yapi summarizes older messages automatically before that happens. `/compact` runs it on demand, and `/settings` turns automatic compaction off.

## Keep the prompt cache warm

Providers keep a cached prompt for a limited time, such as 5 minutes with Anthropic's default retention. yapi can resend the latest request shortly before the cache expires, when it expects that to save money. The `cacheWarming` setting, read from global settings only, picks when:

- **`streaming`**: the default, warms while the agent works
- **`idle`**: also warms between runs
- **`off`**: never warms

`/session` shows the next decision and the tokens billed again after cache misses. "Cache miss notices" in `/settings` shows refreshes and costly misses in the transcript.

## Export and import sessions

`/export` writes the session to a standalone HTML file. `yapi --export <file>.jsonl [output.html]` does the same for a session file from the command line. `/export <path>.jsonl` writes the current branch as a new session file, without the other branches.

`/import <path>.jsonl` copies a session file into the project's session folder and resumes it. When the session's working folder no longer exists, yapi asks whether to resume it in the current folder instead.
