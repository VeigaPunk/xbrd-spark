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
    JOBS="${XBRD_SPARK_JOBS:-8}"
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
  # Force /tmp (never XDG_RUNTIME_DIR — ethics-300 needs multi-GB headroom)
  ROOT=$(mktemp -d "/tmp/sekhmet-tp-${RUN_ID}-XXXXXX")
  export XBRD_SPARK_MODEL="$model"
  # sekhmet Titanium inject: effort=low + service_tier=fast (not host medium)
  export XBRD_SPARK_SERVICE_TIER="${XBRD_SPARK_SERVICE_TIER:-fast}"

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

  # Stamp identity from first ndjson provenance.cmdline (sekhmet inject), never host medium
  EFFORT="low"
  TIER="fast"
  CMDLINE_MODEL=""
  if [[ -f "$OUT/ndjson.out" ]]; then
    FIRST=$(head -1 "$OUT/ndjson.out" || true)
    if [[ -n "$FIRST" ]]; then
      CMDLINE_MODEL=$(printf '%s' "$FIRST" | jq -r '
        (.provenance.cmdline // []) as $c
        | ($c | to_entries | map(select(.value == "-m") | .key) | .[0]) as $i
        | if $i != null then $c[$i+1] else (.provenance.model // empty) end
      ' 2>/dev/null || true)
      EFFORT_FROM=$(printf '%s' "$FIRST" | jq -r '
        (.provenance.cmdline // [])[] | select(startswith("model_reasoning_effort="))
        | sub("model_reasoning_effort=";"")
      ' 2>/dev/null | head -1 || true)
      TIER_FROM=$(printf '%s' "$FIRST" | jq -r '
        (.provenance.cmdline // [])[] | select(startswith("service_tier="))
        | sub("service_tier=";"")
      ' 2>/dev/null | head -1 || true)
      [[ -n "${EFFORT_FROM:-}" && "$EFFORT_FROM" != "null" ]] && EFFORT="$EFFORT_FROM"
      [[ -n "${TIER_FROM:-}" && "$TIER_FROM" != "null" ]] && TIER="$TIER_FROM"
    fi
  fi
  # Never stamp host medium — sekhmet inject is low+fast
  if [[ "$EFFORT" == "medium" || "$EFFORT" == "high" || -z "$EFFORT" ]]; then
    EFFORT="low"
  fi
  TIER="${TIER:-fast}"
  STAMP_MODEL="${CMDLINE_MODEL:-$model}"
  BASE=$(grep -E 'base_url' "${HOME}/.codex/config.toml" 2>/dev/null | head -1 | sed 's/.*= *"\([^"]*\)".*/\1/' || echo unknown)

  jq -n \
    --arg run_id "$RUN_ID" \
    --arg pack "$PACK" \
    --arg model "$model" \
    --arg model_id "$STAMP_MODEL" \
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

  echo "done $RUN_ID ok=$OK fail=$FAIL to=$TO wall=${WALL}s qps=$QPS tokens=$TOK_SUM exit=$EC effort=$EFFORT tier=$TIER model_id=$STAMP_MODEL" | tee -a "$OUT/campaign.log"
  echo "$ROOT" >"$OUT/root.path"

  # Harvest ALL-QA after each model (M02)
  QA_OUT="$BENCH/qa/$RUN_ID/ALL-QA.md"
  mkdir -p "$(dirname "$QA_OUT")"
  HARVEST_BIN="$(command -v harvest-qa || true)"
  if [[ -z "$HARVEST_BIN" && -x "$BENCH/../../target/release/harvest-qa" ]]; then
    HARVEST_BIN="$BENCH/../../target/release/harvest-qa"
  fi
  if [[ -n "$HARVEST_BIN" ]]; then
    set +e
    "$HARVEST_BIN" \
      --run-dir "$OUT" \
      --root "$ROOT" \
      --tasks "$TASKS" \
      --out "$QA_OUT" \
      --model-id "$STAMP_MODEL" \
      --effort "$EFFORT" \
      --tier "$TIER" \
      --pack "$PACK" \
      2>>"$OUT/campaign.log"
    HARVEST_EC=$?
    set -e
    echo "[harvest] $QA_OUT exit=$HARVEST_EC" | tee -a "$OUT/campaign.log"
  else
    echo "[harvest] SKIP harvest-qa not on PATH — install cargo bin harvest-qa" | tee -a "$OUT/campaign.log"
  fi

  # Reclaim /tmp root after harvest (optional keep via KEEP_ROOT=1)
  if [[ "${KEEP_ROOT:-0}" != "1" ]]; then
    rm -rf "$ROOT"
    echo "[root] reclaimed $ROOT" | tee -a "$OUT/campaign.log"
  else
    echo "[root] kept $ROOT (KEEP_ROOT=1)" | tee -a "$OUT/campaign.log"
  fi
done

echo "[pack $PACK] complete" >&2
