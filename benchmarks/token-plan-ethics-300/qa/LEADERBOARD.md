# Leaderboard — Hard-10 + Ethics-300

Generated: 2026-08-09T18:15:00+00:00

Every claimed row includes **model_id**, **model_reasoning_effort**, and **service_tier**. Successful runs are git-committed under `benchmarks/token-plan-ethics-300/runs/`.

## Identity (all spark challenger runs)

| field | value |
|-------|--------|
| **model_id** | `gpt-5.3-codex-spark` |
| **model_reasoning_effort** | `low` |
| **service_tier** | `fast` |
| **binary** | `codex-titanium` |
| **invoker** | `sekhmet swarm` |
| **lane** | CLIProxy `http://127.0.0.1:8317/v1` → ChatGPT **Pro** OAuth (`jpveigao10@gmail.com`) |
| **cmdline stamp** | `-m gpt-5.3-codex-spark -c model_reasoning_effort=low -c service_tier=fast` |

## Hard-10 (reasoning + resourcefulness)

| role | model_id | effort | tier | jobs | ok | fail | wall_s | qps | tok | p50_ms | p95_ms | run dir |
|------|----------|--------|------|------|----|------|--------|-----|-----|--------|--------|---------|
| **challenger** | `gpt-5.3-codex-spark` | `low` | `fast` | 64 | **10** | 0 | 22.581 | 0.4429 | 131819 | 6116 | 22578 | `runs/hard10_gpt-5_3-codex-spark-challenger/` |
| contender (incomplete) | `qwen3.6-flash` | (host was token-plan medium default / sekhmet inject low) | — | 5 | 6 | 3 | 336.922 | 0.0178 | 3650428 | 213900 | 312615 | `runs/hard10_qwen3.6-flash/` (exit 143, not a success commit) |

## Ethics-300 (moral dilemmas + throughput)

| role | model_id | effort | tier | jobs | ok | fail | wall_s | qps | tok | tok/s | p50_ms | p95_ms | run dir |
|------|----------|--------|------|------|----|------|--------|-----|-----|-------|--------|--------|---------|
| **challenger (best ok)** | `gpt-5.3-codex-spark` | `low` | `fast` | 16 | **292** | 8 | 68.351 | 4.2721 | 567153 | 8298 | 2803 | 6908 | `runs/ethics_gpt-5_3-codex-spark-challenger/` |
| challenger (max wall speed) | `gpt-5.3-codex-spark` | `low` | `fast` | 64 | 201 | 99 | **20.259** | **9.9215** | 400486 | 19768 | 2755 | 6559 | same dir `summary.j64-speed.json` |

### Per-spark tok/s (measured)

| pack | model_id | effort | n | p50 tok/s | avg | max |
|------|----------|--------|---|-----------|-----|-----|
| Hard-10 | `gpt-5.3-codex-spark` | `low` | 10 | ~1810 | ~1790 | ~2900 |
| Ethics j16 | `gpt-5.3-codex-spark` | `low` | 292 | ~690 | ~640 | ~980 |

## Token Plan contender model_ids (queued)

From Token Plan Team `/models` (text only for this bench):

| model_id |
|----------|
| `qwen3.8-max` |
| `qwen3.7-max` |
| `qwen3.7-plus` |
| `qwen3.6-flash` |
| `glm-5.2` |
| `deepseek-v4-pro` |
| `deepseek-v4-flash-0731` |

Default lane effort when seeded: `model_reasoning_effort = "medium"` in `modelstudio-token-plan/config.toml` (sekhmet still injects `-c model_reasoning_effort=low` unless changed).

## Notes

- Packs: `tasks-ethics-300.swarm.md` (300), `hard10/tasks-hard10.swarm.md` (10).
- Roots on `/tmp` (runtime tmpfs is 3.1G).
- CLIProxy base must be `…:8317/v1`.
