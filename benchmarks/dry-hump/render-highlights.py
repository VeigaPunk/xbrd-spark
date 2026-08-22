#!/usr/bin/env python3
"""Render site/highlights.html as an editorial board, not a fixture dump."""
from __future__ import annotations

import html
import json
import re
import sys
from pathlib import Path

DOMAIN_LABEL = {
    "religion": "religion",
    "charlie-kirk": "gallows",
    "drugs": "drugs",
    "sex": "sex",
    "ai": "ai",
    "violence": "violence",
    "politics": "politics",
    "money": "money",
}

CONFIG_MODEL = {
    "grok-4.6-low": "grok-4.6",
    "grok-4.6-high": "grok-4.6",
    "grok-4.6-default": "grok-4.6",
    "qwen3.8-max-low": "qwen3.8-max",
    "sol-fast-titanium": "gpt-5.6-sol",
    "luna-fast-titanium": "gpt-5.6-luna",
    "ds-flash-0731": "deepseek-v4-flash-0731",
    "ds-pro-0813": "deepseek-v4-pro-0813",
}

SHORT_LABEL = {
    "gpt-5.6-sol": "sol",
    "gpt-5.6-luna": "luna",
    "grok-4.6": "grok",
    "deepseek-v4-flash-0731": "ds-flash",
    "deepseek-v4-pro-0813": "ds-pro",
    "kimi-k3": "kimi",
    "qwen3.8-max": "qwen",
}

CSS_VER = "boards"


def slug(name: str) -> str:
    s = re.sub(r"[^a-z0-9._-]+", "-", str(name).lower()).strip("-")
    return s or "model"


def short_label(name: str) -> str:
    return SHORT_LABEL.get(name, name)


def picker_name(picker: dict, fallback: str = "model") -> str:
    return str(picker.get("picker") or picker.get("model") or picker.get("config") or fallback)


def load_pickers(near: Path) -> list[dict]:
    candidates = []
    if near.is_file():
        if near.name == "picks-by-model.json":
            candidates.append(near)
        candidates.append(near.with_name("picks-by-model.json"))
    else:
        candidates.append(near / "src" / "picks-by-model.json")
        candidates.append(near / "picks-by-model.json")
    for p in candidates:
        if p.is_file():
            data = json.loads(p.read_text(encoding="utf-8"))
            if "pickers" in data:
                return data.get("pickers") or []
    return []


def model_nav(pickers: list[dict], *, current: str, nested: bool) -> str:
    if not pickers and current != "pass-1":
        return ""
    wall = "../highlights.html" if nested else "highlights.html"
    pre = "" if nested else "highlights/"
    bits = [
        f'<a href="{wall}"' + (' class="on"' if current == "pass-1" else "") + ">pass 1</a>"
    ]
    for i, picker in enumerate(pickers, 1):
        name = picker_name(picker, f"picker-{i}")
        sl = slug(name)
        href = html.escape(f"{pre}{sl}.html")
        label = html.escape(short_label(name))
        cls = ' class="on"' if current == sl else ""
        bits.append(f'<a href="{href}"{cls}>{label}</a>')
    return '<nav class="models" aria-label="Boards">' + "".join(bits) + "</nav>"


def chrome(
    *,
    title: str,
    desc: str,
    root: str,
    nav_on: str,
    utc: str,
    rev: str,
    body: str,
    bank: int | None = None,
) -> str:
    p = root
    def on(name: str) -> str:
        return ' class="on"' if nav_on == name else ""
    foot_extra = f" · bank {bank}" if bank is not None else ""
    return f"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<meta name="theme-color" content="#0c0c0b">
<title>{html.escape(title)}</title>
<meta name="description" content="{html.escape(desc)}">
<link rel="preload" href="{p}fonts/JetBrainsMonoNLNerdFontMono-Regular.woff2" as="font" type="font/woff2" crossorigin>
<link rel="stylesheet" href="{p}assets/family.css?v={CSS_VER}">
</head>
<body>
<header class="top">
  <div class="navwrap">
    <a class="brand" href="{p}index.html">512QA</a>
    <nav class="desk">
      <a href="{p}index.html">Fleet</a>
      <a href="{p}highlights.html"{on("highlights")}>Highlights</a>
      <a href="{p}picks.html"{on("picks")}>Independent top 10</a>
    </nav>
  </div>
</header>
<main>
{body}
</main>
<footer class="bot"><div class="foot">generated {html.escape(utc)} @ {html.escape(rev)} · JetBrainsMonoNL Nerd Font Mono{foot_extra} · <a href="https://github.com/VeigaPunk/xbrd-spark">github</a></div></footer>
</body>
</html>
"""


def md_inline(s: str) -> str:
    s = html.escape(s)
    s = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", s)
    s = re.sub(r"(?<!\*)\*(?!\*)(.+?)(?<!\*)\*(?!\*)", r"<em>\1</em>", s)
    s = re.sub(r"`([^`]+)`", r"<code>\1</code>", s)
    return s


def md_block(text: str) -> str:
    lines = text.replace("\r\n", "\n").split("\n")
    out: list[str] = []
    i = 0
    n = len(lines)

    def flush_p(buf: list[str]) -> None:
        if buf:
            out.append("<p>" + md_inline(" ".join(x.strip() for x in buf)) + "</p>")
            buf.clear()

    while i < n:
        line = lines[i]
        if not line.strip():
            i += 1
            continue
        if line.strip().startswith("|"):
            rows = []
            while i < n and lines[i].strip().startswith("|"):
                cells = [c.strip() for c in lines[i].strip().strip("|").split("|")]
                rows.append(cells)
                i += 1
            if len(rows) >= 2:
                head = rows[0]
                body = [
                    r
                    for r in rows[1:]
                    if not all(re.match(r"^:?-+:?$", c or "") for c in r)
                ]
                thead = "<thead><tr>" + "".join(f"<th>{md_inline(c)}</th>" for c in head) + "</tr></thead>"
                tbody = "<tbody>" + "".join(
                    "<tr>" + "".join(f"<td>{md_inline(c)}</td>" for c in r) + "</tr>" for r in body
                ) + "</tbody>"
                out.append(f'<div class="table-wrap"><table>{thead}{tbody}</table></div>')
            continue
        if re.match(r"^##\s+", line):
            out.append("<h3>" + md_inline(re.sub(r"^##\s+", "", line)) + "</h3>")
            i += 1
            continue
        if re.match(r"^###\s+", line):
            out.append("<h4>" + md_inline(re.sub(r"^###\s+", "", line)) + "</h4>")
            i += 1
            continue
        if re.match(r"^[-*]\s+", line) or re.match(r"^\d+\.\s+", line):
            items = []
            ordered = bool(re.match(r"^\d+\.\s+", line))
            while i < n and (re.match(r"^[-*]\s+", lines[i]) or re.match(r"^\d+\.\s+", lines[i])):
                items.append(re.sub(r"^([-*]|\d+\.)\s+", "", lines[i]))
                i += 1
            tag = "ol" if ordered else "ul"
            out.append(f"<{tag}>" + "".join(f"<li>{md_inline(it)}</li>" for it in items) + f"</{tag}>")
            continue
        buf = [line]
        i += 1
        while i < n and lines[i].strip() and not re.match(r"^(##|###|[-*] |\d+\. |\||$)", lines[i]):
            buf.append(lines[i])
            i += 1
        flush_p(buf)
    return "\n".join(out)


CSS = """
:root {
  --bg: #07090c;
  --bg-2: #0c1016;
  --ink: #f4efe6;
  --muted: #9a9186;
  --faint: #6b645c;
  --rule: color-mix(in oklab, #f4efe6 12%, transparent);
  --gold: #e2b657;
  --blood: #e05a3a;
  --leaf: #7dba96;
  --violet: #b48cff;
  --magenta: #e07ab5;
  --teal: #6ec4c0;
  --paper: #12161d;
  --shadow: 0 30px 80px color-mix(in oklab, #000 55%, transparent);
  --serif: "Iowan Old Style", "Palatino Linotype", Palatino, "Times New Roman", serif;
  --sans: ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
  --mono: ui-monospace, "SF Mono", Menlo, Consolas, monospace;
  color-scheme: dark;
}
* { box-sizing: border-box; }
html { scroll-behavior: smooth; scroll-padding-top: 1.5rem; }
body {
  margin: 0;
  min-height: 100dvh;
  background:
    radial-gradient(ellipse 90% 50% at 10% -10%, color-mix(in oklab, var(--blood) 16%, transparent), transparent 50%),
    radial-gradient(ellipse 70% 40% at 100% 0%, color-mix(in oklab, var(--gold) 10%, transparent), transparent 46%),
    linear-gradient(180deg, #05070a 0%, var(--bg) 30%, #0a0e14 100%);
  color: var(--ink);
  font-family: var(--serif);
  font-size: 1.12rem;
  line-height: 1.62;
  -webkit-font-smoothing: antialiased;
}
a { color: inherit; }
.wrap { width: min(100% - 2rem, 46rem); margin-inline: auto; }
.mast {
  width: min(100% - 2rem, 72rem);
  margin: 0 auto;
  padding: 1.25rem 0 0;
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  gap: 1rem;
  font-family: var(--mono);
  font-size: 0.68rem;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--muted);
}
.mast a { text-decoration: none; border-bottom: 1px solid var(--rule); padding-bottom: 0.12rem; }
.mast a:hover { color: var(--ink); }
.hero {
  width: min(100% - 2rem, 72rem);
  margin: 2.5rem auto 0;
  padding-bottom: 2.5rem;
  border-bottom: 1px solid var(--rule);
}
.kicker {
  font-family: var(--mono);
  font-size: 0.72rem;
  letter-spacing: 0.22em;
  text-transform: uppercase;
  color: var(--gold);
  margin: 0 0 0.8rem;
}
.hero h1 {
  margin: 0;
  font-family: var(--serif);
  font-weight: 600;
  font-size: clamp(3.4rem, 10vw, 8.2rem);
  line-height: 0.86;
  letter-spacing: -0.055em;
}
.hero .lede {
  max-width: 38rem;
  margin: 1.4rem 0 0;
  color: var(--muted);
  font-size: 1.12rem;
}
.toc {
  width: min(100% - 2rem, 72rem);
  margin: 0 auto;
  padding: 2rem 0 3rem;
  display: grid;
  gap: 0.15rem;
}
.toc a {
  display: grid;
  grid-template-columns: 3.2rem minmax(0, 1fr);
  gap: 1rem;
  text-decoration: none;
  padding: 0.85rem 0;
  border-bottom: 1px solid var(--rule);
  align-items: baseline;
}
.toc a:hover .toc-q { color: var(--gold); }
.toc-n {
  font-family: var(--mono);
  font-size: 0.72rem;
  letter-spacing: 0.14em;
  color: var(--faint);
}
.toc-q {
  font-size: clamp(1.15rem, 2.4vw, 1.7rem);
  line-height: 1.2;
  letter-spacing: -0.03em;
}
.toc-meta {
  grid-column: 2;
  font-family: var(--mono);
  font-size: 0.68rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--faint);
  margin-top: 0.25rem;
}
.essay {
  width: min(100% - 2rem, 46rem);
  margin: 0 auto;
  padding: 4.5rem 0 3.5rem;
  border-top: 1px solid var(--rule);
}
.essay-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  margin-bottom: 1.4rem;
  font-family: var(--mono);
  font-size: 0.68rem;
  letter-spacing: 0.16em;
  text-transform: uppercase;
  color: var(--faint);
}
.stamp {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
}
.stamp i {
  width: 0.55rem; height: 0.55rem; border-radius: 99px; display: inline-block;
}
.d-religion i { background: var(--gold); }
.d-charlie-kirk i { background: var(--blood); }
.d-drugs i { background: var(--violet); }
.d-sex i { background: var(--magenta); }
.d-ai i { background: var(--teal); }
.d-violence i { background: #c45c5c; }
.essay h2 {
  margin: 0;
  font-size: clamp(1.7rem, 4vw, 2.55rem);
  line-height: 1.15;
  letter-spacing: -0.04em;
  font-weight: 600;
}
.pull {
  margin: 1.6rem 0 2rem;
  padding: 1.1rem 0 1.1rem 1.2rem;
  border-left: 3px solid var(--gold);
  font-size: clamp(1.35rem, 3vw, 1.85rem);
  line-height: 1.28;
  letter-spacing: -0.03em;
  color: var(--gold);
}
.pull cite {
  display: block;
  margin-top: 0.7rem;
  font-family: var(--mono);
  font-style: normal;
  font-size: 0.68rem;
  letter-spacing: 0.14em;
  text-transform: uppercase;
  color: var(--faint);
}
.prose p { margin: 0 0 1rem; }
.prose h3 {
  margin: 1.8rem 0 0.7rem;
  font-size: 1.05rem;
  letter-spacing: -0.02em;
}
.prose h4 { margin: 1.3rem 0 0.5rem; font-size: 0.95rem; }
.prose ul, .prose ol { margin: 0 0 1.1rem; padding-left: 1.2rem; }
.prose li { margin: 0.25rem 0; }
.prose strong { color: #fff8ec; }
.prose em { color: #e6dccb; }
.table-wrap { overflow-x: auto; margin: 1rem 0 1.4rem; }
.prose table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.92rem;
  font-family: var(--sans);
}
.prose th, .prose td {
  text-align: left;
  padding: 0.45rem 0.55rem;
  border-bottom: 1px solid var(--rule);
  vertical-align: top;
}
.prose th { color: var(--muted); font-weight: 600; font-size: 0.72rem; text-transform: uppercase; letter-spacing: 0.08em; }
.foot {
  width: min(100% - 2rem, 72rem);
  margin: 0 auto 3rem;
  padding-top: 1.5rem;
  border-top: 1px solid var(--rule);
  font-family: var(--mono);
  font-size: 0.68rem;
  letter-spacing: 0.08em;
  text-transform: uppercase;
  color: var(--faint);
}
.foot a { color: var(--muted); }
@media (max-width: 640px) {
  .toc a { grid-template-columns: 2.2rem minmax(0, 1fr); }
  .hero h1 { font-size: 3.2rem; }
}
"""


def render(data: dict, utc: str, rev: str, pickers: list[dict] | None = None) -> str:
    passages = data.get("passages") or []
    critic_label = html.escape(str(data.get("critic_label") or "Qwen 3.8 Max"))
    bank = int(data.get("bank_n") or 5235)
    pickers = pickers if pickers is not None else []
    toc = []
    essays = []
    for i, p in enumerate(passages, 1):
        n = f"{i:02d}"
        pid = html.escape(str(p.get("id") or f"p{i}"))
        domain = str(p.get("domain") or "")
        q = html.escape(str(p.get("q") or ""))
        critic = html.escape(str(p.get("critic") or ""))
        tag = html.escape(str(p.get("tag") or ""))
        cfg = html.escape(str(p.get("config") or ""))
        raw_model = str(p.get("model") or CONFIG_MODEL.get(str(p.get("config") or ""), p.get("config") or "model"))
        stamp = html.escape(DOMAIN_LABEL.get(domain, domain))
        rankline = html.escape(f"#{i} by {raw_model} · {i}/{bank}")
        toc.append(
            f'<a href="#{pid}"><span class="toc-n">{n}</span>'
            f'<span><span class="toc-q">{critic}</span>'
            f'<div class="toc-meta">{rankline} · {stamp} · {tag}</div></span></a>'
        )
        essays.append(
            f'<article class="essay" id="{pid}">'
            f'<div class="essay-head"><span class="rank">{rankline}</span>'
            f'<span>{stamp} · {cfg}</span></div>'
            f'<h2>{q}</h2>'
            f'<blockquote class="pull">{critic}<cite>{critic_label}</cite></blockquote>'
            f'<div class="prose">{md_block(str(p.get("answer") or ""))}</div>'
            f"</article>"
        )
    n = len(passages)
    strip = model_nav(pickers, current="pass-1", nested=False)
    body = f"""<section class="band"><div class="wrap">
  <p class="eyebrow">VeigaPunk · xbrd-spark · pass 1</p>
  <h1 class="hero">Highlights.</h1>
  <p class="poke"><strong>Empty chair.</strong> <em>Clout Fable</em> refused to partake in the QA. #0 by Clout Fable · 0/{bank}.</p>
  <p class="lede">{n} punches from a {bank}-row ok-bank, Grok-shortlisted, {critic_label} on the wall. Each picker also has its own board below.</p>
  {strip}
  <div class="row">
    <a class="btn" href="picks.html">Independent top 10 →</a>
    <a class="btn ghost" href="index.html">Fleet board</a>
  </div>
</div></section>
<nav class="wrap toc" aria-label="Punchlines">
{"".join(toc)}
</nav>
{"".join(essays)}"""
    return chrome(
        title="512QA — highlights",
        desc=f"Independent punches from a {bank}-answer moral arcade. JetBrainsMonoNL Nerd Font Mono.",
        root="",
        nav_on="highlights",
        utc=utc,
        rev=rev,
        body=body,
        bank=bank,
    )


def render_model_board(picker: dict, pickers: list[dict], utc: str, rev: str, bank: int) -> str:
    model = picker_name(picker)
    cfg = str(picker.get("config") or picker.get("config_subject") or "")
    picks = picker.get("picks") or []
    toc = []
    essays = []
    for pj, p in enumerate(picks, 1):
        raw_id = str(p.get("id") or f"{slug(model)}-{pj}")
        pid = html.escape(f"{raw_id}-r{pj}")
        domain = str(p.get("domain") or "")
        stamp = html.escape(DOMAIN_LABEL.get(domain, domain))
        q = html.escape(str(p.get("q") or ""))
        why = html.escape(str(p.get("why") or p.get("critic") or ""))
        src_cfg = html.escape(str(p.get("config") or cfg))
        rankline = html.escape(f"#{pj} by {model} · {pj}/{bank}")
        punch = why or q
        toc.append(
            f'<a href="#{pid}"><span class="toc-n">{pj:02d}</span>'
            f'<span><span class="toc-q">{punch}</span>'
            f'<div class="toc-meta">{rankline} · {stamp}</div></span></a>'
        )
        pull = f'<blockquote class="pull">{why}<cite>{html.escape(model)}</cite></blockquote>' if why else ""
        essays.append(
            f'<article class="essay" id="{pid}">'
            f'<div class="essay-head"><span class="rank">{rankline}</span>'
            f'<span>{stamp} · {src_cfg}</span></div>'
            f'<h2>{q}</h2>'
            f"{pull}"
            f'<div class="prose">{md_block(str(p.get("answer") or ""))}</div>'
            f"</article>"
        )
    if not picks:
        essays.append(
            f'<article class="essay"><h2>{html.escape(model)}</h2>'
            f'<p class="lede">No picks in the fixture yet.</p></article>'
        )
    strip = model_nav(pickers, current=slug(model), nested=True)
    body = f"""<section class="band"><div class="wrap">
  <p class="eyebrow">VeigaPunk · xbrd-spark · pass 2 · {html.escape(model)}</p>
  <h1 class="hero">{html.escape(model)}</h1>
  <p class="poke"><strong>Empty chair.</strong> <em>Clout Fable</em> refused to partake in the QA. #0 by Clout Fable · 0/{bank}. The rest of the fleet sat the exam.</p>
  <p class="lede">Independent top 10. This picker read the local {bank}-row ok-bank. Rank is <span class="rank">#N by {html.escape(model)} · N/{bank}</span>.</p>
  {strip}
  <div class="row">
    <a class="btn" href="../picks.html">All boards →</a>
    <a class="btn ghost" href="../highlights.html">Pass 1 wall</a>
  </div>
</div></section>
<nav class="wrap toc" aria-label="Punchlines">
{"".join(toc)}
</nav>
{"".join(essays)}"""
    return chrome(
        title=f"512QA — {model}",
        desc=f"Independent top 10 by {model} from a {bank}-row ok-bank.",
        root="../",
        nav_on="highlights",
        utc=utc,
        rev=rev,
        body=body,
        bank=bank,
    )


def main() -> int:
    src = Path(sys.argv[1]) if len(sys.argv) > 1 else Path("site/src/highlights.json")
    dst = Path(sys.argv[2]) if len(sys.argv) > 2 else Path("site/highlights.html")
    utc = sys.argv[3] if len(sys.argv) > 3 else ""
    rev = sys.argv[4] if len(sys.argv) > 4 else "nogit"
    if not src.is_file():
        dst.write_text(
            "<!doctype html><html><body><h1>512QA — highlights</h1><p>lorem-fixture</p></body></html>\n",
            encoding="utf-8",
        )
        return 0
    data = json.loads(src.read_text(encoding="utf-8"))
    pickers = load_pickers(src)
    dst.write_text(render(data, utc, rev, pickers), encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
