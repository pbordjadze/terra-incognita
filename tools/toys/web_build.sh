#!/usr/bin/env bash
# Builds Terra for the web: the iPhone's library as WebAssembly (toys/terra-web), its bindings, the page
# (toys/terra-web/index.html) and Terra's data (toys/terra/manifest.json) into one directory any static
# server can serve as is (the owner's site serves it from an unlisted path).
#
#     bash tools/toys/web_build.sh [out-dir] [--shot]
#     python3 -m http.server -d toys/build/web 8000    # then open http://localhost:8000 (WebGPU)
#
# --shot also opens it in headless Chromium (tools/kit/webgpu_shot.cjs) and writes <out-dir>/../web-shot.png,
# failing if the page draws nothing.
#
# Needs the wasm32-unknown-unknown target (bash ci/rust.sh wasm32-unknown-unknown) and wasm-bindgen-cli
# 0.2.130 (cargo install wasm-bindgen-cli --version 0.2.130 --locked; the crate pins the same).
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SHOT=0; ARGS=()
for a in "$@"; do if [ "$a" = --shot ]; then SHOT=1; else ARGS+=("$a"); fi; done
OUT="$(python3 -c "import os,sys; print(os.path.abspath(sys.argv[1]))" "${ARGS[0]:-$ROOT/toys/build/web}")"
cd "$ROOT/toys"
cargo build --release --target wasm32-unknown-unknown -p terra-web
rm -rf "$OUT"
mkdir -p "$OUT"
wasm-bindgen --target web --no-typescript --out-dir "$OUT" target/wasm32-unknown-unknown/release/terra_web.wasm
if command -v wasm-opt > /dev/null 2>&1; then wasm-opt -O3 -o "$OUT/terra_web_bg.wasm" "$OUT/terra_web_bg.wasm"; fi
cp terra-web/index.html "$OUT/index.html"
cp terra/icon.png "$OUT/icon.png"
python3 "$ROOT/tools/toys/assets.py" terra "$OUT/data" > /dev/null
# the page's list of the data and each file's size (its loading bar)
python3 - "$OUT/data" << 'PY'
import json, sys
from pathlib import Path
d = Path(sys.argv[1])
index = {p.name: p.stat().st_size for p in sorted(d.iterdir()) if p.name != 'index.json'}
(d / 'index.json').write_text(json.dumps(index, indent=1) + '\n')
PY
echo "web build in $OUT: $(du -sh "$OUT" | cut -f1), wasm $(wc -c < "$OUT/terra_web_bg.wasm" | tr -d ' ') bytes"
if [ "$SHOT" = 1 ]; then
  node "$ROOT/tools/kit/webgpu_shot.cjs" "$OUT" "$(dirname "$OUT")/web-shot.png" --size 390x844 --dpr 1 --wait 60
fi
