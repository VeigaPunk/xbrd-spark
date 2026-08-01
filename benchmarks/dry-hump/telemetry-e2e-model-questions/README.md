# telemetry-e2e-model-questions

End-to-end stress: **OpenCode model generates 64 moral-dilemma questions**, then **Sekhmet** runs them as a 64-way Titanium swarm (`-j 8` or host max).

Unlike `telemetry-12x` (fixed dry-hump domain packs), each model produces its **own** question set, then answers under Sekhmet.

## Per-run artifacts

```
runs/<id>/
  tasks.txt              # 64 generated questions
  ndjson.out             # sekhmet swarm NDJSON
  summary.json           # aggregated metrics (incl. sekhmet_tokens_*)
  gen_telemetry.json     # opencode generation tokens
  gen.events.jsonl
```

## Leaderboard

Regenerate:

```bash
./benchmarks/dry-hump/telemetry-e2e-model-questions/gen-leaderboard.sh
```

## Notes

- Requires fresh `sekhmet` (`cargo install --path . --force`) for `usage_tokens` on NDJSON.
- Temp roots under `/tmp/sekhmet-e2e-*` should be deleted after harvest.
- Incomplete primary dry-hump 0-ok rows are unrelated; this campaign always invokes sekhmet after gen.
