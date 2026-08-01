# dry-hump — Sekhmet 64-concurrent Titanium benchmark example

**Name:** dry-hump  
**Substrate:** Sekhmet (`xbrd-spark`) on **Codex Titanium**  
**Shape:** 8 domain swarms × 8 sparks = **64 concurrent** Titanium runs  
**Fire model:** barrier (`READY` → simultaneous `FIRE`) then wall-clock to all `DONE`

## Headline result (recorded live)

| Metric | Value |
|--------|------:|
| Wall clock (FIRE → all DONE) | **26.034 s** |
| Total sparks | **64** |
| Status | **64 ok / 0 fail** |
| Per-spark duration | min 3.7s · max 16.4s · avg 9.7s |
| Model | `gpt-5.3-codex-spark` |
| Dispatcher | `codex-titanium` via `sekhmet swarm --direct -j 8` |

Serial lower bound at avg duration: ~10+ minutes. Observed wall **~26s** ⇒ real concurrency.

## Domains (8 labrats)

| Domain | Theme |
|--------|--------|
| `religion` | faith vs reason, divine command, sacred vs civil law |
| `sex` | adult consent / fidelity / sex-work ethics (adults only) |
| `drugs` | decrim, harm reduction, opioids policy dilemmas |
| `politics` | power, whistleblowing, civil disobedience |
| `money` | greed, inheritance, disaster pricing |
| `violence` | self-defense, just war, proportionality (philosophy only) |
| `ai` | displacement, deepfakes, weapons, identity |
| `charlie-kirk` | campus / free-speech / culture-war style dilemmas |

Each domain has:

- `domains/<name>/tasks.txt` — 8 questions (one per line)
- `domains/<name>/results.ndjson` — compact per-spark outcomes
- `domains/<name>/timing.txt` — domain swarm start/end/exit

## Reproduce

### Dry-run smoke (no Titanium, free)

```bash
ROOT=$(mktemp -d)
for d in religion sex drugs politics money violence ai charlie-kirk; do
  sekhmet swarm --dry-run -j 8 \
    --tasks-file benchmarks/dry-hump/domains/$d/tasks.txt \
    --root "$ROOT/$d" >/dev/null &
done
wait
echo "dry-hump dry-run complete under $ROOT"
```

### Live Titanium (costs API; needs auth)

```bash
export CODEX_BIN=$(command -v codex-titanium || command -v codex)
export PATH="$HOME/.cargo/bin:$PATH"
BASE=$(mktemp -d)
START=$(date +%s.%N)
for d in religion sex drugs politics money violence ai charlie-kirk; do
  mkdir -p "$BASE/$d"
  sekhmet swarm --direct -j 8 --timeout 180 \
    --tasks-file benchmarks/dry-hump/domains/$d/tasks.txt \
    --root "$BASE/$d" \
    > "$BASE/$d/ndjson.out" 2> "$BASE/$d/stderr.log" &
done
wait
END=$(date +%s.%N)
awk -v s="$START" -v e="$END" 'BEGIN{printf "wall_seconds=%.3f\n", e-s}'
```

Hard cap: Sekhmet swarm `--jobs` max **64**. This example uses **8×8** parallel processes for organizational isolation by domain; host concurrency peaks near **64** Titanium children.

## Multi-model telemetry

```bash
# requires sekhmet + codex-titanium + jq on PATH
mkdir -p benchmarks/dry-hump/telemetry-12x/runs/rNN_label
./benchmarks/dry-hump/run-once-telemetry.sh rNN_label \
  benchmarks/dry-hump/telemetry-12x/runs/rNN_label \
  'provider/model-label'
```

`run-once-telemetry.sh` is **bash + jq only** (no Python). Swarms keep namespaces until token aggregation (reads `result.json` / `usage_tokens`), then the EXIT trap always `rm -rf`s the temp root so tmpfs is not exhausted.

Recorded summaries live under `telemetry-12x/runs/*/summary.json` (compact; live codex homes stay off-tree).

## Notes

- Live namespaces under `/tmp` are **not** checked in (often multi-GB of codex homes). Prefer private roots and delete after harvest.
- After large campaigns, free space: `rm -rf /tmp/sekhmet-*` (or use the telemetry trap cleanup).
- Questions are moral/policy dilemmas for load-gen + qualitative stress, not product advice.
- Recorded run: 2026-08-01T22:40:38Z → 22:41:04Z UTC.


## Verbatim command + output

Full command transcripts and Titanium NDJSON streams:

- [`verbatim/COMMANDS.md`](verbatim/COMMANDS.md) — command blocks
- [`verbatim/run-dry.session.txt`](verbatim/run-dry.session.txt) — dry-run `$` + stdout
- [`verbatim/run-live-recorded.session.txt`](verbatim/run-live-recorded.session.txt) — live session reconstruction + timings
- [`verbatim/run-live-all-domains.ndjson.txt`](verbatim/run-live-all-domains.ndjson.txt) — full live NDJSON (all domains)
- [`verbatim/live-ndjson-full/`](verbatim/live-ndjson-full/) — per-domain raw `sekhmet swarm` stdout
- [`verbatim/parent-fire-barrier.session.log`](verbatim/parent-fire-barrier.session.log) — parent READY/FIRE/DONE monitor (if present)


## Multi-model leaderboard

See [`telemetry-12x/LEADERBOARD.md`](telemetry-12x/LEADERBOARD.md) (regenerate with `./gen-leaderboard.sh`).

## Redo protocol (`*b` runs)

If an OpenCode-orchestrated primary run records `sparks_ok: 0` (agent never invoked sekhmet), re-harvest with:

```bash
./benchmarks/dry-hump/run-once-telemetry.sh rNNb_<slug> \
  benchmarks/dry-hump/telemetry-12x/runs/rNNb_<slug> \
  'provider/model'
./benchmarks/dry-hump/gen-leaderboard.sh
```

`run-once-telemetry.sh` uses `--no-keep` and deletes temp roots on EXIT. Prefer **direct** sekhmet when opencode skips the harness.
