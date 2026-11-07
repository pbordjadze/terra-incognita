#!/usr/bin/env bash
# Proves `adrift-check play-all` plays every plan exactly as separate `play` launches do: the full
# set (the GPU runner's: every plan with "steps" in adrift-check's, touch's, look's and sea's
# plans) played both ways, then each plan's log compared line for line with its timings (the
# measured ms, the export's seconds) and its output directory blanked, and every PNG and CSV the
# plans wrote compared byte for byte. The mp4s are only counted (the encoder's own threads make them
# differ run to run, either way). Prints the wall time of each way. Tens of minutes on Linux's
# software Vulkan (tools/adrift/linux-setup.sh).
#
#     bash tools/adrift/play_all_same.sh [work-dir] [-j N]   # default adrift/build/out/play-all-same
#
# Exits non-zero if anything differs. Its first run (4 CPUs, lavapipe, release): 81 plans, 162 logs
# and 205 PNGs and CSVs the same; one launch a plan 2222 s, play-all (-j 4) 1371 s.
set -euo pipefail
cd "$(dirname "$0")/../.."
WORK=${1:-adrift/build/out/play-all-same}
shift || true
JOBS=("$@")
python3 tools/adrift/assets.py adrift/build/assets --link > /dev/null
(cd adrift && cargo build --release -q -p adrift-check)
BIN="$PWD/adrift/target/release/adrift-check"
DIRS=(adrift/crates/adrift-check/plans adrift/crates/adrift-core/src/touch/plans adrift/crates/adrift-core/src/look/plans adrift/crates/adrift-core/src/sea/plans)
rm -rf "$WORK"; mkdir -p "$WORK/each" "$WORK/all"
WORK=$(cd "$WORK" && pwd)

# the names play-all gives: a name two plans share takes its module's before it
declare -A count=()
plans=()
for d in "${DIRS[@]}"; do
  for p in "$d"/*.json; do
    grep -q '"steps"' "$p" || continue
    plans+=("$p"); n=$(basename "$p" .json); count[$n]=$(( ${count[$n]:-0} + 1 ))
  done
done
name_of() {
  local n; n=$(basename "$1" .json)
  if [ "${count[$n]}" -gt 1 ]; then echo "$(basename "$(dirname "$(dirname "$1")")")-$n"; else echo "$n"; fi
}

echo "== ${#plans[@]} plans, one launch each"
t0=$(date +%s.%N)
for p in "${plans[@]}"; do
  n=$(name_of "$p")
  if (cd adrift && "$BIN" play "../$p" "$WORK/each/$n" > "$WORK/each/$n.log" 2>&1); then echo "ok   $n"; else echo "FAIL $n"; fi
done > "$WORK/each.txt"
t1=$(date +%s.%N)
echo "== the same plans, play-all ${JOBS[*]}"
(cd adrift && "$BIN" play-all "${DIRS[@]/#adrift\//}" "$WORK/all" "${JOBS[@]}") > "$WORK/all.txt" || true
t2=$(date +%s.%N)

status=0
# the lines each run printed (ok/FAIL, in order)
if ! diff <(awk '{print $1, $2}' "$WORK/each.txt" | LC_ALL=C sort -k2) <(grep -E '^(ok|FAIL)' "$WORK/all.txt" | awk '{print $1, $2}' | LC_ALL=C sort -k2); then
  echo "DIFFERENT: the ok/FAIL lines"; status=1
fi
norm() {
  sed -E -e "s#$WORK/(each|all)/#OUT/#g" -e 's/[0-9]+\.[0-9]+ (ms|worst)/# \1/g' -e 's/ in [0-9]+\.[0-9]+s,/ in #s,/' "$1"
}
logs=0; logs_same=0
for p in "${plans[@]}"; do
  n=$(name_of "$p")
  for f in "$n.log" "$n/play.log"; do
    [ -f "$WORK/each/$f" ] || [ -f "$WORK/all/$f" ] || continue
    logs=$((logs + 1))
    if diff <(norm "$WORK/each/$f") <(norm "$WORK/all/$f") > "$WORK/diff.txt" 2>&1; then
      logs_same=$((logs_same + 1))
    else
      echo "DIFFERENT: $f"; head -6 "$WORK/diff.txt" | sed 's/^/     /'; status=1
    fi
  done
done
files=0; files_same=0
while IFS= read -r f; do
  files=$((files + 1))
  if cmp -s "$WORK/each/$f" "$WORK/all/$f"; then files_same=$((files_same + 1)); else echo "DIFFERENT: $f"; status=1; fi
done < <(cd "$WORK/each" && find . \( -name '*.png' -o -name '*.csv' \) | sort)
extra=$(comm -13 <(cd "$WORK/each" && find . -type f -not -name '*.mp4' | sort) <(cd "$WORK/all" && find . -type f -not -name '*.mp4' | sort))
# (sed: a prefix on every line, which ${var//} cannot write)
# shellcheck disable=SC2001
if [ -n "$extra" ]; then echo "DIFFERENT: only play-all wrote:"; echo "$extra" | sed 's/^/     /'; status=1; fi
mp4s=$(cd "$WORK/each" && find . -name '*.mp4' | wc -l); mp4s_all=$(cd "$WORK/all" && find . -name '*.mp4' | wc -l)

echo "logs: $logs_same of $logs the same (timings aside); PNGs and CSVs: $files_same of $files byte for byte; mp4s: $mp4s and $mp4s_all"
echo "plans: $(grep -c '^ok ' "$WORK/each.txt") ok, $(grep -c '^FAIL ' "$WORK/each.txt") failed (each); $(grep -c '^ok ' "$WORK/all.txt") ok, $(grep -c '^FAIL ' "$WORK/all.txt") failed (play-all)"
awk -v a="$t0" -v b="$t1" -v c="$t2" 'BEGIN { printf "wall: one launch a plan %.0f s; play-all %.0f s\n", b - a, c - b }'
[ $status = 0 ] && echo "SAME" || echo "NOT THE SAME"
exit $status
