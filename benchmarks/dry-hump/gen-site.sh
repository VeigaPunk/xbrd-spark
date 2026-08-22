#!/usr/bin/env bash
# gen-site.sh — render the 512QA campaign site from
# telemetry-512qa-multi/runs/*/summary.json. bash + jq only; no frameworks,
# no JS build step, no external assets. Strictly read-only wrt runs/
# (per-domain breakdowns read local answers.jsonl when present).
#
# Emits:
#   site/index.html          fleet leaderboard (completion desc, junk last)
#   site/highlights.html     editorial board from site/src/highlights.json (pass 1)
#   site/picks.html          pass 2 independent top 10 from site/src/picks-by-model.json
#   site/cfg/<config>.html   per-config page w/ per-domain breakdown
#
# Honesty rules (non-negotiable):
#   * configs with a .junk field -> "quota-stub" badge, sort last regardless of numbers
#   * wall_seconds == 0          -> derive wall: sum .duration_s from gen-telemetry.jsonl,
#                                   else sum .barrier_s from batch-times.jsonl;
#                                   display "~Ns (derived)"; if unreconstructable -> "—"
#   * ok + fail < expected       -> "incomplete" badge
#   * missing per-domain data    -> "—" never a guess
# Idempotent: re-running rebuilds the same pages from current summaries
# (footer timestamp aside). jq parse failure on any summary = hard fail.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
RUNS="$HERE/telemetry-512qa-multi/runs"
OUTDIR="$HERE/site"
CFGDIR="$OUTDIR/cfg"
OUT="$OUTDIR/index.html"
TMP=$(mktemp)
TMP2=$(mktemp)
trap 'rm -f "$TMP" "$TMP2"' EXIT

mkdir -p "$OUTDIR" "$CFGDIR"

DOMAINS=(religion sex drugs politics money violence ai charlie-kirk)

rows=()
for p in "$RUNS"/*/summary.json; do
  [[ -f "$p" ]] || continue
  dir=${p%/*}
  cfg=${dir##*/}

  # Derived wall for zero-wall runs; never fabricated.
  derived=null
  if jq -e '(.wall_seconds // 0) == 0' "$p" >/dev/null; then
    gt="$dir/gen-telemetry.jsonl"
    bt="$dir/batch-times.jsonl"
    if [[ -s $gt ]]; then
      derived=$(jq -s '([.[].duration_s // 0] | add) // 0' "$gt")
    elif [[ -s $bt ]]; then
      derived=$(jq -s '([.[].barrier_s // 0] | add) // 0' "$bt")
    fi
  fi

  # Per-domain cells/ok from local answers.jsonl (retry-merged, unique-by-id last-wins).
  doms="[]"
  for d in "${DOMAINS[@]}"; do
    f="$dir/$d/answers.jsonl"
    if [[ -s $f ]]; then
      stat=$(jq -s '{n:length, ok:[.[]|select(.status=="ok")]|length}' "$f")
    else
      stat='null'
    fi
    doms=$(jq -c --arg d "$d" --argjson s "$stat" '. + [{domain:$d, cells:$s}]' <<<"$doms")
  done

  rows+=("$(jq -c --arg dir_cfg "$cfg" --argjson derived "$derived" --argjson domains "$doms" '
    {
      config:    (.config // $dir_cfg),
      lane:      (.lane // "—"),
      model:     (.model // "—"),
      ok:        (.answers_ok // 0),
      fail:      (.answers_fail // 0),
      expected:  (.jobs_expected // 0),
      wall:      (.wall_seconds // 0),
      derived:   $derived,
      mut_ok:    (if (.mutations | type) == "object" then .mutations.ok else null end),
      mut_total: (if (.mutations | type) == "object" then .mutations.total else null end),
      junk_flag: has("junk"),
      junk:      (.junk // null),
      domains:   $domains
    }
    | .rate = (if .expected > 0 then .ok / .expected else -1 end)
    | .incomplete = ((.ok + .fail) < .expected)
  ' "$p")")
done

if ((${#rows[@]} == 0)); then
  echo "gen-site.sh: no summary.json found under $RUNS" >&2
  exit 1
fi

UTC=$(date -u +%Y-%m-%dT%H:%M:%SZ)
REV=$(git -C "$HERE" rev-parse --short HEAD 2>/dev/null || echo nogit)
N=${#rows[@]}

CSS=$(cat <<'CSSEOF'
:root { color-scheme: light dark; }
* { box-sizing: border-box; }
body {
  font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  margin: 2rem auto; max-width: 1180px; padding: 0 1rem;
  background: #f6f7f9; color: #17181c; line-height: 1.45;
}
h1 { font-size: 1.35rem; margin-bottom: 0.2rem; }
.muted { color: #6b7280; font-size: 0.82rem; }
a { color: #2563eb; text-decoration: none; }
a:hover { text-decoration: underline; }
code { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.9em; }
table {
  border-collapse: collapse; width: 100%; margin-top: 1rem;
  font-size: 0.88rem; background: #ffffff;
  border: 1px solid #d9dce1; border-radius: 8px; overflow: hidden;
}
thead th {
  text-align: left; font-size: 0.72rem; text-transform: uppercase;
  letter-spacing: 0.05em; color: #4b5563; background: #eceef1;
  padding: 0.55rem 0.7rem; border-bottom: 2px solid #d3d7dd; white-space: nowrap;
}
tbody td { padding: 0.45rem 0.7rem; border-bottom: 1px solid #e5e7eb; white-space: nowrap; }
tbody tr:last-child td { border-bottom: none; }
th.n, td.n { text-align: right; font-variant-numeric: tabular-nums; }
.mono { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: 0.85rem; }
tbody tr:nth-child(even) td { background: rgba(15, 23, 42, 0.025); }
td.rj { background: rgba(220, 38, 38, 0.07); }
td.ri { background: rgba(217, 119, 6, 0.07); }
tbody tr:nth-child(even) td.rj { background: rgba(220, 38, 38, 0.10); }
tbody tr:nth-child(even) td.ri { background: rgba(217, 119, 6, 0.10); }
tbody tr:hover td { background: rgba(59, 130, 246, 0.10); }
td.rj:first-child { box-shadow: inset 3px 0 0 #dc2626; }
td.ri:first-child { box-shadow: inset 3px 0 0 #d97706; }
.badge {
  display: inline-block; padding: 0.08rem 0.5rem; border-radius: 999px;
  font-size: 0.72rem; font-weight: 600; letter-spacing: 0.01em;
}
.badge.bad  { background: #fee2e2; color: #991b1b; }
.badge.warn { background: #fef3c7; color: #92400e; }
.stats { display: flex; gap: 1.2rem; flex-wrap: wrap; margin: 0.8rem 0 0.2rem; }
.stat { background: #ffffff; border: 1px solid #d9dce1; border-radius: 8px; padding: 0.5rem 0.9rem; }
.stat .v { font-size: 1.25rem; font-weight: 700; font-variant-numeric: tabular-nums; }
.stat .k { font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.05em; color: #6b7280; }
.qa-card {
  background: #ffffff; border: 1px solid #d9dce1; border-radius: 8px;
  padding: 0.9rem 1rem; margin-top: 1rem;
}
.qa-card h2 { font-size: 1rem; margin: 0 0 0.45rem; }
.qa-card p { margin: 0.35rem 0; }
.qa-card .answer, .qa-card pre { white-space: pre-wrap; }
.qa-card details { margin-top: 0.4rem; }
.chip { display:inline-block; margin-right:0.35rem; font-size:0.72rem; color:#6b7280; }
footer { margin-top: 1.1rem; font-size: 0.8rem; color: #6b7280; }
@media (prefers-color-scheme: dark) {
  body { background: #0f1115; color: #e5e7eb; }
  table, .stat, .qa-card { background: #171a21; border-color: #2a2f3a; }
  thead th { background: #20242e; color: #9ca3af; border-bottom-color: #2f3542; }
  tbody td { border-bottom-color: #262b36; }
  tbody tr:nth-child(even) td { background: rgba(255, 255, 255, 0.028); }
  td.rj { background: rgba(248, 113, 113, 0.10); }
  td.ri { background: rgba(251, 191, 36, 0.10); }
  tbody tr:nth-child(even) td.rj { background: rgba(248, 113, 113, 0.14); }
  tbody tr:nth-child(even) td.ri { background: rgba(251, 191, 36, 0.14); }
  tbody tr:hover td { background: rgba(96, 165, 250, 0.14); }
  td.rj:first-child { box-shadow: inset 3px 0 0 #f87171; }
  td.ri:first-child { box-shadow: inset 3px 0 0 #fbbf24; }
  .badge.bad  { background: #451a1a; color: #fca5a5; }
  .badge.warn { background: #451f03; color: #fcd34d; }
  .muted { color: #9ca3af; }
  footer { color: #9ca3af; }
}
CSSEOF
)

page_head() { # $1=title $2=subtitle  $3=asset prefix ('.' or '..')
  local prefix=${3:-.}
  cat <<PGHEAD
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="dark">
<meta name="theme-color" content="#0c0c0b">
<title>$1</title>
<link rel="preload" href="$prefix/fonts/JetBrainsMonoNLNerdFontMono-Regular.woff2" as="font" type="font/woff2" crossorigin>
<link rel="stylesheet" href="$prefix/assets/family.css?v=table-1w">
<style>
$CSS
body { font-family: var(--font, ui-monospace, monospace); background: var(--bg, #0c0c0b); color: var(--fg, #eceae4); }
</style>
</head>
<body>
<h1>$1</h1>
<p class="muted">$2</p>
PGHEAD
}

# ---------- index ----------
{
page_head "512QA campaign — results" \
"Self-contained static site generated by <code>gen-site.sh</code> from <code>telemetry-512qa-multi/runs/*/summary.json</code>. Wall times marked <em>(derived)</em> are reconstructed, not measured. Click a config for its per-domain breakdown — or skip the spreadsheet and read <a href=\"highlights.html\">the highlights</a> (pass 1) or <a href=\"picks.html\">independent top 10</a> (pass 2)."
cat <<'TBLHEAD'
<table>
<thead>
<tr><th>config</th><th>lane</th><th>model</th><th class="n">ok</th><th class="n">fail</th><th class="n">expected</th><th class="n">completion</th><th class="n">wall_s</th><th class="n">mutations ok/total</th><th>flags</th></tr>
</thead>
<tbody>
TBLHEAD

printf '%s\n' "${rows[@]}" | jq -sr '
  def h: tostring | @html;
  def numfmt: if . >= 1 then (round | tostring) else tostring end;
  def wallfmt:
    if .wall == 0 then
      if .derived == null then "—"
      else "~\(.derived | numfmt)s (derived)" end
    else "\(.wall | numfmt)s" end;
  def compfmt:
    if .expected > 0 then "\((((1000 * .ok) / .expected) | round) / 10)%"
    else "—" end;
  def mutfmt:
    if .mut_ok == null then "—" else "\(.mut_ok)/\(.mut_total // 0)" end;
  def flagfmt:
      (if .junk_flag then "<span class=\"badge bad\" title=\"\(.junk | h)\">quota-stub</span> <span class=\"muted\">\(.junk | h)</span>" else "" end)
    + (if .incomplete then " <span class=\"badge warn\">incomplete</span>" else "" end)
    + (if (.junk_flag or .incomplete) | not then "<span class=\"muted\">—</span>" else "" end);
  sort_by([(if .junk_flag then 1 else 0 end), (-.rate), .config])
  | .[]
  | (if .junk_flag then "rj" elif .incomplete then "ri" else "rn" end) as $c
  | "<tr>"
    + "<td class=\"mono \($c)\"><a href=\"cfg/\(.config | h).html\">\(.config | h)</a></td>"
    + "<td class=\"\($c)\">\(.lane | h)</td>"
    + "<td class=\"mono \($c)\">\(.model | h)</td>"
    + "<td class=\"n \($c)\">\(.ok)</td>"
    + "<td class=\"n \($c)\">\(.fail)</td>"
    + "<td class=\"n \($c)\">\(.expected)</td>"
    + "<td class=\"n \($c)\">\(compfmt)</td>"
    + "<td class=\"n \($c)\">\(wallfmt)</td>"
    + "<td class=\"n \($c)\">\(mutfmt)</td>"
    + "<td class=\"\($c)\">\(flagfmt)</td>"
    + "</tr>"
'

cat <<FOOT
</tbody>
</table>
<footer>generated $UTC @ $REV · $N configs · <a href="highlights.html">highlights</a> · <a href="picks.html">picks</a> · <a href="https://github.com/VeigaPunk/xbrd-spark">xbrd-spark</a></footer>
</body>
</html>
FOOT
} > "$TMP"
mv "$TMP" "$OUT"

# ---------- highlights (editorial board; own CSS, not the fleet table) ----------
HLJSON="$OUTDIR/src/highlights.json"
python3 "$HERE/render-highlights.py" "$HLJSON" "$OUTDIR/highlights.html" "$UTC" "$REV"

# ---------- picks (pass 2 independent top 10; same editorial CSS as highlights) ----------
PICKJSON="$OUTDIR/src/picks-by-model.json"
python3 "$HERE/render-picks.py" "$PICKJSON" "$OUTDIR/picks.html" "$UTC" "$REV"

# ---------- per-config pages ----------
PAGES=0
for row in "${rows[@]}"; do
  cfg=$(jq -r '.config' <<<"$row")
  page_head "512QA — $cfg" "Per-domain breakdown. Cells are unique question ids after retry merge (last answer wins); missing data shows as —." ".." > "$TMP2"

  jq -r '
    def h: tostring | @html;
    def numfmt: if . >= 1 then (round | tostring) else tostring end;
    def wallfmt:
      if .wall == 0 then
        (if .derived == null or .derived == 0 then "—"
         else "~\(.derived | numfmt)s (derived)" end)
      else "\(.wall | numfmt)s" end;
    def compfmt:
      if .expected > 0 then "\((((1000 * .ok) / .expected) | round) / 10)%"
      else "—" end;
    "<p><code>\(.model | h)</code> · lane <strong>\(.lane | h)</strong> · wall \((wallfmt) | h) · mutations <strong>\(if .mut_ok == null then "—" else "\(.mut_ok)/\(.mut_total // 0)" end | h)</strong>"
      + (if .junk_flag then " · <span class=\"badge bad\" title=\"\(.junk | h)\">quota-stub</span>" else "" end)
      + (if .incomplete then " · <span class=\"badge warn\">incomplete</span>" else "" end)
      + "</p>"
    + "<div class=\"stats\">"
    + "<div class=\"stat\"><div class=\"v\">\(.ok)</div><div class=\"k\">ok</div></div>"
    + "<div class=\"stat\"><div class=\"v\">\(.fail)</div><div class=\"k\">fail</div></div>"
    + "<div class=\"stat\"><div class=\"v\">\(.expected)</div><div class=\"k\">expected</div></div>"
    + "<div class=\"stat\"><div class=\"v\">\(compfmt | h)</div><div class=\"k\">completion</div></div>"
    + "</div>"
    + "<table><thead><tr><th>domain</th><th class=\"n\">cells</th><th class=\"n\">ok</th><th class=\"n\">coverage</th></tr></thead><tbody>"
    + ([.domains[] |
        (if .cells == null then
           "<tr><td class=\"mono\">\(.domain | h)</td><td class=\"n\">—</td><td class=\"n\">—</td><td class=\"n\">—</td></tr>"
         else
           "<tr><td class=\"mono\">\(.domain | h)</td><td class=\"n\">\(.cells.n)</td><td class=\"n\">\(.cells.ok)</td><td class=\"n\">\(if .cells.n > 0 then "\((((1000 * .cells.ok) / .cells.n) | round) / 10)%" else "—" end)</td></tr>"
         end)]
       | join(""))
    + "</tbody></table>"
    + "<footer>generated '"$UTC"' @ '"$REV"' · <a href=\"../index.html\">← fleet index</a></footer>"
  ' <<<"$row" >> "$TMP2"

  cat <<PGTAIL >> "$TMP2"
</body>
</html>
PGTAIL

  mv "$TMP2" "$CFGDIR/$cfg.html"
  PAGES=$((PAGES + 1))
done

echo "wrote $OUT ($N configs) + $PAGES config pages + $OUTDIR/highlights.html + $OUTDIR/picks.html"
