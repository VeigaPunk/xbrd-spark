#!/usr/bin/env bash
# grok-family-queue-v3.sh — remaining matrix after low completed on v2.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/grok-family-queue.log
run() {
  echo "[$(date -u +%H:%M:%S)] START $*" >> "$LOG"
  ./bench-512qa-v3.sh "$@" >> "$LOG" 2>&1
  echo "[$(date -u +%H:%M:%S)] END   $* rc=$?" >> "$LOG"
}
for e in medium high xhigh; do GROK_EFFORT=$e run grok grok-4.6 "grok-4.6-$e" 16; done
for e in low medium high xhigh; do GROK_EFFORT=$e run grok grok-4.5 "grok-4.5-$e" 16; done
run opencode xai/grok-4.20-0309-non-reasoning grok-4.20-non-reasoning 16
run opencode xai/grok-4.20-0309-reasoning grok-4.20-reasoning 16
run opencode xai/grok-4.20-multi-agent-0309 grok-4.20-multi-agent 16
run opencode xai/grok-4.3 grok-4.3 16
run opencode xai/grok-build-0.1 grok-build-0.1 16
echo "[$(date -u +%H:%M:%S)] QUEUE-V3-DONE" >> "$LOG"
