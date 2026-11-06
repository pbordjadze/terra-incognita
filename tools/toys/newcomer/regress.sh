#!/usr/bin/env bash
# The fixed first-minute plans, every round, on both devices, with a contact sheet each (painting-company
# row 246): the same hands each round, so a fix that breaks what worked shows.
#
#     bash tools/toys/newcomer/regress.sh <toy> <out-dir>        # plays <toy>/plans/first-*.json
set -euo pipefail
TOY="${1:?toy}"; OUT="${2:?out}"; ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
for p in "$ROOT"/toys/"$TOY"/plans/first-*.json; do
  n="$(basename "$p" .json)"
  for dev in phone ipad; do
    bash "$ROOT/tools/toys/newcomer/try.sh" "$TOY" "$p" "$OUT/$n-$dev" "$dev" > /dev/null
    python3 "$ROOT/tools/toys/newcomer/sheet.py" "$OUT/$n-$dev" "$OUT/$n-$dev.sheet.png" 10
    echo "$OUT/$n-$dev.sheet.png"
  done
done
