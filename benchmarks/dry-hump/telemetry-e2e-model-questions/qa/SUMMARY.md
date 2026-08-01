# QA rebuild summary (short)

**Blocker:** Titanium `gpt-5.3-codex-spark` usage limit until **2026-08-08** (429/403).  
**Result:** rebuild swarms e2e01–e2e06 → **0 ok / 64 fail** each (empty answers).  
**Evidence:** large `ndjson.qa.out` posted via `scripts/paste-out.sh` (see `PASTE_URLS.txt`).  
**Do not** resume live Titanium until quota reopens. Dry-run gates remain green.

See `STATUS.md` for details and resume commands.
