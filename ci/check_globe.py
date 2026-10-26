#!/usr/bin/env python3
"""Checks the globe's bundled data (Globe/, built by tools/globe/build.py) before a Mac builds it.

    python3 ci/check_globe.py [Globe]

globe.json has every field the app's decoder requires (App/TerraIncognita/Globe/GlobeData.swift: a
missing one leaves the globe blank), the three maps are the sizes and kinds the shader and the tap
lookup read (geo and terrain RGBA, ids grey, 4096 x 2048), no country index in globe.json exceeds
what a grey byte holds, and the fonts are there with their licences. Reads PNG headers itself (no
image library on the runner).
"""
import json, os, struct, sys

root = sys.argv[1] if len(sys.argv) > 1 else 'Globe'
errors = []


def err(msg):
    errors.append(msg)
    print(f'error: {msg}')


def png(path):
    """(width, height, bit depth, colour type) from a PNG's header."""
    with open(path, 'rb') as f:
        head = f.read(26)
    if head[:8] != b'\x89PNG\r\n\x1a\n' or head[12:16] != b'IHDR':
        return None
    return struct.unpack('>IIBB', head[16:26])


KINDS = {'geo.png': 6, 'terrain.png': 6, 'ids.png': 0}   # 6: RGBA, 0: grey
for name, kind in KINDS.items():
    p = os.path.join(root, name)
    h = png(p) if os.path.isfile(p) else None
    if h is None:
        err(f'{name}: missing or not a PNG')
    elif h != (4096, 2048, 8, kind):
        err(f'{name}: {h[0]}x{h[1]}, {h[2]}-bit, colour type {h[3]}; expected 4096x2048, 8-bit, colour type {kind}')

try:
    data = json.load(open(os.path.join(root, 'globe.json')))
except (OSError, ValueError) as e:
    err(f'globe.json: {e}')
    data = {}


def need(where, obj, fields):
    if not isinstance(obj, dict):
        err(f'{where}: not an object')
        return
    for name, kind in fields.items():
        v = obj.get(name)
        if not isinstance(v, kind) or isinstance(v, bool) and kind is not bool:
            err(f'{where}: {name} missing or not {kind}')
    c = obj.get('c')
    if isinstance(c, list) and (len(c) != 2 or not all(isinstance(x, (int, float)) for x in c)):
        err(f'{where}: c is not [lon, lat]')


NUM = (int, float)
for key in ('countries', 'seas', 'capitals'):
    if not isinstance(data.get(key), list) or not data.get(key):
        err(f'globe.json: no {key}')
for i, c in enumerate(data.get('countries') or []):
    need(f'country {i}', c, {'n': str, 'c': list, 'r': NUM, 'min': NUM})
for i, s in enumerate(data.get('seas') or []):
    need(f'sea {i}', s, {'n': str, 'r': NUM, 'ocean': bool, 'c': list})
for i, c in enumerate(data.get('capitals') or []):
    need(f'capital {i}', c, {'n': str, 'c': list})
if len(data.get('countries') or []) > 255:
    err('more countries than a grey byte can index')

for f in ('LibertinusSerif-Regular.otf', 'LibertinusSerif-Italic.otf', 'LibertinusSerif-Semibold.otf',
          'UncialAntiqua-Regular.ttf', 'OFL-Libertinus.txt', 'OFL-UncialAntiqua.txt'):
    if not os.path.isfile(os.path.join(root, 'fonts', f)):
        err(f'fonts/{f} missing')

size = sum(os.path.getsize(os.path.join(d, f)) for d, _, fs in os.walk(root) for f in fs)
print(f'Globe: {len(data.get("countries") or [])} countries, {len(data.get("seas") or [])} seas, '
      f'{len(data.get("capitals") or [])} capitals, {size / 1e6:.1f} MB')
sys.exit(1 if errors else 0)
