#!/usr/bin/env bash
set -euo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
OUT="$HERE/LEADERBOARD.md"
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT
: > "$TMP"
for p in "$HERE"/runs/*/summary.json; do
  [[ -f "$p" ]] || continue
  jq -r '[
    (.run_id // "unknown"),
    (.model // "unknown"),
    ((.swarm_wall_seconds // .e2e_wall_seconds // 0)|tostring),
    ((.sparks_ok // 0)|tostring),
    ((.sparks_fail // 0)|tostring),
    ((.sparks_timeout // 0)|tostring),
    ((.sekhmet_tokens_total // 0)|tostring),
    ((.sekhmet_tokens_avg // 0)|tostring),
    ((.questions // 0)|tostring)
  ] | @tsv' "$p" >> "$TMP"
done
{
  echo "# e2e model-questions leaderboard"
  echo
  echo "OpenCode generates 64 questions → Sekhmet Titanium swarm answers them."
  echo
  echo "## Complete (64 ok)"
  echo
  echo "| rank | run_id | model | swarm wall s | ok | fail | timeout | sekhmet tokens | tok/spark avg | questions |"
  echo "|-----:|--------|-------|-------------:|---:|-----:|--------:|---------------:|--------------:|----------:|"
  awk -F'\t' '$4=="64"' "$TMP" | sort -t$'\t' -k3,3n | awk -F'\t' '{
    printf "| %d | `%s` | `%s` | %s | %s | %s | %s | %s | %s | %s |\n", NR,$1,$2,$3,$4,$5,$6,$7,$8,$9
  }'
  echo
  echo "## Incomplete"
  echo
  echo "| run_id | model | swarm wall s | ok | fail | timeout | tokens |"
  echo "|--------|-------|-------------:|---:|-----:|--------:|-------:|"
  awk -F'\t' '$4!="64"' "$TMP" | sort -t$'\t' -k1,1 | awk -F'\t' '{
    printf "| `%s` | `%s` | %s | %s | %s | %s | %s |\n", $1,$2,$3,$4,$5,$6,$7
  }'
  echo
  echo "_Regenerate: \`./gen-leaderboard.sh\`_"
} > "$OUT"
echo "wrote $OUT"
