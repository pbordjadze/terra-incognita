#!/usr/bin/env bash
# Plays Battle Ink's plans in full, as CI does, into toys/build/battle/<plan>/ (moments, film frames), and
# makes each film watchable (film.mp4 and sheet.png, tools/toys/film.sh). Software Vulkan: minutes a plan.
#
#     bash tools/toys/battle/films.sh [plan ...]        # all of toys/battle/plans when none is named
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
python3 "$ROOT/tools/toys/assets.py" battle --link > /dev/null
cd "$ROOT/toys"
cargo build -q --release -p battle --bin battle-check
PLANS=("$@")
[ ${#PLANS[@]} -gt 0 ] || PLANS=($(cd battle/plans && ls *.json | sed 's/\.json$//'))
for p in "${PLANS[@]}"; do
  rm -rf "build/battle/$p"
  ./target/release/battle-check play "battle/plans/$p.json" "build/battle/$p"
  if [ -d "build/battle/$p/film" ]; then bash ../tools/toys/film.sh "build/battle/$p"; fi
done
