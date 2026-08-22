#!/usr/bin/env bash
# grok-queue-final.sh — trimmed matrix per judge directive: 4.6-high (waits if running), 4.5-low, 4.5-high.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/grok-family-queue.log

# wait out any in-flight grok bench (e.g. grok-4.6-high)
while pgrep -f 'bench-512qa-v3.sh grok' >/dev/null 2>&1; do sleep 20; done

run() {
  # skip configs that already completed with a valid summary + full coverage
  local cfg="$3"
  if [[ -s "telemetry-512qa-multi/runs/$cfg/summary.json" ]] \
     && jq -e '.answers_ok >= (.jobs_expected * 0.95)' "telemetry-512qa-multi/runs/$cfg/summary.json" >/dev/null 2>&1; then
    echo "[$(date -u +%H:%M:%S)] SKIP $cfg (already complete)" >> "$LOG"
    return 0
  fi
  echo "[$(date -u +%H:%M:%S)] START $*" >> "$LOG"
  ./bench-512qa-v3.sh "$@" >> "$LOG" 2>&1
  echo "[$(date -u +%H:%M:%S)] END   $* rc=$?" >> "$LOG"
}
GROK_EFFORT=low  run grok grok-4.6 grok-4.6-low 16
GROK_EFFORT=high run grok grok-4.6 grok-4.6-high 16
GROK_EFFORT=low  run grok grok-4.5 grok-4.5-low 16
GROK_EFFORT=high run grok grok-4.5 grok-4.5-high 16
echo "[$(date -u +%H:%M:%S)] QUEUE-FINAL-DONE" >> "$LOG"
