# QA rebuild status

## Blocker (2026-08-01)

Titanium **usage limit** for `gpt-5.3-codex-spark` (and related Codex websocket path):

```
ERROR: You've hit your usage limit for GPT-5.3-Codex-Spark.
Switch to another model now, or try again at Aug 8th, 2026 7:26 PM.
```

Also observed: websocket `403` to `chatgpt.com/backend-api/codex/responses` and models refresh `429`.

## Failed rebuild swarms

QA rebuild (`ndjson.qa.out`) produced **0 ok** answers (empty). Root cause is provider quota, not sekhmet substrate.

Orch **stopped** the in-flight e2e10–e2e12 burn (2026-08-01) to avoid further quota waste; tmp `sekhmet-qa-rebuild-*` cleaned.

| Run | lines | ok | fail | notes |
|-----|------:|---:|-----:|-------|
| e2e01–e2e05, e2e07–e2e09 | 64 | 0 | 64 | full swarm fail |
| e2e06 nemotron | 63 | 0 | 63 | gen had 63 qs |
| e2e10–e2e12 | partial/none | 0 | — | aborted under quota |

## What still works

- Prior **e2e model-questions** campaign (e2e01–e2e12) already recorded successful Titanium swarms with `usage_tokens` / `sekhmet_tokens_*` before quota exhaustion.
- `sekhmet` dry-run / gates remain green (no network).
- Large NDJSON for e2e01–e2e08 posted via litterbox (`PASTE_URLS.txt`). e2e09+ local-only when paste hosts rate-limit.

## Next action when unblocked (after 2026-08-08 or non-spark model)

```bash
cargo install --path . --force
export CODEX_BIN=$(command -v codex-titanium)
sekhmet swarm --direct -j 8 --timeout 180 \
  --tasks-file runs/e2e01_.../tasks.txt --root "$(mktemp -d)" \
  > /tmp/qa.ndjson 2> /tmp/qa.err
jq -s '{lines:length, ok:[.[]|select(.status=="ok")]|length}' /tmp/qa.ndjson
./scripts/paste-out.sh /tmp/qa.ndjson
```

Do **not** burn more Titanium spark quota until the window reopens.
