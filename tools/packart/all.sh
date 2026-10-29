#!/bin/bash
# Draws every pack's art again, in place, through the one art drawer (tools/packart/render.cjs).
#   tools/packart/all.sh && git status --short Prototypes
# Every drawing is deterministic, so a clean status afterwards says a change to the drawer, Brush or a
# sheet left every pack's art the same to the byte (how the drawers were merged, painting-company row 178).
# A new pack with a sheet adds its line here. About 45 seconds.
set -euo pipefail
cd "$(dirname "$0")/../.."
for p in whales:sea-of-whales monsoon:monsoon-sails beacons:fires-across-the-sea leadline:lead-line lowsea:low-sea vine:vine; do
  node tools/packart/render.cjs "tools/${p%%:*}/art.js" "Prototypes/${p##*:}"
done
node tools/packart/render.cjs tools/herds/draw.cjs Prototypes/herds
node tools/packart/render.cjs tools/water/boto.cjs Prototypes/gathering-of-water-alive
