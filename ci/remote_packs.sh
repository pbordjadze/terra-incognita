#!/usr/bin/env bash
# Packs a remote copy of the delivery-check pack whose words differ from the bundled one, so a UI
# test can prove a downloaded revision is what arrives:
#   ci/remote_packs.sh <outdir>     writes <outdir>/catalog.json, packs/hello-pack/r1/ and open/
# Serve <outdir> (e.g. python3 -m http.server -d <outdir>) and point the app's -packs-url at its
# catalog.json. Plain bash and python3 only: it also runs on the macOS runners (no GNU sed -i).
set -euo pipefail
OUT="${1:?usage: ci/remote_packs.sh <outdir>}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# the folder keeps its name: a pack's id is its folder's name
cp -R "$ROOT/Prototypes/hello-pack" "$TMP/hello-pack"
python3 - "$TMP/hello-pack/main.js" <<'PY'
import sys
path = sys.argv[1]
text = open(path, encoding='utf-8').read()
if 'First words.' not in text:
    sys.exit('First words. is not in ' + path + ': the remote copy would not differ from the bundled one')
open(path, 'w', encoding='utf-8').write(text.replace('First words.', 'Second words.'))
PY

# a fresh catalog, so the remote copy is always revision 1
mkdir -p "$OUT"
rm -f "$OUT/catalog.json"
python3 "$ROOT/tools/playground/playground.py" pack --from "$TMP/hello-pack" --out "$OUT"
cat "$OUT/catalog.json"
