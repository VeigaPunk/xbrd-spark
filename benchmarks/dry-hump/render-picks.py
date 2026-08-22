#!/usr/bin/env python3
"""Render site/picks.html — pass 2 independent top 10, same CSS as highlights."""
from __future__ import annotations

import html
import importlib.util
import json
import re
import sys
from pathlib import Path

_hl = Path(__file__).with_name("render-highlights.py")
_spec = importlib.util.spec_from_file_location("render_highlights", _hl)
_mod = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(_mod)
CSS = _mod.CSS
DOMAIN_LABEL = _mod.DOMAIN_LABEL
md_block = _mod.md_block


def _stamp(domain: str) -> tuple[str, str]:
    dclass = "d-" + re.sub(r"[^a-z0-9-]", "", domain)
    return dclass, html.escape(DOMAIN_LABEL.get(domain, domain))


def render(data: dict, utc: str, rev: str) -> str:
    pickers = data.get("pickers") or []
    label = html.escape(str(data.get("label") or "pass 2 — each model’s top 10 (independent)"))
    body: list[str] = []
    toc: list[str] = []
    n = 0
    for pi, picker in enumerate(pickers, 1):
        model = str(picker.get("picker") or picker.get("model") or picker.get("config") or f"picker-{pi}")
        cfg = str(picker.get("config") or "")
        picks = picker.get("picks") or []
        hid = html.escape(re.sub(r"[^a-z0-9-]+", "-", model.lower()).strip("-") or f"p{pi}")
        meta = html.escape(" · ".join(x for x in (model, cfg) if x))
        bank = int(data.get("bank_n") or picker.get("bank_n") or 5235)
        toc.append(
            f'<a href="#{hid}"><span class="toc-n">{pi:02d}</span>'
            f'<span><span class="toc-q">{html.escape(model)}</span>'
            f'<div class="toc-meta">{len(picks)}/10 · {html.escape(cfg) or "independent"} · bank {bank}</div></span></a>'
        )
        for pj, p in enumerate(picks, 1):
            n += 1
            pid = html.escape(str(p.get("id") or f"{hid}-{pj}"))
            domain = str(p.get("domain") or "")
            dclass, stamp = _stamp(domain)
            q = html.escape(str(p.get("q") or ""))
            why = html.escape(str(p.get("why") or p.get("critic") or ""))
            src_cfg = html.escape(str(p.get("config") or cfg))
            cite = html.escape(model)
            rankline = html.escape(f"#{pj} by {model} · {pj}/{bank}")
            pull = f'<blockquote class="pull">{why}<cite>{cite}</cite></blockquote>' if why else ""
            body.append(
                f'<article class="essay" id="{pid}">'
                f'<div class="essay-head"><span class="rank">{rankline}</span>'
                f'<span>{stamp} · {src_cfg}</span></div>'
                f'<h2>{q}</h2>'
                f"{pull}"
                f'<div class="prose">{md_block(str(p.get("answer") or ""))}</div>'
                f"</article>"
            )
        if not picks:
            body.append(
                f'<article class="essay" id="{hid}">'
                f'<div class="essay-head"><span class="stamp"><i></i>picker</span>'
                f"<span>{meta}</span></div>"
                f"<h2>{html.escape(model)}</h2>"
                f'<p class="lede">No picks in the fixture yet.</p>'
                f"</article>"
            )
    if not pickers:
        toc_html = ""
        body_html = (
            '<section class="essay">'
            "<h2>Boards not in yet.</h2>"
            '<p class="lede">Boards land after each model reads the local pack. '
            "This page is the pass 2 slot — independent top 10s, not the pass 1 shortlist.</p>"
            "</section>"
        )
    else:
        toc_html = f'<nav class="toc" aria-label="Pickers">{"".join(toc)}</nav>'
        body_html = "".join(body)
    bank = int(data.get("bank_n") or 5235)
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<meta name="theme-color" content="#0c0c0b">
<title>512QA — independent top 10</title>
<meta name="description" content="{label}">
<link rel="preload" href="fonts/JetBrainsMonoNLNerdFontMono-Regular.woff2" as="font" type="font/woff2" crossorigin>
<link rel="stylesheet" href="assets/family.css">
</head>
<body>
<header class="top">
  <div class="navwrap">
    <a class="brand" href="index.html">512QA</a>
    <nav class="desk">
      <a href="index.html">Fleet</a>
      <a href="highlights.html">Highlights</a>
      <a class="on" href="picks.html">Independent top 10</a>
    </nav>
  </div>
</header>
<main>
<section class="band"><div class="wrap">
  <p class="eyebrow">VeigaPunk · xbrd-spark · pass 2</p>
  <h1 class="hero">Independent top 10</h1>
  <p class="lede">{label}. Each picker read the local {bank}-row ok-bank. Rank is <span class="rank">#N by model · N/{bank}</span>. Not the pass 1 shortlist.</p>
</div></section>
{toc_html}
{body_html}
</main>
<footer class="bot"><div class="foot">generated {html.escape(utc)} @ {html.escape(rev)} · JetBrainsMonoNL Nerd Font Mono · bank {bank}</div></footer>
</body>
</html>
"""


def main() -> int:
    src = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("site/src/picks-by-model.json")
    dst = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("site/picks.html")
    utc = sys.argv[3] if len(sys.argv) > 3 else ""
    rev = sys.argv[4] if len(sys.argv) > 4 else "nogit"
    if not src.is_file():
        data = {
            "generated": utc,
            "pass": 2,
            "label": "pass 2 — each model’s top 10 (independent)",
            "pickers": [],
        }
    else:
        data = json.loads(src.read_text(encoding="utf-8"))
    dst.write_text(render(data, utc, rev), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
