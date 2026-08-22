#!/usr/bin/env bash
# bench-status.sh — one-shot fleet snapshot for inline polling
B=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump/telemetry-512qa-multi/runs
L=/home/vgpnk/Projects/xbgst/xbrd-spark/benchmarks/dry-hump/telemetry-512qa-multi/logs
printf '%-32s %-9s %s\n' "CONFIG" "STATE" "DETAIL"
for c in "$B"/*/; do
  n=$(basename "$c")
  s="$c/summary.json"
  if [[ -s "$s" ]] && jq -e . "$s" >/dev/null 2>&1; then
    row=$(jq -c '{w:.wall_seconds,ok:.answers_ok,exp:.jobs_expected,fail:.answers_fail,tps:.tps_cumulative,mut:.mutations}' "$s")
    printf '%-32s %-9s %s\n' "$n" "DONE" "$row"
  else
    gen=$(wc -l < "$c/gen-telemetry.jsonl" 2>/dev/null || echo 0)
    ans=$(cat "$c"/[a-z]*/answers.jsonl 2>/dev/null | wc -l)
    exp=$(wc -l < "$c/jobs.tsv" 2>/dev/null || echo '?')
    alive=$(pgrep -cf "bench-512qa-v[23].sh .* $n" 2>/dev/null || true)
    [[ "$alive" =~ ^[0-9]+$ && "$alive" -gt 0 ]] && st=LIVE || st=DEAD
    printf '%-32s %-9s gen:%s ans:%s/%s\n' "$n" "$st" "$gen/8" "$ans" "$exp"
  fi
done
echo "--- queue tail ---"
tail -2 "$L/grok-family-queue.log" 2>/dev/null | sed 's/^/  /'
echo "--- load ---"
uptime | sed 's/^/  /'; free -g | awk 'NR==2{print "  mem_used:"$3"G avail:"$7"G"}'
