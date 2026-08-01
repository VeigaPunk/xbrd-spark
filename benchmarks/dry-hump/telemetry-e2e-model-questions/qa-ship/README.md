# e2e Q→A ship pack

Full question→answer dumps for model-authored e2e tasks.

- **Questions:** each OpenCode orchestrator (`../runs/*/tasks.txt`)
- **Answers:** live Sekhmet on Codex Titanium with `gpt-5.4-mini` (spark model usage-limited until 2026-08-08; original spark bodies cleaned with `--no-keep`)
- **Fixed domain pack (gpt-5.3-codex-spark answers):** `DOMAINS-fixed-pack-QA.txt`

Rebuild: `QA_SHIP_JOBS=12 XBRD_SPARK_MODEL=gpt-5.4-mini python3 ../rebuild-qa-ship.py`
