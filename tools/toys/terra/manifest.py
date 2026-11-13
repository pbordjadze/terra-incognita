#!/usr/bin/env python3
"""Terra's manifest is every file the toys it holds load (toys/terra/manifest.json): their manifests
merged. This writes it from them, or with --check fails when it is out of step (a toy that gained a file,
or two toys naming one file differently).

    python3 tools/toys/terra/manifest.py [--check]
"""
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HELD = ['murmuration', 'elkmont', 'nazare', 'meander', 'frost', 'throats', 'bats', 'huddle', 'wave', 'catatumbo', 'bubblers', 'barchans', 'pando']
NOTE = ("Terra's own data (the globe's, and the ground round its scenes) and every file the toys Terra holds load (their manifests merged: no name differs), by the names their code "
        "uses (tools/toys/assets.py assembles them; the IPA bundles them as data/). Written by "
        "tools/toys/terra/manifest.py; never hand-edit.")

# Terra's own data, beside the toys': the kit's globe (Globe/: the coast, terrain, the months' ground and ice,
# the sea floor, the night), and the ground round the scenes it holds (tools/toys/terra/region.py); row 258.
# (The world's rivers, toys/terra/data/rivers.bin from tools/toys/terra/rivers.py, are off the globe for now.)
OWN = {'geo.png': 'Globe/geo.png', 'terrain.png': 'Globe/terrain.png',
       'floor.png': 'Globe/ground/floor.png', 'night.jpg': 'Globe/ground/night.jpg'}
OWN.update({f'land-{m:02}.jpg': f'Globe/ground/{m:02}.jpg' for m in range(1, 13)})
OWN.update({f'ice-{m:02}.png': f'Globe/ground/ice-{m:02}.png' for m in range(1, 13)})
OWN.update({f'{n}.jpg': f'toys/terra/data/{n}.jpg' for n in ['beni', 'hamwall', 'hamwall-near']})
files = dict(OWN)
for toy in HELD:
    for name, src in json.loads((ROOT / 'toys' / toy / 'manifest.json').read_text())['files'].items():
        if files.get(name, src) != src:
            sys.exit(f'{toy}: {name} is {src}, another toy has it as {files[name]}')
        files[name] = src
text = json.dumps({'_': NOTE, 'files': dict(sorted(files.items()))}, indent=2) + '\n'
out = ROOT / 'toys' / 'terra' / 'manifest.json'
if '--check' in sys.argv:
    if out.read_text() != text:
        sys.exit('toys/terra/manifest.json is out of step with the toys: run python3 tools/toys/terra/manifest.py')
    print(f'terra manifest: {len(files)} files, in step')
else:
    out.write_text(text)
    print(f'terra manifest: {len(files)} files written')
