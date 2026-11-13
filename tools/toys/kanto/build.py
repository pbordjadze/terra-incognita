#!/usr/bin/env python3
"""Builds Kanto's plate: the Kanto plain as Tero et al. (2010) laid it out for the slime mold.

    python3 tools/toys/kanto/build.py <cache-dir>    # writes toys/kanto/data/plate.png, plate.json

The plate is the region between WEST..EAST and SOUTH..NORTH on a plain grid (W x H cells, a cell about
220 m; x east, y south, the latitude's squeeze kept: a cell is as wide on the ground as it is tall).

plate.png, RGB:

  R  the light: Tero et al. lit what the mold should not cross as the region's own geography does, the
     mountains (Physarum shies from light). Here the ground's height lights it, 0 below LOW_M rising to
     255 at HIGH_M (AWS Terrain Tiles, Terrarium encoding, zoom 9: SRTM and others)
  G  the agar: 255 on land (Natural Earth 10 m land), 0 at sea (the sea is off the plate)
  B  0

plate.json: the plate's bounds and grid; the real railways as polylines in cells, for the comparison at
the end (Natural Earth 10 m railroads: the main lines, JR's trunk network about Tokyo among them), on
the unlit plain only; and the oat flakes, as Tero laid them at the region's towns: GeoNames' cities (JP.zip; PPLC,
PPLA, PPLA2), the most populous first, kept when none kept lies within SPACING_KM; TOWNS of them, Tokyo
the first (where the mold starts). Each: name, lon, lat, population, and its cell (x, y).

Sources are fetched into the cache when missing. Never hand-edit the output; rebuild it. Needs numpy,
Pillow and pyshp.
"""
import csv
import io
import itertools
import json
import math
import sys
import urllib.request
import zipfile
from pathlib import Path

import numpy as np
import shapefile
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'toys' / 'kanto' / 'data'
WEST, EAST, SOUTH, NORTH = 138.95, 140.95, 34.15, 37.45
MID = math.radians((SOUTH + NORTH) / 2)
W = 768
H = round(W * (NORTH - SOUTH) / ((EAST - WEST) * math.cos(MID)))
TOWNS = 36
# the towns are the Kanto plain's (Tero et al.'s region); the plate runs on north and south to fill a phone
TOWN_SOUTH, TOWN_NORTH = 34.85, 37.0
SPACING_KM = 14.0
LOW_M, HIGH_M = 150.0, 700.0
NE = 'https://naciscdn.org/naturalearth/10m/'
GEONAMES = 'https://download.geonames.org/export/dump/JP.zip'
TILES = 'https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{z}/{x}/{y}.png'


def fetch(url, path):
    if not path.exists():
        print('fetching', url, flush=True)
        path.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(url, path)
    return path


def to_cell(lon, lat):
    return ((lon - WEST) / (EAST - WEST) * W, (NORTH - lat) / (NORTH - SOUTH) * H)


def shapes(cache, name):
    z = fetch(NE + name + '.zip', cache / (name.split('/')[-1] + '.zip'))
    with zipfile.ZipFile(z) as f:
        stem = name.split('/')[-1]
        r = shapefile.Reader(shp=io.BytesIO(f.read(stem + '.shp')), shx=io.BytesIO(f.read(stem + '.shx')),
                             dbf=io.BytesIO(f.read(stem + '.dbf')))
        for sh in r.iterShapes():
            if sh is None:
                continue
            parts = [*sh.parts, len(sh.points)]
            for a, b in itertools.pairwise(parts):
                yield sh.points[a:b]


def near(ring, pad=1.0):
    return any(WEST - pad < x < EAST + pad and SOUTH - pad < y < NORTH + pad for x, y in ring)


def land(cache):
    im = Image.new('L', (W * 4, H * 4), 0)
    d = ImageDraw.Draw(im)
    for ring in shapes(cache, 'physical/ne_10m_land'):
        if near(ring):
            d.polygon([(x * 4, y * 4) for x, y in (to_cell(*p) for p in ring)], fill=255)
    for ring in shapes(cache, 'physical/ne_10m_lakes'):
        if near(ring):
            d.polygon([(x * 4, y * 4) for x, y in (to_cell(*p) for p in ring)], fill=0)
    return np.asarray(im.resize((W, H), Image.Resampling.BOX), np.float32)


def rails(cache, agar, light, towns):
    """The railways' lines in cells, cut where they leave the plate's dark plain (the lit mountains were
    never the mold's to cross) or the towns' own box, points under 2 cells apart merged."""
    xs = [t['cell'][0] for t in towns]
    ys = [t['cell'][1] for t in towns]
    box = (min(xs) - 8, max(xs) + 8, min(ys) - 8, max(ys) + 8)
    out = []
    for line in shapes(cache, 'cultural/ne_10m_railroads'):
        if not near(line, 0.2):
            continue
        run = []
        for x, y in (to_cell(*p) for p in line):
            on = 0 <= x < W and 0 <= y < H and agar[int(y), int(x)] > 64 and light[int(y), int(x)] < 45
            on = on and box[0] < x < box[1] and box[2] < y < box[3]
            if on and (not run or math.hypot(x - run[-1][0], y - run[-1][1]) >= 2):
                run.append([round(x, 1), round(y, 1)])
            elif not on:
                if len(run) > 1:
                    out.append(run)
                run = []
        if len(run) > 1:
            out.append(run)
    return out


def tile_xy(lon, lat, z):
    n = 2 ** z
    y = (1 - math.asinh(math.tan(math.radians(lat))) / math.pi) / 2 * n
    return (lon + 180) / 360 * n, y


def height(cache, z=9):
    """The ground's height (m) at each cell, from the Terrarium tiles, bilinear."""
    x0, y0 = tile_xy(WEST, NORTH, z)
    x1, y1 = tile_xy(EAST, SOUTH, z)
    tx = range(int(x0), int(x1) + 1)
    ty = range(int(y0), int(y1) + 1)
    sheet = np.zeros((len(ty) * 256, len(tx) * 256), np.float32)
    for j, y in enumerate(ty):
        for i, x in enumerate(tx):
            p = fetch(TILES.format(z=z, x=x, y=y), cache / 'tiles' / f'{z}-{x}-{y}.png')
            a = np.asarray(Image.open(p).convert('RGB'), np.float32)
            sheet[j * 256:(j + 1) * 256, i * 256:(i + 1) * 256] = a[..., 0] * 256 + a[..., 1] + a[..., 2] / 256 - 32768
    lon = WEST + (np.arange(W) + 0.5) / W * (EAST - WEST)
    lat = NORTH - (np.arange(H) + 0.5) / H * (NORTH - SOUTH)
    gx = (np.array([tile_xy(v, 0, z)[0] for v in lon]) - tx[0]) * 256 - 0.5
    gy = (np.array([tile_xy(0, v, z)[1] for v in lat]) - ty[0]) * 256 - 0.5
    X, Y = np.meshgrid(gx, gy)
    i0 = np.clip(X.astype(int), 0, sheet.shape[1] - 2)
    j0 = np.clip(Y.astype(int), 0, sheet.shape[0] - 2)
    fx, fy = X - i0, Y - j0
    s = sheet
    return ((s[j0, i0] * (1 - fx) + s[j0, i0 + 1] * fx) * (1 - fy)
            + (s[j0 + 1, i0] * (1 - fx) + s[j0 + 1, i0 + 1] * fx) * fy)


def towns(cache, agar):
    z = fetch(GEONAMES, cache / 'JP.zip')
    with zipfile.ZipFile(z) as f:
        rows = list(csv.reader(io.TextIOWrapper(f.open('JP.txt'), encoding='utf8'), delimiter='\t'))
    found = []
    for r in rows:
        if r[6] != 'P' or r[7] not in ('PPLC', 'PPLA', 'PPLA2') or not r[14]:
            continue
        lat, lon = float(r[4]), float(r[5])
        x, y = to_cell(lon, lat)
        if TOWN_SOUTH < lat < TOWN_NORTH and 4 <= x < W - 4 and 4 <= y < H - 4 and agar[int(y), int(x)] > 128:
            found.append((int(r[14]), r[2] or r[1], lon, lat, x, y))
    found.sort(reverse=True)
    km = (EAST - WEST) * 111.32 * math.cos(MID) / W
    kept = []
    for pop, name, lon, lat, x, y in found:
        if all(math.hypot(x - k['cell'][0], y - k['cell'][1]) * km > SPACING_KM for k in kept):
            kept.append({'name': name, 'lon': round(lon, 4), 'lat': round(lat, 4), 'population': pop,
                         'cell': [round(x, 1), round(y, 1)]})
        if len(kept) == TOWNS:
            break
    return kept


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    cache = Path(sys.argv[1])
    agar = land(cache)
    h = height(cache)
    light = np.clip((h - LOW_M) / (HIGH_M - LOW_M), 0, 1) * 255
    OUT.mkdir(parents=True, exist_ok=True)
    rgb = np.stack([light * (agar > 64), agar, np.zeros_like(agar)], -1).round().clip(0, 255).astype(np.uint8)
    Image.fromarray(rgb, 'RGB').save(OUT / 'plate.png', optimize=True)
    t = towns(cache, agar)
    (OUT / 'plate.json').write_text(json.dumps({
        '_': 'The plate (cells), the railways (Natural Earth) and the oat flakes: the region\'s towns '
             f'(GeoNames), the most populous first, none within {SPACING_KM:g} km of another; Tokyo first. '
             'Built by tools/toys/kanto/build.py; never hand-edit.',
        'plate': {'west': WEST, 'east': EAST, 'south': SOUTH, 'north': NORTH, 'w': W, 'h': H},
        'towns': t, 'rails': rails(cache, agar, light, t)}, ensure_ascii=False, separators=(',', ':')) + '\n')
    print(f'plate {W} x {H}, {len(t)} towns:', ', '.join(x['name'] for x in t))


if __name__ == '__main__':
    main()
