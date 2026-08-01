# scope-fanout-64 — concurrent scope snapshot stress

Public **synthetic** 64-section payload for `sekhmet swarm -j 64 --scope`.
Unlike copyrighted screenplay dumps, these files are repo-safe load generators.

## Dry-run (free)

```bash
export PATH="$HOME/.cargo/bin:$PATH"
ROOT=$(mktemp -d)
sekhmet swarm --dry-run -j 64 --no-keep \
  --scope benchmarks/scope-fanout-64/payload \
  --tasks-file benchmarks/scope-fanout-64/tasks.txt \
  --root "$ROOT" | tee benchmarks/scope-fanout-64/results/dry-ndjson.out
```

Expect 64 NDJSON lines with `status=ok` (dry-run stubs).

## Live Titanium (API cost)

```bash
export CODEX_BIN=$(command -v codex-titanium || command -v codex)
ROOT=$(mktemp -d)
sekhmet swarm --direct -j 64 --timeout 180 --no-keep \
  --scope benchmarks/scope-fanout-64/payload \
  --tasks-file benchmarks/scope-fanout-64/tasks.txt \
  --root "$ROOT" | tee benchmarks/scope-fanout-64/results/live-ndjson.out
```

## Notes

- Hard cap remains **64** concurrent runners.
- Prefer `--no-keep` so tmpfs does not fill with codex homes.
- Do **not** commit third-party screenplays here; keep those local/out-of-tree.
