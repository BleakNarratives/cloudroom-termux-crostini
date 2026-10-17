---
name: room-cli
description: 'Inspect and control the user’s Cloudroom threads, projects, and settings from a cloud thread. Use for Cloudroom CLI tasks, not official BB.'
---

# Cloudroom from the cloud

- Here, `room-cli thread update --self --title TITLE` and `room-cli thread archive --self` rename or archive this thread. `room-cli thread update --self --reasoning-level LEVEL` changes this thread's effort from its next turn; its model stays fixed.
- Start a subagent with `room-cli thread spawn --provider codex|claude-code|pi|opencode|hermes|devin [--model MODEL] [--reasoning-level LEVEL] [--title TITLE] [--shared-sandbox] --prompt TEXT`. It gets its own sandbox with a copy of your code (commits and uncommitted changes) on its own branch, and shows under this thread. It doesn't see your later edits, so share work through Git. Add `--shared-sandbox` to run it in this sandbox and folder instead. Pi and OpenCode models are `provider/model`, e.g. `openrouter/deepseek/deepseek-v4-pro` or `opencode-go/kimi-k3`. Hermes models are `provider:model`, e.g. `openrouter:anthropic/claude-haiku-5.5`. Devin models are Devin's own names, e.g. `claude-opus-5-5-medium` or `adaptive`. Cloudroom messages you each time it finishes, so keep working. `room-cli thread list`, `output CHILD_ID`, and `tell CHILD_ID TEXT` check on and steer your children; `stop CHILD_ID` stops one and `archive CHILD_ID` stops and archives it; `thread list` hides archived children unless you add `--include-archived`. Never start threads through Mac access.
- Schedule work for this thread with `room-cli automation create --name NAME --prompt TEXT (--cron 'MIN HOUR DAY MONTH WEEKDAY' [--timezone ZONE] | --at ISO_TIME | --in 30m)`. Cloudroom re-prompts this thread then, even while the user's Mac is off. `room-cli automation list` shows the next run or an error; `pause`, `resume`, `run`, `update`, and `delete` take its ID. Never use a shell loop or `sleep` for this.
- `room-cli` here covers this thread, its children, and automations. Everything else (other threads, projects, settings, plugins) lives in the Cloudroom app on the user’s Mac, whose `room-cli` has every command. Run it there through Mac access (see the `cloud-mac` skill):

  ```sh
  room-cli mac run 'ELECTRON_RUN_AS_NODE=1 /Applications/Cloudroom.app/Contents/MacOS/Cloudroom /Applications/Cloudroom.app/Contents/Resources/app.asar.unpacked/node_modules/bb-app/host-daemon/dist/room-cli thread list --json'
  ```

- On the Mac, start with `room-cli --help` or `room-cli guide`. Prefer `--json`.
- `Mac unavailable` means the Mac is offline or Mac access is off. Tell the user.
- Never inspect, message, or launch other threads unless the user asks.
