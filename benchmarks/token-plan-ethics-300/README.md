# Token Plan — Ethics-300 + Hard-10 (reason + resourcefulness)

**Lane:** `modelstudio-token-plan` (Alibaba Token Plan Team, `sk-sp-`, intl)  
**Isolation:** sekhmet L3 seeds host `~/.codex` → each spark gets private `CODEX_HOME`; workers always get fnm on PATH.  
**Binary:** `codex-titanium` via `CODEX_BIN`.

## Packs

| Pack | File | N | Measures |
|------|------|---|----------|
| Ethics-300 | `tasks-ethics-300.swarm.md` | 300 | Moral dilemmas across 15 domains; verdict/principle/edge-case/residual-wrong |
| Hard-10 | `hard10/tasks-hard10.swarm.md` | 10 | Pure reasoning + **resourcefulness** (math, CS, physics, econ, bio, law, crypto, geopolitics, meta, constraint-survival) |

## Models (text only)

See `models.txt` (image/audio models on the same plan are skipped for this text bench).

## Run

```bash
# seeds host ~/.codex from token-plan, runs all models, restores sekhmet host
./benchmarks/token-plan-ethics-300/scripts/run-all.sh
# or packs separately:
./benchmarks/token-plan-ethics-300/scripts/run-pack.sh ethics
./benchmarks/token-plan-ethics-300/scripts/run-pack.sh hard10
```

## Metrics

Per model + pack in `runs/<pack>_<model>/summary.json`:

- `sparks_ok` / `fail` / `timeout`
- `wall_seconds` (pack wall clock — **raw throughput**)
- `qps` = ok / wall
- `sekhmet_tokens_*` when present
- `p50/p95_duration_ms` from NDJSON provenance when present

QA narrative: `qa/LEADERBOARD.md` after `scripts/gen-leaderboard.sh`.

## Host CODEX safety

Runner backs up sekhmet host config to `~/.xbgst/codex-lanes/_host-backup-sekhmet/` and restores on EXIT.
