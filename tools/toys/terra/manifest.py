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
HELD = ['whales', 'murmuration', 'mold', 'fireflies', 'bell', 'swell', 'meander', 'frost', 'throats', 'catatumbo', 'bubblers', 'barchans']
NOTE = ("Terra's own data (its rivers) and every file the toys Terra holds load (their manifests merged: no name differs), by the names their code "
        "uses (tools/toys/assets.py assembles them; the IPA bundles them as data/). Written by "
        "tools/toys/terra/manifest.py; never hand-edit.")

# Terra's own data, beside the toys': the world's rivers (tools/toys/terra/rivers.py) and the ground round
# the scenes it holds (tools/toys/terra/region.py); row 258
OWN = {'rivers.bin': 'toys/terra/data/rivers.bin'}
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
