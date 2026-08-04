# dry-hump — OAuth `gpt-5.6-luna` + `service_tier=fast`

**Auth:** ChatGPT OAuth (Codex Titanium), not platform API key.  
**Shape:** 8 domains × 8 jobs = **64 concurrent** Titanium sparks via `sekhmet swarm --direct -j 8`.  
**Run id:** `luna_oauth_fast_20260804T164452Z`

## Config

| Knob | Value |
|------|--------|
| `XBRD_SPARK_MODEL` | `gpt-5.6-luna` |
| `XBRD_SPARK_SERVICE_TIER` | `fast` (Codex Fast mode; maps to priority processing) |
| `model_reasoning_effort` | `low` (always set by sekhmet) |
| `XBRD_SPARK_FALLBACK_MODEL` | `none` (direct luna measurement) |

Codex CLI flags (per spark):

```text
codex-titanium exec -m gpt-5.6-luna \
  -c model_reasoning_effort=low \
  -c service_tier=fast \
  ...
```

Docs: [Codex config `service_tier`](https://developers.openai.com/codex/config-reference) — `fast` maps to request value `priority`.

## Headline metrics

| Metric | Value |
|--------|------:|
| Wall (FIRE → all DONE) | **44.577 s** |
| Sparks | **64 ok / 0 fail / 0 timeout** |
| NDJSON lines | 64 |
| Tokens total | **514 610** |
| Tokens / spark | min 4853 · max 15316 · **avg 8040.8** |
| Baseline (spark, prior) | 26.034 s wall · 64 ok (README dry-hump headline) |

## Per-domain wall (seconds)

| Domain | Domain wall ≈ | ok | tokens |
|--------|---------------:|---:|-------:|
| religion | 32.9 | 8 | 76546 |
| sex | 37.7 | 8 | 67251 |
| drugs | 32.7 | 8 | 62484 |
| politics | 44.6 | 8 | 54633 |
| money | 38.8 | 8 | 68225 |
| violence | 24.1 | 8 | 57897 |
| ai | 24.2 | 8 | 55416 |
| charlie-kirk | 37.4 | 8 | 72158 |

## Artifacts

- `runs/luna_oauth_fast_20260804T164452Z/summary.json`
- `runs/luna_oauth_fast_20260804T164452Z/run.log`

Reproduce:

```bash
export XBRD_SPARK_MODEL=gpt-5.6-luna
export XBRD_SPARK_FALLBACK_MODEL=none
export XBRD_SPARK_SERVICE_TIER=fast
./benchmarks/dry-hump/run-once-telemetry.sh luna_oauth_fast_rerun \
  benchmarks/dry-hump/telemetry-luna-oauth-fast/runs/luna_oauth_fast_rerun \
  'oauth/gpt-5.6-luna+service_tier=fast'
```
