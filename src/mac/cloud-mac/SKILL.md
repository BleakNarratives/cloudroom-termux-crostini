---
name: cloud-mac
description: 'Run commands and copy files on the user’s paired Mac from a Cloudroom cloud thread. Use when a task needs the user’s own computer: its files, logs, configs, repositories, or logins.'
---

# The user's Mac

- `room-cli mac run 'COMMAND'` runs it on the user's Mac as the user, in a login shell in their home folder. It prints the output and exits with the command's code. Add `--cwd DIR` to change folder, or `--stdin` to forward input.
- `room-cli mac pull MAC_PATH [VM_FOLDER]` copies a Mac file or folder here. `room-cli mac push VM_PATH [MAC_FOLDER]` copies to the Mac (default: its home folder). Limit: 2 GiB compressed. Over 16 MiB travels in pieces and can take minutes, so run big copies in the background.
- Add `--write` to any `run` that changes the Mac: editing files, installing, opening apps, or stopping processes. The user picks Mac access: Off, Read-only, Ask first, or Full. Below Full, other commands run read-only and `--write` is refused (Read-only) or waits for the user's approval (Ask first). Under Ask first, the user has up to 5 minutes to approve, so give that command a tool timeout of at least 6 minutes; stopping it sooner cancels the request. `push` always counts as a change. If a change is refused or denied, don't retry; tell the user what you need.
- Read-only commands can't use `ps` or `top`; use `pgrep -fl` or `lsof` instead.
- `Mac unavailable` means it is offline, asleep, or access is off. Continue the cloud work; tell the user only if the Mac is essential.
- `Mac not responding` means it is connected but did not pick up the command in time: asleep, very busy, or a slow link. Retry once; then treat it like `Mac unavailable`.
- `Operation not permitted` in Desktop, Documents, or Downloads means macOS privacy blocks it. Ask the user to update Cloudroom, then turn on Cloudroom in System Settings → Privacy & Security → Files and Folders. Older versions need `python3` turned on there instead.
- If the Mac disconnects mid-command, the command keeps running there. Check it with `room-cli mac result JOB`. Never re-run it automatically.
- This is the user's real computer. Read before changing. Never delete or overwrite their work unless asked.
