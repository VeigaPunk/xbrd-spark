# telemetry-l2-l3-j64

L1/L3 sekhmet benches for **xbgst** (godspeed) on **gpt-5.6-luna** + `service_tier=fast` via **codex-titanium**.

## Headline result

**x8 × j=64 → 512/512 ok** — run `runs/x8x64_redo_20260806T230842Z/` (`GATE.txt`, `REPORT.md`).

Luna works. Substrate works. **tmpfs default for huge swarms was wrong** (see REPORT honesty / TODO).

## Layout

| Path | Meaning |
|------|---------|
| `QA-LATEST.md` | Single-file Q&A from earlier dry-hump harvest |
| `runs/qa_telemetry_*` | 8×8 dry-hump Q&A + telemetry |
| `runs/x8x64_*` | 8 workers × 64 Q each (gen + answer) |
| `runs/l1l2l3_*` | 9-pane dry-hump domain pack |

## Ops note

For any run ≥64 concurrent live sparks, set:

```bash
export XBRD_SPARK_ROOT="${XBRD_SPARK_ROOT:-$HOME/.cache/xbrd-spark}"
```

Do not rely on `/tmp` tmpfs alone.
