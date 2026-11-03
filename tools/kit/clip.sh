#!/usr/bin/env bash
# Encodes a clip from rendered frames and their sound, ready to post (TikTok, Reels, Shorts): H.264
# High profile, yuv420p, the frames' own size (1080x1920 from `check clip`), AAC 48 kHz stereo,
# +faststart (the index first, so it plays while it loads). The video is encoded at a high quality
# (CRF 18): the platform re-encodes on upload (tools/adrift/tiktok.sh shows what that does), so the
# master should be clean. `<toy>-check clip <plan> <out.mp4>` (toy-kit check.rs) renders the frames
# and calls this; it can also be run by hand:
#
#     bash tools/kit/clip.sh <frames-dir> <out.mp4> [fps=30]
#
# <frames-dir> holds 00000.png, 00001.png, … and audio.wav (silent if absent). Prints the clip's
# streams and size.
set -euo pipefail
dir="${1:?usage: clip.sh <frames-dir> <out.mp4> [fps=30]}"
out="${2:?usage: clip.sh <frames-dir> <out.mp4> [fps=30]}"
fps="${3:-30}"
[ -f "$dir/00000.png" ] || { echo "clip.sh: no $dir/00000.png"; exit 1; }
if [ -f "$dir/audio.wav" ]; then
  audio=(-i "$dir/audio.wav")
else
  audio=(-f lavfi -i "anullsrc=channel_layout=stereo:sample_rate=48000")
fi
mkdir -p "$(dirname "$out")"
ffmpeg -loglevel error -y -framerate "$fps" -i "$dir/%05d.png" "${audio[@]}" \
  -map 0:v:0 -map 1:a:0 \
  -c:v libx264 -preset slow -profile:v high -level 4.2 -crf 18 -pix_fmt yuv420p \
  -vf "scale=trunc(iw/2)*2:trunc(ih/2)*2,format=yuv420p" -r "$fps" -g "$((fps * 2))" \
  -c:a aac -b:a 192k -ar 48000 -ac 2 -shortest \
  -movflags +faststart "$out"
ffprobe -v error -show_entries stream=codec_name,profile,width,height,pix_fmt,r_frame_rate,sample_rate,channels \
  -show_entries format=duration,size -of compact=p=0 "$out"
