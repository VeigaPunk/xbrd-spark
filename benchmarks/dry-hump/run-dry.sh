#!/usr/bin/env bash
# dry-hump dry-run: 8 domains × 8 tasks, concurrent swarms, no Titanium
set -euo pipefail
ROOT=${1:-$(mktemp -d /tmp/sekhmet-dry-hump-XXXXXX)}
HERE=$(cd "$(dirname "$0")" && pwd)
export PATH="${HOME}/.cargo/bin:${PATH}"
echo "dry-hump dry-run root=$ROOT"
START=$(date +%s.%N)
for d in religion sex drugs politics money violence ai charlie-kirk; do
  mkdir -p "$ROOT/$d"
  sekhmet swarm --dry-run -j 8 \
    --tasks-file "$HERE/domains/$d/tasks.txt" \
    --root "$ROOT/$d" \
    > "$ROOT/$d/ndjson.out" 2> "$ROOT/$d/stderr.log" &
done
wait
END=$(date +%s.%N)
awk -v s="$START" -v e="$END" 'BEGIN{printf "wall_seconds=%.3f\n", e-s}'
ok=0
for d in religion sex drugs politics money violence ai charlie-kirk; do
  c=$(grep -c '"status":"ok"' "$ROOT/$d/ndjson.out" || true)
  ok=$((ok+c))
done
echo "ok_records=$ok expected=64"
