#!/usr/bin/env bash
# Validates the look's WGSL with naga, each pass's shader behind view.wgsl as the Rust prepends it.
#     bash tools/adrift/wgsl_check.sh            (cargo install naga-cli first)
set -euo pipefail
cd "$(dirname "$0")/../.."
L=adrift/crates/adrift-core/src/look
tmp=$(mktemp -d)
status=0
for f in "$L"/*.wgsl; do
  [ "$(basename "$f")" = view.wgsl ] && continue
  cat "$L/view.wgsl" "$f" > "$tmp/$(basename "$f")"
  if naga "$tmp/$(basename "$f")" > "$tmp/out" 2>&1; then echo "ok   $(basename "$f")"; else echo "FAIL $(basename "$f")"; cat "$tmp/out"; status=1; fi
done
rm -rf "$tmp"
exit $status
