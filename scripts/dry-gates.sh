#!/usr/bin/env bash
# dry-gates.sh — network-free sekhmet ship gates (no Titanium, no Pastebin).
# Use while provider quota is exhausted or offline.
set -euo pipefail
ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

echo "== cargo check =="
cargo check --all-targets

echo "== cargo test =="
cargo test

echo "== cargo clippy =="
cargo clippy --all-targets -- -D warnings

echo "== release build =="
cargo build --release

echo "== --help / --version =="
./target/release/sekhmet --help >/dev/null
./target/release/sekhmet --version
./target/release/xbrd-spark --version

echo "== dry-run swarm (3 tasks) =="
TMP=$(mktemp -d)
printf 'alpha\nbeta\ngamma\n' > "$TMP/tasks.txt"
./target/release/sekhmet swarm --dry-run -j 3 --no-keep \
  --tasks-file "$TMP/tasks.txt" --root "$TMP/root" \
  > "$TMP/ndjson.out" 2>"$TMP/err"
./scripts/swarm-summary.sh "$TMP/ndjson.out"
LINES=$(wc -l < "$TMP/ndjson.out" | tr -d ' ')
test "$LINES" = "3"
rm -rf "$TMP"

echo "== paste-out offline check (expect exit 2 without PASTEBIN_API_DEV_KEY) =="
if [[ -z "${PASTEBIN_API_DEV_KEY:-}" ]]; then
  set +e
  echo "offline" | ./scripts/paste-out.sh >/dev/null 2>"$ROOT/target/paste-out-offline.err"
  EC=$?
  set -e
  test "$EC" = "2"
  grep -q 'PASTEBIN_API_DEV_KEY' "$ROOT/target/paste-out-offline.err"
  echo "paste-out offline: exit 2 as required (no key, no fallback)"
else
  echo "paste-out: key present — skip offline negative test"
fi

echo "ALL DRY GATES GREEN"
