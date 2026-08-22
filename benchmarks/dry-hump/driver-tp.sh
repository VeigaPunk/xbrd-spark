#!/usr/bin/env bash
# driver-tp.sh — token-plan track: qwen3.8-max at P16 barrier-batches.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/driver-tp.log
echo "[$(date -u +%H:%M:%S)] START qwen38" >> "$LOG"
rm -rf telemetry-512qa-multi/runs/qwen3.8-max-low
./bench-512qa-v3.sh qwen38 qwen3.8-max qwen3.8-max-low 16 >> "$LOG" 2>&1
echo "[$(date -u +%H:%M:%S)] END qwen rc=$?" >> "$LOG"
