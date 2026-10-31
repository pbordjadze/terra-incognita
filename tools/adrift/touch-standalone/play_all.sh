#!/usr/bin/env bash
# Plays every exported touch plan on the real build (adrift-check, software Vulkan) and checks each
# play.log against its expectations. Minutes, not seconds: the frames are drawn on the CPU.
#
#     bash tools/adrift/touch-standalone/play_all.sh [out-dir]      # default adrift/build/out/motion
set -euo pipefail
cd "$(dirname "$0")/../../.."
OUT=${1:-adrift/build/out/motion}
PLANS=adrift/crates/adrift-core/src/touch/plans/check
(cd adrift && cargo build -q -p adrift-check)
status=0
for plan in "$PLANS"/*.json; do
  name=$(basename "$plan" .json)
  needs=$(python3 -c "import json,sys; print(' '.join(json.load(open(sys.argv[1])).get('needs', [])))" "$plan")
  if [ -n "$needs" ]; then echo "skip $name.json: adrift-check has no $needs step yet"; continue; fi
  (cd adrift && ./target/debug/adrift-check play "../$plan" "../$OUT/$name" > /dev/null 2>&1) || { echo "FAIL $name: adrift-check"; status=1; continue; }
  python3 tools/adrift/touch-standalone/check_play.py "$plan" "$OUT/$name" || status=1
done
exit $status
