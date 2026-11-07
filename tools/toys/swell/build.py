#!/usr/bin/env python3
"""Builds Swell's sea (toys/swell/data/depth.bin) from the globe's own depths: never hand-edit it.

    python3 tools/toys/swell/build.py

The globe's equirectangular grid (longitude across from -180, latitude down from 90) at half a degree:
720 x 360 little-endian uint16, the sea's mean depth in metres over each cell, 0 for land (a cell is land
when half or more of it is). Depths from Globe/terrain.png's G (sea: 127 - depth / 87 m, GEBCO through
tools/globe/build.py), land from Globe/geo.png's R (the coast's signed distance, land >= 128).
"""
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
W, H = 720, 360

terrain = np.asarray(Image.open(ROOT / 'Globe/terrain.png'))
geo = np.asarray(Image.open(ROOT / 'Globe/geo.png'))
land = geo[..., 0] >= 128
g = terrain[..., 1].astype(np.float64)
depth = np.where(land, 0.0, np.clip(127.0 - g, 0, None) * 87.0)

def box(a):
    im = Image.fromarray(a.astype(np.float32), mode='F')
    return np.asarray(im.resize((W, H), Image.BOX))

frac = box(land.astype(np.float32))
sea_depth = box(depth) / np.maximum(1.0 - frac, 1e-3)
out = np.where(frac >= 0.5, 0, np.clip(np.round(sea_depth), 1, 11000)).astype('<u2')
dst = ROOT / 'toys/swell/data/depth.bin'
dst.parent.mkdir(parents=True, exist_ok=True)
dst.write_bytes(out.tobytes())
print(f'{dst}: {W}x{H}, sea {np.mean(out > 0):.2f}, deepest {out.max()} m, mean {out[out > 0].mean():.0f} m')
