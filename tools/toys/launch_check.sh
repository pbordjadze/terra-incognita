#!/usr/bin/env bash
# What a toy needs to open on a phone that the Linux renders don't prove, checked on Linux:
#   1. every shader the toy builds at launch (dumped by toy-kit's Gpu::shader with TOY_DUMP_SHADERS while
#      a 0.2 s plan plays) translated to Metal Shading Language by naga at MSL 2.4, as wgpu does on iOS
#      (Linux runs them through SPIR-V, a different translator);
#   2. with an IPA (--ipa <file>), every file of the toy's manifest bundled under data/ by its name.
# It cannot run the Metal compiler or the iPhone's GPU; the device's own log (toy-log.txt, in the Files
# app under the toy's name) is the last word.
#
#     bash tools/toys/launch_check.sh <toy> [--ipa <toy.ipa>]     # needs naga-cli 26 (cargo install naga-cli --version 26.0.0)
set -euo pipefail
TOY="$1"; shift
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
if [ "${1:-}" = "--ipa" ]; then
  (cd "$WORK" && unzip -q "$(cd "$(dirname "$2")" && pwd)/$(basename "$2")")
  python3 - "$ROOT/toys/$TOY/manifest.json" "$WORK"/Payload/*.app/data <<'PY'
import json, os, sys
names = json.load(open(sys.argv[1]))['files']
have = set(os.listdir(sys.argv[2]))
missing = [n for n in names if n not in have]
print(f"bundle: {len(names) - len(missing)} of {len(names)} manifest files in the IPA")
sys.exit(f"missing from the IPA: {missing}" if missing else 0)
PY
  exit 0
fi
python3 "$ROOT/tools/toys/assets.py" "$TOY" --link > /dev/null
printf '{"seconds":0.2,"fps":10,"shots":[0.1]}' > "$WORK/plan.json"
cd "$ROOT/toys"
TOY_DUMP_SHADERS="$WORK/shaders" cargo run -q --release -p "$TOY" --bin "$TOY-check" -- play "$WORK/plan.json" "$WORK/out" --size 390x844 --ppp 1 > /dev/null
fail=0
for f in "$WORK"/shaders/*.wgsl; do
  if out=$(naga --metal-version 2.4 "$f" "${f%.wgsl}.metal" 2>&1); then echo "msl ok: $(basename "$f")"; else echo "MSL FAILS: $(basename "$f"): $out"; fail=1; fi
done
exit $fail
