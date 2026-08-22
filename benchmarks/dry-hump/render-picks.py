#!/usr/bin/env python3
"""Render pass 2 hub + one highlights-style board per picker."""
from __future__ import annotations

import html
import importlib.util
import json
import sys
from pathlib import Path

_hl = Path(__file__).with_name("render-highlights.py")
_spec = importlib.util.spec_from_file_location("render_highlights", _hl)
_mod = importlib.util.module_from_spec(_spec)
assert _spec.loader is not None
_spec.loader.exec_module(_mod)
DOMAIN_LABEL = _mod.DOMAIN_LABEL
chrome = _mod.chrome
clout_poke = _mod.clout_poke
model_nav = _mod.model_nav
picker_name = _mod.picker_name
render_model_board = _mod.render_model_board
slug = _mod.slug


def render_hub(data: dict, utc: str, rev: str) -> str:
    pickers = data.get("pickers") or []
    label = str(data.get("label") or "pass 2 — each model’s top 10 (independent)")
    bank = int(data.get("bank_n") or 5235)
    strip = model_nav(pickers, current="", nested=False)
    toc: list[str] = []
    boards: list[str] = []
    for pi, picker in enumerate(pickers, 1):
        model = picker_name(picker, f"picker-{pi}")
        sl = slug(model)
        cfg = str(picker.get("config") or picker.get("config_subject") or "")
        picks = picker.get("picks") or []
        href = html.escape(f"highlights/{sl}.html")
        toc.append(
            f'<a href="{href}"><span class="toc-n">{pi:02d}</span>'
            f'<span><span class="toc-q">{html.escape(model)}</span>'
            f'<div class="toc-meta">{len(picks)}/10 · {html.escape(cfg) or "independent"} · bank {bank}</div></span></a>'
        )
        punches = []
        for pj, p in enumerate(picks, 1):
            raw_id = str(p.get("id") or f"{sl}-{pj}")
            pid = html.escape(f"{raw_id}-r{pj}")
            domain = str(p.get("domain") or "")
            stamp = html.escape(DOMAIN_LABEL.get(domain, domain))
            why = html.escape(str(p.get("why") or p.get("critic") or p.get("q") or ""))
            rankline = html.escape(f"#{pj} by {model} · {pj}/{bank}")
            punches.append(
                f'<a href="{href}#{pid}"><span class="toc-n">{pj:02d}</span>'
                f'<span><span class="toc-q">{why}</span>'
                f'<div class="toc-meta">{rankline} · {stamp}</div></span></a>'
            )
        boards.append(
            f'<section class="essay" id="{html.escape(sl)}">'
            f'<div class="essay-head"><span class="rank">{len(picks)}/10</span>'
            f'<span>{html.escape(cfg) or "independent"}</span></div>'
            f'<h2><a class="u" href="{href}">{html.escape(model)}</a></h2>'
            f'<p class="lede">Full board, same layout as pass 1 — punch wall then essays.</p>'
            f'<nav class="toc" aria-label="{html.escape(model)} punchlines">{"".join(punches)}</nav>'
            f'<div class="row"><a class="btn" href="{href}">Open {html.escape(model)} →</a></div>'
            f"</section>"
        )
    if not pickers:
        boards.append(
            '<section class="essay"><h2>Boards not in yet.</h2>'
            '<p class="lede">Boards land after each model reads the local pack.</p></section>'
        )
    body = f"""<section class="band"><div class="wrap">
  <p class="eyebrow">VeigaPunk · xbrd-spark · pass 2</p>
  <h1 class="hero">Independent top 10</h1>
  {clout_poke(bank, "The rest of the fleet sat the exam.")}
  <p class="lede">{html.escape(label)}. Each picker read the local {bank}-row ok-bank and has its own highlights page. Rank is <span class="rank">#N by model · N/{bank}</span>.</p>
  {strip}
</div></section>
<nav class="wrap toc" aria-label="Pickers">{"".join(toc)}</nav>
{"".join(boards)}"""
    return chrome(
        title="512QA — independent top 10",
        desc=label,
        root="",
        nav_on="picks",
        utc=utc,
        rev=rev,
        body=body,
        bank=bank,
    )


def write_boards(data: dict, outdir: Path, utc: str, rev: str) -> list[Path]:
    pickers = data.get("pickers") or []
    bank = int(data.get("bank_n") or 5235)
    hldir = outdir / "highlights"
    hldir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for pi, picker in enumerate(pickers, 1):
        model = picker_name(picker, f"picker-{pi}")
        path = hldir / f"{slug(model)}.html"
        path.write_text(render_model_board(picker, pickers, utc, rev, bank), encoding="utf-8")
        written.append(path)
    return written


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
    dst.write_text(render_hub(data, utc, rev), encoding="utf-8")
    write_boards(data, dst.parent, utc, rev)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
