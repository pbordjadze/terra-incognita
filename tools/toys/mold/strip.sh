#!/bin/bash
# Plays one of Mold's plans and lays the named shots side by side (their globe's band), half size, so a
# round's tuning can be seen in one picture:
#
#     bash tools/toys/mold/strip.sh <out-name> <plan-name> <shot,shot,...>   # toys/build/mold/<out>/strip.png
#
# e.g. bash tools/toys/mold/strip.sh s1 start first,network,found
set -e
cd "$(dirname "$0")/../../../toys"
cargo run -q --release -p mold --bin mold-check -- play "mold/plans/$2.json" "build/mold/$1" | grep -E "error|panic" -A5 || true
python3 - "build/mold/$1" "$3" <<'PY'
import sys
from PIL import Image
o, names = sys.argv[1], sys.argv[2].split(',')
ims = [Image.open(f'{o}/{n}.png') for n in names]
h = ims[0].height
ims = [i.crop((0, int(h * 0.22), i.width, int(h * 0.78))) for i in ims]
s = Image.new('RGB', (sum(i.width for i in ims), ims[0].height))
x = 0
for i in ims:
    s.paste(i, (x, 0)); x += i.width
s.resize((s.width // 2, s.height // 2)).save(f'{o}/strip.png')
print(f'{o}/strip.png')
PY
