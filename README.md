#!/usr/bin/env python3
"""
[DNA_TAG]
ORIGIN: Moto4
PILLAR: dynamic_auto_tag
PATH: README.md
ROLE: Auto-tagged by Code Sentinel on directory traversal.
LAST_SYNC: 2026-08-01T19:15:00
[/DNA_TAG]
"""

# Cloudroom Core — BleakNarratives Android Termux & ChromeOS Crostini Edition

> **New in this fork:** [PORTABLE_TERMUX_CROSTINI.md](PORTABLE_TERMUX_CROSTINI.md) — First rootless native build for **Android Termux (`aarch64-linux-android`)** and **ChromeOS Crostini (`x86_64` / `aarch64`)** with zero Ubuntu VM, zero `proot`, zero `systemd`, built-in **Scrying Cockpit & Diff Theater (`/cockpit`)**, **Whorl / MOLT 27-Pillar ACP Substrate (`install/whorl-acp.py`)**, **Vigil + Tribunal + TruthSleuth Gate**, and the hidden **`.grimoire` / `JaneBox`** sanctum (`./install/portable-start.sh`).

## Core Technical Pillars

**B**uilding the substrate with ACP JSON-RPC 2.0.
**A**utonomous agents managed by rootless Whorl modules.
**R**ust-native performance for Android Bionic/Crostini.
**D**iff Theater providing real-time cockpit feedback.
**I**solated user-space storage via single-user guard.
**L**ow-RAM linker profiles for stable OOM-free execution.
**D**ecentralized history tracking, locally encrypted.
**O**rchestrated Vigil/Tribunal/TruthSleuth Anti-Hallucination Gate.

[Website](https://www.cloudroom.dev) · [Desktop app](https://github.com/davidondrej/cloudroom-gui) · [Changelog](https://www.cloudroom.dev/changelog) · [Security](https://www.cloudroom.dev/security)

Your agents get their own room in the cloud.

Cloudroom core runs coding agents on your own Linux machine and saves their chat history. It powers the cloud side of the [Cloudroom app](https://github.com/davidondrej/cloudroom-gui), which runs Claude Code, Codex, and Pi on your Mac or in the cloud.

- Works with Claude Code, Codex, Pi, OpenCode, Hermes, Cursor, and Devin.
- Agents use your own logins to call their models.
- One Rust service per machine. No Cloudroom account needed.

Don't want to host it yourself? Hosted Cloudroom is invite-only for now. [Join the waitlist](https://www.cloudroom.dev/#waitlist).

![Cloudroom architecture: one cloud sandbox per agent, each running Cloudroom core and the agent.](docs/architecture.png)

## Get started

- [Install on Ubuntu 24.04](docs/setup.md) with one script, on a VPS, a home server, or a spare laptop. No database needed.
- [Reach it from your laptop](docs/setup.md#reach-it-from-your-other-devices) with Tailscale, and [upgrade](docs/setup.md#upgrade) in place.
- [HTTP API](docs/api.md): every endpoint, event, and error code.
- [Linux builds](https://github.com/davidondrej/cloudroom-core/releases): tested binaries for each release.

## Security

- Agents run as a separate Linux user, with no admin rights and no `sudo`.
- Agents can't read the core's token or history files.
- Every API request needs a secret token. The API listens only on localhost unless you put it behind HTTPS.
- History stays on the machine's own disk. [Back it up](docs/setup.md#3-saved-history) like any other folder.
- Command Guard blocks a few disastrous commands, like wiping a home folder.

Agents run without approval prompts, so give each trusted user their own machine.
Report vulnerabilities privately: see [SECURITY.md](SECURITY.md).

## Desktop app

[Download the Cloudroom app](https://github.com/davidondrej/cloudroom-gui/releases) for Apple Silicon Macs (Linux alpha). [Connect it to your own core](docs/setup.md#connect-the-desktop-app) with a URL and token, no account needed. The core also works without it.

## Build and test

Needs Rust 1.89+, Git, and the agent CLIs you want to run.

```sh
cargo build --locked --release
cargo test --locked --all-targets
```

More docs: [storage](docs/storage.md) · [harnesses](docs/harnesses.md) · [session lifecycle](docs/session-lifecycle.md) · [diagnostics](docs/observability.md) · [dashboard API](docs/dashboard.md)

## Contributing

Issues and pull requests are welcome. For big changes, open an issue first.
This is pre-release software. Licensed under [Apache 2.0](LICENSE).
