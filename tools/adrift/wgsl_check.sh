#!/usr/bin/env bash
# Validates the look's marks' WGSL with naga, each behind the View struct (gpu/view.rs), common.wgsl
# and marks.wgsl, as look::marks_wgsl prepends them.
#     bash tools/adrift/wgsl_check.sh            (cargo install naga-cli first)
set -euo pipefail
cd "$(dirname "$0")/../.."
L=adrift/crates/adrift-core/src/look
tmp=$(mktemp -d)
sed -n '/^pub const VIEW_WGSL/,/^"#;/p' adrift/crates/adrift-core/src/gpu/view.rs | sed '1d;$d' > "$tmp/view.wgsl"
status=0
for f in floats wake maybes silver words; do
  cat "$tmp/view.wgsl" "$L/common.wgsl" "$L/marks.wgsl" "$L/$f.wgsl" > "$tmp/$f.wgsl"
  if naga "$tmp/$f.wgsl" > "$tmp/out" 2>&1; then echo "ok   $f.wgsl"; else echo "FAIL $f.wgsl"; cat "$tmp/out"; status=1; fi
done
rm -rf "$tmp"
exit $status
