# dry-hump multi-model × Sekhmet 64-swarm

| # | run_id | model | sekhmet wall s | ok/64 | sekhmet tokens | opencode wall s | opencode tokens |
|--:|--------|-------|---------------:|------:|---------------:|----------------:|----------------:|
| 1 | `r01_opencode_big-pickle` | `opencode/big-pickle` | 24.299 | 64/64 | 215835 | 29.866 | 18531.0 |
| 2 | `r02b_opencode_deepseek-v4-flash-free` | `opencode/deepseek-v4-flash-free` | 23.819 | 64/64 | 0 | 31.106 | 28980.0 |
| 3 | `r02_opencode_deepseek-v4-flash-free` | `opencode/deepseek-v4-flash-free` | 18.549 | 64/64 | 189486 | 29.362 | 30008.0 |
| 4 | `r03_opencode_laguna-s-2_1-free` | `opencode/laguna-s-2.1-free` | 25.094 | 64/64 | 222500 | 74.383 | 39911.0 |
| 5 | `r04b_opencode_ling-3_0-flash-free` | `opencode/ling-3.0-flash-free` | 22.170 | 64/64 | 0 | null | null |
| 6 | `r04_opencode_ling-3_0-flash-free` | `opencode/ling-3.0-flash-free` | 0.014 | 0/64 | 0 | 6.986 | 30263.0 |
| 7 | `r05_opencode_mimo-v2_5-free` | `opencode/mimo-v2.5-free` | 0.012 | 0/64 | 0 | 12.449 | 40575.0 |
| 8 | `r06_opencode_nemotron-3-ultra-free` | `opencode/nemotron-3-ultra-free` | 0.012 | 0/64 | 0 | 24.13 | 31246.0 |
| 9 | `r07_opencode_north-mini-code-free` | `opencode/north-mini-code-free` | 0.011 | 0/64 | 0 | 23.383 | 24875.0 |
| 10 | `r08_openai_gpt-5_3-codex-spark` | `openai/gpt-5.3-codex-spark` | 22.090 | 64/64 | 0 | 10.766 | 43136.0 |
| 11 | `r09_openai_gpt-5_4-mini-fast` | `openai/gpt-5.4-mini-fast` | 19.014 | 64/64 | 219107 | 57.951 | 101936.0 |
| 12 | `r10_openai_gpt-5_5-fast` | `openai/gpt-5.5-fast` | 19.738 | 64/64 | 0 | 34.607 | 23338.0 |
| 13 | `r11_openai_gpt-5_6-luna-fast` | `openai/gpt-5.6-luna-fast` | 24.754 | 64/64 | 0 | 49.305 | 59571.0 |
| 14 | `r12_openai_gpt-5_6-sol-fast` | `openai/gpt-5.6-sol-fast` | 19.647 | 64/64 | 0 | 41.633 | 30925.0 |

**Totals:** sekhmet_ok=640 sekhmet_tokens=846928 sekhmet_wall_sum=219.22299999999998s

Includes redo `r02b` (direct/opencode re-harvest). `r08` direct re-run after initial skip.
