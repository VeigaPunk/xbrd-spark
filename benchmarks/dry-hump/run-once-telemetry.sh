#!/usr/bin/env bash
# One dry-hump live pass: 8 domains × 8 concurrent Titanium sparks via sekhmet.
# Args: <run_id> <out_dir> [model_label]
# Rust/bash only (no Python). Always removes the temp ROOT after aggregation to avoid /tmp quota blowups.
set -euo pipefail
RUN_ID=${1:?run_id}
OUT=${2:?out_dir}
MODEL_LABEL=${3:-unknown}
export PATH="${HOME}/.cargo/bin:${PATH}"
export CODEX_BIN=${CODEX_BIN:-$(command -v codex-titanium || command -v codex)}
HERE=$(cd "$(dirname "$0")" && pwd)
DOMAINS=(religion sex drugs politics money violence ai charlie-kirk)
ROOT=$(mktemp -d "/tmp/sekhmet-12x-${RUN_ID}-XXXXXX")
cleanup() { rm -rf "$ROOT"; }
trap cleanup EXIT

mkdir -p "$OUT"
START_ISO=$(date -u -Iseconds)
START_EPOCH=$(date +%s.%N)
HOST=$(hostname 2>/dev/null || echo unknown)
SEK=$(command -v sekhmet)
CODEX_VER=$("$CODEX_BIN" --version 2>&1 | head -1 || true)

for d in "${DOMAINS[@]}"; do
  mkdir -p "$ROOT/$d"
  (
    DS=$(date +%s.%N)
    # Keep namespaces until after aggregation so result.json token parse works.
    # ROOT is always rm -rf'd by the EXIT trap (tmpfs-safe).
    sekhmet swarm --direct -j 8 --timeout 180 \
      --tasks-file "$HERE/domains/$d/tasks.txt" \
      --root "$ROOT/$d" \
      > "$ROOT/$d/ndjson.out" \
      2> "$ROOT/$d/stderr.log"
    EC=$?
    DE=$(date +%s.%N)
    echo "exit=$EC start=$DS end=$DE" > "$ROOT/$d/timing.txt"
    exit $EC
  ) &
done
wait
END_EPOCH=$(date +%s.%N)
END_ISO=$(date -u -Iseconds)

# Aggregate ok/fail + tokens with bash + jq (no Python).
if ! command -v jq >/dev/null 2>&1; then
  echo "jq required for summary aggregation" >&2
  exit 2
fi

WALL=$(awk -v s="$START_EPOCH" -v e="$END_EPOCH" 'BEGIN{printf "%.3f", e-s}')
TOKENS_FILE=$(mktemp)
DOMAINS_JSON="{}"
TOTAL_OK=0
TOTAL_FAIL=0
TOTAL_TIMEOUT=0
TOTAL_LINES=0

for d in "${DOMAINS[@]}"; do
  ND="$ROOT/$d/ndjson.out"
  OK=0; FAIL=0; TIMEOUT=0; LINES=0
  if [[ -f "$ND" ]]; then
    while IFS= read -r line; do
      [[ -z "$line" ]] && continue
      LINES=$((LINES + 1))
      st=$(printf '%s' "$line" | jq -r '.status // empty' 2>/dev/null || true)
      case "$st" in
        ok) OK=$((OK + 1)) ;;
        timeout) TIMEOUT=$((TIMEOUT + 1)) ;;
        *) FAIL=$((FAIL + 1)) ;;
      esac
    done < "$ND"
  fi
  DTOK_SUM=0
  DTOK_N=0
  while IFS= read -r rf; do
    [[ -z "$rf" ]] && continue
    # Prefer structured field from sekhmet finalize (usage_tokens), then log scrape.
    tok=$(jq -r '.usage_tokens // empty' "$rf" 2>/dev/null || true)
    if [[ -z "${tok:-}" || "$tok" == "null" ]]; then
      text=$(jq -r '(.stderr // "") + "\n" + (.stdout // "")' "$rf" 2>/dev/null || true)
      # shellcheck disable=SC2001
      tok=$(printf '%s\n' "$text" | tr -d '\r' | sed -n \
        -e 's/.*[Tt]okens used[^0-9]*\([0-9,][0-9,]*\).*/\1/p' \
        -e 's/.*total_tokens[^0-9]*\([0-9,][0-9,]*\).*/\1/p' \
        | head -1 | tr -d ',')
    fi
    if [[ -n "${tok:-}" && "$tok" != "null" ]]; then
      echo "$tok" >> "$TOKENS_FILE"
      DTOK_SUM=$((DTOK_SUM + tok))
      DTOK_N=$((DTOK_N + 1))
    fi
  done < <(find "$ROOT/$d" -path '*/out/result.json' 2>/dev/null || true)
  TIMING=""
  [[ -f "$ROOT/$d/timing.txt" ]] && TIMING=$(cat "$ROOT/$d/timing.txt")
  DOMAINS_JSON=$(jq -c --arg d "$d" --argjson ok "$OK" --argjson fail "$FAIL" \
    --argjson timeout "$TIMEOUT" --argjson lines "$LINES" \
    --argjson tsum "$DTOK_SUM" --argjson tn "$DTOK_N" --arg timing "$TIMING" \
    '.[$d] = {ok:$ok, fail:$fail, timeout:$timeout, lines:$lines, tokens_sum:$tsum, tokens_n:$tn, timing:$timing}' \
    <<<"$DOMAINS_JSON")
  TOTAL_OK=$((TOTAL_OK + OK))
  TOTAL_FAIL=$((TOTAL_FAIL + FAIL))
  TOTAL_TIMEOUT=$((TOTAL_TIMEOUT + TIMEOUT))
  TOTAL_LINES=$((TOTAL_LINES + LINES))
done

TOKENS_TOTAL=0
TOKENS_N=0
TOKENS_MIN=null
TOKENS_MAX=null
TOKENS_AVG=null
if [[ -s "$TOKENS_FILE" ]]; then
  TOKENS_N=$(wc -l < "$TOKENS_FILE" | tr -d ' ')
  TOKENS_TOTAL=$(awk '{s+=$1} END{print s+0}' "$TOKENS_FILE")
  TOKENS_MIN=$(sort -n "$TOKENS_FILE" | head -1)
  TOKENS_MAX=$(sort -n "$TOKENS_FILE" | tail -1)
  TOKENS_AVG=$(awk -v s="$TOKENS_TOTAL" -v n="$TOKENS_N" 'BEGIN{printf "%.1f", (n?s/n:0)}')
fi
rm -f "$TOKENS_FILE"

jq -n \
  --arg run_id "$RUN_ID" \
  --arg model_label "$MODEL_LABEL" \
  --argjson wall_seconds "$WALL" \
  --arg start_iso "$START_ISO" \
  --arg end_iso "$END_ISO" \
  --arg start_epoch "$START_EPOCH" \
  --arg end_epoch "$END_EPOCH" \
  --arg host "$HOST" \
  --arg sekhmet_bin "$SEK" \
  --arg codex_bin "$CODEX_BIN" \
  --arg codex_version "$CODEX_VER" \
  --arg root "$ROOT" \
  --argjson sparks_ok "$TOTAL_OK" \
  --argjson sparks_fail "$TOTAL_FAIL" \
  --argjson sparks_timeout "$TOTAL_TIMEOUT" \
  --argjson ndjson_lines "$TOTAL_LINES" \
  --argjson tokens_total "$TOKENS_TOTAL" \
  --argjson tokens_n "$TOKENS_N" \
  --argjson tokens_min "${TOKENS_MIN}" \
  --argjson tokens_max "${TOKENS_MAX}" \
  --argjson tokens_avg "${TOKENS_AVG}" \
  --argjson domains "$DOMAINS_JSON" \
  '{
    run_id: $run_id,
    model_label: $model_label,
    wall_seconds: $wall_seconds,
    start_iso: $start_iso,
    end_iso: $end_iso,
    start_epoch: $start_epoch,
    end_epoch: $end_epoch,
    host: $host,
    sekhmet_bin: $sekhmet_bin,
    codex_bin: $codex_bin,
    codex_version: $codex_version,
    root: $root,
    sparks_expected: 64,
    sparks_ok: $sparks_ok,
    sparks_fail: $sparks_fail,
    sparks_timeout: $sparks_timeout,
    ndjson_lines: $ndjson_lines,
    tokens_total: $tokens_total,
    tokens_n: $tokens_n,
    tokens_min: $tokens_min,
    tokens_max: $tokens_max,
    tokens_avg: $tokens_avg,
    domains: $domains
  }' > "$OUT/summary.json"

jq -c \
  --arg run_id "$RUN_ID" \
  --arg model "$MODEL_LABEL" \
  --argjson wall_s "$WALL" \
  --argjson ok "$TOTAL_OK" \
  --argjson fail "$TOTAL_FAIL" \
  --argjson timeout "$TOTAL_TIMEOUT" \
  --argjson tokens "$TOKENS_TOTAL" \
  --arg root "$ROOT" \
  '{run_id:$run_id, model:$model, wall_s:$wall_s, ok:$ok, fail:$fail, timeout:$timeout, tokens:$tokens, root:$root}' \
  <<<"{}"
