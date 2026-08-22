#!/usr/bin/env bash
# xask_shim.sh — flag compatibility + truthful tier selection.
set -euo pipefail

ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
SHIM="$ROOT/scripts/xask"

# Sanitize ambient model env so default-slug assertions are host-env independent.
default_out="$(env -u XBRD_SPARK_MODEL -u XBRD_SPARK_FALLBACK_MODEL \
  "$SHIM" -d --spark --gs codex ping)"
grep -F 'XBRD_SPARK_MODEL=gpt-5.3-codex-spark' <<<"$default_out"
if grep -F 'XBRD_SPARK_FALLBACK_MODEL=none' <<<"$default_out" >/dev/null; then
  echo "xask_shim: fallback must not be hardcoded to none" >&2
  exit 1
fi

luna_out="$(env -u XBRD_SPARK_MODEL -u XBRD_SPARK_FALLBACK_MODEL \
  "$SHIM" -d --model-id gpt-5.6-luna -e low --spark --gs codex ping)"
grep -E 'sekhmet|gpt-5.6-luna|model_reasoning_effort=low|service_tier=default' <<<"$luna_out"

fast_out="$(env -u XBRD_SPARK_MODEL -u XBRD_SPARK_FALLBACK_MODEL \
  "$SHIM" -d --service-tier fast --model-id gpt-5.6-luna -e low --spark --gs codex ping)"
grep -F 'XBRD_SPARK_SERVICE_TIER=fast' <<<"$fast_out"
grep -F 'service_tier=fast' <<<"$fast_out"

if env -u XBRD_SPARK_MODEL -u XBRD_SPARK_FALLBACK_MODEL "$SHIM" -d --service-tier flex codex ping >/dev/null 2>&1; then
  echo "xask_shim: unsupported tier unexpectedly succeeded" >&2
  exit 1
fi
echo "xask_shim: OK"
