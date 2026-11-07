#!/usr/bin/env python3
"""Builds Meander's floodplain: the Rio Beni's meander belt in the Bolivian Amazon lowlands as Sentinel-2
saw it on 15 August 2024, with today's river taken out and its old loops (oxbow lakes, scroll scars) kept.

    pip install rasterio numpy pillow scipy
    python3 tools/toys/meander/build.py [<cache dir>]   # writes toys/meander/data/plain.jpg, plain.json

Source: Copernicus Sentinel-2 L2A true-colour (TCI, 10 m), tiles 19LFF and 19LGF of 2024-08-15 (cloud
cover under 0.01 %), read as cloud-optimised GeoTIFFs from the AWS open-data bucket sentinel-cogs
(Element 84's earth-search STAC). Licence: Copernicus open data; credit "Contains modified Copernicus
Sentinel data 2024".

The sheet: a square of SIDE metres centred on the belt (UTM 19S, CENTRE), turned by TURN degrees so the
belt runs straight up the screen (the river flows north, toward Riberalta: up), PX pixels a side.

Today's river is taken out, so the river the reader lays is the only one: the channel and its bare sand
bars (bright, warm, unvegetated pixels joined to the long channel) are filled from the forest around
them (a normalised blur, outside in), with a little of the forest's own grain put back. What stays is
measured: the oxbow lakes, the scroll scars, the forest and the pastures beyond the belt.

plain.jpg   RGB, sRGB, row 0 the top (downstream)
plain.json  {"side_m", "px", "centre_utm", "turn_deg", "date", "tiles", "lonlat_centre"}

Never hand-edit the output; rebuild it.
"""
import json
import os
import sys
from pathlib import Path

import numpy as np
import rasterio
from PIL import Image
from rasterio.enums import Resampling
from rasterio.windows import from_bounds
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / "toys" / "meander" / "data"
BASE = "https://sentinel-cogs.s3.us-west-2.amazonaws.com/sentinel-s2-l2a-cogs/19/L"
TILES = ["GF/2024/8/S2B_19LGF_20240815_0_L2A", "FF/2024/8/S2B_19LFF_20240815_0_L2A"]
CENTRE = (711800.0, 8566130.0)  # UTM 19S metres: the belt's middle, about 67.03 W, 12.97 S
TURN = 29.0                       # degrees, anticlockwise: the belt's run from straight up
SIDE = 22000.0                    # metres a side of the sheet
PX = 2048
READ_RES = 10.0


def read(cache):
    half = SIDE * 0.75  # enough to turn without corners
    x0, y0, x1, y1 = CENTRE[0] - half, CENTRE[1] - half, CENTRE[0] + half, CENTRE[1] + half
    n = int(2 * half / READ_RES)
    f = Path(cache) / f"beni_{int(CENTRE[0])}_{int(CENTRE[1])}_{n}.npy"
    if f.exists():
        return np.load(f)
    acc = np.zeros((n, n, 3), np.uint8)
    for t in TILES:
        with rasterio.open(f"{BASE}/{t}/TCI.tif") as d:
            win = from_bounds(x0, y0, x1, y1, d.transform)
            a = d.read(window=win, out_shape=(3, n, n), boundless=True, fill_value=0, resampling=Resampling.average)
            a = np.moveaxis(a, 0, -1)
            m = (a.sum(-1) > 0) & (acc.sum(-1) == 0)
            acc[m] = a[m]
    os.makedirs(cache, exist_ok=True)
    np.save(f, acc)
    return acc


def turn(img):
    im = Image.fromarray(img).rotate(TURN, resample=Image.BICUBIC)
    n = img.shape[0]
    c = int(SIDE / READ_RES)
    o = (n - c) // 2
    im = im.crop((o, o, o + c, o + c)).resize((PX, PX), Image.LANCZOS)
    return np.asarray(im).astype(np.float32)


def take_out_river(a):
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    lum = 0.3 * r + 0.59 * g + 0.11 * b
    # bare sand and the sediment-laden channel: bright and warm (red over green), unlike forest or lakes
    sand = (lum > 95) & (r > g * 1.02) & (r > b * 1.25)
    lab, n = ndimage.label(ndimage.binary_closing(sand, iterations=2))
    sizes = ndimage.sum(np.ones_like(lum), lab, range(1, n + 1))
    # the channel is one long body; keep components that are large (the river and its joined bars)
    keep = np.zeros(n + 1, bool)
    keep[1:] = sizes > 4000
    river = keep[lab]
    # pastures are also warm and bright: the river is the long thin one, so drop blobs that are fat
    lab2, _n2 = ndimage.label(river)
    out = np.zeros_like(river)
    for i, sl in enumerate(ndimage.find_objects(lab2), 1):
        comp = lab2[sl] == i
        area = comp.sum()
        h, w = comp.shape
        if max(h, w) > PX * 0.25 and area / (h * w) < 0.25:
            out[sl] |= comp
    river = ndimage.binary_dilation(out, iterations=4)
    # fill from outside in: a normalised blur of the ground round the channel (its low tones)
    known = ~river
    low = a.copy()
    for s in (3, 6, 12, 24, 48, 96):
        w = ndimage.gaussian_filter(known.astype(np.float32), s)
        v = np.stack([ndimage.gaussian_filter(np.where(known, low[..., c], 0), s) for c in range(3)], -1) / np.maximum(w, 1e-4)[..., None]
        done = ~known & (w > 0.1)
        low = np.where(done[..., None], v, low)
        known = known | done
    # and real forest's grain on it: each filled pixel's detail copied from the nearest forest beside it
    lum = 0.3 * a[..., 0] + 0.59 * a[..., 1] + 0.11 * a[..., 2]
    forest = ~river & (lum > 40) & (lum < 90) & (a[..., 1] >= a[..., 0])
    hp = a - np.stack([ndimage.gaussian_filter(a[..., c], 6) for c in range(3)], -1)
    grain = np.zeros_like(a)
    need = river.copy()
    for d in (40, 70, 110, 160, 220, 300):
        for dy, dx in ((0, d), (0, -d), (d, 0), (-d, 0), (d, d), (-d, -d), (d, -d), (-d, d)):
            src_ok = np.roll(forest, (dy, dx), axis=(0, 1))
            take = need & src_ok
            grain[take] = np.roll(hp, (dy, dx), axis=(0, 1))[take]
            need &= ~take
    a = np.where(river[..., None], low + grain, a)
    return np.clip(a, 0, 255).astype(np.uint8), river


def main():
    cache = sys.argv[1] if len(sys.argv) > 1 else os.path.expanduser("~/.cache/meander")
    img = turn(read(cache))
    out, river = take_out_river(img)
    OUT.mkdir(parents=True, exist_ok=True)
    Image.fromarray(out).save(OUT / "plain.jpg", quality=88, optimize=True)
    if os.environ.get("MEANDER_DEBUG"):
        Image.fromarray(img.astype(np.uint8)).save(Path(cache) / "plain_raw.png")
        Image.fromarray((river * 255).astype(np.uint8)).save(Path(cache) / "river_mask.png")
    lon, lat = -67.03, -12.97
    json.dump({"side_m": SIDE, "px": PX, "centre_utm": CENTRE, "utm_zone": "19S", "turn_deg": TURN,
               "date": "2024-08-15", "tiles": TILES, "lonlat_centre": [lon, lat]},
              (OUT / "plain.json").open("w"), indent=1)
    print("wrote", OUT / "plain.jpg", "river px", int(river.sum()))


if __name__ == "__main__":
    main()
