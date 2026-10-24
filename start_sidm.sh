#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$DIR/sidm-core/target/release/sidm-core"

# Check if rebuild is needed (missing binary or source code updated)
REBUILD=0
if [ ! -f "$BIN" ]; then
    REBUILD=1
elif [ "$1" == "--build" ] || [ "$1" == "-b" ]; then
    REBUILD=1
elif [ -n "$(find "$DIR/sidm-core/src" "$DIR/sidm-core/static" -newer "$BIN" 2>/dev/null | head -n 1)" ]; then
    REBUILD=1
fi

if [ "$REBUILD" -eq 1 ]; then
    echo "⚡ Building / Updating SIDM Core Engine (Optimized Release)..."
    cd "$DIR/sidm-core"
    cargo build --release
    cd "$DIR"
fi

echo "=================================================="
echo "    ⚡ SIDM - Smart Internet Download Manager     "
echo "=================================================="
echo "🌐 Web Dashboard: http://127.0.0.1:45678"
echo "🔌 Browser Extension: Load unpacked '$DIR/sidm-extension'"
echo "--------------------------------------------------"

# Ensure port 45678 is free by terminating any previous instances
echo "🔄 Stopping previous SIDM instances on port 45678..."
pkill -f "$BIN" 2>/dev/null || true
pkill -f "sidm-core server" 2>/dev/null || true
if command -v fuser >/dev/null 2>&1; then
  fuser -k 45678/tcp >/dev/null 2>&1 || true
fi
sleep 0.5

# Auto-open dashboard window in background
(
  sleep 1
  if command -v google-chrome >/dev/null 2>&1; then
    google-chrome --app=http://127.0.0.1:45678 >/dev/null 2>&1 &
  elif command -v chromium >/dev/null 2>&1; then
    chromium --app=http://127.0.0.1:45678 >/dev/null 2>&1 &
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open http://127.0.0.1:45678 >/dev/null 2>&1 &
  fi
) &

exec "$BIN" server
