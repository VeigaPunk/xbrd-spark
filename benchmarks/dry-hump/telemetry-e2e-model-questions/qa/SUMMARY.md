# QA rebuild summary (short)

**Blocker:** Titanium `gpt-5.3-codex-spark` usage limit until **2026-08-08** (429/403).  
**Result:** rebuild swarms e2e01–e2e09 → **0 ok** each (empty answers); e2e10 aborted; e2e11–12 not started.  
**Evidence:** `ndjson.qa.out` + Q→A md under `qa/`; large dumps in `PASTE_URLS.txt` (litterbox).  
**Do not** resume live Titanium until quota reopens. Dry-run gates remain green.

See `STATUS.md` for details and resume commands.
