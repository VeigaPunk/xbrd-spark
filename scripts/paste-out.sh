#!/usr/bin/env bash
# paste-out.sh — post large sekhmet/swarm dumps to Pastebin ONLY; print URL only.
#
# HARD RULE (non-negotiable for sekhmet / xbrd-spark agents):
#   Host: pastebin.com ONLY (api.pastebin.com / pastebin.com).
#   No litterbox, catbox, 0x0, paste.rs, dpaste, hastebin, gist-as-fallback,
#   transfer.sh, or any other paste/file host. If Pastebin fails: exit non-zero
#   and report — do NOT switch backends.
#
# Usage:
#   sekhmet swarm ... > /tmp/swarm.ndjson 2> /tmp/swarm.err
#   ./scripts/paste-out.sh /tmp/swarm.ndjson
#   ./scripts/paste-out.sh /tmp/swarm.err
#   jq -c . /tmp/swarm.ndjson | ./scripts/paste-out.sh
#
# Env (required):
#   PASTEBIN_API_DEV_KEY  — API dev key from https://pastebin.com/doc_api
# Optional:
#   PASTEBIN_API_USER_KEY — user key for pastes under your account
#   PASTEBIN_EXPIRE       — N / 10M / 1H / 1D / 1W / 2W / 1M / 6M / 1Y (default 1M)
#   PASTEBIN_PRIVATE      — 0 public | 1 unlisted | 2 private (default 1)
#   PASTE_NAME            — paste title / filename label
set -euo pipefail

API_URL="https://pastebin.com/api/api_post.php"
EXPIRE=${PASTEBIN_EXPIRE:-1M}
PRIVATE=${PASTEBIN_PRIVATE:-1}
TMP=
cleanup() { [[ -n "${TMP:-}" && -f "$TMP" ]] && rm -f "$TMP"; }
trap cleanup EXIT

if [[ -z "${PASTEBIN_API_DEV_KEY:-}" ]]; then
  echo "paste-out: PASTEBIN_API_DEV_KEY is required (pastebin.com only — no fallback hosts)" >&2
  echo "paste-out: create a free key at https://pastebin.com/doc_api and export it" >&2
  exit 2
fi

if [[ $# -ge 1 && -f "$1" ]]; then
  FILE=$1
  NAME=${PASTE_NAME:-$(basename "$1")}
else
  TMP=$(mktemp)
  cat > "$TMP"
  FILE=$TMP
  NAME=${PASTE_NAME:-paste.txt}
fi

BYTES=$(wc -c < "$FILE" | tr -d ' ')
if [[ "$BYTES" -eq 0 ]]; then
  echo "paste-out: empty input" >&2
  exit 1
fi

# Pastebin free tier ~512 KiB per paste; refuse silently oversized so agents split
MAX=${PASTEBIN_MAX_BYTES:-524288}
if [[ "$BYTES" -gt "$MAX" ]]; then
  echo "paste-out: input is ${BYTES} bytes > pastebin limit ${MAX}; split the file and re-run" >&2
  exit 4
fi

CODE=$(cat "$FILE")

# Build form; never print the key
ARGS=(
  -fsS --max-time 90
  -d "api_option=paste"
  -d "api_dev_key=${PASTEBIN_API_DEV_KEY}"
  --data-urlencode "api_paste_code@${FILE}"
  --data-urlencode "api_paste_name=${NAME}"
  -d "api_paste_format=text"
  -d "api_paste_private=${PRIVATE}"
  -d "api_paste_expire_date=${EXPIRE}"
)
if [[ -n "${PASTEBIN_API_USER_KEY:-}" ]]; then
  ARGS+=(-d "api_user_key=${PASTEBIN_API_USER_KEY}")
fi

RESP=$(curl "${ARGS[@]}" "$API_URL" 2>/tmp/paste-out.curl.err || true)
RESP=$(printf '%s' "$RESP" | tr -d '\r' | head -n1 | tr -d '[:space:]')

if [[ -z "$RESP" ]]; then
  echo "paste-out: empty response from pastebin.com (see curl err)" >&2
  [[ -s /tmp/paste-out.curl.err ]] && head -c 400 /tmp/paste-out.curl.err >&2
  echo >&2
  exit 3
fi

# API errors look like: Bad API request, ...
if [[ "$RESP" != http* ]]; then
  echo "paste-out: pastebin.com rejected paste: $RESP" >&2
  echo "paste-out: NO FALLBACK — fix key/quota/size; do not use other hosts" >&2
  exit 3
fi

# Must be pastebin.com
case "$RESP" in
  https://pastebin.com/*|http://pastebin.com/*) ;;
  *)
    echo "paste-out: refusing non-pastebin URL: $RESP" >&2
    exit 3
    ;;
esac

printf '%s\n' "$RESP"
echo "paste-out: backend=pastebin.com bytes=$BYTES name=$NAME" >&2
exit 0
