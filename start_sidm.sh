#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$DIR/sidm-core/target/release/sidm-core"

if [ ! -f "$BIN" ]; then
    BIN="$DIR/sidm-core/target/debug/sidm-core"
fi

if [ ! -f "$BIN" ]; then
    echo "⚡ Building SIDM Core Engine (Optimized Release)..."
    cd "$DIR/sidm-core"
    cargo build --release
    BIN="$DIR/sidm-core/target/release/sidm-core"
fi

echo "=================================================="
echo "    ⚡ SIDM - Smart Internet Download Manager     "
echo "=================================================="
echo "🌐 Web Dashboard: http://127.0.0.1:45678"
echo "🔌 Browser Extension: Load unpacked '$DIR/sidm-extension'"
echo "--------------------------------------------------"

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
