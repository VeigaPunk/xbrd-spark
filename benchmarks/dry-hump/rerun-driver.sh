#!/usr/bin/env bash
# rerun-driver.sh — sequential config reruns on v3 (batch-barrier), one quota pool at a time.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/rerun-driver.log

rerun() { # lane model cfg PAR — wipe tainted dir, regen banks fresh, full v3 pass
  echo "[$(date -u +%H:%M:%S)] RERUN-START $*" >> "$LOG"
  rm -rf "telemetry-512qa-multi/runs/$3"
  ./bench-512qa-v3.sh "$@" >> "$LOG" 2>&1
  echo "[$(date -u +%H:%M:%S)] RERUN-END $3 rc=$?" >> "$LOG"
}

# opencode-go pair (own pool)
rerun opencode opencode-go/ox-alpha-free ox-alpha-free 32
rerun opencode opencode-go/glm-5.3 glm-5.3 32
# free tier: strictly sequential, one config at a time
rerun opencode opencode/hy3-free hy3-free 16
rerun opencode opencode/mimo-v2.5-free mimo-v2.5-free 16
rerun opencode opencode/muse-spark-1.2-contributor-free muse-spark-1.2-contributor-free 16
rerun opencode opencode/nemotron-3-ultra-free nemotron-3-ultra-free 16
rerun opencode opencode/nemotron-3.5-lightning-free nemotron-3.5-lightning-free 16
rerun opencode opencode/x-preview-f-free x-preview-f-free 16
# token plan solo, conservative batch
rerun qwen38 qwen3.8-max qwen3.8-max-low 8
echo "[$(date -u +%H:%M:%S)] DRIVER-DONE" >> "$LOG"
