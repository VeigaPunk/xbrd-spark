# dry-hump 12× OpenCode orchestrators × Sekhmet 64-swarm

Each row: **OpenCode model** dispatches the dry-hump harness; **Sekhmet** runs 8 domains × 8 concurrent **Codex Titanium** sparks (`gpt-5.3-codex-spark`).

| # | model (opencode) | sekhmet wall s | ok/64 | sekhmet tokens | opencode wall s | opencode tokens | cost | run_id |
|--:|------------------|---------------:|------:|---------------:|----------------:|----------------:|-----:|--------|
| 1 | `opencode/big-pickle` | 24.299 | 64/64 | 215835 | 29.866 | 18531 | 0 | `r01_opencode_big-pickle` |
| 2 | `opencode/deepseek-v4-flash-free` | 23.819 | 64/64 | 0 | 31.106 | 28980 | 0 | `r02b_opencode_deepseek-v4-flash-free` |
| 3 | `opencode/laguna-s-2.1-free` | 25.094 | 64/64 | 222500 | 74.383 | 39911 | 0 | `r03_opencode_laguna-s-2_1-free` |
| 4 | `opencode/ling-3.0-flash-free` | 22.17 | 64/64 | 0 | 28.02 | 30042 | 0 | `r04b_opencode_ling-3_0-flash-free` |
| 5 | `opencode/mimo-v2.5-free` | 23.639 | 64/64 | 0 | 30.695 | 29913 | 0 | `r05b_opencode_mimo-v2_5-free` |
| 6 | `opencode/nemotron-3-ultra-free` | 23.544 | 64/64 | 0 | 33.664 | 30207 | 0 | `r06b_opencode_nemotron-3-ultra-free` |
| 7 | `opencode/north-mini-code-free` | 22.95 | 64/64 | 0 | 39.684 | 24495 | 0 | `r07b_opencode_north-mini-code-free` |
| 8 | `openai/gpt-5.3-codex-spark` | 20.99 | 64/64 | 0 | 24.591 | 15232 | 0 | `r08b_openai_gpt-5_3-codex-spark` |
| 9 | `openai/gpt-5.4-mini-fast` | 19.014 | 64/64 | 219107 | 57.951 | 101936 | 0 | `r09_openai_gpt-5_4-mini-fast` |
| 10 | `openai/gpt-5.5-fast` | 19.738 | 64/64 | 0 | 34.607 | 23338 | 0 | `r10_openai_gpt-5_5-fast` |
| 11 | `openai/gpt-5.6-luna-fast` | 24.754 | 64/64 | 0 | 49.305 | 59571 | 0 | `r11_openai_gpt-5_6-luna-fast` |
| 12 | `openai/gpt-5.6-sol-fast` | 19.647 | 64/64 | 0 | 41.633 | 30925 | 0 | `r12_openai_gpt-5_6-sol-fast` |

**Perfect runs:** 12/12  |  **sekhmet_ok sparks:** 768  |  **sekhmet_tokens (where captured):** 657442  |  **opencode_tokens:** 433081  |  **sekhmet_wall_sum:** 269.7s

DeepSeek v4 (`opencode/deepseek-v4-flash-free`): included as run #2 (r02b redo + original r02).
