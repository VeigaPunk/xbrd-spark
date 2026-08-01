# QA rebuild status

## Blocker (2026-08-01)

Titanium **usage limit** for `gpt-5.3-codex-spark` (and related Codex websocket path):

```
ERROR: You've hit your usage limit for GPT-5.3-Codex-Spark.
Switch to another model now, or try again at Aug 8th, 2026 7:26 PM.
```

Also observed: websocket `403` to `chatgpt.com/backend-api/codex/responses` and models refresh `429`.

## Failed rebuild swarms

QA rebuild runs (`ndjson.qa.out`) completed with **0 ok** (empty answers). Root cause is provider quota, not sekhmet substrate.

| Run | lines | ok | fail | notes |
|-----|------:|---:|-----:|-------|
| e2e01–e2e05, e2e07–e2e09 | 64 | 0 | 64 | full swarm fail |
| e2e06 nemotron | 63 | 0 | 63 | gen had 63 qs |
| e2e10 gpt-5.5-fast | 1 | 0 | 1 | aborted mid-campaign |
| e2e11–e2e12 | — | — | — | not started (quota) |

## What still works

- Prior **e2e model-questions** campaign (e2e01–e2e12) already recorded successful Titanium swarms with `usage_tokens` / `sekhmet_tokens_*` before quota exhaustion.
- `sekhmet` dry-run / gates remain green (no network).
- `scripts/paste-out.sh` posts large NDJSON via litterbox (see `PASTE_URLS.txt`).

## Next action when unblocked (after 2026-08-08 or non-spark model)

```bash
cargo install --path . --force
export CODEX_BIN=$(command -v codex-titanium)
# after Aug 8 2026 or with a non-spark model:
sekhmet swarm --direct -j 8 --timeout 180 \
  --tasks-file runs/e2e01_.../tasks.txt --root "$(mktemp -d)" \
  > /tmp/qa.ndjson 2> /tmp/qa.err
jq -s '{lines:length, ok:[.[]|select(.status=="ok")]|length}' /tmp/qa.ndjson
./scripts/paste-out.sh /tmp/qa.ndjson
```

Do **not** burn more Titanium spark quota until the window reopens.
