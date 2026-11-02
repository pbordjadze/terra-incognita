#!/usr/bin/env bash
# Re-encodes an Adrift export roughly as TikTok does on upload (R22): H.264 at about 3.5 Mbit/s,
# 1080x1920, its frame rate kept (60 at most), then pulls stills at the given seconds from both
# the master and the re-encode, side by side, so the look is judged after the platform's
# compression, not on the master. Nothing is uploaded anywhere.
#
#     bash tools/adrift/tiktok.sh <master.mp4> <out-dir> [seconds...]     (default: 2 10 20)
#
# Writes <out-dir>/tiktok.mp4 and <out-dir>/at-<s>s.png (master left, re-encode right), and
# prints both files' sizes and bitrates.
set -euo pipefail
in="${1:?usage: tiktok.sh <master.mp4> <out-dir> [seconds...]}"
out="${2:?usage: tiktok.sh <master.mp4> <out-dir> [seconds...]}"
shift 2
secs=("$@")
[ ${#secs[@]} -gt 0 ] || secs=(2 10 20)
mkdir -p "$out"
ffmpeg -loglevel error -y -i "$in" -vf "scale=1080:1920:flags=lanczos,fps='min(60,source_fps)'" \
  -c:v libx264 -preset medium -profile:v high -b:v 3500k -maxrate 4500k -bufsize 7000k \
  -pix_fmt yuv420p -an "$out/tiktok.mp4"
for f in "$in" "$out/tiktok.mp4"; do
  ffprobe -v error -show_entries format=size,bit_rate -show_entries stream=codec_name,r_frame_rate \
    -of compact=p=0:nk=1 "$f" | tr '\n' ' '
  echo " $f"
done
for s in "${secs[@]}"; do
  ffmpeg -loglevel error -y -ss "$s" -i "$in" -frames:v 1 "$out/m.png"
  ffmpeg -loglevel error -y -ss "$s" -i "$out/tiktok.mp4" -frames:v 1 "$out/t.png"
  ffmpeg -loglevel error -y -i "$out/m.png" -i "$out/t.png" -filter_complex hstack "$out/at-${s}s.png"
  rm -f "$out/m.png" "$out/t.png"
  echo "$out/at-${s}s.png"
done
