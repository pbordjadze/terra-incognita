"""What every Adrift data sheet shares: the grid, the month tiles, the encodings. Imported by the
builders beside it; not run on its own.

Every sheet is equirectangular, longitude across from 180 W at the left edge, latitude down from
90 N at the top, cell centres at half-cells. A sheet of months is a 4 x 3 grid of tiles, tile
(m % 4, m // 4) holding month m (0 = January); each tile is the month's W x H cells with a one-cell
border repeated from the opposite side east-west and from the edge north-south, so bilinear
sampling wraps and clamps by itself. The app reads `adrift/data/manifest.json`, which every
builder writes its sheet's entry into: size, tile, the encoding's constants, and the credit owed.
"""
import json
from pathlib import Path
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'adrift' / 'data'
MANIFEST = OUT / 'manifest.json'


def tile(a):
    """A month's cells (rows north to south) with the one-cell border round them."""
    a = np.concatenate([a[:, -1:], a, a[:, :1]], axis=1)
    return np.concatenate([a[:1], a, a[-1:]], axis=0)


def months(tiles):
    """Twelve tiles (each already bordered) laid 4 x 3."""
    th, tw = tiles[0].shape[:2]
    sheet = np.zeros((th * 3, tw * 4) + tiles[0].shape[2:], tiles[0].dtype)
    for m, t in enumerate(tiles):
        x, y = (m % 4) * tw, (m // 4) * th
        sheet[y:y + th, x:x + tw] = t
    return sheet


def save(name, data, mode, entry):
    """Writes the sheet and its manifest entry; prints the size."""
    OUT.mkdir(parents=True, exist_ok=True)
    path = OUT / name
    Image.fromarray(data, mode).save(path, optimize=True)
    manifest = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else {}
    entry = dict(entry)
    entry['file'] = name
    entry['width'], entry['height'] = int(data.shape[1]), int(data.shape[0])
    manifest[Path(name).stem] = entry
    MANIFEST.write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')
    print(f'{path.relative_to(ROOT)} {path.stat().st_size / 1e6:.2f} MB')


def regrid(a, lat_src, lon_src, w, h):
    """Bilinear from a (lat, lon) grid (any monotone latitudes, longitudes evenly spaced from
    lon_src[0] wrapping) onto the sheet's W x H half-cell centres."""
    lat = 90 - (np.arange(h) + 0.5) * 180 / h
    lon = -180 + (np.arange(w) + 0.5) * 360 / w
    # latitude: interpolate along axis 0 (clamping past the ends)
    order = np.argsort(lat_src)
    ls, a = lat_src[order], a[order]
    fy = np.interp(lat, ls, np.arange(len(ls)))
    y0 = np.clip(np.floor(fy).astype(int), 0, len(ls) - 2)
    ty = (fy - y0)[:, None]
    a = a[y0] * (1 - ty) + a[y0 + 1] * ty
    # longitude: evenly spaced, wrapping
    dl = 360 / len(lon_src)
    fx = ((lon - lon_src[0]) % 360) / dl
    x0 = np.floor(fx).astype(int) % len(lon_src)
    tx = (fx - np.floor(fx))[None, :]
    return a[:, x0] * (1 - tx) + a[:, (x0 + 1) % len(lon_src)] * tx
