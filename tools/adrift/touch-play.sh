#!/usr/bin/env bash
# Plays every touch plan (adrift/crates/adrift-core/src/touch/plans/*.json) on the real globe with
# adrift-check and reports which passed their expectations. Minutes on Linux's software Vulkan.
#
#     bash tools/adrift/touch-play.sh [out-dir]      # default adrift/build/out/motion
set -euo pipefail
cd "$(dirname "$0")/../.."
OUT=${1:-adrift/build/out/motion}
mkdir -p "$OUT"
python3 tools/adrift/assets.py adrift/build/assets --link > /dev/null
(cd adrift && cargo build -q -p adrift-check)
status=0
for plan in adrift/crates/adrift-core/src/touch/plans/*.json; do
  name=$(basename "$plan" .json)
  if (cd adrift && ./target/debug/adrift-check play "../$plan" "../$OUT/$name" > "../$OUT/$name.log" 2>&1); then
    echo "ok   $name"
  else
    echo "FAIL $name"; grep -h "EXPECT FAILED\|rror" "$OUT/$name.log" | head -5 | sed 's/^/     /'; status=1
  fi
done
exit $status
