# HANDOFF — 512QA completion with paid models

**Repo:** `xbrd-spark` · **Campaign:** `benchmarks/dry-hump/telemetry-512qa-multi/`
**Date:** 2026-08-22 · **Fleet state:** see `site/index.html` (regenerate: `./gen-site.sh`)

## What this is

The 8×64 moral-dilemma QA campaign across model lanes. Free/open lanes underperformed
or died on quota. This handoff completes the matrix using the **paid lanes only**, which
are proven working as of this date.

## Proven-working invocations (verified 2026-08-22)

| Lane | Command | Proof |
|---|---|---|
| sekhmet / Titanium | `./bench-512qa-v3.sh sekhmet gpt-5.6-luna <cfg> 64` | luna-fast-titanium 512/512, sol 544 rows/0 fail |
| tp / token-plan deepseek | `./bench-512qa-tp.sh tp alibaba-token-plan/deepseek-v4-pro-0813 <cfg> 16` | ds-pro 397/445, ds-flash 445/512 |
| grok | `GROK_EFFORT=high ./bench-512qa-v3.sh grok grok-4.6 <cfg> 16` | grok-4.6-high 515/512 |
| opencode-go | `./bench-512qa-v3.sh opencode opencode-go/glm-5.3 <cfg> 16` | glm-5.3 507/512 |

## Rerun targets, priority order (paid)

Each rerun: `rm -rf telemetry-512qa-multi/runs/<cfg>` first (script reuses stale banks otherwise).

1. **codex-spark-golden** — junk quota-stub (0/544). `sekhmet gpt-5.3-codex-spark codex-spark-golden 64`.
2. **ox-alpha-free** — partial bank only (273 cells vs 512). Full rerun on opencode lane.
3. **ds-pro-0813 residual fails (83)** + **ds-flash-0731 (107)** — full rerun via tp lane
   (`SKIP_MAIN` retry-only mode does NOT exist; banks are cached but answers redo).
   Note ds-pro undergenerated its bank (445≠512) — e2e gen quirk, acceptable or force fixed-bank mode.
4. **qwen3.8-max-low (15 fail)**, **grok-4.6-default (1)**, **grok-4.6-low (16)**, **grok-4.6-high (13)** — cheap top-ups.
5. **hy3-free (158 fail) / mimo-v2.5-free (156)** — retry phases were interrupted mid-run;
   full paid-lane rerun if these models matter, else drop from matrix.

## Hard rules learned here (do not rediscover)

- **Never pass `--direct` to `sekhmet swarm`** — clap-illegal since f34bc59; it aborts every cell with a usage error that looks like a quota failure. Fixed in v1/v2/v3/tp bench scripts (d801efe).
- **Concurrency cap:** ONE titanium instance at a time (j8×8 domains = 64 sparks). Two concurrent instances hit 128 and storm-failed ~40% of cells (recovered by the built-in j16 retry pass, but don't rely on it).
- **Never edit bench scripts while an instance is running** — bash reads by byte offset; in-place edits corrupt live interpreters.
- **Mutations axis refusals are real:** deepseek-v4 both profiles 0%, gpt-5.6-luna 0% empty answers; sol 32/32 and grok partial. Record honestly, do not "fix".
- **Religion bank was edited mid-campaign** (63→68 questions, Aug 21 23:53). Runs before that used the old bank; cross-config religion comparisons are approximate.
- Stale binary hazard: refresh `~/.local/bin/sekhmet` after `cargo install` (PATH shadows `~/.cargo/bin`).
- Big blobs → pastebin.com ONLY (`./scripts/paste-out.sh`), never into chat.

## Verification gates before ANY commit (mandatory posture)

```bash
jq -e . telemetry-512qa-multi/runs/<cfg>/summary.json            # parses
[ "$(jq -r .answers_ok ...)" -ge 400 ]                            # sane count
./gen-site.sh                                                    # site refresh
git add -f telemetry-512qa-multi/runs/<cfg>/summary.json         # runs/ is gitignored
git add site/index.html site/cfg/
git commit -m "bench(dry-hump): <cfg> collected <ok>/<exp> via <lane>; ..."
git push origin main
```

One commit per verified collection. Never commit unverified summaries.
If a summary is torn/empty: `./rebuild-summary.sh <cfg>`, patch lane/model from driver-log evidence, then gate again.
