#!/usr/bin/env bash
# xask_shim.sh — flag-compat + forced --direct smoke
set -euo pipefail

command -v xask
xask -d --model-id gpt-5.6-luna -e low --spark --gs codex ping | tee /tmp/xask-m3.debug
grep -E 'sekhmet|gpt-5.6-luna|model_reasoning_effort=low|service_tier=fast' /tmp/xask-m3.debug
echo "xask_shim: OK"
