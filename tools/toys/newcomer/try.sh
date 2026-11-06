#!/usr/bin/env bash
# A newcomer's hands on a toy, headless (painting-company row 246): play a plan of touches on the toy as
# its check binary does, draw the iOS shell's text on the shots, and list the pictures to look at. It is
# what a "fresh tester" subagent drives, a round at a time, without reading the toy's code
# (NEWCOMER.md beside this is the brief such a tester is given).
#
#     bash tools/toys/newcomer/try.sh <toy> <plan.json> <out-dir> [phone|ipad]
#
# phone: 393x852 points at 3 px a point (1179x2556); ipad: the 13-inch iPad, 1032x1376 at 2 (2064x2752).
# The plan's own "size"/"ppp" are overridden by the device. Needs the toy's check built
# (cd toys && cargo build --release -p <toy> --bin <toy>-check) and its data linked
# (python3 tools/toys/assets.py <toy> --link). Each run starts the app fresh from launch, so a session
# is a plan that grows: add the next touch and play it again.
set -euo pipefail
TOY="${1:?toy}"; PLAN="${2:?plan.json}"; OUT="${3:?out dir}"; DEV="${4:-phone}"
ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
case "$DEV" in
  phone) SIZE=1179x2556; PPP=3 ;;
  ipad) SIZE=2064x2752; PPP=2 ;;
  *) echo "device: phone or ipad"; exit 2 ;;
esac
mkdir -p "$OUT"
BIN="$ROOT/toys/target/release/$TOY-check"
[ -x "$BIN" ] || { echo "build it first: cd toys && cargo build --release -p $TOY --bin $TOY-check"; exit 1; }
( cd "$ROOT/toys" && "$BIN" play "$(realpath "$PLAN")" "$(realpath "$OUT")" --size "$SIZE" --ppp "$PPP" --no-tap-check ) 2>&1 | grep -v '^shot' | grep -v '^\s*$' || true
python3 "$ROOT/tools/kit/shell_text.py" "$OUT" > /dev/null 2>&1 || true
python3 "$ROOT/tools/toys/newcomer/touches.py" "$PLAN" "$OUT" "$PPP" || true
# the pictures, in time order: the shell's text drawn in where there was any
for f in $(ls "$OUT"/*.png 2> /dev/null | grep -v '\.shell\.png$' | grep -v '\.stretch\.png$'); do
  s="${f%.png}.shell.png"; [ -f "$s" ] && echo "$s" || echo "$f"
done
[ -d "$OUT/film" ] && echo "film: $(ls "$OUT/film" | wc -l) frames in $OUT/film (every frame a PNG)"
exit 0
