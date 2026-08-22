#!/usr/bin/env bash
# gen-site.sh — render site/index.html for the 512QA campaign from
# telemetry-512qa-multi/runs/*/summary.json. bash + jq only; no frameworks,
# no JS build step, no external assets. Strictly read-only wrt runs/.
#
# Honesty rules (non-negotiable):
#   * configs with a .junk field -> "quota-stub" badge, sort last regardless of numbers
#   * wall_seconds == 0          -> derive wall: sum .duration_s from gen-telemetry.jsonl,
#                                   else sum .barrier_s from batch-times.jsonl;
#                                   display "~Ns (derived)"; if unreconstructable -> "—"
#   * ok + fail < expected       -> "incomplete" badge
# Idempotent: re-running rebuilds the same page from current summaries
# (footer timestamp aside). jq parse failure on any summary = hard fail.
set -euo pipefail

HERE=$(cd "$(dirname "$0")" && pwd)
RUNS="$HERE/telemetry-512qa-multi/runs"
OUTDIR="$HERE/site"
OUT="$OUTDIR/index.html"
TMP=$(mktemp)
trap 'rm -f "$TMP"' EXIT

mkdir -p "$OUTDIR"

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

  rows+=("$(jq -c --arg dir_cfg "$cfg" --argjson derived "$derived" '
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
      junk:      (.junk // null)
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

{
cat <<'HEAD'
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="color-scheme" content="light dark">
<title>512QA campaign results — xbrd-spark dry-hump</title>
<style>
:root { color-scheme: light dark; }
* { box-sizing: border-box; }
body {
  font-family: system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
  margin: 2rem auto; max-width: 1180px; padding: 0 1rem;
  background: #f6f7f9; color: #17181c; line-height: 1.45;
}
h1 { font-size: 1.35rem; margin-bottom: 0.2rem; }
.muted { color: #6b7280; font-size: 0.82rem; }
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
footer { margin-top: 1.1rem; font-size: 0.8rem; color: #6b7280; }
@media (prefers-color-scheme: dark) {
  body { background: #0f1115; color: #e5e7eb; }
  table { background: #171a21; border-color: #2a2f3a; }
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
</style>
</head>
<body>
<h1>512QA campaign — results</h1>
<p class="muted">Self-contained static page generated by <code>gen-site.sh</code> from
<code>telemetry-512qa-multi/runs/*/summary.json</code> (bash + jq only, no external assets).
Read-only snapshot of <code>runs/</code> at generation time; live runs without a
<code>summary.json</code> are excluded. Wall times marked <em>(derived)</em> are
reconstructed from per-domain telemetry, not measured.</p>
<table>
<thead>
<tr><th>config</th><th>lane</th><th>model</th><th class="n">ok</th><th class="n">fail</th><th class="n">expected</th><th class="n">completion</th><th class="n">wall_s</th><th class="n">mutations ok/total</th><th>flags</th></tr>
</thead>
<tbody>
HEAD

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
    + "<td class=\"mono \($c)\">\(.config | h)</td>"
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
<footer>generated $UTC @ $REV, $N configs</footer>
</body>
</html>
FOOT
} > "$TMP"
mv "$TMP" "$OUT"
echo "wrote $OUT ($N configs)"
