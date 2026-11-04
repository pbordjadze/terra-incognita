#!/usr/bin/env python3
"""Terra's manifest is every file the toys it holds load (toys/terra/manifest.json): their manifests
merged. This writes it from them, or with --check fails when it is out of step (a toy that gained a file,
or two toys naming one file differently).

    python3 tools/toys/terra/manifest.py [--check]
"""
import json, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HELD = ['whales', 'murmuration', 'mold', 'fireflies', 'bell', 'swell', 'battle', 'jam']
NOTE = ("Every file the toys Terra holds load (their manifests merged: no name differs), by the names their code "
        "uses (tools/toys/assets.py assembles them; the IPA bundles them as data/). Written by "
        "tools/toys/terra/manifest.py; never hand-edit.")

files = {}
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
