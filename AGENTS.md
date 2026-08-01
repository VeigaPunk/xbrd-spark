# xbrd-spark — Agent surface

This is the pure L3 worker substrate. No judge, no distiller, no coordination logic lives here.

Agents that should call it:
- labrat (default channel for cheap probes)
- mutation-tester (with --scope when FS mutation needed)
- executor (one-shot subtasks)
- any delegated agent or external CLI that wants namespaced codex-spark

Invocation contract:
```
xbrd-spark run [--id ...] [--scope PATH] [--direct] [--deterministic] --task "..."
xbrd-spark collect <id...>   # NDJSON for higher distiller
xbrd-spark gc --max-age 2
```

Double-work is intentional. Emit everything; let the layer above distill.
