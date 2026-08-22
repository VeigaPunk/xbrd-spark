#!/usr/bin/env bash
# xask_shim.sh — flag compatibility + truthful tier selection.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
SHIM="$ROOT/scripts/xask"

default_out="$($SHIM -d --model-id gpt-5.6-luna -e low --spark --gs codex ping)"
grep -E 'sekhmet|gpt-5.6-luna|model_reasoning_effort=low|service_tier=default' <<<"$default_out"

fast_out="$($SHIM -d --service-tier fast --model-id gpt-5.6-luna -e low --spark --gs codex ping)"
grep -F 'XBRD_SPARK_SERVICE_TIER=fast' <<<"$fast_out"
grep -F 'service_tier=fast' <<<"$fast_out"

if "$SHIM" -d --service-tier flex codex ping >/dev/null 2>&1; then
  echo "xask_shim: unsupported tier unexpectedly succeeded" >&2
  exit 1
fi
echo "xask_shim: OK"
