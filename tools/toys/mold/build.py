#!/usr/bin/env python3
"""Builds Mold's ground: where the mold may go and what it eats, on its own grid.

    python3 tools/toys/mold/build.py            # writes toys/mold/data/ground.png

ground.png is an equirectangular 4096 x 2048 sheet (a cell about 10 km at the equator) (west edge at 180 W, north edge at 90 N, as the
globe's maps), RGB:

  R  the food: the Earth's lights at night (NASA's Black Marble, the same sheet as the globe's
     Globe/ground/night.jpg), at its own resolution, on land only
     (lights at sea, fishing fleets and flares, are left out: the mold cannot go there)
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


def main():
    geo = np.asarray(Image.open(ROOT / 'Globe/geo.png')).astype(np.float32)
    fine = (geo[..., 0] - 128.0) / 8.0
    coarse = (geo[..., 1] - 128.0) * 2.0
    d = np.where(np.abs(coarse) > 14.0, coarse, fine)          # geo pixels, land positive
    gh, gw = d.shape
    k = gw // W
    land_geo = d > -4.0 * (gw / 4096)                            # about 40 km of slack
    land = land_geo.reshape(H, k, W, k).any(axis=(1, 3))
    night = np.asarray(Image.open(ROOT / 'Globe/ground/night.jpg').convert('L')).astype(np.float32)
    kn = night.shape[1] // W
    food = night.reshape(H, kn, W, kn).mean(axis=(1, 3))
    food = np.where(land, food, 0.0)
    out = np.zeros((H, W, 3), np.uint8)
    out[..., 0] = np.clip(food + 0.5, 0, 255).astype(np.uint8)
    out[..., 1] = np.where(land, 255, 0).astype(np.uint8)
    dst = ROOT / 'toys/mold/data/ground.png'
    dst.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(out, 'RGB').save(dst, optimize=True)
    print(f'{dst}: land {land.mean():.3f} of cells, lit (>8) {(food > 8).mean():.4f}, {dst.stat().st_size / 1e6:.1f} MB')


if __name__ == '__main__':
    main()
