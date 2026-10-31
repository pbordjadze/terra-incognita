#!/usr/bin/env bash
# Builds Adrift's web shell: the wasm (release), its bindings, the page and the data, into
# adrift/build/web/, a directory any static server can serve (and a claude.ai artifact can hold).
#
#     bash tools/adrift/web-build.sh [out-dir]
#     python3 -m http.server -d adrift/build/web 8000    # then open http://localhost:8000
#
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli 0.2.130 (cargo install wasm-bindgen-cli
# --version 0.2.130; the crate's wasm-bindgen is pinned to the same).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
OUT="${1:-$ROOT/adrift/build/web}"
cd "$ROOT/adrift"
cargo build --release --target wasm32-unknown-unknown -p adrift-web
mkdir -p "$OUT"
wasm-bindgen --target web --no-typescript --out-dir "$OUT" target/wasm32-unknown-unknown/release/adrift_web.wasm
if command -v wasm-opt > /dev/null 2>&1; then wasm-opt -O3 -o "$OUT/adrift_web_bg.wasm" "$OUT/adrift_web_bg.wasm"; fi
cp crates/adrift-web/index.html "$OUT/index.html"
python3 "$ROOT/tools/adrift/assets.py" "$OUT/assets" > /dev/null
du -sh "$OUT" | cut -f1 | xargs echo "web build in $OUT:"
ls -l "$OUT/adrift_web_bg.wasm" | awk '{print "wasm", $5, "bytes"}'
