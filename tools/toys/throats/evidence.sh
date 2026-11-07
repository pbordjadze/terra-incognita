#!/usr/bin/env bash
# Assembles a critic's evidence for Throats (painting-company row 257): plays the plans named, films
# them, and gathers full-size stills, 1:1 crops, frames and the verdict log into one directory.
#
#     bash tools/toys/throats/evidence.sh <out-dir> <plan> [<plan>...]     # from the repository root
#
# Each plan is toys/throats/plans/<plan>.json; its shots become <out>/<plan>-<shot>.png, its film
# <out>/films/<plan>/ (film.mp4, sheet.png, frames/, plan.json). Crops: the middle 600 x 600 pixels of
# every shot, at 1:1, in <out>/crops/.
set -euo pipefail
out=$1; shift
root=$(cd "$(dirname "$0")/../../.." && pwd)
mkdir -p "$out/crops" "$out/films"
(cd "$root/toys" && cargo build --release -q -p throats --bin throats-check)
for plan in "$@"; do
  run="$out/runs/$plan"
  rm -rf "$run"
  (cd "$root/toys" && target/release/throats-check play "throats/plans/$plan.json" "$run" | grep -v '^shot' || true)
  for png in "$run"/*.png; do
    [ -e "$png" ] || continue
    name="$plan-$(basename "$png")"
    cp "$png" "$out/$name"
    python3 - "$png" "$out/crops/$name" <<'PY'
import sys
from PIL import Image
im = Image.open(sys.argv[1]); w, h = im.size
im.crop((w // 2 - 300, h // 2 - 300, w // 2 + 300, h // 2 + 300)).save(sys.argv[2])
PY
  done
  if [ -d "$run/film" ]; then
    bash "$root/tools/toys/film.sh" "$run" >/dev/null
    mkdir -p "$out/films/$plan"
    mv "$run/film.mp4" "$run/sheet.png" "$out/films/$plan/"
    rm -rf "$out/films/$plan/frames"; mv "$run/film" "$out/films/$plan/frames"
    cp "$root/toys/throats/plans/$plan.json" "$out/films/$plan/plan.json"
  fi
done
rm -rf "$out/runs"
cp "$root/toys/throats/ROUNDS.md" "$out/ROUNDS.md" 2>/dev/null || echo "(no rounds yet)" > "$out/ROUNDS.md"
echo "evidence in $out"
