#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BIN="$DIR/sidm-core/target/debug/sidm-core"

if [ ! -f "$BIN" ]; then
    echo "⚡ Building SIDM Core Engine..."
    cd "$DIR/sidm-core"
    cargo build
fi

echo "=================================================="
echo "    ⚡ SIDM - Smart Internet Download Manager     "
echo "=================================================="
echo "🌐 Web Dashboard: http://127.0.0.1:45678"
echo "🔌 Browser Extension: Load unpacked '$DIR/sidm-extension'"
echo "--------------------------------------------------"

exec "$BIN" server
