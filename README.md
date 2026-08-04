# xbrd-spark · **Sekhmet**

**Always-available swarm dispatch substrate** — layer 3 of xbreed.

Marketplace name: **`sekhmet`** ([ds4cc-marketplace](https://github.com/VeigaPunk/ds4cc-marketplace)).  
Binaries: **`sekhmet`** and **`xbrd-spark`** (same surface, Rust only — no Python).

Ships against **Codex Titanium** (`codex-titanium` / `codex` symlink) — [codex-titanium](https://github.com/VeigaPunk/codex-titanium).

**Model routing (everything else equal — same namespace, swarm, NDJSON for xbgst):**
| | Model | Env |
|---|---|---|
| Primary | `gpt-5.3-codex-spark` | `XBRD_SPARK_MODEL` |
| Fallback chain | `gpt-5.6-luna-fast` → `gpt-5.6-luna` (effort **low**) | `XBRD_SPARK_FALLBACK_MODEL` (comma list) |
| Force fallback | (skip primary) | `XBRD_SPARK_USE_FALLBACK=1` |
| Disable fallback | — | `XBRD_SPARK_FALLBACK_MODEL=none` |

On primary `usage_limit` (or model-unsupported), sekhmet walks the fallback chain with `model_reasoning_effort=low`. ChatGPT-auth Codex blocks the `*-luna-fast` slug; the next entry `gpt-5.6-luna` is used and **sticky** for the process. Recorded in meta as `model` + optional `model_fallback_from`.

Dispatcher resolve order: `CODEX_BIN` → `codex-titanium` → `codex`.

Routes executions through **Titanium** (primary codex-spark, luna-fast when sparks are out) with:

- **Always callable** — default channel for labrat swarms and pure worker sparks
- **Up to 64 concurrent runners** — `sekhmet swarm -j N` (hard cap 64)
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

- Unique `spark_id` per invocation (UUID v4 by default; `--deterministic` → `sp-` + first 16 hex of `sha256(task|scope)`).
- **Exclusive namespace**: if `$ROOT/<id>` already exists, run bails with `spark namespace already exists: {id}` (no clobber). Concurrent *different* ids are intentional double-work.
- **Setup rollback**: if setup fails after exclusive create (ensure_dirs / seed / task write / rsync) *before* meta/finalize, the namespace is removed so the id is reusable. Dispatcher/spawn failures after meta keep the emit-record behavior (no delete when `keep=true`).
- `TMPDIR`, `CODEX_HOME`, `CARGO_*`, `XDG_*`, `HOME` forced inside the namespace.
- Auth/config seeded into `codex-home` with `0o600` files / `0o700` dirs where supported (unix). Prefer `XDG_RUNTIME_DIR` / a private root over world-writable shared `/tmp` in multi-tenant settings.
- Optional `--scope PATH` must be a **directory** (bail otherwise); rsync snapshot excludes `.git`, `target`, `node_modules`, … (mutation-harbor style). Symlinks follow rsync `-a` defaults (not `--copy-links`).
- Never writes into the invoker’s worktree or shared git state.
- Artifacts are content-addressed so identical results hash-collide for free.
- Root override: `--root` or env `XBRD_SPARK_ROOT`.
- Initial `meta.json` and finalize paths write via `*.tmp` + rename.

## CLI

```bash
# Single shot (any agent / script)
xbrd-spark run --task "write a rust function that ..."
echo "probe hypothesis X" | xbrd-spark run --id sp-labrat-42

# Dry-run (no xask/codex; full namespace + stub result + NDJSON)
xbrd-spark run --dry-run --task "probe" --root /tmp/xbrd-spark-smoke

# With FS context for mutation / labrat that need files
xbrd-spark run --scope . --task "mutate the boundary check in src/lib.rs and run tests"

# Prefer direct Codex Titanium (skip xask loadout) for absolute min latency
xbrd-spark run --direct --task "..."
# or the always-on alias:
sekhmet run --direct --task "..."

# Swarm: up to 64 concurrent Titanium runners (NDJSON per completion)
printf 'task A\ntask B\ntask C\n' | sekhmet swarm --direct -j 16 --tasks-file - --root "$(mktemp -d)"
# tasks file: one prompt per line, or JSONL {"task":"...","id":"sp-...","scope":"/path"}

# Deterministic id from task+scope hash (stable; collision risk under concurrent same task)
xbrd-spark run --deterministic --task "..."

# Delete namespace after run (default is keep; use gc for bulk cleanup)
xbrd-spark run --no-keep --dry-run --task "ephemeral probe"

# Collect for distiller (NDJSON); at least one id required
xbrd-spark collect sp-aaa sp-bbb sp-ccc

# GC old namespaces (default 2h)
xbrd-spark gc --max-age 2

# Inspect
xbrd-spark status sp-aaa
```

Key run flags: `--id`, `--task` / `--task-file` / stdin, `--scope`, `--ro`, `--timeout`, `--direct`, `--root` / `XBRD_SPARK_ROOT`, `--no-keep`, `--deterministic`, `--dry-run`.

Exit non-zero on spark failure, but the structured record is still emitted so double-work can be distilled. Live spawn uses `env_clear` + allowlisted env only.

### Flags (enforcement)

- **`--timeout SECS`**: wall-clock kill when `SECS > 0` (poll `try_wait` + process-group `SIGKILL` on Linux); result status is `timeout` (not `fail`). After kill, stdout/stderr reader joins are **bounded** (~2s) so orphan pipe holders cannot hang the spark forever (logs may truncate). `0` waits unlimited. Recorded in `meta.timeout_secs`.
- **`--ro`**: **forces the codex dispatcher** with `--sandbox read-only` (skips xask so sandbox is actually enforced). Without `--ro`, prefers xask when present, else codex with `danger-full-access`. Recorded in `meta.ro`.
- **`--scope`**: must be a directory; rsync snapshot into `workspace/` even on `--dry-run` (when rsync is available). Non-directory paths fail setup and roll back the namespace.
- **Provenance**: `meta` also records `direct`, `dry_run`, and `timeout_secs` for every run.
- **`usage_tokens`**: best-effort parse from dispatcher stdout/stderr (`tokens used`, `total_tokens`, …) written into `meta.json`, `out/result.json`, and NDJSON when present.
- **Spawn/dispatcher errors**: after namespace + initial meta exist, failures finalize with status `error`, emit NDJSON, and exit non-zero (record still present for distill).

## Open residuals

- Concurrent sparks sharing host `~/.codex` token lifetime / refresh races (seed copies auth, does not isolate refresh).
- Host global caches (cargo registry, rustup) and ports are not namespaced; prefer private `XBRD_SPARK_ROOT` / `XDG_RUNTIME_DIR`.
- rsync does not `--copy-links` (symlink semantics = rsync `-a`).
- Deterministic ids collide under concurrent same task+scope.
- After timeout kill, reader joins are bounded (~2s); escaped pipe holders may truncate logs / leak reader threads — spark still finalizes `timeout`.
- `gc --max-age` reaps old `status=running` by **age only** (no live PID/`/proc` probe); long `--timeout 0` jobs past max-age can be deleted.
- Live codex smoke / CI workflows are optional and out of default gates.
- Non-unix: seed `0o700`/`0o600` permission bits are no-ops.
- Large live swarms can fill `/tmp` (tmpfs quota); use private roots, `gc`, and delete `/tmp/sekhmet-*` after harvest or tests will fail with `Disk quota exceeded`.

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
# CI / gates without live model
xbrd-spark run --dry-run --task "smoke"
```

Higher layer receives the NDJSON records, hashes content, clusters duplicates, keeps the Pareto survivors. Double execution is intentional and cheap.

## Install / build

```bash
cargo install --path . --force  # refresh ~/.cargo/bin after pulls
# or without install:
cargo build --release
# binary: target/release/xbrd-spark
```

Real runs need `codex` or `xask` on `PATH`; `--ro` requires `codex`; `--dry-run` needs neither.

Smoke:

```bash
xbrd-spark --help
xbrd-spark run --help
xbrd-spark run --dry-run --task 'probe' --root "$(mktemp -d)"
xbrd-spark status --help
```

## Hard constraints (aligned with xbgst / xbrd)

- Rust only for the substrate itself.
- No worktrees.
- No internal coordination / judge logic.
- Explicit provenance + content hashes so distill is trivial.
- Resource honesty: namespace forces `TMPDIR`/`HOME`/`CODEX_HOME`/…; host ports and other global caches remain a residual ceiling. Prefer `--ro` for pure probes (codex sandbox read-only).

## Relation to mutation-harbor

The `--scope` path re-uses the exact exclude list and rsync pattern from `scripts/mutation-harbor-scaffold.sh` in xbrd-gdsp-fknpft. This substrate generalizes that pattern from “mutation only” to “any pure worker”.

---

*Keep moves that improve any axis and harm none. Let the frontier walk itself.*

## Benchmark example: dry-hump

64-concurrent Titanium moral-dilemma stress (8 domains × 8 sparks):

See [`benchmarks/dry-hump/`](benchmarks/dry-hump/) — recorded **64/64 ok in ~26s** wall clock.

```bash
./benchmarks/dry-hump/run-dry.sh    # free dry-run smoke
./benchmarks/dry-hump/run-live.sh   # live Codex Titanium (API cost)
```

## Benchmarks

- [`benchmarks/dry-hump`](benchmarks/dry-hump) — multi-model 8×8 Titanium load
- [`benchmarks/scope-fanout-64`](benchmarks/scope-fanout-64) — public synthetic 64-way `--scope` swarm (dry-run safe)

### Offline / quota dry gates

While Titanium or Pastebin is unavailable:

```bash
./scripts/dry-gates.sh
```

Network-free: `cargo test`/`clippy`, dry-run 3-task swarm, paste-out offline negative check.

### Large swarm dumps → pastebin.com ONLY

Post NDJSON/logs with [`scripts/paste-out.sh`](scripts/paste-out.sh) (**pastebin.com API only**; requires `PASTEBIN_API_DEV_KEY`).  
In-session: **pastebin.com URL + short summary** via `scripts/swarm-summary.sh`.  
If Pastebin fails: **stop and report** — do not use any other paste host.
