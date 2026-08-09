#!/usr/bin/env bash
# hard10 first (validate models), then ethics-300 (speed + moral volume)
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
export PATH="${HOME}/.cargo/bin:${HOME}/.local/bin:${PATH}"

echo "=== HARD10 (reason + resourcefulness) ==="
bash "$HERE/run-pack.sh" hard10

echo "=== ETHICS-300 (moral dilemmas + raw speed) ==="
bash "$HERE/run-pack.sh" ethics

bash "$HERE/gen-leaderboard.sh"
echo "ALL DONE -> $HERE/../qa/LEADERBOARD.md"
