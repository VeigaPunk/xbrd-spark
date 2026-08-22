#!/usr/bin/env bash
# catchup.sh — post-wave repairs: titanium retries + v2-era redos. Sequential, probe-gated.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
cd "$BASE" || exit 1
LOG=telemetry-512qa-multi/logs/catchup.log
log(){ echo "[$(date -u +%H:%M:%S)] $*" >> "$LOG"; }

# wait for both primary drivers to fully drain
while pgrep -f 'rerun-driver.s[h]' >/dev/null || pgrep -f 'grok-queue-final.s[h]' >/dev/null \
   || pgrep -f 'bench-512qa-v[23].sh' >/dev/null; do sleep 30; done
log "primary waves drained"

titanium_alive() {
  timeout 90 bash -c 'CODEX_BIN=$(command -v codex-titanium || command -v codex); \
    echo "Reply OK" | "$CODEX_BIN" exec -m gpt-5.6-luna -c model_reasoning_effort=low \
    -c service_tier=fast --skip-git-repo-check --color never "$(cat)"' 2>/dev/null | grep -q OK
}

run(){ log "START $*"; ./bench-512qa-v3.sh "$@" >> "$LOG" 2>&1; log "END $3 rc=$?"; }

# 1. golden lane retry — probe gate, 3 attempts with cool-down
for i in 1 2 3; do
  if titanium_alive; then
    rm -rf telemetry-512qa-multi/runs/codex-spark-golden
    run sekhmet gpt-5.3-codex-spark codex-spark-golden 64
    break
  fi
  log "titanium quota still gated (attempt $i); cooling down 300s"; sleep 300
done

# 2. sol-fast redo on v3 (old data predates armor)
if titanium_alive; then rm -rf telemetry-512qa-multi/runs/sol-fast-titanium; run sekhmet gpt-5.6-sol sol-fast-titanium 64; fi

# 3. v2-era grok casualties — clean v3 redos
rm -rf telemetry-512qa-multi/runs/grok-4.6-medium; GROK_EFFORT=medium run grok grok-4.6 grok-4.6-medium 16
rm -rf telemetry-512qa-multi/runs/grok-4.6-default; run grok grok-4.6 grok-4.6-default 16
rm -rf telemetry-512qa-multi/runs/grok-4.6-low; GROK_EFFORT=low run grok grok-4.6 grok-4.6-low 16
log "CATCHUP-DONE"
