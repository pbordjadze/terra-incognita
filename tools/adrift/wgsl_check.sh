#!/usr/bin/env bash
# Validates the look's WGSL with naga: the ground and the sea behind the View struct (gpu/view.rs) and
# common.wgsl, as look::wgsl prepends them; the marks behind those and marks.wgsl (look::marks_wgsl).
#     bash tools/adrift/wgsl_check.sh            (cargo install naga-cli first)
set -euo pipefail
cd "$(dirname "$0")/../.."
L=adrift/crates/adrift-core/src/look
tmp=$(mktemp -d)
sed -n '/^pub const VIEW_WGSL/,/^"#;/p' adrift/crates/adrift-core/src/gpu/view.rs | sed '1d;$d' > "$tmp/view.wgsl"
status=0
for f in ground sea; do
  cat "$tmp/view.wgsl" "$L/common.wgsl" "$L/$f.wgsl" > "$tmp/$f.wgsl"
  if naga "$tmp/$f.wgsl" > "$tmp/out" 2>&1; then echo "ok   $f.wgsl"; else echo "FAIL $f.wgsl"; cat "$tmp/out"; status=1; fi
done
for f in floats wake maybes silver words; do
  cat "$tmp/view.wgsl" "$L/common.wgsl" "$L/marks.wgsl" "$L/$f.wgsl" > "$tmp/$f.wgsl"
  if naga "$tmp/$f.wgsl" > "$tmp/out" 2>&1; then echo "ok   $f.wgsl"; else echo "FAIL $f.wgsl"; cat "$tmp/out"; status=1; fi
done
rm -rf "$tmp"
exit $status
