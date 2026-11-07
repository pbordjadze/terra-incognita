#!/usr/bin/env bash
# A Frost critic's evidence from the played `listen` plan (painting-company row 254, after Screw's,
# tools/toys/screw/evidence.sh): the film with its sound, strips of consecutive frames at each touch,
# the sound as listen.py's spectrogram and its numbers (each touch's first sound, clicks, loudness,
# the notes), the stills, and the synth's own tests (pitch by height, frost damping the ring, levels).
#
#     cd toys && cargo run --release -p frost --bin frost-check -- play frost/listen.json build/frost/out/listen --no-tap-check
#     bash tools/toys/frost/evidence.sh toys/build/frost/out/listen <evidence-dir>
set -euo pipefail
S="$1"; EV="$2"; HERE="$(cd "$(dirname "$0")" && pwd)"; ROOT="$HERE/../../.."
mkdir -p "$EV"
[ -d "$S/film" ] || { echo "no $S/film: play listen.json first"; exit 1; }
bash "$ROOT/tools/toys/film.sh" "$S" 10 10 > /dev/null
cp "$S/film.mp4" "$EV/listen.mp4"; cp "$S/sheet.png" "$EV/sheet.png"
strip() {  # name, first frame, frames, step (frames at 10 a second)
  local files=() i
  for ((i = 0; i < $3; i++)); do files+=(-i "$(printf '%s/film/%04d.png' "$S" $(( $2 + i * $4 )))"); done
  ffmpeg -loglevel error -y "${files[@]}" -filter_complex "$(for ((i = 0; i < $3; i++)); do printf '[%d]scale=240:-2[s%d];' $i $i; done)$(for ((i = 0; i < $3; i++)); do printf '[s%d]' $i; done)hstack=inputs=$3" "$EV/$1.png"
}
strip 1-opening 0 6 24          # 0..12 s: the frost creeping in by itself
strip 2-tap-on-frost 129 6 3    # 12.9..14.4 s
strip 3-wipe 149 6 5            # 14.9..17.4 s: the winding wipe and its regrowth
strip 4-tap-bare 184 6 3        # 18.4..19.9 s
strip 5-drag-up 199 6 5         # 19.9..22.4 s
strip 6-four-taps 229 6 4       # 22.9..24.9 s
strip 7-rest 254 6 6            # 25.4..28.4 s
strip 8-shake 309 6 6           # 30.9..33.9 s
for f in "$S"/[0-9]*.png; do cp "$f" "$EV/shot-$(basename "$f")"; done
python3 "$ROOT/tools/toys/screw/listen.py" "$ROOT/toys/frost/listen.json" "$S" --measure > "$EV/listen.txt"
cp "$S/listen.png" "$EV/listen.png"
(cd "$ROOT/toys" && cargo test --release -p frost --lib -- --nocapture --test-threads 1 2>&1 | grep -oE "measured:.*|test result: .*|^test .*(ok|FAILED)$") > "$EV/tests.txt" || true
cp "$ROOT/toys/frost/listen.json" "$EV/listen-plan.json"
ls "$EV"
