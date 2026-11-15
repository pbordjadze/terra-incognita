#!/usr/bin/env bash
# Makes a played plan's film watchable: an MP4 of its frames (with its audio.wav if there is one) and a
# contact sheet of every Nth frame, for a critic or a session to look at without a device.
#
#     bash tools/toys/film.sh <out-dir> [plan.json]          # <out>/film.mp4, <out>/sheet.png
#     bash tools/toys/film.sh <out-dir> [fps=10] [every=5]   # the older form: a frame rate given
#
# Given the plan, the film plays in real time (a frame every `film.every` seconds of the plan) with the
# sound cut to the film's own span, so what is heard lines up with what is seen; a film sparser than ten
# frames a second plays at ten as a time-lapse (its speed in the log), and then without sound.
set -euo pipefail
OUT="$1"; FPS=10; EVERY=5; FROM=0; SOUND=1
[ -d "$OUT/film" ] || { echo "no $OUT/film (the plan needs \"film\")"; exit 1; }
if [[ "${2:-}" == *.json ]]; then
  read -r FROM FPS SOUND SPEED < <(python3 -c '
import json, sys
f = json.load(open(sys.argv[1]))["film"]
s = f["every"]
print(f.get("from", 0), f"{max(1 / s, 10):g}", int(s <= 0.1 + 1e-9), f"{s * 10:g}")' "$2")
  [ "$SOUND" = 1 ] || echo "film: a time-lapse at ${SPEED}x, without its sound"
else
  FPS="${2:-10}"; EVERY="${3:-5}"
fi
AUDIO=()
[ -f "$OUT/audio.wav" ] && [ "$SOUND" = 1 ] && AUDIO=(-ss "$FROM" -i "$OUT/audio.wav" -c:a aac -shortest)
ffmpeg -loglevel error -y -framerate "$FPS" -i "$OUT/film/%04d.png" "${AUDIO[@]}" \
  -vf "scale='min(1080,iw)':-2" -c:v libx264 -pix_fmt yuv420p "$OUT/film.mp4"
ffmpeg -loglevel error -y -i "$OUT/film/%04d.png" -vf "select='not(mod(n\,$EVERY))',scale=360:-2,tile=6x3" -frames:v 1 "$OUT/sheet.png"
ls -l "$OUT/film.mp4" "$OUT/sheet.png"
