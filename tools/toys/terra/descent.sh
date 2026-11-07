#!/usr/bin/env bash
# The scale loop's evidence (painting-company row 258): Terra's descent plans played on the phone (and the
# India descent on the 13-inch iPad too), each made a film (H.264, 20 fps) with its plan beside it, into
# one directory a critic is handed (tools/toys/newcomer/SCALE_CRITIC.md is its brief).
#
#     bash tools/toys/terra/descent.sh <out-dir>
#     ONLY="meander-phone murmuration-phone" bash tools/toys/terra/descent.sh <out-dir>   # just those films
#
# Needs Terra's check built (cd toys && cargo build --release -p terra --bin terra-check) and its data
# linked (python3 tools/toys/assets.py terra --link). Prints each plan's slowest update (CPU) as a line.
set -euo pipefail
OUT="$(realpath -m "${1:?out dir}")"
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
BIN="$ROOT/toys/target/release/terra-check"
mkdir -p "$OUT"
play() { # plan device name
  local plan="$ROOT/toys/terra/plans/$1.json" dev="$2" name="$3" size ppp
  if [ -n "${ONLY:-}" ] && ! [[ " $ONLY " == *" $name "* ]]; then return 0; fi
  case "$dev" in phone) size=1179x2556; ppp=3 ;; ipad) size=2064x2752; ppp=2 ;; esac
  rm -rf "$OUT/raw/$name"
  ( cd "$ROOT/toys" && "$BIN" play "$plan" "$OUT/raw/$name" --size "$size" --ppp "$ppp" --no-tap-check ) > "$OUT/raw/$name.log" 2>&1 \
    || { echo "$name: FAILED (see $OUT/raw/$name.log)"; return 1; }
  ffmpeg -loglevel error -y -framerate 20 -i "$OUT/raw/$name/film/%04d.png" -vf "scale='min(720,iw)':-2" -c:v libx264 -crf 20 -pix_fmt yuv420p "$OUT/$name.mp4"
  cp "$plan" "$OUT/$name.json"
  # (the frames are full-size PNGs, gigabytes a round: only the film is kept, and the shots)
  rm -rf "$OUT/raw/$name/film"
  echo "$name: $(grep -o 'update [0-9.]* ms' "$OUT/raw/$name.log" | sort -t' ' -k2 -n | tail -1)"
}
mkdir -p "$OUT/raw"
play descent phone india-phone &
play descent-meander phone meander-phone &
play descent-murmuration phone murmuration-phone &
wait
play descent-ipad ipad india-ipad
ls -l "$OUT"/*.mp4
