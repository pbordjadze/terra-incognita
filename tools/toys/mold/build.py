#!/usr/bin/env python3
"""Builds Mold's ground: where the mold may go and what it eats, on its own grid.

    python3 tools/toys/mold/build.py            # writes toys/mold/data/ground.png

ground.png is an equirectangular 4096 x 2048 sheet (a cell about 10 km at the equator) (west edge at 180 W, north edge at 90 N, as the
globe's maps), RGB:

  R  the food: oat flakes laid where the Earth's lights at night are (NASA's Black Marble, the same
     sheet as the globe's Globe/ground/night.jpg), as Tero et al. laid them at Tokyo's towns: each
     lit place's brightest point, the brightest first, none within SPACING km of a brighter one
     (a greedy Poisson-disc on the sphere), a small heap each, larger and richer the brighter the
     light; on land only (lights at sea, fishing fleets and flares, are left out: the mold cannot
     go there)
  G  the land the mold may cross: 255 on land and within about 40 km of
     a coast, so narrow straits (Dover, the Bosporus, Messina) are crossed and the open sea is not;
     from the coast's signed distance in Globe/geo.png (tools/globe/build.py, Natural Earth)
  B  0

Never hand-edit the output; rebuild it.
"""
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
W, H = 4096, 2048
SPACING = 55.0          # km between flakes
FLOOR = 40              # the dimmest light (of 255) that gets a flake


def flakes(lights):
    """The flakes, as a food sheet 0..255: each local peak of the lights (the brightest in its 5 x 5
    cells, at least FLOOR), the brightest first, kept when no flake already lies within SPACING km."""
    pad = np.pad(lights, 2, mode='edge')
    peak = np.ones_like(lights, bool)
    for dy in range(5):
        for dx in range(5):
            if (dy, dx) != (2, 2):
                peak &= lights >= pad[dy:dy + H, dx:dx + W]
    ys, xs = np.nonzero(peak & (lights >= FLOOR))
    order = np.argsort(-lights[ys, xs], kind='stable')
    km = np.pi * 6371.0 / H                           # a cell's height
    r = SPACING / km
    taken = np.zeros((H, W), bool)
    food = np.zeros((H, W), np.float32)
    yy, xx = np.mgrid[-8:9, -8:9]
    kept = 0
    for i in order:
        y, x = ys[i], xs[i]
        if taken[y, x]:
            continue
        kept += 1
        c = max(np.cos((0.5 - (y + 0.5) / H) * np.pi), 0.05)
        rx = int(np.ceil(r / c))
        ry = int(np.ceil(r))
        y0, y1 = max(y - ry, 0), min(y + ry + 1, H)
        cols = np.arange(x - rx, x + rx + 1) % W
        dy = (np.arange(y0, y1) - y)[:, None]
        dx = (np.arange(-rx, rx + 1) * c)[None, :]
        taken[y0:y1][:, cols] |= dy * dy + dx * dx <= r * r
        # the heap: brighter lights a richer and broader flake
        a = (lights[y, x] / 255.0) ** 0.6
        sig = 0.9 + 1.4 * a
        g = 255.0 * a * np.exp(-((yy) ** 2 + (xx * c) ** 2) / (2 * sig * sig))
        gy, gx = np.clip(y + yy, 0, H - 1), (x + xx) % W
        np.maximum.at(food, (gy, gx), g)
    print(f'{kept} flakes from {len(order)} peaks')
    return food


def main():
    geo = np.asarray(Image.open(ROOT / 'Globe/geo.png')).astype(np.float32)
    fine = (geo[..., 0] - 128.0) / 8.0
    coarse = (geo[..., 1] - 128.0) * 2.0
    d = np.where(np.abs(coarse) > 14.0, coarse, fine)          # geo pixels, land positive
    _gh, gw = d.shape
    k = gw // W
    land_geo = d > -4.0 * (gw / 4096)                            # about 40 km of slack
    land = land_geo.reshape(H, k, W, k).any(axis=(1, 3))
    night = np.asarray(Image.open(ROOT / 'Globe/ground/night.jpg').convert('L')).astype(np.float32)
    kn = night.shape[1] // W
    lights = night.reshape(H, kn, W, kn).mean(axis=(1, 3))
    lights = np.where(land, lights, 0.0)
    food = np.where(land, flakes(lights), 0.0)
    out = np.zeros((H, W, 3), np.uint8)
    out[..., 0] = np.clip(food + 0.5, 0, 255).astype(np.uint8)
    out[..., 1] = np.where(land, 255, 0).astype(np.uint8)
    dst = ROOT / 'toys/mold/data/ground.png'
    dst.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(out, 'RGB').save(dst, optimize=True)
    print(f'{dst}: land {land.mean():.3f} of cells, fed (>8) {(food > 8).mean():.4f}, {dst.stat().st_size / 1e6:.1f} MB')


if __name__ == '__main__':
    main()
