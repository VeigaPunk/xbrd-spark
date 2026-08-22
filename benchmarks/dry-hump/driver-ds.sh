#!/usr/bin/env bash
# driver-ds.sh — token-plan deepseek pair: sequential after the qwen track drains.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/driver-ds.log
while pgrep -f 'driver-tp.s[h]' >/dev/null || pgrep -f 'bench-512qa-v3.sh qwen3[8]' >/dev/null; do sleep 30; done
run(){ echo "[$(date -u +%H:%M:%S)] START $*" >> "$LOG"; rm -rf "telemetry-512qa-multi/runs/$2"; ./bench-512qa-tp.sh tp "$1" "$2" 16 >> "$LOG" 2>&1; echo "[$(date -u +%H:%M:%S)] END $2 rc=$?" >> "$LOG"; }
run alibaba-token-plan/deepseek-v4-pro-0813 ds-pro-0813
run alibaba-token-plan/deepseek-v4-flash-0731 ds-flash-0731
echo "[$(date -u +%H:%M:%S)] DS-TRACK-DONE" >> "$LOG"
