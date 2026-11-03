#!/usr/bin/env python3
"""Builds the fireflies: one for each patch of the Earth's lights at night (painting-company row 204).

    python3 tools/toys/fireflies/build.py            # writes toys/fireflies/data/fireflies.bin

Reads Terra's night sheet (Globe/ground/night.jpg, built by tools/globe/night.py from NASA's Black
Marble, 4096x2048 equirectangular, one channel), and places fireflies on its lit pixels, brightest
first, none closer than SPACING_KM to another (a greedy Poisson-disc sample on the sphere), so a
city is one firefly and a dense lit region a field of them. Each firefly's weight is the light of
the lit pixels nearest to it (its share of the sheet), normalised so the brightest region is 1.

Layout (little-endian): u32 count, then count records of three f32: longitude and latitude in
degrees, weight 0..1. Never hand-edit it; rebuild.
"""
import struct
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
SRC = ROOT / 'Globe' / 'ground' / 'night.jpg'
OUT = ROOT / 'toys' / 'fireflies' / 'data' / 'fireflies.bin'
THRESHOLD = 0.10      # a pixel counts as lit above this (the sheet's 0..1)
SPACING_KM = 75.0     # no two fireflies closer than this
EARTH_KM = 6371.0


def main():
    a = np.asarray(Image.open(SRC).convert('L')).astype(np.float32) / 255.0
    h, w = a.shape
    ys, xs = np.nonzero(a > THRESHOLD)
    v = a[ys, xs]
    lon = (xs + 0.5) / w * 360.0 - 180.0
    lat = 90.0 - (ys + 0.5) / h * 180.0
    order = np.argsort(-v, kind='stable')
    lonr, latr = np.radians(lon), np.radians(lat)
    xyz = np.stack([np.cos(latr) * np.sin(lonr), np.sin(latr), np.cos(latr) * np.cos(lonr)], 1)
    cell = SPACING_KM / EARTH_KM  # radians; grid cells in 3D of this size
    grid = {}
    chosen = []
    lim = 2.0 * np.sin(cell / 2.0)  # chord length
    for i in order:
        p = xyz[i]
        k = tuple(np.floor(p / lim).astype(int))
        ok = True
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                for dz in (-1, 0, 1):
                    for j in grid.get((k[0] + dx, k[1] + dy, k[2] + dz), ()):
                        if np.sum((xyz[j] - p) ** 2) < lim * lim:
                            ok = False
                            break
                    if not ok:
                        break
                if not ok:
                    break
            if not ok:
                break
        if ok:
            grid.setdefault(k, []).append(i)
            chosen.append(i)
    chosen = np.array(chosen)
    # each lit pixel's light (times its area on the sphere) goes to the nearest firefly
    cx = xyz[chosen]
    area = np.cos(latr)
    weight = np.zeros(len(chosen))
    for s in range(0, len(xyz), 20000):
        d = xyz[s:s + 20000] @ cx.T
        near = np.argmax(d, 1)
        np.add.at(weight, near, v[s:s + 20000] * area[s:s + 20000])
    weight = np.sqrt(weight / weight.max())
    OUT.parent.mkdir(parents=True, exist_ok=True)
    with open(OUT, 'wb') as f:
        f.write(struct.pack('<I', len(chosen)))
        for n, i in enumerate(chosen):
            f.write(struct.pack('<fff', lon[i], lat[i], weight[n]))
    print(f'{len(chosen)} fireflies from {len(xyz)} lit pixels -> {OUT.relative_to(ROOT)}')


if __name__ == '__main__':
    main()
