#!/usr/bin/env bash
# swarm-summary.sh — one-line in-session summary from sekhmet NDJSON (no paste, no bloat).
# Usage: ./scripts/swarm-summary.sh /tmp/swarm.ndjson
set -euo pipefail
F=${1:-/dev/stdin}
if [[ "$F" != /dev/stdin && ! -f "$F" ]]; then
  echo "swarm-summary: missing file: $F" >&2
  exit 1
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "swarm-summary: jq required" >&2
  exit 2
fi
jq -s -c '
  def reasons:
    [.[] | .fail_reason // .provenance.fail_reason // empty];
  {
    lines: length,
    ok: [.[]|select(.status=="ok")]|length,
    fail: [.[]|select(.status=="fail")]|length,
    timeout: [.[]|select(.status=="timeout")]|length,
    error: [.[]|select(.status=="error")]|length,
    usage_tokens_sum: ([.[]|.usage_tokens//empty|numbers]|add//0),
    usage_tokens_n: ([.[]|select(.usage_tokens!=null)]|length),
    fail_reasons: (reasons | group_by(.) | map({(.[0]): length}) | add // {})
  }
' "$F"
