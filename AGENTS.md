# xbrd-spark — Agent surface

This is the pure L3 worker substrate. No judge, no distiller, no coordination logic lives here.

Agents that should call it:
- labrat (default channel for cheap probes)
- mutation-tester (with --scope when FS mutation needed)
- executor (one-shot subtasks)
- any delegated agent or external CLI that wants namespaced codex-spark

Invocation contract:
```
xbrd-spark run [--id ...] [--task | --task-file | stdin] [--scope PATH] \
  [--direct] [--deterministic] [--dry-run] [--ro] [--timeout SECS] \
  [--root PATH | $XBRD_SPARK_ROOT] [--no-keep] --task "..."
xbrd-spark collect <id...> [--root PATH]
xbrd-spark gc --max-age 2 [--root PATH]
xbrd-spark status <id> [--root PATH]
```

Key flags:
- `--dry-run` — full namespace + stub result + NDJSON; does not spawn xask/codex
- `--deterministic` — stable id from task+scope hash (`sp-` + first 16 hex of sha256); collision risk under concurrent same task
- `--no-keep` — delete namespace after run (default is keep artifacts; gc later)
- `--scope` — must be a directory; rsync into workspace even on dry-run (mutation-harbor excludes)
- `--direct` — prefer codex over xask
- `--ro` — forces codex `--sandbox read-only` (skips xask); recorded in meta
- `--timeout` — wall-clock kill when >0; after kill stdout/stderr joins bounded ~2s; in meta.timeout_secs
- `--root` / `XBRD_SPARK_ROOT` — isolation root (else `$XDG_RUNTIME_DIR/xbrd-spark` or `/tmp/xbrd-spark`)

Exclusive ns; setup rollback (id reusable); gc age-only for running. Seeded auth 0o600/0o700 on unix.

Double-work is intentional. Emit everything; let the layer above distill.
