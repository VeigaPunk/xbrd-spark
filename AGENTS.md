# Sekhmet / xbrd-spark — Agent surface

Always-available pure L3 swarm dispatch substrate (xbreed layer 3). No judge, no distiller, no coordination logic lives here.

**Runtime target:** Codex Titanium (`CODEX_BIN` → `codex-titanium` → non-stub `codex`; omarchy npx stub skipped) on **ChatGPT OAuth** (not platform API key). **Never** symlink titanium as `codex`. **`xask-l3`** = thin `sekhmet run --direct` shim on `PATH` (PATH **`xask`** is xbreed protocol ask).

**Models (xbgst L3 workers — same isolation/swarm surface):**
- Primary: **`gpt-5.3-codex-spark`** (`XBRD_SPARK_MODEL`)
- Fallback chain (crate default): **`gpt-5.6-luna`** — override or disable via `XBRD_SPARK_FALLBACK_MODEL`
- Always on Titanium path:
  - `-c model_reasoning_effort=low`
  - `-c service_tier=default` (neutral default; opt in to Fast mode with `XBRD_SPARK_SERVICE_TIER=fast`)
- Swarm: **`-j 64`** default / hard cap (`XBRD_SPARK_JOBS`)
- Force fallback without probing primary: `XBRD_SPARK_USE_FALLBACK=1` (uses the crate chain `gpt-5.6-luna` unless env overrides)
- Disable auto-fallback: `XBRD_SPARK_FALLBACK_MODEL=none` (empty / `off` / `0` also disable)
- Meta: `model` + optional `model_fallback_from`

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
sekhmet swarm -f tasks.txt -j 64 --direct [--dry-run] [--ro] [--timeout SECS] \
  [--scope PATH] [--root PATH] [--fail-fast]
sekhmet collect <id...> [--root PATH]
sekhmet gc --max-age 2 [--root PATH]
sekhmet status <id> [--root PATH]
```

Key flags:
- `--dry-run` — full namespace + stub result + NDJSON; does not spawn titanium/xask-l3
- `swarm -j N` — concurrent pool **1..=64** (hard cap); env `XBRD_SPARK_JOBS`; NDJSON per completion
- `--deterministic` — stable id from task+scope hash (`sp-` + first 16 hex of sha256); collision risk under concurrent same task
- `--no-keep` — delete namespace after run (default is keep artifacts; gc later)
- `--scope` — must be a directory; rsync into workspace even on dry-run (mutation-harbor excludes)
- `--direct` — Titanium path (default **on**); `xask-l3` is a thin `sekhmet --direct` shim (PATH `xask` is xbreed protocol)
- `--ro` — forces titanium `--sandbox read-only`; recorded in meta
- `--timeout` — wall-clock kill when >0; after kill stdout/stderr joins bounded ~2s; in meta.timeout_secs
- `--root` / `XBRD_SPARK_ROOT` — isolation root (else `$XDG_RUNTIME_DIR/xbrd-spark` or `/tmp/xbrd-spark`)
- `CODEX_BIN` — pin Titanium binary path; else `codex-titanium` then non-stub `codex` (omarchy npx stub skipped; never symlink titanium→`codex`)
- `XBRD_SPARK_SERVICE_TIER` — validated `default|fast`; defaults to explicit `default`, and unsupported values fail before dispatch
- `--no-direct` — legacy loadout only (prefer `xask-l3` shim or `--direct`; never PATH `xask`)

Every `run` and every `swarm` member receives the byte-exact vendored
`godspeed-core/directive.md` at the shared dispatch boundary. Never replace it
with a minimal/summary skill. The actual delegated prompt always ends in exactly
one literal `| godspeed`; repeated or case-variant terminal markers normalize to
that single closer. This behavior has no opt-out.

Exclusive ns; setup rollback (id reusable); gc age-only for running. Seeded auth 0o600/0o700 on unix.

Double-work is intentional. Emit everything; let the layer above distill. Max swarm concurrency: **64**.

Gates (local ship):
```
cargo check && cargo test && cargo build --release && cargo clippy --all-targets -- -D warnings
target/release/sekhmet --help
```

CI: `.github/workflows/ci.yml` runs check / test / clippy / release --help on push to main.

Optional live smoke: `XBRD_SPARK_LIVE=1 cargo test --test live_smoke -- --ignored --nocapture`

Install (refresh PATH binary after pull):
```
cargo install --path . --force
```
Stale `~/.cargo/bin/sekhmet` may lack `usage_tokens` / `--version`.

Live Titanium tip: prefer `swarm -j 8` (or lower) under provider rate limits; `-j 64` can yield fail/null usage_tokens.

## Large outputs → pastebin.com ONLY (mandatory, non-negotiable)

Do **not** dump multi-KB swarm NDJSON, logs, or transcripts into chat/TUI.

**Host: [pastebin.com](https://pastebin.com) only.** No litterbox, catbox, 0x0, paste.rs, dpaste, hastebin, gist-as-paste, transfer.sh, or any other paste host. If Pastebin fails, **stop and report** — do not switch domains.

```bash
export PASTEBIN_API_DEV_KEY=...   # https://pastebin.com/doc_api
sekhmet swarm --direct -j 8 --tasks-file tasks.txt --root "$ROOT" \
  > /tmp/swarm.ndjson 2> /tmp/swarm.err
# short in-session summary only:
jq -s '{lines:length, ok:[.[]|select(.status=="ok")]|length, fail:[.[]|select(.status=="fail")]|length}' /tmp/swarm.ndjson
# full blob → pastebin.com URL only:
./scripts/paste-out.sh /tmp/swarm.ndjson
./scripts/paste-out.sh /tmp/swarm.err
```

Keep in-session: **pastebin.com URL + short counts** (ok/fail/timeout/wall). Script: `scripts/paste-out.sh` (pastebin.com API only).

## Provider quota

If Titanium returns **usage_limit** / `model_unsupported` / `model_chatgpt_unsupported` on the primary model, sekhmet **automatically retries the fallback chain** — crate default `gpt-5.6-luna`, override or disable with `XBRD_SPARK_FALLBACK_MODEL` (effort low + the selected service tier), then latches that model for the rest of the process. A chain entry equal to the primary is skipped, so an explicit luna primary does not auto-retry onto itself.

For 403 websocket / 429 rate_limit / auth failures: no model fallback — stop or lower `-j`, prefer dry-run gates.

Short swarm counts (in-session): `./scripts/swarm-summary.sh /tmp/swarm.ndjson` then `./scripts/paste-out.sh /tmp/swarm.ndjson`.

## Offline / no Titanium mode

When neither spark nor fallback can run (missing dispatcher / full auth outage):
- Run network-free gates: `./scripts/dry-gates.sh`
- Pastebin: requires `PASTEBIN_API_DEV_KEY`; if missing, report blocker — **no alternate hosts**.
