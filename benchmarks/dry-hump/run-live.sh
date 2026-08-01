#!/usr/bin/env bash
# dry-hump LIVE: 8×8 concurrent Codex Titanium via sekhmet (costs API)
set -euo pipefail
export CODEX_BIN=${CODEX_BIN:-$(command -v codex-titanium || command -v codex)}
export PATH="${HOME}/.cargo/bin:${PATH}"
ROOT=${1:-$(mktemp -d /tmp/sekhmet-dry-hump-live-XXXXXX)}
HERE=$(cd "$(dirname "$0")" && pwd)
echo "dry-hump LIVE root=$ROOT CODEX_BIN=$CODEX_BIN"
START=$(date +%s.%N)
for d in religion sex drugs politics money violence ai charlie-kirk; do
  mkdir -p "$ROOT/$d"
  sekhmet swarm --direct -j 8 --timeout 180 \
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
  echo "$d ok=$c"
done
echo "ok_records=$ok expected=64"
