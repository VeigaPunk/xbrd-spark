# e2e model-questions leaderboard

OpenCode generates 64 questions → Sekhmet Titanium swarm answers them.

## Complete (64 ok)

| rank | run_id | model | swarm wall s | ok | fail | timeout | sekhmet tokens | tok/spark avg | questions |
|-----:|--------|-------|-------------:|---:|-----:|--------:|---------------:|--------------:|----------:|
| 1 | `e2e05_opencode_mimo-v2_5-free` | `opencode/mimo-v2.5-free` | 17.796 | 64 | 0 | 0 | 198302 | 3098.5 | 64 |
| 2 | `e2e01_opencode_big-pickle` | `opencode/big-pickle` | 17.9 | 64 | 0 | 0 | 207170 | 3237.0 | 64 |
| 3 | `e2e07_opencode_north-mini-code-free` | `opencode/north-mini-code-free` | 17.95 | 64 | 0 | 0 | 203118 | 3173.7 | 64 |
| 4 | `e2e03_opencode_laguna-s-2_1-free` | `opencode/laguna-s-2.1-free` | 18.066 | 64 | 0 | 0 | 197133 | 3080.2 | 64 |
| 5 | `e2e02_opencode_deepseek-v4-flash-free` | `opencode/deepseek-v4-flash-free` | 18.406 | 64 | 0 | 0 | 205871 | 3216.7 | 64 |
| 6 | `e2e09_openai_gpt-5_4-mini-fast` | `openai/gpt-5.4-mini-fast` | 20.889 | 64 | 0 | 0 | 203440 | 3178.8 | 64 |
| 7 | `e2e11_openai_gpt-5_6-luna-fast` | `openai/gpt-5.6-luna-fast` | 21.302 | 64 | 0 | 0 | 166362 | 2599.4 | 64 |
| 8 | `e2e04_opencode_ling-3_0-flash-free` | `opencode/ling-3.0-flash-free` | 22.605 | 64 | 0 | 0 | 162676 | 2541.8 | 64 |
| 9 | `e2e10_openai_gpt-5_5-fast` | `openai/gpt-5.5-fast` | 22.824 | 64 | 0 | 0 | 205727 | 3214.5 | 64 |
| 10 | `e2e08_openai_gpt-5_3-codex-spark` | `openai/gpt-5.3-codex-spark` | 25.02 | 64 | 0 | 0 | 169596 | 2649.9 | 64 |
| 11 | `e2e12_openai_gpt-5_6-sol-fast` | `openai/gpt-5.6-sol-fast` | 25.721 | 64 | 0 | 0 | 205539 | 3211.5 | 64 |

## Incomplete

| run_id | model | swarm wall s | ok | fail | timeout | tokens |
|--------|-------|-------------:|---:|-----:|--------:|-------:|
| `e2e06b_opencode_nemotron-3-ultra-free` | `opencode/nemotron-3-ultra-free` | 19.856 | 57 | 7 | 0 | 171555 |
| `e2e06_opencode_nemotron-3-ultra-free` | `opencode/nemotron-3-ultra-free` | 25.547 | 63 | 0 | 0 | 193452 |

_Regenerate: `./gen-leaderboard.sh`_
