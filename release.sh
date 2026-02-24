#!/usr/bin/env bash
set -euo pipefail

cargo build --release

BIN="target/release/vibetone"
echo "Built: $BIN ($(du -h "$BIN" | cut -f1))"

cp "$BIN" /usr/local/bin/vibetone
echo "Installed to /usr/local/bin/vibetone"
echo "Run from anywhere:  vibetone"
