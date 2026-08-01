# dry-hump — commands & output (verbatim)

## 1) Dry-run (free) — re-captured at ship time

See `run-dry.session.txt` for full `$ command` + stdout.

```bash
cd /path/to/xbrd-spark
./benchmarks/dry-hump/run-dry.sh
```

## 2) Live 64× concurrent Titanium — recorded session

Barrier + FIRE (parent):

```bash
date -u -Iseconds | tee /tmp/sekhmet-64/fire_wall_start_iso
date +%s.%N | tee /tmp/sekhmet-64/fire_wall_start
: > /tmp/sekhmet-64/FIRE
# ... wait for DONE_* ...
date +%s.%N | tee /tmp/sekhmet-64/fire_wall_end
date -u -Iseconds | tee /tmp/sekhmet-64/fire_wall_end_iso
```

Each of 8 domain workers (after FIRE):

```bash
export CODEX_BIN=$(command -v codex-titanium || command -v codex)
export PATH="$HOME/.cargo/bin:$PATH"
ROOT=/tmp/sekhmet-64/<domain>/root
mkdir -p "$ROOT"
START=$(date +%s.%N)
sekhmet swarm --direct -j 8 --timeout 180 \
  --tasks-file /tmp/sekhmet-64/<domain>/tasks.txt \
  --root "$ROOT" \
  > /tmp/sekhmet-64/<domain>/ndjson.out \
  2> /tmp/sekhmet-64/<domain>/stderr.log
EC=$?
END=$(date +%s.%N)
echo "exit=$EC start=$START end=$END" > /tmp/sekhmet-64/<domain>/timing.txt
```

Or single-script live harness:

```bash
./benchmarks/dry-hump/run-live.sh
```

## Recorded wall clock

- start: see `../results/fire_wall_start_iso`
- end: see `../results/fire_wall_end_iso`
- **wall_seconds ≈ 26.034**
- **64/64 status=ok**

Full Titanium NDJSON streams: `live-ndjson-full/*.ndjson` and `run-live-all-domains.ndjson.txt`.
