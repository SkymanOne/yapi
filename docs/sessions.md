# Manage sessions

yapi saves every conversation as a JSONL file under `~/.yapi/agent/sessions/`, grouped by project. The format is Pi's, so Pi and yapi open each other's sessions.

## Continue work

```sh
yapi -c                      # continue the last session in this folder
yapi -r                      # pick a session to resume
yapi --session 3f2a          # open a session by file or id prefix
yapi --fork 3f2a             # copy a session into a new one
yapi --no-session            # do not save this run
```

Inside yapi, `/resume` opens the session picker, `/new` starts a fresh session and `/name` names the current one.

## Branch a session

A session is a tree. When you go back to an earlier message, the abandoned branch is kept, so no work is lost.

- `/tree` shows every branch and jumps to any earlier point. By default, pressing Escape twice on an empty editor opens it too. The `doubleEscapeAction` setting changes this to `fork` or `none`.
- `/fork` starts a new session from an earlier message.
- `/clone` duplicates the current session.

## Compact context

Long sessions fill the model's context window. yapi summarizes older messages automatically before that happens. `/compact` runs it on demand, and `/settings` turns automatic compaction off.

## Keep the prompt cache warm

Providers such as Anthropic cache a request's prompt for a few minutes. While the agent works, yapi sends the latest request again with a one-token reply shortly before its cache entry expires, when that is expected to save at least $0.05. Each refresh and its cost are recorded in the session. The `cacheWarming` setting is `streaming` by default. `idle` also keeps the cache warm between runs, and `off` stops warming. Only the global settings set it, because each refresh costs money. `/session` shows the next decision and its costs, and the prompt tokens billed again after cache misses. Turn on "Cache miss notices" in `/settings` to see refreshes, costly cache misses, summary costs and the thinking blocks Anthropic dropped from a request in the transcript.

## Export and import

`/export` writes the session to a standalone HTML file. `yapi --export <file>.jsonl [output.html]` does the same for a session file from the command line. `/export <path>.jsonl` writes the current branch as a new session file, without the other branches.

`/import <path>.jsonl` copies a session file into the project's session folder and resumes it.
