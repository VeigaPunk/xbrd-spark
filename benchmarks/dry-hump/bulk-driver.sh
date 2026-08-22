#!/usr/bin/env bash
# bulk-driver.sh — after all primary/catchup tracks drain, run boot-amortized bulk
# passes for every config that has a healthy bank. One pool at a time.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/bulk-driver.log
log(){ echo "[$(date -u +%H:%M:%S)] $*" >> "$LOG"; }

while pgrep -f 'driver-(go|fre[e]|tp|d[s]).sh|grok-queue-final.s[h]|catchup.s[h]|rerun-driver.s[h]' >/dev/null \
   || pgrep -f 'bench-512qa-v[23].sh|bench-512qa-tp.s[h]' >/dev/null; do sleep 30; done
log "all tracks drained; starting bulk passes"

bulk(){ log "BULK $2 $3"; ./bench-bulk.sh "$@" >> "$LOG" 2>&1; log "BULK-END $3 rc=$?"; }

# xai pool
GROK_EFFORT=high  bulk grok grok-4.6 grok-4.6-high 16
GROK_EFFORT=low   bulk grok grok-4.6 grok-4.6-low 16
GROK_EFFORT=low   bulk grok grok-4.5 grok-4.5-low 16
GROK_EFFORT=high  bulk grok grok-4.5 grok-4.5-high 16
bulk grok grok-4.6 grok-4.6-default 16
bulk grok grok-4.6 grok-4.6-medium 16
# opencode-go pool
bulk opencode opencode-go/ox-alpha-free ox-alpha-free 32
bulk opencode opencode-go/glm-5.3 glm-5.3 32
# free tier (sequential)
for m in hy3-free mimo-v2.5-free muse-spark-1.2-contributor-free nemotron-3-ultra-free nemotron-3.5-lightning-free x-preview-f-free; do
  bulk opencode "opencode/$m" "$m" 14
done
# token plan pool
bulk qwen38 qwen3.8-max qwen3.8-max-low 16
bulk tp alibaba-token-plan/deepseek-v4-pro-0813 ds-pro-0813 16
bulk tp alibaba-token-plan/deepseek-v4-flash-0731 ds-flash-0731 16
log "BULK-DRIVER-DONE"
