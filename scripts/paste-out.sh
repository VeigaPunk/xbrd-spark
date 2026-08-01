#!/usr/bin/env bash
# paste-out.sh — post large sekhmet/swarm dumps to a paste host; print URL only.
#
# DIRECTIVE (sekhmet owners / agents):
#   Large outputs, logs, swarm NDJSON dumps, and multi-KB blobs MUST go to a paste
#   service. In-session keep only the URL + a short summary — never dump multi-KB
#   into the TUI/chat.
#
# Usage:
#   sekhmet swarm ... > /tmp/swarm.ndjson 2> /tmp/swarm.err
#   ./scripts/paste-out.sh /tmp/swarm.ndjson
#   ./scripts/paste-out.sh /tmp/swarm.err
#   jq -c . /tmp/swarm.ndjson | ./scripts/paste-out.sh
#
# Env:
#   PASTE_BACKEND=paste_rs|0x0|catbox|litterbox|dpaste|auto
#   PASTE_NAME=filename.ext
set -euo pipefail

BACKEND=${PASTE_BACKEND:-auto}
TMP=
cleanup() { [[ -n "${TMP:-}" && -f "$TMP" ]] && rm -f "$TMP"; }
trap cleanup EXIT

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

upload_one() {
  local be=$1
  case "$be" in
    paste_rs|paste.rs)
      # paste.rs often 500s on large NDJSON; skip >64KiB in auto path
      if [[ "$BYTES" -gt 65536 ]]; then
        return 1
      fi
      curl -fsS --max-time 60 --data-binary @"$FILE" https://paste.rs
      ;;
    0x0|0x0.st)
      curl -fsS --max-time 60 -F "file=@${FILE};filename=${NAME}" https://0x0.st
      ;;
    catbox|catbox.moe)
      curl -fsS --max-time 90 -F "reqtype=fileupload" -F "fileToUpload=@${FILE}" \
        https://catbox.moe/user/api.php
      ;;
    litterbox)
      # temporary 1h–72h file host; reliable when catbox/0x0 flaky
      curl -fsS --max-time 90 -F "reqtype=fileupload" -F "time=72h" \
        -F "fileToUpload=@${FILE}" \
        https://litterbox.catbox.moe/resources/internals/api.php
      ;;
    dpaste)
      # text only; good for medium dumps
      if [[ "$BYTES" -gt 500000 ]]; then
        return 1
      fi
      curl -fsS --max-time 60 --data-urlencode "content@${FILE}" \
        -d "syntax=text" -d "expiry_days=30" \
        https://dpaste.com/api/v2/
      ;;
    *)
      return 2
      ;;
  esac
}

try_backends() {
  local list=("$@")
  local be url
  for be in "${list[@]}"; do
    if url=$(upload_one "$be" 2>/dev/null); then
      url=$(printf '%s' "$url" | tr -d '\r' | awk 'NR==1{print; exit}')
      if [[ -n "$url" && "$url" == http* ]]; then
        USED=$be
        URL=$url
        return 0
      fi
    fi
    echo "paste-out: backend=$be failed, trying next…" >&2
  done
  return 1
}

USED=
URL=
case "$BACKEND" in
  auto)
    # Prefer durable free hosts first; litterbox before flaky catbox/0x0 spam walls
    try_backends litterbox paste_rs dpaste catbox 0x0 || {
      echo "paste-out: all backends failed" >&2
      exit 3
    }
    ;;
  *)
    URL=$(upload_one "$BACKEND") || {
      echo "paste-out: backend=$BACKEND failed" >&2
      exit 3
    }
    USED=$BACKEND
    ;;
esac

printf '%s\n' "$URL"
echo "paste-out: backend=$USED bytes=$BYTES name=$NAME" >&2
