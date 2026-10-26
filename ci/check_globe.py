#!/usr/bin/env python3
"""Checks the globe's bundled data (Globe/, built by tools/globe/build.py) before a Mac builds it.

    python3 ci/check_globe.py [Globe]

globe.json has every field the app's decoder requires (App/TerraIncognita/Globe/GlobeData.swift: a
missing one leaves the globe blank), the three maps are the sizes and kinds the shader and the tap
lookup read (geo and terrain RGBA, ids grey, 4096 x 2048), and so are the Persian style's ornaments
(RGBA: the band 1152 x 256, the braid 1024 x 256, the finial 256 x 256, the rosette 96 x 96, the
caption's headpiece 2048 x 256), no country index in globe.json exceeds what a grey byte holds, the fonts are there
with their licences, and the type (type.json) keeps the design rules: bundled fonts, no name under
the smallest size, every ink at least the set contrast against its halo or ground. Reads PNG
headers itself (no image library on the runner).
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


# the maps, and the Persian style's ornaments (tools/globe/ornament.py): size, colour type (6: RGBA,
# 0: grey), all 8-bit, as the app's uploads read them
KINDS = {'geo.png': (4096, 2048, 6), 'terrain.png': (4096, 2048, 6), 'ids.png': (4096, 2048, 0),
         'band.png': (1152, 256, 6), 'fleuron.png': (256, 256, 6), 'rosette.png': (96, 96, 6),
         'headpiece.png': (2048, 256, 6), 'braid.png': (1024, 256, 6)}
for name, (w, h0, kind) in KINDS.items():
    p = os.path.join(root, name)
    h = png(p) if os.path.isfile(p) else None
    if h is None:
        err(f'{name}: missing or not a PNG')
    elif h != (w, h0, 8, kind):
        err(f'{name}: {h[0]}x{h[1]}, {h[2]}-bit, colour type {h[3]}; expected {w}x{h0}, 8-bit, colour type {kind}')


def no_nan(name):
    raise ValueError(f'{name} (Swift\'s JSONDecoder refuses it)')


try:
    data = json.load(open(os.path.join(root, 'globe.json')), parse_constant=no_nan)
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

# the type (type.json, CLAUDE.md › Design rules): every face's font bundled, every name at least the
# smallest size, every ink against what is round it (a name's halo, a caption's ground, the
# cartouche's gold) at least the contrast it sets
def luminance(hexs):
    c = [int(hexs.lstrip('#')[i:i + 2], 16) / 255 for i in (0, 2, 4)]
    c = [x / 12.92 if x <= 0.03928 else ((x + 0.055) / 1.055) ** 2.4 for x in c]
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]


def contrast(a, b):
    la, lb = luminance(a), luminance(b)
    return (max(la, lb) + 0.05) / (min(la, lb) + 0.05)


def font_file(name):
    return any(os.path.isfile(os.path.join(root, 'fonts', name + ext)) for ext in ('.otf', '.ttf'))


try:
    t = json.load(open(os.path.join(root, 'type.json')), parse_constant=no_nan)
except (OSError, ValueError) as e:
    err(f'type.json: {e}')
    t = {}
for key, kind in (('reading', dict), ('readerMax', NUM), ('minSize', NUM), ('contrast', NUM), ('largeText', NUM),
                  ('largeTextBold', NUM), ('largeContrast', NUM), ('halo', NUM), ('gap', NUM), ('touch', NUM),
                  ('countryTiers', list), ('styles', dict)):
    if not isinstance(t.get(key), kind) or isinstance(t.get(key), bool):
        err(f'type.json: {key} missing or not {kind}')
r = t.get('reading') or {}
if not (isinstance(r.get('shortSide'), list) and len(r['shortSide']) == 2 and all(isinstance(v, NUM) for v in r['shortSide'])
        and r['shortSide'][1] > r['shortSide'][0] and isinstance(r.get('factor'), list) and len(r['factor']) == 2
        and all(isinstance(v, NUM) for v in r['factor'])):
    err('type.json: reading needs shortSide [small, large] and factor [at small, at large], numbers')
if not (isinstance(t.get('countryTiers'), list) and len(t['countryTiers']) == 2 and all(isinstance(v, NUM) for v in t['countryTiers'])):
    err('type.json: countryTiers needs two label ranks')
least, enough = t.get('minSize', 10), t.get('contrast', 4.5)


def needed(size, font):
    """The contrast a line needs: less for large text as WCAG has it (18 pt, or 14 pt bold; iOS points
    taken as its points)."""
    bold = 'Bold' in font or 'Semibold' in font
    large = size >= (t.get('largeTextBold', 1e9) if bold else t.get('largeText', 1e9))
    return t.get('largeContrast', enough) if large else enough


def typed(where, obj, fields):
    """Every field the app's decoder requires (GlobeType, CaptionStyle), of its type."""
    if not isinstance(obj, dict):
        err(f'{where}: not an object')
        return False
    ok = True
    for name, kind in fields.items():
        v = obj.get(name)
        if not isinstance(v, kind) or isinstance(v, bool) and kind is not bool:
            err(f'{where}: {name} missing or not {kind}')
            ok = False
    return ok


NAMES = ('ocean', 'sea', 'country1', 'country2', 'country3', 'capital', 'selected')
styles = t.get('styles') or {}
for sname in ('wikipedia', 'persian'):                       # (GlobeStyle's cases)
    if sname not in styles:
        err(f'type.json: no style {sname}')
for sname, st in styles.items():
    if not typed(f'type.json {sname}', st, {'names': dict, 'caption': dict, 'captionGround': str}):
        continue
    names = st['names']
    for n in NAMES:
        f = names.get(n)
        if not typed(f'type.json {sname} {n}', f, {'font': str, 'size': NUM, 'tracking': NUM, 'ink': str, 'halo': str}):
            continue
        if not font_file(f['font']):
            err(f'type.json {sname} {n}: font {f["font"]} not in fonts/')
        if f['size'] < least:
            err(f'type.json {sname} {n}: size {f["size"]} under {least}')
        try:
            c = contrast(f['ink'], f['halo'])
            if c < enough:
                err(f'type.json {sname} {n}: {f["ink"]} on {f["halo"]} is {c:.2f}:1, under {enough}:1')
        except (KeyError, ValueError, TypeError):
            err(f'type.json {sname} {n}: ink and halo must be #RRGGBB')
    cap, ground = st['caption'], st['captionGround']
    for line in ('eyebrow', 'title', 'text'):
        l = cap.get(line)
        if not typed(f'type.json {sname} caption {line}', l, {'font': str, 'size': NUM, 'color': str}):
            continue
        if not font_file(l['font']):
            err(f'type.json {sname}: caption {line} font {l["font"]} not in fonts/')
        try:
            need, c = needed(l['size'], l['font']), contrast(l['color'], ground)
            if c < need:
                err(f'type.json {sname}: caption {line} {l["color"]} on {ground} is {c:.2f}:1, under {need}:1')
        except (KeyError, ValueError, TypeError):
            err(f'type.json {sname}: caption {line} color and captionGround must be #RRGGBB')
    ca = st.get('cartouche')
    if ca is not None and typed(f'type.json {sname} cartouche', ca, {'font': str, 'size': NUM, 'ink': str, 'ground': str, 'box': list}):
        if not font_file(ca['font']) or len(ca['box']) != 4 or not all(isinstance(v, NUM) for v in ca['box']):
            err(f'type.json {sname}: cartouche needs a bundled font and a box [x, y, width, height]')
        elif contrast(ca['ink'], ca['ground']) < needed(ca['size'] * 0.55, ca['font']) or ca['size'] * 0.55 < least:
            err(f'type.json {sname}: the cartouche title is too faint or, shrunk to 0.55, too small')

size = sum(os.path.getsize(os.path.join(d, f)) for d, _, fs in os.walk(root) for f in fs)
print(f'Globe: {len(data.get("countries") or [])} countries, {len(data.get("seas") or [])} seas, '
      f'{len(data.get("capitals") or [])} capitals, {size / 1e6:.1f} MB')
sys.exit(1 if errors else 0)
