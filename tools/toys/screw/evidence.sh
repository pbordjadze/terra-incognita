#!/usr/bin/env bash
# A Screw critic's evidence from a round's played plans (painting-company row 250): the start plan's film
# as strips of consecutive frames at each moment (a chop, the hold, the rewind, the brake), its sound as
# listen.py's picture and numbers, the info demo's sheet, the rules cards, and the deck's tests (the sound
# measured sample by sample).
#
#     bash tools/toys/screw/evidence.sh <toys/build/screw/out> <evidence-dir>
# (run the plans first, from toys/: cargo run --release -p screw --bin screw-check -- play screw/plans/<p>.json build/screw/out/<p>)
set -euo pipefail
OUT="$1"; EV="$2"; HERE="$(cd "$(dirname "$0")" && pwd)"; ROOT="$HERE/../../.."
mkdir -p "$EV"
python3 "$ROOT/tools/kit/shell_text.py" "$OUT" > /dev/null
S="$OUT/start"
strip() {  # name, first frame, frames, step
  local files=() i
  for ((i = 0; i < $3; i++)); do files+=(-i "$(printf '%s/film/%04d.png' "$S" $(( $2 + i * $4 )))"); done
  ffmpeg -loglevel error -y "${files[@]}" -filter_complex "$(for ((i = 0; i < $3; i++)); do printf '[%d]scale=300:-2[s%d];' $i $i; done)$(for ((i = 0; i < $3; i++)); do printf '[s%d]' $i; done)hstack=inputs=$3" "$EV/$1.png"
}
[ -d "$S/film" ] || { echo "no $S/film: play start.json first"; exit 1; }
# (frames at 30 a second: t * 30)
strip 1-first-tap 40 6 3      # 1.33..1.93 s: the tap that starts it
strip 2-push-down 150 6 10    # 5.0..6.7 s: the finger pushing the record down
strip 3-chop 268 6 2          # 8.93..9.27 s: a chop's kick
strip 4-hold 360 6 9          # 12.0..13.5 s: the chop held
strip 5-rewind 494 6 4        # 16.47..17.13 s: the flick back
strip 6-brake 628 6 6         # 20.93..22.1 s: the label's brake
for n in first playing screwed held rewind stopped; do cp "$S/$n.shell.png" "$EV/shot-$n.png" 2>/dev/null || cp "$S/$n.png" "$EV/shot-$n.png"; done
[ -f "$S/film.mp4" ] && cp "$S/film.mp4" "$EV/start.mp4"
python3 "$HERE/listen.py" "$ROOT/toys/screw/plans/start.json" "$S" > "$EV/listen.txt"
cp "$S/listen.png" "$EV/listen.png"
[ -f "$OUT/info/sheet.png" ] && cp "$OUT/info/sheet.png" "$EV/info-sheet.png"
for n in empty/blank:first-launch empty/first-tap:first-tap rules/rules:rules-rules rules/origins:rules-origins; do
  f="$OUT/${n%%:*}.shell.png"; [ -f "$f" ] || f="$OUT/${n%%:*}.png"; cp "$f" "$EV/${n##*:}.png"
done
(cd "$ROOT/toys" && cargo test --release -p screw -- --nocapture --test-threads 1 2>&1 | grep -oE "measured:.*|test result: .*|^test .*(ok|FAILED)$") > "$EV/tests.txt" || true
cp "$ROOT/toys/screw/plans/start.json" "$EV/start-plan.json"
ls "$EV"
