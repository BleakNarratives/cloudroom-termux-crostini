#!/usr/bin/env python3
"""
[DNA_TAG]
ORIGIN: Moto4
PILLAR: dynamic_auto_tag
PATH: PORTABLE_TERMUX_CROSTINI.md
ROLE: Auto-tagged by Code Sentinel on directory traversal.
LAST_SYNC: 2026-08-01T19:15:00
[/DNA_TAG]
"""

# Cloudroom Core — BleakNarratives Termux (Android) & ChromeOS Crostini Edition

> **First native rootless Cloudroom Core build for Android Termux (`aarch64-linux-android`) & ChromeOS Crostini (`x86_64` / `aarch64`).**  
> **No Ubuntu VM. No `proot`. No `systemd`. No `sudo` / `useradd`.**  
> Powered by the **Whorl / MOLT 27-Pillar ACP Substrate**, **Vigil + Tribunal + TruthSleuth Anti-Hallucination Gate**, and the built-in **Scrying Cockpit & Diff Theater (`/cockpit`)**.

---

## 1. What Was Solved (Code Judo / Constraint Arbitrage)

Upstream `davidondrej/cloudroom-core` assumes a dedicated Ubuntu 24.04 VM with root, `systemd`, `cgroup v2`, `useradd` (`cloudroom` + `cloudroom-agent`), `/code` at filesystem root, and `/tmp` bind-mounts.

This build flips those constraints into native rootless primitives:

1. **Android Bionic (`target_os = "android"`) + Linux Parity**:
   - Expanded all `#[cfg(target_os = "linux")]` gates across `src/workspace/storage.rs`, `src/runtime/linux.rs`, `src/runtime/terminal.rs`, `src/runtime/files.rs`, `src/preview/mod.rs`, and `src/observability/system.rs` to `any(target_os = "linux", target_os = "android")`.
   - Fixed Bionic `ioctl` request width (`type IoctlRequest = i32`) and PTY constants (`TIOCSCTTY`, `TIOCSWINSZ`, `O_NOCTTY`, `O_NONBLOCK`) in `src/runtime/terminal.rs`.
   - Added `O_PATH` traversal fallback in `src/sync/files.py` (`directory()`) so walking `/data/data/com.termux/files/...` survives Android's `0771` (`--x` execute-only, non-readable) permissions on `/data` and `/data/data` while keeping `O_NOFOLLOW | O_DIRECTORY` symlink protection.
2. **Rootless Single-User Storage Guard (`rootless: true` & `code_dir`)**:
   - Added `rootless: bool` and `code_dir: Option<PathBuf>` to `workspace::storage::Policy`.
   - Keeps real `statvfs` disk-capacity monitoring and `CLOUDROOM_WORKLOAD=<id>` process-group containment active without requiring a separate `cloudroom-agent` UID, `setuid`/`setgid`, or `/code` at `/`.
   - Automatically provisions `$STATE_DIR/tmp` on the monitored disk so Crostini/Linux machines where `/tmp` is `tmpfs` pass `check_filesystems` without root `unshare --mount`.
3. **Termux + Low-RAM Linker Profile (`.cargo/config.toml`)**:
   - Uses `mold` (`-C link-arg=-fuse-ld=mold`) on `aarch64-linux-android` plus `codegen-units = 1`, `opt-level = "z"`, `lto = "thin"`, `strip = true`, `debug = false` — producing a **6.9 MB** self-contained binary without triggering Android or Crostini 2.6 GB RAM OOM kills.
4. **Built-in Scrying Cockpit & Diff Theater (`GET /cockpit`)**:
   - Embedded directly in the `cloudroom` binary (`src/cockpit.html` served at `/` and `/cockpit`).
   - Includes a touch/drag **Scrying Ring + Virtual Thumbstick**, **Gyroscopic Tilt (`DeviceOrientationEvent`)** navigation for phones/tablets, **☯ Zen / 🔥 BLEGH Metalcore Diff Theater** (with 42 Hz WebAudio thall synth hits), and **👄 MouthTape** formant speech synthesis.
5. **Whorl / MOLT 27-Pillar ACP Substrate (`install/whorl-acp.py`)**:
   - Implements Cloudroom's JSON-RPC 2.0 Agent Client Protocol (ACP) and mounts 27 local ecosystem pillars (`scout-empire` 40 scouts, `axescout`, `market-scout`, `code-city`, `meridian`, `the-werkz`, `valet-vault`, `lob-bin`, `whorl-molt`, `boardroom`, `askhole-hotseat`, `lookingglass`, `thoth`, `zeroclaw`, `merkaba`, `akasha`, `kindred`, `mikeyswarm`, `pytch`, `agent-twoie`, `agentic-almanac`, `vigil`, `tribunal`, `truthsleuth`, `drift-guard`, and the hidden `.grimoire` / `JaneBox` sanctum).
   - Every response is audited by the **Vigil + Tribunal + TruthSleuth Gate** to flag stub/spaghetti markers.

---

## 2. Quickstart (Crostini & Android Termux)

```bash
./install/portable-start.sh
```

That single script:
1. Builds `target/release/cloudroom` (if not already built) using the low-RAM profile.
2. Initializes `~/.cloudroom/{storage.json,token,history,cache,code}` with `0700`/`0600` permissions.
3. Installs the Whorl ACP substrate into `~/.local/bin/hermes` (backed by `~/.hermes/state.db`).
4. Starts `cloudroom` on `127.0.0.1:9840` and verifies `/v1/health` and `/v1/ready`.
5. Prints your authenticated **Scrying Cockpit URL**:
   `http://127.0.0.1:9840/cockpit?token=<YOUR_TOKEN>`

### Slash Commands in Any Session
- `/fleet` — List all 27 mounted BleakNarratives ecosystem pillars
- `/scouts list` / `/scouts report` — Run the 40-Scout Empire CLI & portfolio report
- `/lookingglass <intent>` — Run a seeded Lookingglass recursive chain & eventuality mirror
- `/zeroclaw` — Verify the native Android/Termux ZeroClaw runtime
- `/grimoire` or `/janebox` — Unseal the hidden `.grimoire` & `JaneBox` Easter-Egg sanctum
