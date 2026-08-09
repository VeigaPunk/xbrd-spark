#!/usr/bin/env bash
# Run ethics-300 or hard10 across all Token Plan text models via sekhmet.
# Usage: run-pack.sh ethics|hard10 [model...]
set -euo pipefail

export PATH="${HOME}/.cargo/bin:${HOME}/.local/bin:${HOME}/.local/share/fnm:${PATH}"
BENCH="$(cd "$(dirname "$0")/.." && pwd)"
PACK="${1:?pack: ethics|hard10}"
shift || true

case "$PACK" in
  ethics)
    TASKS="$BENCH/tasks-ethics-300.swarm.md"
    JOBS="${XBRD_SPARK_JOBS:-8}"
    TIMEOUT="${TIMEOUT:-180}"
    ;;
  hard10)
    TASKS="$BENCH/hard10/tasks-hard10.swarm.md"
    JOBS="${XBRD_SPARK_JOBS:-5}"
    TIMEOUT="${TIMEOUT:-360}"
    ;;
  *) echo "pack must be ethics|hard10" >&2; exit 2 ;;
esac

if [[ $# -gt 0 ]]; then
  MODELS=("$@")
else
  mapfile -t MODELS < <(grep -v '^#' "$BENCH/models.txt" | grep -v '^$')
fi

CODEX_BIN="${CODEX_BIN:-$(command -v codex-titanium || command -v codex)}"
SEK="$(command -v sekhmet)"
LANE_TP="${HOME}/.xbgst/codex-lanes/modelstudio-token-plan"
BACKUP="${HOME}/.xbgst/codex-lanes/_host-backup-sekhmet"
mkdir -p "$BACKUP" "$BENCH/runs" "$BENCH/qa"

seed_token_plan_host() {
  mkdir -p "${HOME}/.codex"
  if [[ ! -f "$BACKUP/config.toml" ]]; then
    cp -a "${HOME}/.codex/config.toml" "${HOME}/.codex/auth.json" "$BACKUP/" 2>/dev/null || true
  fi
  cp -a "$LANE_TP/config.toml" "${HOME}/.codex/config.toml"
  cp -a "$LANE_TP/auth.json" "${HOME}/.codex/auth.json"
  chmod 600 "${HOME}/.codex/config.toml" "${HOME}/.codex/auth.json"
}

restore_host() {
  if [[ -f "$BACKUP/config.toml" ]]; then
    cp -a "$BACKUP/config.toml" "${HOME}/.codex/config.toml"
    cp -a "$BACKUP/auth.json" "${HOME}/.codex/auth.json" 2>/dev/null || true
    chmod 600 "${HOME}/.codex/config.toml" "${HOME}/.codex/auth.json" 2>/dev/null || true
    echo "[restore] host ~/.codex restored from sekhmet backup" >&2
  fi
}
trap restore_host EXIT

seed_token_plan_host
echo "[seed] host CODEX -> modelstudio-token-plan (base=$(grep base_url "${HOME}/.codex/config.toml"))" >&2

N_TASKS=$(wc -l < "$TASKS")
export CODEX_BIN
export XBRD_SPARK_FALLBACK_MODEL=none
export XBRD_SPARK_JOBS="$JOBS"

for model in "${MODELS[@]}"; do
  safe="$(printf '%s' "$model" | tr -c 'A-Za-z0-9._-' '_')"
  RUN_ID="${PACK}_${safe}"
  OUT="$BENCH/runs/$RUN_ID"
  mkdir -p "$OUT"
  ROOT=$(mktemp -d "${XDG_RUNTIME_DIR:-/tmp}/sekhmet-tp-${RUN_ID}-XXXXXX")
  export XBRD_SPARK_MODEL="$model"

  # refresh host model default to match (sekhmet also passes -m via env)
  if grep -q '^model ' "${HOME}/.codex/config.toml"; then
    sed -i "s/^model = .*/model = \"$model\"/" "${HOME}/.codex/config.toml"
  fi

  echo "==== $RUN_ID model=$model tasks=$N_TASKS j=$JOBS timeout=$TIMEOUT root=$ROOT ====" | tee "$OUT/campaign.log"
  START_ISO=$(date -u -Iseconds)
  START_EPOCH=$(date +%s.%N)

  set +e
  # Pure L3 default is direct Titanium (no --direct flag on current sekhmet; use --no-direct only if xask wrap needed).
  "$SEK" swarm -j "$JOBS" --timeout "$TIMEOUT" \
    --tasks-file "$TASKS" \
    --root "$ROOT" \
    >"$OUT/ndjson.out" 2>"$OUT/swarm.stderr.log"
  EC=$?
  set -e

  END_EPOCH=$(date +%s.%N)
  END_ISO=$(date -u -Iseconds)
  WALL=$(awk -v s="$START_EPOCH" -v e="$END_EPOCH" 'BEGIN{printf "%.3f", e-s}')

  # Aggregate with jq (no python)
  OK=0; FAIL=0; TO=0; LINES=0
  if [[ -f "$OUT/ndjson.out" ]]; then
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      LINES=$((LINES + 1))
      st=$(printf '%s' "$line" | jq -r '.status // empty' 2>/dev/null || true)
      case "$st" in
        ok) OK=$((OK + 1)) ;;
        timeout) TO=$((TO + 1)) ;;
        *) FAIL=$((FAIL + 1)) ;;
      esac
    done < "$OUT/ndjson.out"
  fi

  TOK_SUM=0; TOK_N=0; TOK_MIN=""; TOK_MAX=""
  DUR_FILE=$(mktemp)
  : >"$DUR_FILE"
  if [[ -f "$OUT/ndjson.out" ]]; then
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      tok=$(printf '%s' "$line" | jq -r '.usage_tokens // empty' 2>/dev/null || true)
      if [[ -n "${tok:-}" && "$tok" != "null" ]]; then
        TOK_SUM=$((TOK_SUM + tok))
        TOK_N=$((TOK_N + 1))
        if [[ -z "$TOK_MIN" || "$tok" -lt "$TOK_MIN" ]]; then TOK_MIN=$tok; fi
        if [[ -z "$TOK_MAX" || "$tok" -gt "$TOK_MAX" ]]; then TOK_MAX=$tok; fi
      fi
      dm=$(printf '%s' "$line" | jq -r '.provenance.duration_ms // empty' 2>/dev/null || true)
      if [[ -n "${dm:-}" && "$dm" != "null" ]]; then
        echo "$dm" >>"$DUR_FILE"
      fi
      # harvest short answer path for QA
      rp=$(printf '%s' "$line" | jq -r '.result_path // empty' 2>/dev/null || true)
      sid=$(printf '%s' "$line" | jq -r '.spark_id // empty' 2>/dev/null || true)
      if [[ -n "$rp" && -f "$rp" ]]; then
        mkdir -p "$OUT/results"
        cp -f "$rp" "$OUT/results/${sid:-unknown}.json" 2>/dev/null || true
      fi
    done < "$OUT/ndjson.out"
  fi

  P50=""; P95=""; DUR_N=0
  if [[ -s "$DUR_FILE" ]]; then
    DUR_N=$(wc -l <"$DUR_FILE")
    sort -n "$DUR_FILE" >"${DUR_FILE}.s"
    P50=$(awk -v n="$DUR_N" 'NR==int((n+1)/2){print; exit}' "${DUR_FILE}.s")
    P95=$(awk -v n="$DUR_N" 'NR==int(n*0.95)+1{print; exit}' "${DUR_FILE}.s")
  fi
  rm -f "$DUR_FILE" "${DUR_FILE}.s"

  TOK_AVG=""
  if [[ "$TOK_N" -gt 0 ]]; then
    TOK_AVG=$(awk -v s="$TOK_SUM" -v n="$TOK_N" 'BEGIN{printf "%.1f", s/n}')
  fi
  QPS=$(awk -v ok="$OK" -v w="$WALL" 'BEGIN{if(w>0) printf "%.4f", ok/w; else print "0"}')
  TPM=$(awk -v s="$TOK_SUM" -v w="$WALL" 'BEGIN{if(w>0) printf "%.1f", s/w; else print "0"}')

  # Stamp identity from host CODEX config + known sekhmet injects
  EFFORT=$(grep -E '^model_reasoning_effort' "${HOME}/.codex/config.toml" 2>/dev/null | head -1 | sed 's/.*= *"\([^"]*\)".*/\1/' || echo unknown)
  EFFORT=${EFFORT:-low}
  # sekhmet Titanium path forces effort=low + service_tier=fast in dispatcher
  TIER="fast"
  BASE=$(grep -E 'base_url' "${HOME}/.codex/config.toml" 2>/dev/null | head -1 | sed 's/.*= *"\([^"]*\)".*/\1/' || echo unknown)

  jq -n \
    --arg run_id "$RUN_ID" \
    --arg pack "$PACK" \
    --arg model "$model" \
    --arg model_id "$model" \
    --arg model_reasoning_effort "$EFFORT" \
    --arg service_tier "$TIER" \
    --arg lane_base_url "$BASE" \
    --arg binary "$CODEX_BIN" \
    --argjson tasks "$N_TASKS" \
    --argjson jobs "$JOBS" \
    --argjson timeout "$TIMEOUT" \
    --argjson swarm_exit "$EC" \
    --argjson sparks_ok "$OK" \
    --argjson sparks_fail "$FAIL" \
    --argjson sparks_timeout "$TO" \
    --argjson ndjson_lines "$LINES" \
    --arg wall_seconds "$WALL" \
    --arg qps "$QPS" \
    --arg tokens_per_sec "$TPM" \
    --argjson sekhmet_tokens_total "${TOK_SUM:-0}" \
    --argjson sekhmet_tokens_n "${TOK_N:-0}" \
    --arg sekhmet_tokens_min "${TOK_MIN}" \
    --arg sekhmet_tokens_max "${TOK_MAX}" \
    --arg sekhmet_tokens_avg "${TOK_AVG}" \
    --arg p50_duration_ms "${P50}" \
    --arg p95_duration_ms "${P95}" \
    --argjson duration_n "${DUR_N:-0}" \
    --arg start_iso "$START_ISO" \
    --arg end_iso "$END_ISO" \
    --arg root "$ROOT" \
    --arg codex_bin "$CODEX_BIN" \
    '{
      run_id:$run_id, pack:$pack,
      model:$model, model_id:$model_id,
      model_reasoning_effort:$model_reasoning_effort,
      service_tier:$service_tier,
      lane_base_url:$lane_base_url,
      binary:$binary, invoker:"sekhmet swarm",
      tasks:$tasks, jobs:$jobs, timeout:$timeout,
      swarm_exit:$swarm_exit, sparks_ok:$sparks_ok, sparks_fail:$sparks_fail, sparks_timeout:$sparks_timeout,
      ndjson_lines:$ndjson_lines, wall_seconds:($wall_seconds|tonumber), qps:($qps|tonumber),
      tokens_per_sec:($tokens_per_sec|tonumber),
      sekhmet_tokens_total:$sekhmet_tokens_total, sekhmet_tokens_n:$sekhmet_tokens_n,
      sekhmet_tokens_min:(if $sekhmet_tokens_min=="" then null else ($sekhmet_tokens_min|tonumber) end),
      sekhmet_tokens_max:(if $sekhmet_tokens_max=="" then null else ($sekhmet_tokens_max|tonumber) end),
      sekhmet_tokens_avg:(if $sekhmet_tokens_avg=="" then null else ($sekhmet_tokens_avg|tonumber) end),
      p50_duration_ms:(if $p50_duration_ms=="" then null else ($p50_duration_ms|tonumber) end),
      p95_duration_ms:(if $p95_duration_ms=="" then null else ($p95_duration_ms|tonumber) end),
      duration_n:$duration_n,
      start_iso:$start_iso, end_iso:$end_iso, root:$root, codex_bin:$codex_bin
    }' >"$OUT/summary.json"

  echo "done $RUN_ID ok=$OK fail=$FAIL to=$TO wall=${WALL}s qps=$QPS tokens=$TOK_SUM exit=$EC" | tee -a "$OUT/campaign.log"

  # keep root for hard10 (small); for ethics reclaim after copy
  if [[ "$PACK" == "ethics" ]]; then
    rm -rf "$ROOT"
  else
    echo "$ROOT" >"$OUT/root.path"
  fi
done

echo "[pack $PACK] complete" >&2
