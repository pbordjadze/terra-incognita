#!/usr/bin/env bash
# Renders every R6 study scene here to PNGs (Visual's check, R6). Run from anywhere after
# tools/adrift/linux-setup.sh and assets.py --link:
#   bash adrift/crates/adrift-core/src/look/scenes/render.sh <out-dir> [scene-name ...]
# Each scene renders on the phone (1179x2556, 3 ppp); look at the PNGs.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="$(mkdir -p "$1" && cd "$1" && pwd)"; shift
cd "$here/../../../../.."   # adrift/
names=("$@")
if [ ${#names[@]} -eq 0 ]; then for f in "$here"/*.json; do names+=("$(basename "$f" .json)"); done; fi
for n in "${names[@]}"; do
  cargo run -q --release -p adrift-check -- render "$here/$n.json" "$out/$n.png" --size 1179x2556 --ppp 3
done
