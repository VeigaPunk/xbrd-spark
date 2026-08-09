# ij-section-bench

**IJ-protocol** long-book **serial baton** sectioning benchmark for sekhmet (xbrd-spark L3).

Measures how many sparks cover a novel-length work in fixed **10-page windows** with handoff (offset + cumsum), reporting fastest/slowest intervals. Name is a **protocol/length metaphor** — not a license to pirate Infinite Jest.

## Legal lock (mandatory)

| Path | Status |
|------|--------|
| Project Gutenberg public-domain (default: Moby-Dick #2701) | **DEFAULT** |
| Embedded synthetic fixture (~60k chars) | Fallback / CI |
| User-owned local file (`corpus-prep --file`) | Optional |
| Infinite Jest / in-copyright via aaron, Libgen, pirate mirrors | **FORBIDDEN** |

Reports must record `legal`, `source`, and `sha256` from `corpus/meta.json`.

## Page model

| Constant | Default |
|----------|---------|
| `page_chars` | 2000 Unicode scalars |
| `pages_per_window` | 10 |
| `window_chars` | 20000 |

## Metrics

See plan `~/.xbgst/plans/2026-08-09-ij-section-bench-plan.md`. Core fields in `out/metrics.json`:

- `spark_count`, `ok` / `fail`
- `fastest_interval_ms`, `slowest_interval_ms`, `ratio_slow_fast`
- `cumsum_wall_ms`, `cumsum_work_chars`, `variance_interval_ms` (population)
- `mode`: `dry-run` | `live`
- `corpus_sha256`

Baton v1 JSON lines: `out/batons.jsonl`.

## Cache honesty

Server **always** sets:

```
Cache-Control: no-store, no-cache, must-revalidate, max-age=0
Pragma: no-cache
```

No ETag / Last-Modified. Re-reads files from disk each request. Query `cb=` accepted and ignored (bust token). Bind **127.0.0.1** only.

## Build

Standalone package (not a workspace member of root `xbrd-spark` crate):

```bash
cd ~/Projects/xbrd-spark/benchmarks/ij-section-bench
cargo build --release
```

Binary: `target/release/ij-section-bench` (or `cargo run -- …`).

## How to run

```bash
cd ~/Projects/xbrd-spark/benchmarks/ij-section-bench

# 1) Corpus (network → Gutenberg; fails → fixture)
cargo run -- corpus-prep --out corpus
# or offline:
cargo run -- corpus-prep --out corpus --fixture

# 2) Serve (no-store)
cargo run -- serve --corpus-dir corpus --port 18765

# 3) Orchestrator serial baton dry-run (overfit 3 windows)
cargo run -- orch --corpus-dir corpus --out-dir out \
  --dry-run --max-windows 3 --start-server --port 18765
```

Gates:

```bash
cargo build -p ij-section-bench   # if using a workspace; else: cargo build
curl -sI "http://127.0.0.1:18765/book.html?cb=t" | rg -i 'no-store'
jq -e '.spark_count==3' out/metrics.json
```

## Layout

```
benchmarks/ij-section-bench/
  Cargo.toml
  README.md
  src/main.rs
  src/fixture_body.md
  schemas/baton.v1.json
  corpus/          # gitignored bulk (prep recipe)
  out/             # batons + metrics + report
  results/         # optional dated runs
```

## Modes

- **serial-baton** (primary): `orch` loops windows in order; each spark gets URL + baton fields.
- **dry-run** (default): `sekhmet run --dry-run` per window (substrate fidelity; free).
- **live**: `--live` (cost; Titanium OAuth). Prefer small `--max-windows`.

## No `.txt` artifacts

New outputs use `.md` / `.json` / `.jsonl` only. Gutenberg source may arrive as remote `.txt` but is normalized into HTML + plain + json.
