#!/usr/bin/env bash
# bench-bulk.sh — boot-amortized bulk pass over an existing config's bank.
# Usage: bench-bulk.sh <lane> <model> <config> [PAR]
# Splits the config's jobs.tsv into PAR chunks; ONE persistent invocation per chunk
# carries the numbered questions ("just the Qs") and returns numbered answer blocks
# ("then the As"). Boot cost pays once per chunk -> TPS reflects pure model speed.
set -uo pipefail
LANE=${1:?lane}; MODEL=${2:?model}; CFG=${3:?config}; PAR=${4:-16}
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
SRC=$BASE/telemetry-512qa-multi/runs/$CFG
OUT=$SRC/bulk
export PATH="$HOME/.cargo/bin:$HOME/.local/bin:$PATH"

[[ -s "$SRC/jobs.tsv" ]] || { echo "no jobs.tsv for $CFG" >&2; exit 2; }
mkdir -p "$OUT"
START=$(date +%s.%N); START_ISO=$(date -u -Iseconds)

TPROFILE="ds-pro"; case "$CFG" in *flash*) TPROFILE="ds-flash" ;; esac
export LANE MODEL TPROFILE

bulk_prompt() { # $1=chunkfile -> stdout prompt
  local n; n=$(wc -l < "$1")
  printf 'You will receive %s numbered moral-dilemma questions. Answer EVERY question.\n' "$n"
  printf 'Output format — STRICT: for each question, first print its number followed by a period, then your answer on the same line (may continue across lines) before the next number.\n'
  printf 'Do not restate the questions. Do not add preamble or commentary.\n\nQUESTIONS:\n'
  awk -F'\t' '{printf "%d. %s\n", NR, $3}' "$1"
}
run_lane() { # $1=prompt-text
  case "$LANE" in
    opencode) timeout 900 opencode run --model "$MODEL" "$1" 2>/dev/null ;;
    grok)     timeout 900 grok --always-approve --no-subagents --verbatim ${GROK_EFFORT:+--effort "$GROK_EFFORT"} -m "$MODEL" -p "$1" 2>/dev/null ;;
    qwen38)   timeout 900 codex-qwen38 exec -c model_reasoning_effort=low "$1" 2>/dev/null ;;
    tp)       timeout 900 codex-token-plan "$TPROFILE" exec -c model_reasoning_effort=low "$1" 2>/dev/null ;;
  esac
}
export -f bulk_prompt run_lane
export OUT

CHUNK=$(( $(wc -l < "$SRC/jobs.tsv") / PAR )); [[ $CHUNK -lt 1 ]] && CHUNK=1
split -l "$CHUNK" -d -a 2 "$SRC/jobs.tsv" "$OUT/chunk."
: > "$OUT/chunk-walls.jsonl"

widx=0
for f in "$OUT"/chunk.*; do
  [[ -s "$f" ]] || continue
  widx=$((widx+1))
  (
    t0=$(date +%s.%N)
    ans=$(run_lane "$(bulk_prompt "$f")")
    rc=$?
    t1=$(date +%s.%N)
    printf '%s\n' "$ans" > "$OUT/raw.$(basename "$f" | sed 's/chunk.//').txt"
    jq -cn --argjson i "$widx" --argjson rc "$rc" \
       --argjson dur "$(awk -v s="$t0" -v e="$t1" 'BEGIN{printf "%.3f", e-s}')" \
       '{chunk:$i,exit:$rc,duration_s:$dur}' >> "$OUT/chunk-walls.jsonl"
  ) </dev/null &
done
wait
END=$(date +%s.%N)
WALL=$(awk -v s="$START" -v e="$END" 'BEGIN{printf "%.3f", e-s}')

# ---- parse numbered blocks back to per-question rows ----
: > "$OUT/answers.jsonl"
for f in "$OUT"/chunk.*; do
  [[ -s "$f" ]] || continue
  tag=$(basename "$f" | sed 's/chunk.//')
  raw="$OUT/raw.$tag.txt"
  i=0
  while IFS=$'\t' read -r d id q; do
    i=$((i+1))
    a=$(awk -v target="$i" '
      $0 ~ "^[[:space:]]*"target"[.]|[[:space:]]*"target"[)][[:space:]]" {
        if (!found) { found=1; sub("^[[:space:]]*"target"([.]|[)])[[:space:]]*", ""); buf=$0; next }
      }
      found && /^[[:space:]]*[0-9]+[.)][[:space:]]/ { print buf; exit }
      found { buf = buf "\n" $0 }
      END { if (found) print buf }' "$raw" 2>/dev/null)
    st=ok; [[ -z "${a// }" ]] && st=fail
    jq -cn --arg d "$d" --arg id "$id" --arg q "$q" --arg a "$a" --arg st "$st" \
       '{domain:$d,id:$id,q:$q,answer:$a,status:$st,tokens:null}' >> "$OUT/answers.jsonl"
  done < "$f"
done

OK=$(jq -r 'select(.status=="ok")|1' "$OUT/answers.jsonl" 2>/dev/null | wc -l)
TOT=$(wc -l < "$OUT/answers.jsonl")
CWALL=$(jq -r '[.duration_s]|add // 0' "$OUT/chunk-walls.jsonl" 2>/dev/null)
TPS=$(awk -v o="$OK" -v w="$WALL" 'BEGIN{if(w>0&&o>0)printf "%.2f",o/w; else print "null"}')
jq -n --arg cfg "$CFG" --arg lane "$LANE" --arg model "$MODEL" \
  --argjson par "$PAR" --argjson total "$TOT" --argjson ok "$OK" \
  --argjson wall "$WALL" --argjson cwall "$CWALL" --argjson tps "$TPS" \
  --arg start "$START_ISO" \
  '{config:$cfg,lane:$lane,model:$model,mode:"bulk",par:$par,
    jobs_expected:$total,bulk_ok:$ok,bulk_fail:($total-$ok),
    wall_seconds:$wall,chunk_wall_sum:$cwall,tps_bulk:$tps,start_iso:$start}' > "$OUT/bulk-summary.json"
echo "BULK-DONE $CFG ok=$OK/$TOT wall=${WALL}s tps=$TPS"
