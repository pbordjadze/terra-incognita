#!/usr/bin/env bash
# A toy's launch check: tools/kit/launch_check.py (what it checks and why) with the toy's check binary,
# manifest and budget (toys/<toy>/toy.json's memory_mb).
#
#     bash tools/toys/launch_check.sh <toy> [--ipa <toy.ipa>]     # needs naga-cli 26 (cargo install naga-cli --version 26.0.0)
set -euo pipefail
TOY="$1"; shift
ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
if [ "${1:-}" = "--ipa" ]; then
  IPA="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
  exec python3 "$ROOT/tools/kit/launch_check.py" --manifest "$ROOT/toys/$TOY/manifest.json" --ipa "$IPA"
fi
python3 "$ROOT/tools/toys/assets.py" "$TOY" --link > /dev/null
(cd "$ROOT/toys" && cargo build -q --release -p "$TOY" --bin "$TOY-check")
exec python3 "$ROOT/tools/kit/launch_check.py" \
  --check "'$ROOT/toys/target/release/$TOY-check' play {plan} {out} --size {size} --ppp {ppp}" \
  --manifest "$ROOT/toys/$TOY/manifest.json" --app-json "$ROOT/toys/$TOY/toy.json"
