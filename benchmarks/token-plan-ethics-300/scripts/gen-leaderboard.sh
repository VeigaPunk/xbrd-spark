#!/usr/bin/env bash
set -euo pipefail
BENCH="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$BENCH/qa/LEADERBOARD.md"
mkdir -p "$BENCH/qa"

{
  echo "# Token Plan leaderboard — Ethics-300 + Hard-10"
  echo
  echo "Generated: $(date -u -Iseconds)"
  echo
  echo "## Hard-10 (reasoning + resourcefulness)"
  echo
  echo "| model | ok | fail | timeout | wall_s | qps | tok_total | p50_ms | p95_ms |"
  echo "|-------|----|------|---------|--------|-----|-----------|--------|--------|"
  for s in "$BENCH"/runs/hard10_*/summary.json; do
    [[ -f "$s" ]] || continue
    jq -r '[.model, .sparks_ok, .sparks_fail, .sparks_timeout, .wall_seconds, .qps, .sekhmet_tokens_total, (.p50_duration_ms//"-"), (.p95_duration_ms//"-")] | @tsv' "$s" \
      | awk -F'\t' '{printf "| %s | %s | %s | %s | %s | %s | %s | %s | %s |\n",$1,$2,$3,$4,$5,$6,$7,$8,$9}'
  done
  echo
  echo "## Ethics-300 (moral dilemmas + throughput)"
  echo
  echo "| model | ok | fail | timeout | wall_s | qps | tok_total | tok/s | p50_ms | p95_ms |"
  echo "|-------|----|------|---------|--------|-----|-----------|-------|--------|--------|"
  for s in "$BENCH"/runs/ethics_*/summary.json; do
    [[ -f "$s" ]] || continue
    jq -r '[.model, .sparks_ok, .sparks_fail, .sparks_timeout, .wall_seconds, .qps, .sekhmet_tokens_total, .tokens_per_sec, (.p50_duration_ms//"-"), (.p95_duration_ms//"-")] | @tsv' "$s" \
      | awk -F'\t' '{printf "| %s | %s | %s | %s | %s | %s | %s | %s | %s | %s |\n",$1,$2,$3,$4,$5,$6,$7,$8,$9,$10}'
  done
  echo
  echo "## Notes"
  echo
  echo "- Lane: Token Plan Team intl (\`modelstudio-token-plan\`)."
  echo "- Speed = pack wall clock and sparks ok/s under sekhmet concurrency."
  echo "- Hard-10 grades quality offline (reason + resourcefulness sections); ethics grades dilemma structure (verdict/principle/edge/residual)."
  echo "- Host \`~/.codex\` restored to sekhmet backup after each pack run."
} >"$OUT"
echo "wrote $OUT"
