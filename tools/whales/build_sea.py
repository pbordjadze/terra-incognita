#!/usr/bin/env python3
"""The sea that comes back over Egypt's Western Desert (Prototypes/sea-of-whales/tethys-1.png to -4.png): four pictures, the deep water to the shore,
in longitude and latitude for `terra.images`, bounds (24, 26, 37, 34). It is an illustration, not a
reconstruction: a generalised late Eocene shore (the Tethys reaching across northern Egypt and Sinai,
Wadi Al-Hitan under a shallow bay), drawn only over today's land (Natural Earth 1:10m land, public
domain), so the Mediterranean, the Gulf of Suez and the Red Sea stay the globe's own sea. Deep water
is a plain blue; the water near the drawn shore is paler and greener, as shallows read from above.

    python3 tools/whales/build_sea.py <folder holding ne_10m_land/ne_10m_land.shp>

Rebuild, never hand-edit. SOURCES.md in the pack says what this picture claims and does not.
"""
import sys, math
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
import shapefile

WEST, SOUTH, EAST, NORTH = 24.0, 26.0, 37.0, 34.0
PX = 0.01                      # degrees a pixel
W, H = int((EAST - WEST) / PX), int((NORTH - SOUTH) / PX)
# The drawn shore, west to east (the sea lies north of it), then the picture's north edge.
SHORE = [(24.0, 27.6), (25.2, 27.7), (26.3, 27.9), (27.4, 28.25), (28.3, 28.65), (29.0, 28.85), (29.6, 28.9),
         (30.05, 29.0), (30.3, 29.12), (30.7, 29.02), (31.15, 29.18), (31.7, 29.05), (32.2, 29.25), (32.75, 29.55),
         (33.3, 29.85), (33.9, 29.9), (34.5, 30.1), (35.2, 30.4), (35.9, 30.95), (36.6, 31.3), (37.0, 31.45)]
DEEP = (60, 128, 170)          # a plain sea blue
SHALLOW = (128, 198, 206)      # the shallows, paler and greener
DEEP_ALPHA, SHALLOW_ALPHA = 0.80, 0.66

def px(lon, lat):
    return (lon - WEST) / PX, (NORTH - lat) / PX

def main(ne_dir):
    shp = Path(ne_dir) / 'ne_10m_land' / 'ne_10m_land.shp'
    sf = shapefile.Reader(str(shp))
    land = Image.new('L', (W, H), 0)
    d = ImageDraw.Draw(land)
    for shape in sf.shapes():
        bb = shape.bbox
        if bb[2] < WEST or bb[0] > EAST or bb[3] < SOUTH or bb[1] > NORTH:
            continue
        parts = list(shape.parts) + [len(shape.points)]
        for a, b in zip(parts, parts[1:]):
            ring = [px(x, y) for x, y in shape.points[a:b]]
            if len(ring) > 2:
                d.polygon(ring, fill=255)        # (holes are lakes: none matter here)
    sea = Image.new('L', (W, H), 0)
    ImageDraw.Draw(sea).polygon([px(x, y) for x, y in SHORE] + [px(EAST, NORTH), px(WEST, NORTH)], fill=255)
    land_a = np.asarray(land, dtype=np.float32) / 255
    sea_a = np.asarray(sea, dtype=np.float32) / 255
    flooded = land_a * sea_a
    # how far from the drawn shore (inside the flood): a blurred shore mask, 0 at the shore, 1 well out
    soft = np.asarray(sea.filter(ImageFilter.GaussianBlur(45)), dtype=np.float32) / 255
    depth = np.clip((soft - 0.5) / 0.5, 0, 1) ** 0.8
    # a faint swell: long low waves, so the water is not a flat tint
    yy, xx = np.mgrid[0:H, 0:W].astype(np.float32)
    swell = 0.5 + 0.5 * np.sin(xx * 0.045 + np.sin(yy * 0.02) * 2.0) * np.sin(yy * 0.03 + xx * 0.01)
    swell += 0.5 + 0.5 * np.sin(xx * 0.021 - yy * 0.017 + np.sin(xx * 0.007) * 3.0)
    swell = (swell / 2) ** 4 * 0.07
    rgb = np.zeros((H, W, 3), dtype=np.float32)
    for i in range(3):
        rgb[..., i] = SHALLOW[i] + (DEEP[i] - SHALLOW[i]) * depth + swell * 255 * 0.6
    alpha = (SHALLOW_ALPHA + (DEEP_ALPHA - SHALLOW_ALPHA) * depth) * flooded
    # the shore itself a hair paler: foam where the water meets the sand
    edge = np.clip((0.5 - np.abs(soft - 0.5)) * 10, 0, 1) * flooded
    rgb += edge[..., None] * 40
    # Four pictures, from the deepest water to the shore, so the sea can come in from the deep and
    # go out from the shore: each band's alpha is the whole picture's inside its band of `depth`,
    # feathered into the next, and together they add up to the one sea.
    dest = Path(__file__).resolve().parents[2] / 'Prototypes' / 'sea-of-whales'
    # (banded by how far from the shore, over a longer reach than the colour's shallows, each band
    # a quarter of the flooded area; the bands add up to the one sea everywhere)
    reach = np.asarray(sea.filter(ImageFilter.GaussianBlur(170)), dtype=np.float32) / 255
    inside = reach[flooded > 0.5]
    lows = [float(np.quantile(inside, q)) for q in (0.75, 0.5, 0.25)] + [-1.0]
    # the picture's west, east and north edges are not the sea's: it thins out over the last degree or so
    xx_deg = WEST + xx * PX
    yy_deg = NORTH - yy * PX
    feather = np.clip((xx_deg - WEST) / 3.0, 0, 1) * np.clip((EAST - xx_deg) / 3.0, 0, 1) * np.clip((NORTH - yy_deg) / 1.5, 0, 1)
    alpha *= feather
    rgb *= (alpha > 0.01)[..., None]    # (nothing drawn where nothing shows: smaller files)
    below = np.zeros_like(alpha)
    for k in range(4):
        cum = np.clip((reach - lows[k]) / 0.03, 0, 1)
        band = np.clip(cum - below, 0, 1)
        below = cum
        out = np.dstack([np.clip(rgb, 0, 255), np.clip(alpha * band, 0, 1) * 255]).astype(np.uint8)
        f = dest / f'tethys-{k + 1}.png'
        Image.fromarray(out, 'RGBA').save(f, optimize=True)
        print(f'{f.name}: {W}x{H}, {f.stat().st_size // 1024} KB, {float((alpha * band > 0.02).mean()) * 100:.1f}% of the picture')

if __name__ == '__main__':
    main(sys.argv[1])
