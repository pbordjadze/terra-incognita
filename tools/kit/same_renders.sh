#!/usr/bin/env bash
# Proves a change to the kit changes no toy's look: every toy's plans (toys/*/plans/*.json) played by
# one set of check binaries, then by another, and every PNG (shots and film frames) and WAV compared
# byte for byte. Run from the repository root, after `python3 tools/toys/assets.py <toy> --link` for
# every toy:
#
#     bash tools/kit/same_renders.sh render <bin-dir> <out-dir> [toy...]  # plays each plan with
#                                                                          # <bin-dir>/<toy>-check
#     bash tools/kit/same_renders.sh compare <out-a> <out-b>               # nonzero if any file differs
#
# Typical use: build the check binaries on the base commit and copy them aside
# (`cp toys/target/release/*-check toys/build/before-bin/`), render into toys/build/before, build
# the change, render toys/target/release into toys/build/after, compare. Each plan's lines are kept
# in <out-dir>/<toy>-<plan>.log (with the kit's touch check, its WARN lines: the touches that did
# nothing). The touch check runs as it does in CI, so the comparison covers the extra frames it draws.
set -euo pipefail
cmd="${1:?usage: same_renders.sh render <bin-dir> <out-dir> [toy...] | compare <out-a> <out-b>}"
case "$cmd" in
render)
  bin="$(cd "${2:?bin-dir}" && pwd)"; out="$(mkdir -p "${3:?out-dir}" && cd "$3" && pwd)"; shift 3
  toys=("$@"); [ ${#toys[@]} -gt 0 ] || toys=(whales murmuration fireflies)
  cd "$(dirname "$0")/../../toys"
  for t in "${toys[@]}"; do
    for p in "$t"/plans/*.json; do
      n="$t-$(basename "$p" .json)"
      start=$(date +%s)
      "$bin/$t-check" play "$p" "$out/$n" > "$out/$n.log" 2>&1 || { echo "FAIL $n (see $out/$n.log)"; exit 1; }
      echo "played $n in $(( $(date +%s) - start )) s"
    done
  done
  ;;
compare)
  a="${2:?out-a}"; b="${3:?out-b}"
  bad=0; n=0
  while IFS= read -r f; do
    n=$((n + 1))
    if [ ! -f "$b/$f" ]; then echo "missing in $b: $f"; bad=$((bad + 1));
    elif ! cmp -s "$a/$f" "$b/$f"; then echo "differs: $f"; bad=$((bad + 1)); fi
  done < <(cd "$a" && find . -type f \( -name '*.png' -o -name '*.wav' \) | sort)
  extra=$(comm -13 <(cd "$a" && find . -type f \( -name '*.png' -o -name '*.wav' \) | sort) \
                   <(cd "$b" && find . -type f \( -name '*.png' -o -name '*.wav' \) | sort) | wc -l)
  [ "$extra" -eq 0 ] || { echo "$extra files only in $b"; bad=$((bad + extra)); }
  echo "$n files compared, $bad differ"
  [ "$bad" -eq 0 ]
  ;;
*) echo "unknown command $cmd"; exit 2 ;;
esac
