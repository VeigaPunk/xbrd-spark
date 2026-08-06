# Sekhmet L3 bench — x8 × j=64 on gpt-5.6-luna (fast)

**Run:** `x8x64_redo_20260806T230842Z`  
**Gate:** **512/512 ok** · Titanium `codex-titanium` · model `gpt-5.6-luna` · `service_tier=fast`

## What we proved

| Claim | Evidence |
|-------|----------|
| L3 sekhmet substrate works at **j=64** | 8 workers each completed 64-answer swarms |
| **Luna + fast** is viable for mass moral-dilemma load | Full campaign answered on luna/fast |
| **8 independent L3 substrates** in parallel | One sekhmet process tree per domain worker |
| **Godspeed** on dispatch | sekhmet injects directive; tasks + orch whip carry godspeed |
| **Question gen + answer** | Each worker owned 64 generated questions then answered them |

## Scoreboard (redo)

| Worker | ok | tokens (usage sum) |
|--------|---:|-------------------:|
| religion | 64 | (prior complete / whip seed) |
| sex | 64 | (prior complete / whip seed) |
| drugs | 64 | 149931 |
| politics | 64 | 290549 |
| money | 64 | 122722 |
| violence | 64 | 337893 |
| ai | 64 | 177572 |
| charlie-kirk | 64 | 262151 |
| **TOTAL** | **512** | **~1.34M+** (partial token accounting on first two) |

## Honesty — what hurt (TODO)

### 1. tmpfs was not configured properly (TODO)

Live namespaces defaulted under `/tmp` (tmpfs, 16G). The first x8×j=64 blast left **~11–13G** of spark trees, hit **disk quota**, and made swarms fail with **“Disk quota exceeded”**. Panes looked “idle” while the substrate was actually failing on disk, not on model quality.

**TODO (ops / sekhmet defaults):**

- Set durable default `XBRD_SPARK_ROOT` off tmpfs (`~/.cache/xbrd-spark` or project runtime).
- Prefer `--no-keep` + EXIT `gc` for large campaigns.
- Document multi-swarm disk budget before 512-way runs.
- Optional: refuse start if free space on root < N×sparks×estimate.

**Mitigation used this run:** move roots to `~/.cache/sekhmet-x8x64/…`, `--no-keep`, delete round dirs after harvest, free `/tmp`.

### 2. Provider rate_limit under full 512 concurrent

First partial (`x8x64_20260806T230335Z`) landed only **200/512** — dominant fail_reason **`rate_limit`** / **`auth_ws`**. Substrate fan-out was fine; the account/host throttled.

**Mitigation:** stagger, retry-until-64-ok per worker, godspeed orch whip that restarts incomplete workers.

### 3. Substrate works wonders anyway

Once disk + retry were fixed, **luna filled the board**. Godspeed orch kept workers working. Sekhmet j=64 pool + titanium is the right L3 shape for xbgst.

## Reproduce

```bash
. ~/.xbgst/env.l3-sekhmet.sh
# durable root (do NOT rely on /tmp tmpfs for 512-way)
export XBRD_SPARK_ROOT="${XBRD_SPARK_ROOT:-$HOME/.cache/xbrd-spark}"
# 8 domain workers × gen/answer — see runs/x8x64_redo_*/ and tmux sekhmet:bench
tmux attach -t sekhmet \; select-window -t bench
```

## Artifacts

- Per worker: `tasks.txt`, `tasks.id.txt`, `ok_ids.txt`, `summary.json`, ndjson rounds
- Campaign: `summary.json`, `GATE.txt`, this `REPORT.md`
