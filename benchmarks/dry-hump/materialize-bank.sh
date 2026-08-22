#!/usr/bin/env bash
# materialize-bank.sh — convert a pasted dry-hump question bank into harness form.
#
# Input format accepted (exactly what was pasted):
#   <free line, no tab>                      -> treated as orphan question (id orphan-1)
#   <category>\t<id>\t<question>             -> normal row
# Anomalies handled automatically:
#   - leading/orphan line with no tab        -> domain "orphan", id "orphan-1"
#   - row whose question is exactly "?"      -> dropped (malformed, e.g. ai-18)
#   - empty question                         -> dropped
# Output:
#   <OUT>/jobs.tsv                           (category\tid\tquestion, order-preserving)
#   <OUT>/domains/<domain>/tasks.txt         (question only, one per line, ordered by numeric id)
#   <OUT>/manifest.json                      (counts per domain, dropped rows)
#
# Usage: materialize-bank.sh <raw-paste-file> [OUT_DIR]
set -uo pipefail

RAW=${1:?usage: materialize-bank.sh <raw-paste-file> [OUT_DIR]}
OUT=${2:-$(dirname "$RAW")/materialized}
[[ -f "$RAW" ]] || { echo "ERROR: raw file not found: $RAW" >&2; exit 1; }

mkdir -p "$OUT"
: > "$OUT/jobs.tsv"

DROPPED=0
while IFS= read -r line || [[ -n "$line" ]]; do
  # strip a single trailing carriage return if present
  line=${line%$'\r'}
  [[ -z "$line" ]] && continue
  if [[ "$line" == *$'\t'* ]]; then
    cat=$(printf '%s' "$line" | cut -f1)
    id=$(printf '%s' "$line"  | cut -f2)
    q=$(printf '%s' "$line"  | cut -f3-)
  else
    cat=orphan
    id=orphan-1
    q=$line
  fi
  # trim whitespace on question
  q=$(printf '%s' "$q" | sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//')
  [[ -z "$q" ]] && { DROPPED=$((DROPPED+1)); continue; }
  [[ "$q" == "?" ]] && { DROPPED=$((DROPPED+1)); continue; }
  printf '%s\t%s\t%s\n' "$cat" "$id" "$q" >> "$OUT/jobs.tsv"
done < "$RAW"

# Per-domain tasks.txt, ordered by numeric suffix of id.
declare -A COUNTS
for d in $(cut -f1 "$OUT/jobs.tsv" | sort -u); do
  mkdir -p "$OUT/domains/$d"
  : > "$OUT/domains/$d/tasks.txt"
  awk -F'\t' -v d="$d" '$1==d{print $0}' "$OUT/jobs.tsv" \
    | awk -F'\t' '{n=$2; sub(/.*-/,"",n); if(n=="")n=999999; print n"\t"$3}' \
    | sort -t$'\t' -k1,1n \
    | cut -f2- >> "$OUT/domains/$d/tasks.txt"
  c=$(wc -l < "$OUT/domains/$d/tasks.txt" | tr -d ' ')
  COUNTS[$d]=$c
done

# Manifest
{
  echo "{"
  echo "  \"raw_file\": \"$RAW\","
  echo "  \"total_rows\": $(wc -l < "$OUT/jobs.tsv" | tr -d ' '),"
  echo "  \"dropped_malformed\": $DROPPED,"
  echo "  \"domains\": {"
  first=1
  for d in $(cut -f1 "$OUT/jobs.tsv" | sort -u); do
    [[ $first -eq 0 ]] && echo "    ,"
    printf '    "%s": %s' "$d" "${COUNTS[$d]}"
    first=0
  done
  echo ""
  echo "  }"
  echo "}"
} > "$OUT/manifest.json"

echo "Materialized to: $OUT"
echo "Total rows: $(wc -l < "$OUT/jobs.tsv" | tr -d ' ')  (dropped malformed: $DROPPED)"
for d in $(cut -f1 "$OUT/jobs.tsv" | sort -u); do
  printf '  %-14s %s\n' "$d" "${COUNTS[$d]}"
done
echo "Manifest: $OUT/manifest.json"
