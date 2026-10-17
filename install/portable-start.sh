#!/usr/bin/env bash
# portable-start.sh — Rootless, zero-proot, zero-Ubuntu launcher for Cloudroom Core
# Targets:
#   1. Android Termux (aarch64-linux-android, Bionic, no root, no systemd, no /code)
#   2. ChromeOS Crostini (x86_64/aarch64 Debian container, unprivileged user mode)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
STATE_ROOT="${CLOUDROOM_PORTABLE_DIR:-$HOME/.cloudroom}"
LISTEN_ADDR="${CLOUDROOM_LISTEN:-127.0.0.1:9840}"
BINARY="$REPO_ROOT/target/release/cloudroom"

is_termux=0
if [ -d "/data/data/com.termux/files/usr" ]; then
  is_termux=1
fi

echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "  CLOUDROOM CORE // PORTABLE ROOTLESS LAUNCHER"
echo "  Host: $(uname -s) $(uname -m) | Termux: $is_termux | Rootless: 1"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ ! -x "$BINARY" ]; then
  echo "[1/4] Building cloudroom-core (low-RAM / mold profile)..."
  if [ "$is_termux" = "1" ]; then
    command -v cargo >/dev/null || {
      echo "Installing Termux Rust & mold toolchain..."
      pkg update -y && pkg install -y rust clang make mold git python nodejs-lts openssl-tool
    }
    (cd "$REPO_ROOT" && cargo build --release -j 2)
  else
    BUILD_TMP="${TMPDIR:-/tmp}/cloudroom-target"
    (cd "$REPO_ROOT" && CARGO_TARGET_DIR="$BUILD_TMP" cargo build --locked --release -j 2)
    mkdir -p "$REPO_ROOT/target/release"
    cp "$BUILD_TMP/release/cloudroom" "$BINARY"
    rm -rf "$BUILD_TMP"
  fi
fi

echo "[2/4] Preparing rootless storage & Whorl ACP substrate in $STATE_ROOT..."
mkdir -p -m 700 \
  "$STATE_ROOT" \
  "$STATE_ROOT/history" \
  "$STATE_ROOT/cache" \
  "$STATE_ROOT/code" \
  "$STATE_ROOT/history/tmp" \
  "$HOME/.local/bin" \
  "$HOME/.hermes" \
  "$HOME/.claude"

chmod 700 "$STATE_ROOT" "$STATE_ROOT/history" "$STATE_ROOT/cache" "$STATE_ROOT/code"

# Wire Whorl ACP substrate into ~/.local/bin/hermes if no real hermes binary is present
if [ ! -x "$HOME/.local/bin/hermes" ]; then
  cp "$SCRIPT_DIR/whorl-acp.py" "$HOME/.local/bin/hermes"
  chmod 755 "$HOME/.local/bin/hermes"
fi
python3 -c '
import sqlite3, pathlib
p = pathlib.Path.home() / ".hermes" / "state.db"
conn = sqlite3.connect(str(p))
conn.execute("CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, cwd TEXT, model TEXT, created_at REAL)")
conn.commit()
'

UID_NOW="$(id -u)"
GID_NOW="$(id -g)"

cat > "$STATE_ROOT/storage.json" <<EOF
{
  "agent_uid": $UID_NOW,
  "agent_gid": $GID_NOW,
  "cache_dir": "$STATE_ROOT/cache",
  "cgroup_root": null,
  "agent_sudo": true,
  "rootless": true,
  "code_dir": "$STATE_ROOT/code",
  "warning_bytes": 500000000,
  "pause_bytes": 100000000,
  "resume_bytes": 200000000
}
EOF
chmod 600 "$STATE_ROOT/storage.json"

if [ ! -f "$STATE_ROOT/token" ]; then
  python3 -c 'import secrets; print(secrets.token_hex(32))' > "$STATE_ROOT/token"
  chmod 600 "$STATE_ROOT/token"
fi
TOKEN="$(cat "$STATE_ROOT/token")"

PIDFILE="$STATE_ROOT/cloudroom.pid"
LOGFILE="$STATE_ROOT/cloudroom.log"

if [ -f "$PIDFILE" ] && kill -0 "$(cat "$PIDFILE")" 2>/dev/null; then
  echo "[3/4] Stopping previous Cloudroom instance (PID $(cat "$PIDFILE"))..."
  kill "$(cat "$PIDFILE")" 2>/dev/null || true
  sleep 1
fi

echo "[3/4] Starting Cloudroom Core on http://$LISTEN_ADDR ..."
export CLOUDROOM_TOKEN="$TOKEN"
export CLOUDROOM_LISTEN="$LISTEN_ADDR"
export CLOUDROOM_STORE="self-hosted"
export CLOUDROOM_STATE_DIR="$STATE_ROOT/history"
export CLOUDROOM_ACCOUNT_HOME="$HOME"
export CLOUDROOM_REPOSITORY="$STATE_ROOT/code"
export CLOUDROOM_LOCAL_HISTORY="1"
export CLOUDROOM_STORAGE_POLICY="$STATE_ROOT/storage.json"
export CLOUDROOM_HARNESS="hermes"

nohup "$BINARY" > "$LOGFILE" 2>&1 &
echo $! > "$PIDFILE"

echo "[4/4] Verifying live health & Scrying Cockpit..."
for _ in $(seq 25); do
  if curl -fsS -H "Authorization: Bearer $TOKEN" "http://$LISTEN_ADDR/v1/health" >/dev/null 2>&1; then
    break
  fi
  sleep 0.2
done

HEALTH="$(curl -fsS -H "Authorization: Bearer $TOKEN" "http://$LISTEN_ADDR/v1/health")"
READY="$(curl -fsS -H "Authorization: Bearer $TOKEN" "http://$LISTEN_ADDR/v1/ready")"

echo "  ✓ Health: $HEALTH"
echo "  ✓ Ready:  $READY"
echo "  ✓ Cockpit URL: http://$LISTEN_ADDR/cockpit?token=$TOKEN"
echo "  ✓ Token file:  $STATE_ROOT/token"
echo "  ✓ Log file:    $LOGFILE"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
