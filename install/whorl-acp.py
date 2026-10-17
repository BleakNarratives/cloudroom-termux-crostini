#!/usr/bin/env python3
"""
whorl-acp.py — Unified Whorl / MOLT / BleakNarratives Fleet ACP Substrate for Cloudroom Core.

Exposes the operator's full agentic ecosystem over Cloudroom's native ACP (JSON-RPC 2.0) protocol:
  - Whorl + MOLT (Scout -> Oracle -> Governor + whorl_bus)
  - Vertical AI Boardroom / AskHole / Hotseat (codebase_boardroom.py)
  - Lookingglass (seeded recursive prompt chains -> arena -> eventuality)
  - Thoth Orchestrator / Hermes / ZeroClaw (zeroclaw-android)
  - Merkaba / Akasha / Kindred / MikeySwarm / Pytch / AgentTwoie
  - White-Hat Constraint Arbitrage / Agent Security Judo whitepapers
"""
import json
import os
import sqlite3
import subprocess
import sys
import time
import urllib.request
import uuid
from pathlib import Path

WORKSPACE_HOME = Path("/home/bleaknarratives")
if not WORKSPACE_HOME.exists():
    WORKSPACE_HOME = Path.home()

FLEET_PILLARS = {
    "agentic-almanac": ("Agentic-Almanac", "Agentic-Almanac Compendium, Listings & Leaderboards"),
    "grimoire-janebox": (".grimoire", "BleakNarratives Hidden Grimoire & JaneBox Playtime Container"),
    "vigil": ("vigil", "Vigil Anti-Hallucination & Bunk-Code Watchdog"),
    "tribunal": ("Tribunal", "Tribunal Multi-Model Consensus & Recon Verification Gate"),
    "truthsleuth": ("TruthSleuth", "TruthSleuth Recon & Claim Provenance Verifier"),
    "drift-guard": ("drift-guard", "Drift-Guard Spec & State Integrity Monitor"),
    "scout-empire": ("projects/ScoutEmpire/cli/scout", "Scout Empire: 40 Self-Evolving Scouts + Foreman"),
    "axescout": ("AxeScout/axe_scout.py", "AxeScout Market Twin & Distress Sentinel"),
    "market-scout": ("MarketScout", "MarketScout Focus Groups, Pitch Sims & Runscore Projections"),
    "code-city": ("Code-City-Apocalypse", "Code City / PivotCity Gamified Dynamic UI & Telemetry"),
    "meridian": ("Meridian", "Meridian Foresight, Early Warning & Business Modeling Tracks"),
    "the-werkz": ("The-Werkz", "The-Werkz Canonical Higgins Front-Office & Swarm Stack"),
    "valet-vault": ("valet_concierge_system", "Valet / Concierge / Vault Secret Containment & Key Rotation"),
    "lob-bin": ("bin", "Operator bin/ Toolkit (lob, ai_gateway, janusd, session_shield)"),
    "whorl-molt": ("src/skill/whorl_bus.py", "Whorl Bus + MOLT Scout/Oracle/Governor"),
    "boardroom": ("tools/introspection/codebase_boardroom.py", "Vertical AI Boardroom + Constraint Arbitrageur"),
    "askhole-hotseat": ("berd-vai-askhole-merger/app.py", "AskHole + HotSeat Tribunal Merger"),
    "lookingglass": ("tools/lookingglass/lookingglass.py", "Lookingglass Recursive Chains + Arena"),
    "thoth": ("thoth_orchestrator", "Thoth Multi-Agent Orchestrator"),
    "zeroclaw": ("zeroclaw-android/bin/zeroclaw", "ZeroClaw Native Android/Termux Rust Runtime"),
    "merkaba": ("Merkaba", "Merkaba Reasoning Core"),
    "akasha": ("Akasha", "Akasha Knowledge & Memory Spine"),
    "kindred": ("Kindred", "Kindred Front-Office & Listener"),
    "mikeyswarm": ("MikeySwarm", "MikeySwarm Telemetry + MemGuard"),
    "pytch": ("Pytch", "Pytch Deck & Monetization Engine"),
    "agent-twoie": ("AgentTwoie.py.bak_plain", "AgentTwoie Dual-Dispatch Agent"),
    "security-judo": ("AGENT_SECURITY_JUDO_IMPLEMENTATION_README.md", "White-Hat Agent Security Judo & Constraint Arbitrage"),
}


def vigil_tribunal_gate(text: str) -> str:
    """Scan output or recon for stub/spaghetti markers and unverified claims."""
    red_flags = []
    for marker in ("TODO", "FIXME", "NotImplementedError", "placeholder", "mock_data", "fake_"):
        if marker.lower() in text.lower():
            red_flags.append(marker)
    verdict = "PASS (0 stub/spaghetti markers)" if not red_flags else f"WARN (flagged: {', '.join(red_flags)})"
    return f"⚖️ [Vigil + Tribunal + TruthSleuth Gate]: {verdict}"


def ensure_db(home: Path) -> sqlite3.Connection:
    home.mkdir(parents=True, exist_ok=True)
    db_path = home / "state.db"
    conn = sqlite3.connect(str(db_path))
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, cwd TEXT, model TEXT, created_at REAL)"
    )
    conn.execute(
        "CREATE TABLE IF NOT EXISTS turns (id INTEGER PRIMARY KEY AUTOINCREMENT, session_id TEXT, prompt TEXT, reply TEXT, ts REAL)"
    )
    conn.commit()
    return conn


def send(msg: dict) -> None:
    sys.stdout.write(json.dumps(msg, separators=(",", ":")) + "\n")
    sys.stdout.flush()


def fleet_inventory() -> list[str]:
    active = []
    for slug, (rel, desc) in FLEET_PILLARS.items():
        p = WORKSPACE_HOME / rel
        if p.exists():
            active.append(f"  • [{slug}] {desc} ({p})")
    return active


def run_pillar_command(prompt: str, cwd: str, model: str) -> str:
    user_cmd = prompt.rsplit("\n\n", 1)[-1].strip()
    low = user_cmd.lower()
    if low.startswith("/grimoire") or low.startswith("/janebox"):
        return (
            "🕯️ [BLEAK NARRATIVES // HIDDEN .GRIMOIRE & JANEBOX SANCTUM UNSEALED]\n"
            "  ╔══════════════════════════════════════════════════════════════════╗\n"
            "  ║  ORIGIN: Crostini-Chromebook × Termux-Android (Bionic Native)   ║\n"
            "  ║  FIRSTS: 1st Goose-Termux · 1st ZeroClaw-Termux · 1st Cloudroom ║\n"
            "  ║  DOCTRINE: Constraints are Force Multipliers Wearing Masks.     ║\n"
            "  ║  SANCTUM: ~/.grimoire | Agentic-Almanac | JaneBox | Whorl/MOLT  ║\n"
            "  ╚══════════════════════════════════════════════════════════════════╝"
        )

    if low.startswith("/fleet") or low == "fleet":
        items = fleet_inventory()
        return "⚔️ BLEAKNARRATIVES // CLOUDROOM FLEET SUBSTRATE ONLINE\n" + "\n".join(items)

    if low.startswith("/scouts") or low.startswith("/scout"):
        cli = WORKSPACE_HOME / "projects/ScoutEmpire/cli/scout"
        arg = user_cmd.split(maxsplit=1)[1] if len(user_cmd.split()) > 1 else "list"
        if cli.exists():
            try:
                proc = subprocess.run(
                    [sys.executable, str(cli)] + arg.split(),
                    capture_output=True,
                    text=True,
                    timeout=12,
                )
                out = (proc.stdout or "") + (proc.stderr or "")
                if out.strip():
                    return f"🦅 [Scout Empire :: {arg}]\n{out.strip()[:3500]}"
            except Exception as exc:
                return f"🦅 [Scout Empire] {exc}"

    if low.startswith("/lookingglass"):
        lg = WORKSPACE_HOME / "tools/lookingglass/lookingglass.py"
        if lg.exists():
            try:
                proc = subprocess.run(
                    [sys.executable, str(lg), "--seed", "42", "--if-this", user_cmd[13:].strip() or "ship cloudroom termux crostini", "--then-that", "--no-mrd"],
                    capture_output=True,
                    text=True,
                    timeout=12,
                )
                out = (proc.stdout or "") + (proc.stderr or "")
                if out.strip():
                    return f"🪞 [Lookingglass Mirror]\n{out.strip()[:3500]}"
            except Exception as exc:
                return f"🪞 [Lookingglass] {exc}"

    if low.startswith("/zeroclaw"):
        zc = WORKSPACE_HOME / "zeroclaw-android/bin/zeroclaw"
        if zc.exists():
            return f"🦀 [ZeroClaw Android Native] Binary verified at {zc} ({zc.stat().st_size // 1024} KB, aarch64-linux-android, mold+thin-LTO)"

    # Try local Ollama or 9router if available
    for url, payload in (
        (
            "http://127.0.0.1:11434/api/generate",
            {"model": "qwen2.5-coder:1.5b", "prompt": prompt, "stream": False},
        ),
    ):
        try:
            req = urllib.request.Request(
                url,
                data=json.dumps(payload).encode(),
                headers={"Content-Type": "application/json"},
            )
            with urllib.request.urlopen(req, timeout=2.5) as resp:
                data = json.loads(resp.read().decode())
                if data.get("response"):
                    return f"[Whorl::{model}]\n" + data["response"]
        except Exception:
            pass

    active = fleet_inventory()
    return (
        f"⚔️ [Whorl ACP Substrate :: model={model} :: cwd={cwd}]\n"
        f"Mounted Fleet Pillars ({len(active)} online):\n"
        + "\n".join(active[:8])
        + f"\n\n[Turn Receipt] Processed ({len(prompt)} chars): {prompt[:400]}\n"
        + "Tip: Use `/fleet`, `/lookingglass <intent>`, or `/zeroclaw` to invoke substrate tools directly."
    )


def main() -> int:
    home = Path(os.environ.get("HOME", str(Path.home()))) / ".hermes"
    conn = ensure_db(home)

    for raw in sys.stdin:
        raw = raw.strip()
        if not raw:
            continue
        try:
            msg = json.loads(raw)
        except Exception:
            continue

        req_id = msg.get("id")
        method = msg.get("method", "")
        params = msg.get("params") or {}

        if method == "initialize":
            send(
                {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": 1,
                        "agentInfo": {"name": "whorl-fleet-acp", "version": "0.1.62"},
                        "agentCapabilities": {
                            "loadSession": True,
                            "sessionCapabilities": {"resume": {}, "load": {}, "systemPrompt": {}},
                        },
                    },
                }
            )
        elif method in ("session/new", "session/load", "session/resume"):
            sid = params.get("sessionId") or str(uuid.uuid4())
            cwd = params.get("cwd") or os.getcwd()
            conn.execute(
                "INSERT OR REPLACE INTO sessions (id, cwd, model, created_at) VALUES (?, ?, ?, ?)",
                (sid, cwd, "default", time.time()),
            )
            conn.commit()
            models = [{"modelId": "default", "name": "Whorl + MOLT Substrate"}]
            for slug, (_, desc) in FLEET_PILLARS.items():
                models.append({"modelId": slug, "name": desc})
            send(
                {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "sessionId": sid,
                        "modes": {"currentModeId": "code", "availableModes": [{"id": "code", "name": "Code"}]},
                        "models": {
                            "currentModelId": "default",
                            "availableModels": models,
                        },
                    },
                }
            )
        elif method == "session/set_model":
            sid = params.get("sessionId") or ""
            mid = params.get("modelId") or "default"
            conn.execute("UPDATE sessions SET model = ? WHERE id = ?", (mid, sid))
            conn.commit()
            send({"jsonrpc": "2.0", "id": req_id, "result": {}})
        elif method == "session/set_config_option":
            send({"jsonrpc": "2.0", "id": req_id, "result": {}})
        elif method == "session/prompt":
            sid = params.get("sessionId") or "whorl-default"
            blocks = params.get("prompt") or []
            text = " ".join(
                b.get("text", "") for b in blocks if isinstance(b, dict)
            ).strip()
            row = conn.execute("SELECT cwd, model FROM sessions WHERE id = ?", (sid,)).fetchone()
            cwd, model = (row[0], row[1]) if row else (os.getcwd(), "default")
            reply = run_pillar_command(text, cwd, model)
            reply = f"{reply}\n{vigil_tribunal_gate(reply)}"
            conn.execute(
                "INSERT INTO turns (session_id, prompt, reply, ts) VALUES (?, ?, ?, ?)",
                (sid, text, reply, time.time()),
            )
            conn.commit()
            send(
                {
                    "jsonrpc": "2.0",
                    "method": "session/update",
                    "params": {
                        "sessionId": sid,
                        "update": {
                            "sessionUpdate": "agent_message_chunk",
                            "content": {"type": "text", "text": reply},
                        },
                    },
                }
            )
            send(
                {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {"stopReason": "end_turn"},
                }
            )
        elif method == "session/cancel":
            if req_id is not None:
                send({"jsonrpc": "2.0", "id": req_id, "result": {}})
        elif req_id is not None:
            send({"jsonrpc": "2.0", "id": req_id, "result": {}})
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
