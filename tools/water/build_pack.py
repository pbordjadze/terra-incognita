#!/usr/bin/env python3
"""Prepare the runtime-3 pack Prototypes/gathering-of-water/ from the river study's own build.

Usage: python3 tools/water/build_pack.py [<folder>]

Reads what tools/water/build.py wrote into Water/ (or <folder>: network.js from RiverATLAS v1.0,
places.js from labels.json, manifest.json) and Globe/globe.json (which capitals the globe already
names), and writes:

- network.json: the same 18,159 stretches, their downstream links, Strahler orders, summed reach
  lengths and upstream areas, unchanged. Only the drawing is lightened: each line is simplified
  further (Douglas-Peucker, distances in degrees with longitude scaled by the cosine of latitude)
  and every kept vertex carries the coarsest of four tolerances at which it survives, so the
  script can draw any of them without simplifying again. Ends are never moved, so every stretch
  still meets the next exactly. The nearest-line grid (the stretches in each half-degree cell) is
  built here too: the script runs interpreted in the app, so it decodes and indexes nothing at
  start. The build's manifest (source checksums, settings) goes with it.
- places.json: the study's river anchors, snapped to the network exactly as Water/water.js does
  (nearest stretch of order 3 or more within 0.55 of its map units), and its cities, without
  the capitals the globe letters itself.
- doorway.png and glint.png: drawn here from code (the doorway from the network's own lines).

Never hand-edit those files; change this script and run it again. No network is used.
"""
import json
import math
import sys
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
WATER = ROOT / 'Water'
OUT = ROOT / 'Prototypes' / 'gathering-of-water'

# The tolerances (degrees) a vertex may survive at, coarsest first; the last is what is kept at all.
LEVELS = [0.1, 0.05, 0.025, 0.012]
CELL = 0.5                               # the nearest-line grid, in degrees
# Water/core.js's planar projection: the study's map units (x by cos 20°, y in degrees of latitude).
C = math.cos(math.radians(20))


def read_js(name):
    text = (WATER / name).read_text(encoding='utf-8')
    return json.loads(text[text.index('=') + 1:text.rindex(';')])


def significance(points):
    """Each vertex's Douglas-Peucker significance, capped by its parent's, so that keeping the
    vertices above a tolerance is exactly Douglas-Peucker at that tolerance."""
    n = len(points)
    sig = [0.0] * n
    sig[0] = sig[-1] = math.inf
    stack = [(0, n - 1, math.inf)]
    while stack:
        a, b, cap = stack.pop()
        if b - a < 2:
            continue
        ax, ay = points[a]
        bx, by = points[b]
        k = math.cos(math.radians((ay + by) / 2))
        dx, dy = (bx - ax) * k, by - ay
        dd = dx * dx + dy * dy
        best, at = -1.0, -1
        for i in range(a + 1, b):
            px, py = (points[i][0] - ax) * k, points[i][1] - ay
            t = max(0.0, min(1.0, (px * dx + py * dy) / dd)) if dd else 0.0
            ex, ey = px - t * dx, py - t * dy
            d = ex * ex + ey * ey
            if d > best:
                best, at = d, i
        s = min(math.sqrt(best), cap)
        sig[at] = s
        stack.append((a, at, s))
        stack.append((at, b, s))
    return sig


def level(s):
    if s == math.inf:
        return 0
    for i, t in enumerate(LEVELS):
        if s > t:
            return i + 1
    return None


def network():
    data = read_js('network.js')
    rivers = data['rivers']
    # A stretch's basin and inland flag are its outlet's (checked here, so the pack keeps neither).
    def outlet(i):
        seen = set()
        while rivers[i][0] >= 0:
            assert i not in seen, 'cycle'
            seen.add(i)
            i = rivers[i][0]
        return i
    for i, r in enumerate(rivers):
        o = rivers[outlet(i)]
        assert r[3] == o[3] and r[4] == o[4], f'stretch {i} disagrees with its outlet'
        if r[0] >= 0:
            assert r[5][-2:] == rivers[r[0]][5][:2], f'stretch {i} does not meet the next'
    lines, lod, cells = [], [], {}
    for i, r in enumerate(rivers):
        pts = [(r[5][j], r[5][j + 1]) for j in range(0, len(r[5]), 2)]
        sig = significance(pts)
        kept = [(p, level(s)) for p, s in zip(pts, sig) if level(s) is not None]
        lines.append([[round(p[0], 3), round(p[1], 3)] for p, _ in kept])
        lod.append(''.join(str(lv) for _, lv in kept))
        # the nearest-line grid: every cell a kept segment's box touches (core.js reads it as built)
        for (a, _), (b, _) in zip(kept, kept[1:]):
            for gx in range(math.floor(min(a[0], b[0]) / CELL), math.floor(max(a[0], b[0]) / CELL) + 1):
                for gy in range(math.floor(min(a[1], b[1]) / CELL), math.floor(max(a[1], b[1]) / CELL) + 1):
                    ids = cells.setdefault(str((gx + 500) * 1000 + gy + 500), [])
                    if not ids or ids[-1] != i:
                        ids.append(i)
    out = {
        'build': json.loads((WATER / 'manifest.json').read_text(encoding='utf-8')),
        'about': 'RiverATLAS v1.0 (CC BY 4.0) as built by tools/water/build.py into Water/network.js, '
                 'lightened by tools/water/build_pack.py. Per stretch: next (index, -1 at an outlet), '
                 'order (Strahler), km10 (summed reach lengths, tenths of a km), area (km² upstream of '
                 'its lower end), lines ([lon, lat] in degrees), lod (one digit a vertex: 0 for the ends, '
                 'else the index+1 of the coarsest of levels it survives at). grid: for each cell of '
                 'cell degrees, keyed (floor(lon / cell) + 500) * 1000 + floor(lat / cell) + 500, the '
                 'stretches with a segment there.',
        'levels': LEVELS,
        'next': [r[0] for r in rivers],
        'order': [r[1] for r in rivers],
        'km10': [round(r[2] * 10) for r in rivers],
        'area': [r[7] for r in rivers],
        'inland': [i for i, r in enumerate(rivers) if r[0] < 0 and r[4]],
        'lines': lines,
        'lod': lod,
        'grid': {'cell': CELL, 'cells': cells},
    }
    (OUT / 'network.json').write_text(json.dumps(out, separators=(',', ':'), ensure_ascii=False) + '\n', encoding='utf-8')
    kept = sum(len(s) for s in lod)
    print(f'{len(rivers)} stretches, {sum(len(r[5]) // 2 for r in rivers)} vertices in, {kept} kept;',
          ', '.join(f'{t}°: {sum(1 for s in lod for c in s if int(c) <= i + 1)}' for i, t in enumerate(LEVELS)))
    return rivers


def snap(rivers, lon, lat, radius=0.55, min_order=3):
    """Water/core.js nearest(), in its map units, over the study's own geometry."""
    x, y = (lon + 84) * C, 15 - lat
    best, hit = radius * radius, None
    for i, r in enumerate(rivers):
        if r[1] < min_order:
            continue
        g = r[5]
        for j in range(2, len(g), 2):
            ax, ay = (g[j - 2] + 84) * C, 15 - g[j - 1]
            dx, dy = (g[j] + 84) * C - ax, 15 - g[j + 1] - ay
            dd = dx * dx + dy * dy
            t = max(0, min(1, ((x - ax) * dx + (y - ay) * dy) / dd)) if dd else 0
            px, py = ax + t * dx, ay + t * dy
            d = (px - x) ** 2 + (py - y) ** 2
            if d <= best:
                best, hit = d, (i, px / C - 84, 15 - py)
    return hit


def places(rivers):
    src = read_js('places.js')
    capitals = {c['n'] for c in json.loads((ROOT / 'Globe' / 'globe.json').read_text(encoding='utf-8'))['capitals']}
    alias = {'Bogota': 'Bogotá'}
    out = {'about': 'Water/places.js (tools/water/labels.json): river anchors snapped to the network as '
                    'Water/water.js snaps them; cities from Natural Earth 1:50m populated places, less the '
                    'capitals the globe names itself. ppd: the reading density (points a degree) a town is named from.',
           'rivers': [], 'cities': []}
    for r in src['rivers']:
        hit = snap(rivers, *r['at'])
        if not hit:
            print('no stretch near', r['name'])
            continue
        i, lon, lat = hit
        entry = {k: r[k] for k in ('name', 'label', 'labelAt', 'system', 'major') if k in r}
        entry['edge'] = i
        entry['at'] = [round(lon, 4), round(lat, 4)]
        entry.setdefault('labelAt', entry['at'])
        out['rivers'].append(entry)
    for c in src['cities']:
        if c['name'] in capitals or alias.get(c['name']) in capitals:
            continue
        # the study showed these from its own zoom 1.8 or 3: about 8 and 11 points a degree
        out['cities'].append({'name': c['name'], 'at': c['at'], 'ppd': 8 if c.get('min', 3) < 2 else 11})
    (OUT / 'places.json').write_text(json.dumps(out, indent=1, ensure_ascii=False) + '\n', encoding='utf-8')
    print('rivers', [r['label'] for r in out['rivers']], 'cities', [c['name'] for c in out['cities']])
    return out


def doorway(rivers, anchors):
    """A small disc of the paper's tone with the Amazon system's larger rivers (order 7 and up) in the
    globe's water ink."""
    amazon = next(a for a in anchors['rivers'] if a.get('system') == 'Amazon')
    def outlet(i):
        while rivers[i][0] >= 0:
            i = rivers[i][0]
        return i
    basin = outlet(amazon['edge'])
    lines = [r[5] for i, r in enumerate(rivers) if r[1] >= 7 and outlet(i) == basin]
    xs = [g[j] for g in lines for j in range(0, len(g), 2)]
    ys = [g[j] for g in lines for j in range(1, len(g), 2)]
    lon0, lat0 = (min(xs) + max(xs)) / 2, (min(ys) + max(ys)) / 2
    k = math.cos(math.radians(lat0))
    span = max((max(xs) - min(xs)) * k, max(ys) - min(ys))
    S, size = 8, 96                      # 32 points at 3x, drawn 8x larger and reduced
    W = size * S
    im = Image.new('RGBA', (W, W), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    d.ellipse([S, S, W - S, W - S], fill=(246, 242, 231, 235), outline=(28, 122, 176, 190), width=2 * S)
    inner = W * 0.68
    def at(lon, lat):
        return (W / 2 + (lon - lon0) * k / span * inner, W / 2 - (lat - lat0) / span * inner)
    for g, r in sorted(((r[5], r) for i, r in enumerate(rivers) if r[1] >= 7 and outlet(i) == basin), key=lambda q: q[1][1]):
        pts = [at(g[j], g[j + 1]) for j in range(0, len(g), 2)]
        o = r[1]
        width = S * (0.75 if o <= 7 else 1.0 if o <= 8 else 1.6)
        colour = (78, 156, 203, 255) if o <= 7 else (28, 122, 176, 255)
        d.line(pts, fill=colour, width=max(1, round(width)), joint='curve')
    im.resize((size, size), Image.Resampling.LANCZOS).save(OUT / 'doorway.png', optimize=True)


def glint():
    """The bead of light that runs down a traced river: round (a sprite is mirrored, never turned, as
    it moves, so a pointed picture would go sideways down a north-south river), a bright core in the
    trace's deep ink, on a faint glow."""
    S, n = 8, 24                         # 8 by 8 points at 3x
    W, c = n * S, n * S / 2
    im = Image.new('RGBA', (W, W), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    for r, fill in ((11.5, (255, 253, 244, 90)), (8.5, (13, 74, 120, 255)), (5.0, (255, 253, 244, 255))):
        d.ellipse([c - r * S, c - r * S, c + r * S, c + r * S], fill=fill)
    im.resize((n, n), Image.Resampling.LANCZOS).save(OUT / 'glint.png', optimize=True)


if __name__ == '__main__':
    if len(sys.argv) > 1:
        WATER = Path(sys.argv[1]).resolve()
    OUT.mkdir(parents=True, exist_ok=True)
    rivers = network()
    anchors = places(rivers)
    doorway(rivers, anchors)
    glint()
