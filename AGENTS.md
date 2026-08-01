# Sekhmet / xbrd-spark — Agent surface

Always-available pure L3 swarm dispatch substrate (xbreed layer 3). No judge, no distiller, no coordination logic lives here.

**Runtime target:** Codex Titanium (`codex-titanium` / `CODEX_BIN`). Model: `gpt-5.3-codex-spark` (`XBRD_SPARK_MODEL`).

Agents that should call it:
- labrat (default channel for cheap probes / swarms)
- mutation-tester (with --scope when FS mutation needed)
- executor (one-shot subtasks)
- any delegated agent or external CLI that wants namespaced titanium sparks

Binaries (identical): `sekhmet` | `xbrd-spark`

Invocation contract:
```
sekhmet run [--id ...] [--task | --task-file | stdin] [--scope PATH] \
  [--direct] [--deterministic] [--dry-run] [--ro] [--timeout SECS] \
  [--root PATH | $XBRD_SPARK_ROOT] [--no-keep] --task "..."
sekhmet swarm -f tasks.txt -j 16 --direct [--dry-run] [--ro] [--timeout SECS] \
  [--scope PATH] [--root PATH] [--fail-fast]
sekhmet collect <id...> [--root PATH]
sekhmet gc --max-age 2 [--root PATH]
sekhmet status <id> [--root PATH]
```

Key flags:
- `--dry-run` — full namespace + stub result + NDJSON; does not spawn titanium/xask
- `swarm -j N` — concurrent pool **1..=64** (hard cap); env `XBRD_SPARK_JOBS`; NDJSON per completion
- `--deterministic` — stable id from task+scope hash (`sp-` + first 16 hex of sha256); collision risk under concurrent same task
- `--no-keep` — delete namespace after run (default is keep artifacts; gc later)
- `--scope` — must be a directory; rsync into workspace even on dry-run (mutation-harbor excludes)
- `--direct` — prefer Codex Titanium over xask (use this for titanium)
- `--ro` — forces titanium `--sandbox read-only` (skips xask); recorded in meta
- `--timeout` — wall-clock kill when >0; after kill stdout/stderr joins bounded ~2s; in meta.timeout_secs
- `--root` / `XBRD_SPARK_ROOT` — isolation root (else `$XDG_RUNTIME_DIR/xbrd-spark` or `/tmp/xbrd-spark`)
- `CODEX_BIN` — pin titanium binary path; else `codex-titanium` then `codex`

Exclusive ns; setup rollback (id reusable); gc age-only for running. Seeded auth 0o600/0o700 on unix.

Double-work is intentional. Emit everything; let the layer above distill. Max swarm concurrency: **64**.

Gates (local ship):
```
cargo check && cargo test && cargo build --release && cargo clippy --all-targets -- -D warnings
target/release/sekhmet --help
```

CI: `.github/workflows/ci.yml` runs check / test / clippy / release --help on push to main.
