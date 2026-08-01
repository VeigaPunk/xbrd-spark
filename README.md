# xbrd-spark

**Pure L3 worker surface** for the xbrd stack.

Routes executions through **codex-spark** (GPT-5.3-Codex-Spark) with:

- **No git worktrees** — namespaced ephemeral dirs only
- **Double-work tolerance** — concurrent identical tasks are fine; higher layer (distiller / the-judge) dedups by content hash + provenance
- **Any-CLI invocable** — labrat, mutation-tester, executor, or plain bash can call it
- **Coordination stays above** — this layer is pure execution at light speed

This is the missing third layer under xbrd / xbgst / xask: the substrate that pure workers (labrat swarms, mutation probes, titanium-style one-shots) share without clashing.

## Axes (godspeed)

1. Isolation / clash-avoidance without worktrees  
2. Double-work tolerance + higher-layer distill  
3. Lightweight pure workers (codex-sparks on low)  
4. Subrouting substrate under xbrd  
5. Reachability by any delegated agent  
6. Coordination above this surface  

## Isolation scheme

```
$XBRD_SPARK_ROOT (or $XDG_RUNTIME_DIR/xbrd-spark or /tmp/xbrd-spark)
└── sp-<uuid>/
    ├── meta.json          # id, hashes, timestamps, cmdline, provenance
    ├── in/task.md
    ├── workspace/         # optional rsync --scope snapshot (mutation-harbor style)
    ├── out/
    │   ├── result.json
    │   ├── manifest.txt
    │   └── artifacts/     # content-addressed (sha256) copies
    ├── logs/
    ├── tmp/  home/  codex-home/  xdg/  cargo-home/ ...
```

- Unique `spark_id` per invocation (UUID v4 by default).
- `TMPDIR`, `CODEX_HOME`, `CARGO_*`, `XDG_*`, `HOME` forced inside the namespace.
- Optional `--scope PATH` does an rsync snapshot (excludes `.git`, `target`, `node_modules`, …) exactly like the existing `mutation-harbor-scaffold.sh`.
- Never writes into the invoker’s worktree or shared git state.
- Artifacts are content-addressed so identical results hash-collide for free.

## CLI

```bash
# Single shot (any agent / script)
xbrd-spark run --task "write a rust function that ..."
echo "probe hypothesis X" | xbrd-spark run --id sp-labrat-42

# With FS context for mutation / labrat that need files
xbrd-spark run --scope . --task "mutate the boundary check in src/lib.rs and run tests"

# Prefer direct codex (skip xask loadout) for absolute min latency
xbrd-spark run --direct --task "..."

# Collect for distiller (NDJSON)
xbrd-spark collect sp-aaa sp-bbb sp-ccc

# GC old namespaces (default 2h)
xbrd-spark gc --max-age 2

# Inspect
xbrd-spark status sp-aaa
```

Exit non-zero on spark failure, but the structured record is still emitted so double-work can be distilled.

## Integration under xbrd

Today labrat / executor / mutation-tester do:

```bash
xask --spark --gs codex "<probe>"
```

They can switch to (or xask can grow a thin wrapper for):

```bash
xbrd-spark run --task "<probe>"
# or with scope for mutation-tester
xbrd-spark run --scope "$REPO" --task "..."
```

Higher layer receives the NDJSON records, hashes content, clusters duplicates, keeps the Pareto survivors. Double execution is intentional and cheap.

## Build

```bash
cargo build --release
# binary lands in target/release/xbrd-spark
# optional: install to ~/.local/bin
```

Requires `xask` or `codex` on `PATH`. Prefer `xask` when present (preserves godspeed loadout + existing flags).

## Hard constraints (aligned with xbgst / xbrd)

- Rust only for the substrate itself.
- No worktrees.
- No internal coordination / judge logic.
- Explicit provenance + content hashes so distill is trivial.
- Soft resource honesty: host ports and global caches outside the forced env vars remain a residual ceiling; document and prefer `--ro` for pure probes.

## Relation to mutation-harbor

The `--scope` path re-uses the exact exclude list and rsync pattern from `scripts/mutation-harbor-scaffold.sh` in xbrd-gdsp-fknpft. This substrate generalizes that pattern from “mutation only” to “any pure worker”.

---

*Keep moves that improve any axis and harm none. Let the frontier walk itself.*
