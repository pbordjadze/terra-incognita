#!/usr/bin/env python3
"""Prepare the data of the pack Prototypes/gathering-of-water-alive/ (A gathering of water, alive).

Usage: python3 tools/water/build_alive.py
       (then node tools/water/boto.cjs for the boto's frames)

Reads Prototypes/gathering-of-water/ (built by tools/water/build_pack.py) and the ocean layer's
currents, App/TerraIncognita/Ocean/OceanCurrents.png (built by tools/ocean/build.py from the NOAA
Global Drifter Program's monthly climatology; its docstring is the sheet's layout), and writes:

- network.json, places.json, glint.png, doorway-glint.png: A gathering of water's own, copied
  unchanged (the river network, its anchors, the glint), so the two packs show the same rivers.
- sea.json: for the mouths of the Amazon, the Tocantins and the Orinoco, where their water goes in
  the sea, month by month: a drawn way out of the estuary (a few hand-placed points along the
  channel, not measured, and a curve from its end out to each seed), then streamlines of that month's mean surface current from seeds across
  the mouth, traced (fourth-order Runge-Kutta, three-hour steps) for at most MAX_DAYS days or until
  they leave the measured sea. They are the month's mean currents, not a day's: the paths a drop
  would take if the month's average held. Each path is thinned to a vertex every 0.12 degrees.
  `length` is a mouth's longest way, in degrees of arc, so the script runs the light along it.
- plume-01.png ... plume-12.png: the Amazon's plume for each month, a picture lying on the globe
  over PLUME_BOUNDS: many drops released across the mouth and carried for PLUME_DAYS days on the
  month's currents, counted where they pass, smoothed, and tinted from the river's silt near the
  mouth to green offshore. Drawn from the currents, not observed: the real plume's extent and
  colour vary year to year (SOURCES.md).
- doorway.png: the doorway up close, drawn from the network as A gathering of water's is, with
  the main stem lit and a boto's back in the mouth.

Never hand-edit those files; change this script and run it again. No network is used.
"""
import json
import math
import shutil
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / 'Prototypes' / 'gathering-of-water'
OUT = ROOT / 'Prototypes' / 'gathering-of-water-alive'
CURRENTS = ROOT / 'App' / 'TerraIncognita' / 'Ocean' / 'OceanCurrents.png'

W, H, TW, TH = 720, 360, 722, 362          # a month's half-degree cells, and its tile with a border
DT = 3 * 3600.0                            # seconds a step
M_PER_DEG = 111_320.0
MAX_DAYS = 50
PLUME_DAYS = 60
PLUME_BOUNDS = [-72.0, -4.0, -24.0, 18.0]  # west, south, east, north
PLUME_SIZE = (480, 220)                    # 0.1 degree a pixel

# The mouths, by the network's outlet stretch: a drawn way out of the estuary (from the outlet's last
# vertex) and the seeds across the mouth where the currents take over.
MOUTHS = {
    '3407': {'name': 'Amazon',
             'estuary': [[-51.081, -0.669], [-50.82, -0.38], [-50.48, -0.02], [-50.12, 0.36], [-49.78, 0.72]],
             # across the shelf: inshore seeds follow the coast, offshore ones meet the current's turn east
             'seeds': [[-49.7, 0.8], [-50.2, 1.9], [-49.0, 1.6], [-48.0, 1.0], [-48.4, 2.4], [-49.3, 2.8],
                       [-47.5, 2.0], [-48.6, 3.4]],
             'plume': True},
    '4050': {'name': 'Tocantins',
             'estuary': [[-49.398, -2.19], [-49.05, -1.75], [-48.72, -1.32], [-48.45, -0.9], [-48.25, -0.55]],
             'seeds': [[-48.2, -0.45], [-47.9, -0.35], [-48.45, -0.2]]},
    '422': {'name': 'Orinoco',
            'estuary': [[-60.981, 8.577], [-60.75, 8.72], [-60.48, 8.86], [-60.22, 9.0]],
            'seeds': [[-60.1, 9.1], [-59.95, 8.85], [-60.3, 9.35]]},
}


def currents():
    """The twelve months' eastward and northward currents (m/s) and how measured (0-1), each H x W."""
    sheet = np.asarray(Image.open(CURRENTS).convert('RGB')).astype(np.float64)
    months = []
    for m in range(12):
        tx, ty = m % 4, m // 4
        tile = sheet[ty * TH + 1:(ty + 1) * TH - 1, tx * TW + 1:(tx + 1) * TW - 1]
        def decode(c):
            s = (c - 128) / 127
            return np.sign(s) * 2.0 * s * s
        months.append((decode(tile[..., 0]), decode(tile[..., 1]), tile[..., 2] / 255))
    return months


def sample(field, lon, lat):
    """Bilinear in the half-degree cells (cell centres at -179.75 + 0.5 i, 89.75 - 0.5 j)."""
    u, v, b = field
    x = (lon + 179.75) / 0.5
    y = (89.75 - lat) / 0.5
    i, j = int(math.floor(x)), int(math.floor(y))
    fx, fy = x - i, y - j
    if not (0 <= j < H - 1):
        return 0.0, 0.0, 0.0
    i0, i1 = i % W, (i + 1) % W
    def at(a):
        return ((1 - fx) * (1 - fy) * a[j, i0] + fx * (1 - fy) * a[j, i1]
                + (1 - fx) * fy * a[j + 1, i0] + fx * fy * a[j + 1, i1])
    return at(u), at(v), at(b)


def step(field, lon, lat):
    """One Runge-Kutta step of DT seconds along the month's mean current; None where unmeasured."""
    def rate(lo, la):
        u, v, b = sample(field, lo, la)
        if b < 0.35:
            return None
        k = math.cos(math.radians(la))
        return u / (M_PER_DEG * max(k, 0.05)), v / M_PER_DEG
    k1 = rate(lon, lat)
    if k1 is None:
        return None
    k2 = rate(lon + k1[0] * DT / 2, lat + k1[1] * DT / 2)
    if k2 is None:
        return None
    k3 = rate(lon + k2[0] * DT / 2, lat + k2[1] * DT / 2)
    if k3 is None:
        return None
    k4 = rate(lon + k3[0] * DT, lat + k3[1] * DT)
    if k4 is None:
        return None
    return (lon + DT / 6 * (k1[0] + 2 * k2[0] + 2 * k3[0] + k4[0]),
            lat + DT / 6 * (k1[1] + 2 * k2[1] + 2 * k3[1] + k4[1]))


def trace(field, lon, lat, days):
    points = [(lon, lat)]
    for _ in range(int(days * 86400 / DT)):
        nxt = step(field, lon, lat)
        if nxt is None:
            break
        lon, lat = nxt
        if not (-90 < lon < 0 and -20 < lat < 30):
            break
        points.append((lon, lat))
    return points


def span(a, b):
    k = math.cos(math.radians((a[1] + b[1]) / 2))
    return math.hypot((b[0] - a[0]) * k, b[1] - a[1])


def thin(points, every=0.12):
    out, run = [points[0]], 0.0
    for a, b in zip(points, points[1:]):
        run += span(a, b)
        if run >= every:
            out.append(b)
            run = 0.0
    if out[-1] != points[-1]:
        out.append(points[-1])
    return out


def length(points):
    return sum(span(a, b) for a, b in zip(points, points[1:]))


def joined(field, estuary, seed):
    """From the estuary's end out to a seed, a curve leaving the way the estuary runs and arriving the
    way the current at the seed runs (a cubic Bezier: no corner where the drawn way meets the traced)."""
    m, before = estuary[-1], estuary[-2]
    u, v, _ = sample(field, seed[0], seed[1])
    out = (m[0] - before[0], m[1] - before[1])
    n = math.hypot(*out) or 1
    out = (out[0] / n, out[1] / n)
    k = math.hypot(u, v) or 1
    flow = (u / k, v / k)
    d = math.hypot(seed[0] - m[0], seed[1] - m[1])
    p1 = (m[0] + out[0] * 0.45 * d, m[1] + out[1] * 0.45 * d)
    p2 = (seed[0] - flow[0] * 0.45 * d, seed[1] - flow[1] * 0.45 * d)
    pts = []
    for i in range(1, 12):
        t = i / 12
        a, b, c, e = (1 - t) ** 3, 3 * (1 - t) ** 2 * t, 3 * (1 - t) * t * t, t ** 3
        pts.append((a * m[0] + b * p1[0] + c * p2[0] + e * seed[0], a * m[1] + b * p1[1] + c * p2[1] + e * seed[1]))
    return pts


def sea(months):
    outlets = {}
    for id, mouth in MOUTHS.items():
        estuary = [tuple(p) for p in mouth['estuary']]
        per_month, longest = [], 0.0
        for field in months:
            paths = []
            for seed in mouth['seeds']:
                stream = trace(field, seed[0], seed[1], MAX_DAYS)
                if len(stream) < 4:
                    continue
                way = thin(estuary + joined(field, estuary, seed) + stream)
                longest = max(longest, length(way))
                paths.append([[round(x, 3), round(y, 3)] for x, y in way])
            per_month.append(paths)
        entry = {'name': mouth['name'], 'length': round(longest, 2), 'months': per_month}
        if mouth.get('plume'):
            entry['picture'] = {'images': ['plume-%02d.png' % (m + 1) for m in range(12)],
                                'bounds': PLUME_BOUNDS, 'opacity': 0.85}
        outlets[id] = entry
    out = {'about': 'Where the lit water goes in the sea: a drawn way out of each estuary, then streamlines of '
                    "the month's mean surface current (NOAA Global Drifter Program climatology, CC BY 4.0), "
                    'from tools/water/build_alive.py. Months 1 to 12.',
           'outlets': outlets}
    (OUT / 'sea.json').write_text(json.dumps(out, separators=(',', ':')) + '\n', encoding='utf-8')
    return outlets


def blur(a, sigma):
    """A Gaussian blur, separable, edges held."""
    r = int(3 * sigma)
    k = np.exp(-0.5 * (np.arange(-r, r + 1) / sigma) ** 2)
    k /= k.sum()
    a = np.pad(a, r, mode='edge')
    a = np.apply_along_axis(lambda x: np.convolve(x, k, mode='valid'), 0, a)
    return np.apply_along_axis(lambda x: np.convolve(x, k, mode='valid'), 1, a)


def plume(months):
    """The Amazon's plume, month by month: drops carried from the mouth, counted, smoothed, tinted."""
    rng = np.random.default_rng(1)
    west, south, east, north = PLUME_BOUNDS
    w, h = PLUME_SIZE
    # the river's silt near the mouth, the green of its dissolved matter offshore (drawn, not sampled)
    silt, green = np.array([178, 142, 82]), np.array([92, 150, 112])
    for m, field in enumerate(months):
        count = np.zeros((h, w))
        age = np.zeros((h, w))
        for _ in range(1400):
            lon, lat = -49.0 + rng.normal(0, 0.7), 1.7 + rng.normal(0, 0.8)
            for n in range(int(PLUME_DAYS * 86400 / DT)):
                nxt = step(field, lon, lat)
                if nxt is None:
                    break
                lon, lat = nxt
                # a little spread, as eddies spread a plume (not measured)
                lon += rng.normal(0, 0.05)
                lat += rng.normal(0, 0.05)
                x, y = int((lon - west) / (east - west) * w), int((north - lat) / (north - south) * h)
                if not (0 <= x < w and 0 <= y < h):
                    break
                fade = math.exp(-n * DT / 86400 / 40)
                count[y, x] += fade
                age[y, x] += fade * n * DT / 86400
        mean_age = np.where(count > 0, age / np.maximum(count, 1e-9), 0)
        density = blur(count, 4)
        mean_age = blur(mean_age, 4)
        d = density / max(1e-9, np.percentile(density[density > 0], 99))
        alpha = np.clip(d, 0, 1) ** 0.55
        t = np.clip(mean_age / 25, 0, 1)[..., None]
        rgb = silt * (1 - t) + green * t
        # (the silt is thickest near the mouth; the green thins out to sea)
        image = np.dstack([rgb, alpha * (190 - 70 * t[..., 0])]).clip(0, 255).astype(np.uint8)
        Image.fromarray(image, 'RGBA').save(OUT / ('plume-%02d.png' % (m + 1)), optimize=True)


def doorway():
    """The doorway up close: the Amazon's network in a ring, its main stem lit, a boto's back at the mouth."""
    net = json.loads((SRC / 'network.json').read_text(encoding='utf-8'))
    nxt, order, lines = net['next'], net['order'], net['lines']
    n = len(nxt)
    outlet = [-1] * n
    def find(i):
        path = []
        while nxt[i] >= 0 and outlet[i] < 0:
            path.append(i)
            i = nxt[i]
        o = outlet[i] if outlet[i] >= 0 else i
        for k in path + [i]:
            outlet[k] = o
        return o
    for i in range(n):
        find(i)
    amazon = [i for i in range(n) if outlet[i] == 3407 and order[i] >= 5]
    # the main stem: from the Ucayali's anchor down
    stem, i = set(), 7076
    while i >= 0:
        stem.add(i)
        i = nxt[i]
    size, k = 96, 4
    big = size * k
    im = Image.new('RGBA', (big, big), (0, 0, 0, 0))
    d = ImageDraw.Draw(im)
    cx, cy, r = big / 2, big / 2, big / 2 - 3 * k
    d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=(245, 241, 230, 245), outline=(58, 134, 186, 255), width=int(2.2 * k))
    west, east, south, north = -79.0, -49.0, -16.0, 5.0
    scale = 0.86 * 2 * r / (east - west)
    def at(lon, lat):
        return (cx + (lon - (west + east) / 2) * scale, cy - (lat - (south + north) / 2) * scale)
    for i in sorted(amazon, key=lambda i: (i in stem, order[i])):
        pts = [at(x, y) for x, y in lines[i]]
        if i in stem:
            d.line(pts, fill=(11, 61, 102, 255), width=int(2.6 * k), joint='curve')
        else:
            d.line(pts, fill=(78, 156, 203, 230), width=max(1, int((0.35 * order[i] - 1.2) * k)), joint='curve')
    # a boto's back breaking the water at the mouth (pink, as A gathering of water, alive draws it)
    mx, my = at(-51.5, -0.9)
    d.chord([mx - 7 * k, my - 6 * k, mx + 7 * k, my + 6 * k], 180, 360, fill=(226, 138, 154, 255), outline=(142, 78, 92, 255), width=k)
    d.arc([mx - 11 * k, my - 2 * k, mx + 11 * k, my + 3 * k], 0, 180, fill=(245, 241, 230, 255), width=k)
    im.resize((size, size), Image.Resampling.LANCZOS).save(OUT / 'doorway.png', optimize=True)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    for name in ('network.json', 'places.json', 'glint.png', 'doorway-glint.png'):
        shutil.copyfile(SRC / name, OUT / name)
    months = currents()
    outlets = sea(months)
    for id, o in outlets.items():
        print(o['name'], 'longest way', o['length'], 'degrees; paths a month', [len(p) for p in o['months']])
    plume(months)
    doorway()


if __name__ == '__main__':
    main()
