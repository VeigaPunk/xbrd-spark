# Web-UI-only features map (2026) — Grok + ChatGPT + Notebook

Empirical CDP snapshot on musketeer-chrome (Chrome for Testing). Use with **fnm multishell** + pre-authed burner profile.

## Grok (`grok.com`) — the-musketeer

| Feature | Access | Automation note |
|---------|--------|-----------------|
| Chat (default) | `/` | `contenteditable` **Ask Grok anything** |
| Model: Auto / Fast / Expert / Heavy / Build | `button[aria-label="Model select"]` | Default CLI: **Expert** |
| Imagine (image/video gen) | `/imagine` sidebar **Imagine** | Templates, New Generation |
| Automations | sidebar **Automations** | Web-only scheduled flows |
| Skills and Connectors | sidebar | Web-only integrations |
| Projects | sidebar Projects / Add project | Workspace grouping |
| Private chat | Switch to Private Chat | Ephemeral |
| Attach / Voice / Dictation | composer | Attach files; voice Ctrl+⇧O |
| Search | Ctrl+K | History search |
| SuperGrok session | signed-in profile | No API substitute for full web surface |

CLI: `GROK_MODE=Expert|Fast|Heavy|Auto|Build grok "…"` · host `musketeer-chrome`.

## ChatGPT (`chatgpt.com`) — the-puppeteer

| Feature | Access | Automation note |
|---------|--------|-----------------|
| Chat / Work surfaces | radiogroup **Select chat surface** | `CHITCHAT_SURFACE=Chat|Work` |
| Pro model pill | composer **Pro** | Legacy model-switcher testids often missing |
| Deep Research / Image / Web search | `composer-plus-btn` menu | `--deep-research` / `--image` / `--web-search` |
| Projects / Library / Scheduled / Plugins | sidebar | Web-gated product areas |
| Temporary chat | header control | No history |
| File upload | plus menu + `input[type=file]` | Prefer attach for large docs |
| Canvas / agent modes | when shown in plus menu | UI-gated |
| PDF / file generation | model responses | Download from chat; no local FS for agent |

CLI: `chitchat --new-chat --model pro "…"` with **inline or GitHub URLs only**.

## Gemini Notebook (`notebook.google.com`) — the-almanacker

See `WEB-UI-AUTOMATION-CDP-RUNBOOK.md` Studio appendix. Prefer **Audio Overview → Deep Dive → Long**.

## Shared law

```
fnm multishell → agent-browser → musketeer-chrome (9222) → pre-auth on target domains
```
