#!/usr/bin/env bash
# driver-free.sh — free-tier track: 2 configs concurrent x P14 waves (RAM-managed).
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/driver-free.log
run(){ echo "[$(date -u +%H:%M:%S)] START $3" >> "$LOG"; rm -rf "telemetry-512qa-multi/runs/$3"; ./bench-512qa-v3.sh opencode "$2" "$3" 14 >> "$LOG" 2>&1; echo "[$(date -u +%H:%M:%S)] END $3 rc=$?" >> "$LOG"; }
run opencode opencode/hy3-free hy3-free &
P1=$!
sleep 45
run opencode opencode/mimo-v2.5-free mimo-v2.5-free &
P2=$!
wait $P1 $P2
run opencode opencode/muse-spark-1.2-contributor-free muse-spark-1.2-contributor-free &
P1=$!
sleep 45
run opencode opencode/nemotron-3-ultra-free nemotron-3-ultra-free &
P2=$!
wait $P1 $P2
run opencode opencode/nemotron-3.5-lightning-free nemotron-3.5-lightning-free &
P1=$!
sleep 45
run opencode opencode/x-preview-f-free x-preview-f-free &
P2=$!
wait $P1 $P2
echo "[$(date -u +%H:%M:%S)] FREE-TRACK-DONE" >> "$LOG"
