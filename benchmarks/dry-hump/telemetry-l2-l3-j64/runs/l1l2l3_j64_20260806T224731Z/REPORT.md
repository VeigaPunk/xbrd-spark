# Sekhmet dry-hump telemetry — l1l2l3_j64_20260806T224731Z

## Architecture
- **L1:** Grok orch (this session) + pane `ORCH` live progress
- **L3:** 8 parallel `sekhmet swarm --direct -j 8` (one domain each) = **64 concurrent**
- **Display:** tmux session `sekhmet` window `bench` (9 panes, tiled)

## Results
| Domain | ok | tokens_sum |
|--------|---:|-----------:|
| religion | 8 | 42522 |
| sex | 8 | 35524 |
| drugs | 8 | 42624 |
| politics | 8 | 37702 |
| money | 8 | 37532 |
| violence | 8 | 37202 |
| ai | 8 | 33051 |
| charlie-kirk | 8 | 37925 |
| **TOTAL** | **64** | **304082** |

Usage tokens from sekhmet `result.json` `usage_tokens` (Titanium total tokens per spark).

## Reproduce
```bash
. ~/.xbgst/env.l3-sekhmet.sh
# attach: tmux attach -t sekhmet \; select-window -t bench
# or re-run dry-hump harness:
bash ~/.xbgst/scripts/run-luna-j64-fast.sh
```
