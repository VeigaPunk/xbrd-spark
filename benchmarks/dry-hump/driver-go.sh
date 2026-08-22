#!/usr/bin/env bash
# driver-go.sh — opencode-go track: waits out ox-alpha, then glm at P32 barrier-batches.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/driver-go.log
while pgrep -f 'bench-512qa-v3.sh opencode opencode-go/ox-alpha-fre[e]' >/dev/null; do sleep 20; done
echo "[$(date -u +%H:%M:%S)] START glm-5.3" >> "$LOG"
rm -rf telemetry-512qa-multi/runs/glm-5.3
./bench-512qa-v3.sh opencode opencode-go/glm-5.3 glm-5.3 32 >> "$LOG" 2>&1
echo "[$(date -u +%H:%M:%S)] END glm rc=$?" >> "$LOG"
