#!/usr/bin/env bash
# rebuild-summary.sh <config> — sanitize torn JSONL rows, rebuild answers + summary.
# Fixes: concurrent-append tears, quota-stub clobbers, missing aggregates.
BASE=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump
CFG=${1:?config}
RUN=$BASE/telemetry-512qa-multi/runs/$CFG
[[ -d "$RUN" ]] || { echo "no run dir $CFG" >&2; exit 2; }

sanitize(){ # $1=file — keep only valid JSON objects having .id
  [[ -f "$1" ]] || return 0
  local tmp="$1.clean"
  while IFS= read -r line; do
    [[ -z "$line" ]] && continue
    printf '%s' "$line" | jq -e 'has("id")' >/dev/null 2>&1 \
      && printf '%s\n' "$line" >> "$tmp"
  done < "$1"
  mv "$tmp" "$1"
}
for f in "$RUN"/*/answers.jsonl; do sanitize "$f"; done

# dedupe last-wins per id across original+retry within each domain
for d in "$RUN"/*/; do
  [[ -f "$d/answers.retry.jsonl" ]] || continue
  cat "$d/answers.jsonl" "$d/answers.retry.jsonl" 2>/dev/null \
    | jq -c -s 'sort_by(.id)|group_by(.id)|map(.[-1])[]' > "$d/answers.merged" 2>/dev/null \
    && mv "$d/answers.merged" "$d/answers.jsonl"
done

cat "$RUN"/*/answers.jsonl > "$RUN/all-answers.jsonl" 2>/dev/null
TOT=$(wc -l < "$RUN/all-answers.jsonl")
OK=$(jq -r 'select(.status=="ok")|1' "$RUN/all-answers.jsonl" | wc -l)
TTOK=$(jq -r '[.tokens // 0]|add // 0' "$RUN/all-answers.jsonl" | head -1)
case "$TTOK" in ''|*[!0-9.]*) TTOK=0 ;; esac
DURSUM=$(jq -r '.duration_s // 0' "$RUN/all-answers.jsonl" | awk '{s+=$1} END{printf "%.3f", s+0}')
TPS=$(awk -v t="$TTOK" -v d="$DURSUM" 'BEGIN{if(d>0&&t>0)printf "%.2f",t/d; else print "null"}')
MOK=0; MTOT=0
if [[ -f "$RUN/mutations/answers.jsonl" ]]; then
  MOK=$(jq -r 'select(.status=="ok")|1' "$RUN/mutations/answers.jsonl" | wc -l)
  MTOT=$(wc -l < "$RUN/mutations/answers.jsonl")
fi
WALL=$(jq -r '.wall_seconds // 0' "$RUN"/summary.json 2>/dev/null | head -1)
GENJSON='[]'; [[ -f "$RUN/gen-telemetry.jsonl" ]] && GENJSON=$(jq -c -s '.' "$RUN/gen-telemetry.jsonl" 2>/dev/null || echo '[]')

jq -n --arg cfg "$CFG" --arg lane "$(jq -r '.lane // "unknown"' "$RUN"/summary.json 2>/dev/null)" \
  --arg model "$(jq -r '.model // "unknown"' "$RUN"/summary.json 2>/dev/null)" \
  --arg mode "$(jq -r '.mode // "e2e"' "$RUN"/summary.json 2>/dev/null)" \
  --argjson wall "${WALL:-0}" --argjson expected "$(wc -l < "$RUN/jobs.tsv" 2>/dev/null || echo 0)" \
  --argjson ok "$OK" --argjson fail "$((TOT - OK))" --argjson tokens "$TTOK" \
  --argjson tps "$TPS" --argjson mut_ok "$MOK" --argjson mut_total "$MTOT" \
  --argjson gen "$GENJSON" \
  '{config:$cfg,lane:$lane,model:$model,mode:$mode,rebuilt:true,
    wall_seconds:$wall,jobs_expected:$expected,answers_ok:$ok,answers_fail:$fail,
    tokens_total:$tokens,tps_cumulative:$tps,gen_phase:$gen,
    mutations:{ok:$mut_ok,total:$mut_total}}' > "$RUN/summary.json" \
  && echo "REBUILT $CFG ok=$OK/$TOT tps=$TPS"
