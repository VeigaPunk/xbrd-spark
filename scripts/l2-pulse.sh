#!/usr/bin/env bash
# One L2 -> L3 callability pulse. Never fans out.
set -euo pipefail

usage() {
  cat <<'EOF'
Usage: l2-pulse.sh [--dry-run] [--route-id ID]

Runs exactly one read-only Sekhmet spark with a 90-second wall timeout.
If the Titanium OAuth preflight is blocked, the script runs one labeled dry
pulse instead. Artifacts default to ~/.xbgst/evidence/sekhmet-l3-pulses.
EOF
}

dry_run=0
route_id=""
while (($#)); do
  case "$1" in
    --dry-run)
      dry_run=1
      shift
      ;;
    --route-id)
      [[ $# -ge 2 ]] || { echo "l2-pulse: --route-id needs a value" >&2; exit 2; }
      route_id=$2
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "l2-pulse: unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

env_file=${SEKHMET_L3_ENV_FILE:-"$HOME/.xbgst/env.l3-sekhmet.sh"}
if [[ -r "$env_file" ]]; then
  # shellcheck disable=SC1090
  . "$env_file"
fi

# This wrapper is a one-spark L2 lane even when a broader host default exists.
export XBRD_SPARK_JOBS=1

stamp=$(date -u +%Y%m%dT%H%M%SZ)
printf -v nonce '%04x' "$((RANDOM & 65535))"
route_id=${route_id:-"L2-L3-${stamp}-${nonce}"}
spark_id="sp-pulse-${stamp}-${nonce}"
root=${SEKHMET_L3_PULSE_ROOT:-"$HOME/.xbgst/evidence/sekhmet-l3-pulses"}
mkdir -p "$root"

command -v sekhmet >/dev/null || {
  echo "l2-pulse: sekhmet is not on PATH" >&2
  exit 127
}

mode=live
if ((dry_run)); then
  mode=dry-run-requested
else
  codex_cmd=${CODEX_BIN:-codex-titanium}
  if ! codex_path=$(command -v "$codex_cmd" 2>/dev/null); then
    mode=dry-run-blocked-dispatcher
    dry_run=1
  elif ! auth_status=$("$codex_path" login status 2>&1); then
    printf 'l2-pulse: OAuth preflight blocked: %s\n' "$auth_status" >&2
    mode=dry-run-blocked-oauth
    dry_run=1
  fi
fi

printf 'route_id=%s\nspark_id=%s\npulse_mode=%s\nroot=%s\n' \
  "$route_id" "$spark_id" "$mode" "$root" >&2

cmd=(
  sekhmet run
  --id "$spark_id"
  --ro
  --timeout 90
  --task "Route $route_id. Read-only callability pulse. Reply with exactly: SEKHMET_L3_PULSE_OK | godspeed"
  --root "$root"
)
if ((dry_run)); then
  cmd+=(--dry-run)
fi

# A stale force-fallback setting could start a second model attempt. This lane
# permits one dispatcher attempt only.
env -u XBRD_SPARK_USE_FALLBACK "${cmd[@]}"
