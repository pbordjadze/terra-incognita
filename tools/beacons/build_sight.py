#!/usr/bin/env python3
"""Which peaks of the Aegean can see which (Prototypes/fires-across-the-sea/sight.js): for every pair of
peaks in data.js within the sum of their horizons (3.86 km × √height each, with standard refraction),
the line of sight is checked against the globe's own terrain (Globe/terrain.png, whose green channel
is 128 + height / 50 m on land, from GEBCO through tools/globe/build.py; about ten kilometres a pixel):
the ray from fire to fire, over an Earth made 7/6 larger for refraction, must clear the ground at every
sample between them. A fire is taken to burn 4 m above its peak.

    python3 tools/beacons/build_sight.py

Rebuild, never hand-edit. Coarse ground means a thin ridge may be missed; a broad range is not.
"""
import json, math, re
from pathlib import Path
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
PACK = ROOT / 'Prototypes' / 'fires-across-the-sea'
R = 6371.0
K = 0.13                      # refraction: the ray bends down a little, so the Earth seems larger
RE = R / (1 - K)
FIRE = 4.0                    # metres above the peak

def peaks():
    text = (PACK / 'data.js').read_text(encoding='utf-8')
    out = []
    for m in re.finditer(r"\{ id: '([^']+)', name: '[^']*', lon: ([\d.]+), lat: ([\d.]+), h: (\d+) \}", text):
        out.append({'id': m.group(1), 'lon': float(m.group(2)), 'lat': float(m.group(3)), 'h': int(m.group(4))})
    return out

def unit(lon, lat):
    return np.array([math.cos(lat * math.pi / 180) * math.sin(lon * math.pi / 180), math.sin(lat * math.pi / 180), math.cos(lat * math.pi / 180) * math.cos(lon * math.pi / 180)])

def main():
    im = np.asarray(Image.open(ROOT / 'Globe' / 'terrain.png').convert('RGBA'))
    g = im[..., 1].astype(np.int32)
    height = np.where(g >= 128, (g - 128) * 50, 0).astype(np.float64)     # metres above the sea, 0 at sea
    H, W = height.shape
    P = peaks()
    sees = {p['id']: [] for p in P}
    checked = 0
    for i in range(len(P)):
        for j in range(i + 1, len(P)):
            a, b = P[i], P[j]
            ua, ub = unit(a['lon'], a['lat']), unit(b['lon'], b['lat'])
            w = math.acos(max(-1, min(1, float(ua @ ub))))
            d = w * R
            if d > 3.86 * (math.sqrt(a['h']) + math.sqrt(b['h'])) or d < 1:
                continue
            checked += 1
            n = max(8, int(d / 4))
            ok = True
            ha, hb = a['h'] + FIRE, b['h'] + FIRE
            for k in range(1, n):
                t = k / n
                sa, sb = math.sin((1 - t) * w) / math.sin(w), math.sin(t * w) / math.sin(w)
                p = ua * sa + ub * sb
                lon = math.atan2(p[0], p[2]) * 180 / math.pi
                lat = math.asin(max(-1, min(1, p[1]))) * 180 / math.pi
                x = int((lon + 180) / 360 * W) % W
                y = min(H - 1, max(0, int((90 - lat) / 180 * H)))
                ground = height[y, x]
                # the ray's height above the (refracted) sphere at this point: a straight line between
                # two points at heights ha and hb over a sphere of radius RE
                x1, x2 = d * t, d * (1 - t)
                ray = (ha * (1 - t) + hb * t) - (x1 * x2) / (2 * RE) * 1000
                if ground > ray:
                    ok = False
                    break
            if ok:
                sees[a['id']].append([b['id'], round(d)])
                sees[b['id']].append([a['id'], round(d)])
    out = PACK / 'sight.js'
    out.write_text('/* Which peak can see which, worked out over the globe\'s own terrain by tools/beacons/build_sight.py:\n'
                   '   for each peak, the peaks whose fire it could see, with the distance in km. */\n'
                   'window.SIGHT = ' + json.dumps(sees, separators=(',', ':')) + ';\n', encoding='utf-8')
    pairs = sum(len(v) for v in sees.values()) // 2
    print(f'{out}: {pairs} pairs see each other of {checked} within their horizons ({len(P)} peaks)')
    lonely = [k for k, v in sees.items() if not v]
    if lonely:
        print('no one sees:', ', '.join(lonely))

if __name__ == '__main__':
    main()
