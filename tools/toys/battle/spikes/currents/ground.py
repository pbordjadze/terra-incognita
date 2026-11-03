#!/usr/bin/env python3
"""Bake Cannae's ground for the currents spike.

  python3 ground.py <tile dir>

Terrarium elevation tiles (z13, x 4460..4465, y 3059..3064: the lower Ofanto from the hills to the
sea) are downloaded into <tile dir> when missing (AWS open data,
s3.amazonaws.com/elevation-tiles-prod/terrarium). The river is not drawn by hand: the elevation is
pit-filled (priority flood) and D8 flow accumulation finds where the water gathers, which is the
Aufidus's floodplain thalweg.

Writes ground.js beside this script: a rotated window of the plain between the river and the Cannae
ridge, turned so the Roman front (facing south-west, river on its right flank, one common
reconstruction) faces down the screen. Up is north-east. Grid 256x192 cells, about 20 m each:
elevation in decimetres and the river's accumulation (0..255, log). Also writes previews into
<tile dir>.
"""
import heapq, json, math, os, subprocess, sys
import numpy as np
from PIL import Image
from scipy.ndimage import gaussian_filter as gf, map_coordinates

XS, YS, Z = range(4460, 4466), range(3059, 3065), 13


def px(lon, lat):
    n = 2 ** Z
    x = (lon + 180) / 360 * n
    y = (1 - math.asinh(math.tan(math.radians(lat))) / math.pi) / 2 * n
    return (x - XS[0]) * 256, (y - YS[0]) * 256


def main():
    d = sys.argv[1]
    os.makedirs(d, exist_ok=True)
    E = np.zeros((256 * len(YS), 256 * len(XS)))
    for j, y in enumerate(YS):
        for i, x in enumerate(XS):
            p = os.path.join(d, f'z13_{x}_{y}.png')
            if not os.path.exists(p):
                subprocess.run(['curl', '-sS', '-o', p,
                    f'https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{Z}/{x}/{y}.png'], check=True)
            a = np.asarray(Image.open(p).convert('RGB')).astype(float)
            E[j*256:(j+1)*256, i*256:(i+1)*256] = a[..., 0]*256 + a[..., 1] + a[..., 2]/256 - 32768
    E = np.maximum(E, -1)  # the sea
    # --- flow accumulation at half resolution
    S = gf(E, 1.5)[::2, ::2]
    H, W = S.shape
    filled = S.copy()
    seen = np.zeros_like(S, bool)
    pq = []
    for y in range(H):
        for x in range(W):
            if y in (0, H-1) or x in (0, W-1) or S[y, x] <= 0:
                heapq.heappush(pq, (S[y, x], y, x)); seen[y, x] = True
    nb = [(-1,-1),(-1,0),(-1,1),(0,-1),(0,1),(1,-1),(1,0),(1,1)]
    order = []
    eps = 1e-4
    while pq:
        e, y, x = heapq.heappop(pq)
        order.append((y, x))
        for dy, dx in nb:
            yy, xx = y+dy, x+dx
            if 0 <= yy < H and 0 <= xx < W and not seen[yy, xx]:
                seen[yy, xx] = True
                filled[yy, xx] = max(S[yy, xx], e + eps)
                heapq.heappush(pq, (filled[yy, xx], yy, xx))
    acc = np.ones_like(S)
    for y, x in reversed(order):  # highest first
        best, by, bx = 0, -1, -1
        for dy, dx in nb:
            yy, xx = y+dy, x+dx
            if 0 <= yy < H and 0 <= xx < W:
                drop = (filled[y, x] - filled[yy, xx]) / (1.414 if dy and dx else 1)
                if drop > best:
                    best, by, bx = drop, yy, xx
        if by >= 0:
            acc[by, bx] += acc[y, x]
    R = np.log1p(acc) / np.log1p(acc.max())
    Rfull = np.kron(R, np.ones((2, 2)))
    # --- the window: centre on the plain north-west of Canne della Battaglia (41.2967 N, 16.1528 E)
    cx, cy = px(16.1528, 41.2967)
    ang = math.radians(float(os.environ.get('ANG', 45)))  # screen-down = south-west
    off_x, off_y = float(os.environ.get('OX', -82)), float(os.environ.get('OY', -97))
    cx += off_x; cy += off_y
    gw, gh = 256, 192
    span = float(os.environ.get('SPAN', 330))  # source pixels across (z13 ~14.4 m/px here)
    u = np.linspace(-0.5, 0.5, gw) * span
    v = np.linspace(-0.5, 0.5, gh) * span * gh / gw
    U, V = np.meshgrid(u, v)
    # screen +x, +y(down) -> map: rotate by ang (map up = north)
    ca, sa = math.cos(ang), math.sin(ang)
    MX = cx + U * ca - V * sa
    MY = cy + U * sa + V * ca
    el = map_coordinates(gf(E, 0.7), [MY, MX], order=1)
    rv = map_coordinates(Rfull, [MY, MX], order=1)
    mpp = 40075016 * math.cos(math.radians(41.3)) / (2 ** Z * 256)
    # the river's line: from the strongest downstream (top-edge) cell, greedily up the
    # accumulation, one row at a time, then smoothed (the river has moved since 216 BC anyway)
    x = int(np.argmax(rv[0, :int(gw * 0.6)]))
    line = [(x, 0)]
    for y in range(1, gh):
        lo_, hi_ = max(0, x - 3), min(gw, x + 4)
        x = lo_ + int(np.argmax(rv[y, lo_:hi_] + 0.02 * -np.abs(np.arange(lo_, hi_) - x)))
        line.append((x, y))
    from scipy.ndimage import gaussian_filter1d
    lx = gaussian_filter1d(np.array([p[0] for p in line], float), 6, mode='nearest')
    out_line = [[round(float(a), 1), y] for y, a in enumerate(lx)][::2]
    out = {'riverLine': out_line, 'w': gw, 'h': gh, 'metresPerCell': round(span * mpp / gw, 1),
           'elev_dm': [int(round(x * 10)) for x in el.flatten()],
           'river': [int(round(x * 255)) for x in rv.flatten()]}
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, 'ground.js'), 'w') as f:
        f.write('// Baked by ground.py from Terrarium elevation tiles (AWS open data). Do not hand-edit.\n')
        f.write('window.GROUND=' + json.dumps(out, separators=(',', ':')) + ';\n')
    # previews
    hp = np.clip((gf(E, 2) - gf(E, 25)) * 25 + 128, 0, 255)
    img = np.stack([hp]*3, -1)
    img[..., 2] = np.maximum(img[..., 2], (Rfull > 0.55) * 255)
    for (a, b) in [(0, 0), (gw-1, 0), (0, gh-1), (gw-1, gh-1)]:
        img[int(MY[b, a])-4:int(MY[b, a])+4, int(MX[b, a])-4:int(MX[b, a])+4] = [255, 0, 0]
    Image.fromarray(img.astype(np.uint8)).save(os.path.join(d, 'ground_preview.png'))
    lo, hi = np.percentile(el, 2), np.percentile(el, 98)
    w = np.clip((el - lo) / (hi - lo), 0, 1)
    hpw = np.clip((el - gf(el, 12)) * 0.25 + 0.5, 0, 1)
    Image.fromarray((hpw * 255).astype(np.uint8)).resize((768, 576)).save(os.path.join(d, 'window_hp.png'))
    wi = np.stack([w, w, np.maximum(w, rv > 0.55)], -1)
    Image.fromarray((wi * 255).astype(np.uint8)).resize((768, 576)).save(os.path.join(d, 'window_preview.png'))
    print('elev', el.min(), el.max(), 'm/cell', out['metresPerCell'])


if __name__ == '__main__':
    main()
