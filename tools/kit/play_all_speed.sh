#!/usr/bin/env bash
# Measures what `play-all` saves for one toy: its plans played one launch each (as toys.yml does),
# then all in one process on N threads; both without the touch check, so only the launching and the
# sharing out differ. Run from the repository root after `python3 tools/toys/assets.py <toy> --link`
# and a release build of the toy's check:
#
#     bash tools/kit/play_all_speed.sh <toy> [threads=4]     # writes toys/build/speed/<toy>-{one,all}
set -euo pipefail
toy="${1:?usage: play_all_speed.sh <toy> [threads]}"; j="${2:-4}"
cd "$(dirname "$0")/../../toys"
bin="target/release/$toy-check"
out="build/speed/$toy"
rm -rf "$out-one" "$out-all"
start=$(date +%s.%N)
for p in "$toy"/plans/*.json; do
  "$bin" play "$p" "$out-one/$(basename "$p" .json)" --no-tap-check > /dev/null
done
one=$(echo "$(date +%s.%N) - $start" | bc)
start=$(date +%s.%N)
"$bin" play-all "$toy/plans" "$out-all" -j "$j" --no-tap-check
all=$(echo "$(date +%s.%N) - $start" | bc)
printf 'one launch a plan: %.1f s; play-all -j %s: %.1f s (%.2fx)\n' "$one" "$j" "$all" "$(echo "$one / $all" | bc -l)"
