# e2e model-questions leaderboard

OpenCode generates 64 questions → Sekhmet Titanium swarm answers them.

## Complete (64 ok)

| rank | run_id | model | swarm wall s | ok | fail | timeout | sekhmet tokens | tok/spark avg | questions |
|-----:|--------|-------|-------------:|---:|-----:|--------:|---------------:|--------------:|----------:|
| 1 | `e2e01_opencode_big-pickle` | `opencode/big-pickle` | 17.9 | 64 | 0 | 0 | 207170 | 3237.0 | 64 |
| 2 | `e2e03_opencode_laguna-s-2_1-free` | `opencode/laguna-s-2.1-free` | 18.066 | 64 | 0 | 0 | 197133 | 3080.2 | 64 |
| 3 | `e2e02_opencode_deepseek-v4-flash-free` | `opencode/deepseek-v4-flash-free` | 18.406 | 64 | 0 | 0 | 205871 | 3216.7 | 64 |

## Incomplete

| run_id | model | swarm wall s | ok | fail | timeout | tokens |
|--------|-------|-------------:|---:|-----:|--------:|-------:|

_Regenerate: `./gen-leaderboard.sh`_
