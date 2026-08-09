# Web UI automation runbook — the only stack that works

**Audience:** agents (the-puppeteer / the-musketeer / kimi / notebook adapters) and humans wiring **agent-browser** over CDP.  
**Rule:** do **not** invent a disposable Chromium, do **not** point CDP at daily Chrome, do **not** assume ChatGPT can read `/home/...` paths.

This is the **exact** pattern used successfully on the host (snapshot 2026-08-06):  
**fnm multishell → node + agent-browser → burner Chrome for Testing (Canary install via musketeer) → pre-auth on target domains → CDP loopback 9222.**

---

## Architecture (one picture)

```
fnm multishell (node 24 + agent-browser on PATH)
        │
        ▼
agent-browser --cdp http://127.0.0.1:9222
        │
        ▼
musketeer-chrome  (= Chrome for Testing / Canary burner)
  --user-data-dir=~/.local/share/the-musketeer/chrome-profile
  --remote-debugging-address=127.0.0.1
  --remote-debugging-port=9222
        │
        ▼
Pre-authenticated tabs only for domains you automate:
  chatgpt.com | grok.com | kimi.com | notebooklm.google.com | …
        │
        ▼
chitchat / grok-web / other thin CLIs  (fire-and-forget into that browser)
```

Shared family: **one** burner profile, **one** CDP port, **many** product tabs. Wrong-tab attach is a real bug class — always select by URL.

---

## 1) fnm multishell (mandatory for node tools)

Stock `/usr/bin/node` may be missing. `chitchat` even hardcodes `/usr/bin/node` for `chitchat-batch.mjs` — that path **fails** unless you either:

- run under **fnm multishell** and patch the node path to `$(command -v node)`, or  
- ensure `node` is on PATH from fnm before any agent-browser call.

**Bootstrap every agent shell:**

```bash
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
eval "$(fnm env --shell bash)"
export PATH="$(dirname "$(command -v node)"):$PATH"

command -v node          # must be under .../fnm_multishells/.../bin/node
node -v                  # e.g. v24.18.1
command -v agent-browser # same multishell bin dir (npm -g under fnm)
```

**Why multishell:** isolates node/npm so agent-browser matches the node major, and keeps Grok/Codex agent shells reproducible.

**chitchat node fix (when `/usr/bin/node` missing):**

```bash
WORKDIR="$HOME/.grok/installed-plugins/the-puppeteer-"*   # or ~/.local/lib/ds4cc/the-puppeteer
# preferred explicit:
WORKDIR="$HOME/.grok/installed-plugins/the-puppeteer-2e958a87"
NODE="$(command -v node)"
sed "s|/usr/bin/node|$NODE|g" "$WORKDIR/chitchat" > "$WORKDIR/chitchat-fnm-run"
chmod +x "$WORKDIR/chitchat-fnm-run"
# run from WORKDIR so chitchat-batch.mjs resolves next to the script
```

---

## 2) Burner Chrome (Chrome for Testing / Canary) — not system Chromium

**Launcher:** `musketeer-chrome` → `~/.local/bin/musketeer-chrome`

| Piece | Path |
|-------|------|
| Binary | `~/.local/share/the-musketeer/chrome-canary/current/chrome` |
| Profile | `~/.local/share/the-musketeer/chrome-profile` (**must not be a symlink**) |
| CDP | `127.0.0.1:9222` only (loopback) |
| Version (example) | Google Chrome for Testing **151.x** (major ≥ 149 required) |

**Launch:**

```bash
# if nothing on 9222:
musketeer-chrome &
# default opens notebooklm / grok / chatgpt / kimi tabs

# verify
curl -sS --fail http://127.0.0.1:9222/json/version
agent-browser --cdp http://127.0.0.1:9222 tab list
```

**Flags that matter** (from the launcher):  
`--user-data-dir=<burner profile>` · `--remote-debugging-address=127.0.0.1` · `--remote-debugging-port=9222` · `--remote-allow-origins=http://127.0.0.1:9222,...` · isolated TMPDIR under `~/.cache/the-musketeer/chrome-tmp`.

### What does NOT work

| Anti-pattern | Why |
|--------------|-----|
| System `/usr/bin/chromium` + random `--user-data-dir` | Empty/unauthed profile; not the family burner |
| `ds4cc-cdp.service` hardened unit with `ProtectHome=read-only` | Breaks NSS/cookies; not the musketeer profile |
| Daily personal Chrome with remote debugging | Chrome may refuse CDP on default profile; mixes personal session |
| Assuming `CHITCHAT_CDP_PROFILE=~/.local/share/ds4cc/chromium-cdp` | Puppeteer default in chitchat — **stale vs musketeer profile**; if 9222 already up, chitchat attaches to **whatever** is listening (must be musketeer-chrome) |

**Correct contract:** **always** ensure **musketeer-chrome** owns port 9222 before `chitchat` / agent-browser.

---

## 3) Pre-auth on every domain you will automate

Auth is **cookies in the burner profile**, not API keys in the agent.

One-time (human) in the **musketeer-chrome** window:

1. Open each target and sign in fully (2FA if needed).
2. Leave sessions warm; re-auth when cookies expire.

**Domains we keep ready on this host (tab list example):**

| Domain | Adapter / use |
|--------|----------------|
| `https://chatgpt.com/` | **the-puppeteer** / `chitchat` |
| `https://grok.com/` | **the-musketeer** / `grok-web` |
| `https://www.kimi.com/` | **the-kimiraikoner** |
| `https://notebooklm.google.com/` / Gemini notebook | **the-almanacker** |

Verify:

```bash
agent-browser --cdp http://127.0.0.1:9222 tab list
# expect chatgpt.com / grok.com / kimi.com / notebook* without "Log in" walls
```

If snapshot shows **Log in**, stop — do not “fix” with a new profile.

---

## 4) agent-browser over CDP

```bash
eval "$(fnm env --shell bash)"
export PATH="$(dirname "$(command -v node)"):$HOME/.local/bin:$PATH"

AB=(agent-browser --cdp http://127.0.0.1:9222)

"${AB[@]}" tab list
"${AB[@]}" tab t5                    # pick by URL match, not guess
"${AB[@]}" open https://chatgpt.com --wait 4000ms
"${AB[@]}" snapshot -i
"${AB[@]}" focus '#prompt-textarea'
# uploads, clicks, etc.
```

**Tab discipline:** parse `tab list` for `chatgpt.com` / `grok.com` / … and select that `tN`. Opening a new tab without need multiplies wrong-tab bugs.

---

## 5) the-puppeteer (`chitchat`) fire-and-forget

```bash
eval "$(fnm env --shell bash)"
export PATH="$(dirname "$(command -v node)"):$HOME/.local/bin:$PATH"

# CDP must already be musketeer-chrome
curl -sS --fail http://127.0.0.1:9222/json/version >/dev/null

# Prefer public URLs or FULL inline markdown — ChatGPT has NO local FS access
printf '%s' "$PROMPT" | chitchat-fnm-run --stdin --new-chat
```

**Prompt rules for puppeteer targets:**

1. **Never** send only `/home/user/...` paths.  
2. **Do** send `https://github.com/.../blob/main/...` and/or **paste full markdown**.  
3. Fire-and-forget: do not poll for the model answer in the CLI; read it in the ChatGPT tab.

---

## 6) Minimal “make it work” checklist

```text
[ ] eval "$(fnm env --shell bash)" && node + agent-browser from fnm multishell
[ ] musketeer-chrome running; curl 127.0.0.1:9222/json/version → Chrome for Testing 15x
[ ] tab list shows pre-authed chatgpt.com / grok.com / … (no login wall)
[ ] agent-browser --cdp http://127.0.0.1:9222 …
[ ] chitchat uses that CDP (not a second random chromium)
[ ] prompts carry public links or inline content (no local-only paths)
[ ] 1Password extension bridge: the-janitor cdp-bridge status → READY (or ensure + restart Chrome)
```

### 6b) 1Password extension on the burner (the-janitor)

Custom `--user-data-dir` does **not** inherit daily Chrome Native Messaging Hosts.
`musketeer-chrome` and `the-janitor cdp-bridge ensure` install:

`~/.local/share/the-musketeer/chrome-profile/NativeMessagingHosts/com.1password.1password.json`
→ `/opt/1Password/1Password-BrowserSupport`

Extension expected in profile: Nightly `gejiddohjgogedgjnonbofjigllpkmbf` (or stable).

```bash
the-janitor cdp-bridge ensure   # write NMH if missing
the-janitor cdp-bridge status   # NMH + ext + desktop + CDP
# after first ensure on a live browser: restart musketeer-chrome
the-janitor cdp-bridge open-popup   # optional human unlock UI via agent-browser
```

CLI secrets (`op` / `the-janitor run`) stay the default for agent injection; the
extension path is for interactive autofill / pre-auth on product tabs. Never
scrape vault secrets from the extension into agent chat.

---

## 7) Family plugins (same CDP burner)

| Plugin | CLI | Target |
|--------|-----|--------|
| the-puppeteer | `chitchat` | chatgpt.com |
| the-musketeer | `grok-web` / musketeer | grok.com |
| the-kimiraikoner | kimi adapter | kimi.com |
| the-almanacker | notebook adapter | notebooklm.google.com |

All expect: **fnm multishell + musketeer-chrome burner + pre-auth + agent-browser --cdp 9222**.

---

## 8) Snapshot of a known-good host (2026-08-06)

```
fnm 1.39.0
node v24.18.1  (…/fnm_multishells/…/bin/node)
agent-browser → fnm node_modules/agent-browser/bin/agent-browser-linux-x64
musketeer-chrome → Chrome for Testing 151.0.7922.71
profile → ~/.local/share/the-musketeer/chrome-profile
CDP → http://127.0.0.1:9222
tabs → kimi, grok, chatgpt (Pro), notebook/gemini (pre-authed)
```

If any line differs (especially system Chromium on 9222 or unauthed tabs), **fix the stack before debugging selectors**.

---

*Maintainer note: chitchat’s default profile path (`~/.local/share/ds4cc/chromium-cdp`) is legacy. Operational SSoT is musketeer-chrome + `~/.local/share/the-musketeer/chrome-profile` on 9222.*

---

## Appendix: Gemini Notebook Studio DOM (2026 snapshot)

Host: `https://notebook.google.com/notebook/<uuid>` (rebrand from notebooklm.google.com).

| Surface | Selector / signal |
|---------|-------------------|
| Create notebook | `button[aria-label="Create new notebook"]` or **Create notebook** |
| Notebook title | `input.title-input` or top textbox with notebook name |
| Sources | **Add source** / `button.add-source-button`; source rows `button.source-stretched-button` |
| Studio panel | `section.studio-panel` / **Studio** heading |
| Artifact cards | `div.create-artifact-button-container` + `.create-label-container` |
| Audio Overview open | click label **Audio Overview** |
| Audio customize dialog | heading **Customize Audio Overview** |
| Modes | radios Deep Dive / Brief / Critique / Debate (aria-label) |
| Length | **Short** / **Default** / **Long** (en-US 2026; not Shorter/Longer) |
| Focus prompt | `textarea[aria-label="What should the AI hosts focus on in this episode?"]` |
| Generate | dialog button **Generate** |

**the-almanacker ≥ 0.2.1** accepts both hosts and these Studio labels. Always: **fnm multishell + musketeer-chrome + pre-auth**.
