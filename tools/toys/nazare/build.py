#!/usr/bin/env python3
"""Builds Nazaré's sea floor: the canyon and the shelf off Praia do Norte, Portugal.

    python3 tools/toys/nazare/build.py <cache-dir>    # writes toys/nazare/data/floor.bin, floor.json

The floor is the region between WEST..EAST and SOUTH..NORTH on a plain grid (W x H cells, CELL_M metres
a side; x east, y south), the height of the ground in metres (negative under the sea) from EMODnet
Bathymetry's mean depth (its Digital Terrain Model, 1/16 arc-minute, about 115 m; the land's heights
come with it), its lone spikes taken out, sampled bilinearly.

floor.bin: W x H little-endian int16, the ground's height in decimetres, row by row from the north.
floor.json: the bounds, W, H, CELL_M, and the places named on it (Nazaré's headland, Praia do Norte).

The source is fetched into the cache when missing. Never hand-edit the output; rebuild it. Needs numpy and
Pillow.
"""
import json
import math
import sys
import urllib.request
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'toys/nazare/data'
WEST, EAST, SOUTH, NORTH = -9.30, -9.03, 39.40, 39.82
CELL_M = 50.0
MID = math.radians((SOUTH + NORTH) / 2)
M_PER_DEG = 111_195.0
W = round((EAST - WEST) * M_PER_DEG * math.cos(MID) / CELL_M)
H = round((NORTH - SOUTH) * M_PER_DEG / CELL_M)
# the source's own grid: 1/16 arc-minute, a little past the floor's bounds
STEP = 1 / 960
PAD = 0.02
WCS = ('https://ows.emodnet-bathymetry.eu/wcs?service=WCS&version=1.0.0&request=GetCoverage'
       '&coverage=emodnet:mean&crs=EPSG:4326&format=image/tiff&interpolation=nearest'
       '&BBOX={w},{s},{e},{n}&resx={r}&resy={r}')
PLACES = {
    'headland': [-9.0850, 39.6040],  # Nazaré's lighthouse, the Forte de São Miguel Arcanjo
    'praia_do_norte': [-9.0845, 39.6105],
    'town': [-9.0700, 39.6010],
}


def fetch(cache):
    path = Path(cache) / 'emodnet-nazare.tif'
    if not path.exists():
        url = WCS.format(w=WEST - PAD, s=SOUTH - PAD, e=EAST + PAD, n=NORTH + PAD, r=STEP)
        path.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(url, timeout=120) as r:
            path.write_bytes(r.read())
    return np.array(Image.open(path), dtype=np.float64)


def sample(src):
    """The source (north row first, its cells' centres from WEST - PAD + STEP/2) at each floor cell's centre."""
    rows, cols = src.shape
    west, north = WEST - PAD, NORTH + PAD
    lon = WEST + (np.arange(W) + 0.5) / W * (EAST - WEST)
    lat = NORTH - (np.arange(H) + 0.5) / H * (NORTH - SOUTH)
    fx = np.clip((lon - west) / STEP - 0.5, 0, cols - 1.001)
    fy = np.clip((north - lat) / STEP - 0.5, 0, rows - 1.001)
    x0, y0 = fx.astype(int), fy.astype(int)
    ax, ay = fx - x0, fy - y0
    a = src[np.ix_(y0, x0)] * (1 - ax)[None, :] + src[np.ix_(y0, x0 + 1)] * ax[None, :]
    b = src[np.ix_(y0 + 1, x0)] * (1 - ax)[None, :] + src[np.ix_(y0 + 1, x0 + 1)] * ax[None, :]
    return a * (1 - ay)[:, None] + b * ay[:, None]


def despike(src):
    """The source's lone soundings out: a 3 x 3 median where a cell stands more than 40 m off it (the
    survey's holes and spikes; the canyon's walls, many cells long, stay)."""
    win = np.lib.stride_tricks.sliding_window_view(np.pad(src, 1, mode='edge'), (3, 3))
    med = np.median(win, axis=(2, 3))
    return np.where(np.abs(src - med) > 40, med, src)


def cell(lon, lat):
    return [round((lon - WEST) / (EAST - WEST) * W, 1), round((NORTH - lat) / (NORTH - SOUTH) * H, 1)]


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    src = fetch(sys.argv[1])
    src = np.where(np.isfinite(src) & (np.abs(src) < 1e4), src, 0.0)
    floor = sample(despike(src))
    OUT.mkdir(parents=True, exist_ok=True)
    dm = np.clip(np.round(floor * 10), -32768, 32767).astype('<i2')
    (OUT / 'floor.bin').write_bytes(dm.tobytes())
    (OUT / 'floor.json').write_text(json.dumps({
        '_': "Nazaré's floor (tools/toys/nazare/build.py): floor.bin is W x H int16 decimetres, north row first.",
        'west': WEST, 'east': EAST, 'south': SOUTH, 'north': NORTH, 'w': W, 'h': H, 'cell_m': CELL_M,
        'places': {k: {'lonlat': v, 'cell': cell(*v)} for k, v in PLACES.items()},
    }, indent=1) + '\n')
    sea = floor < 0
    print(f'floor {W} x {H} at {CELL_M:.0f} m; sea {sea.mean():.0%}, deepest {floor.min():.0f} m')


if __name__ == '__main__':
    main()
