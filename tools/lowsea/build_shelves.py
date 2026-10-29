#!/usr/bin/env python3
"""The continental shelves as dry land (Prototypes/low-sea/shelf-<name>.png): eight pictures in longitude
and latitude for `terra.images`, each a region's sea floor shallower than 200 m painted as land, so
that when the sea goes out the globe shows where the shore lay when the ice was greatest and the sea
about 120 m lower (the 200 m contour is the nearest Natural Earth draws; the ice age's shore was a
little inside it). Built from Natural Earth 1:10m land and bathymetry (the K_200 layer: water deeper
than 200 m), public domain. A faint grain and a paler rim where the old shore met the sea.

    python3 tools/lowsea/build_shelves.py <folder holding ne_10m_land/ and ne_10m_bathymetry_K_200/>

Rebuild, never hand-edit. Prints each picture's bounds for data.js.
"""
import sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFilter
import shapefile

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Prototypes' / 'low-sea'
PX = 0.02                       # degrees a pixel
# the regions: name, bounds (west, south, east, north)
REGIONS = [
    ('doggerland', (-12.0, 47.0, 12.0, 62.0)),
    ('sunda', (95.0, -12.0, 122.0, 14.0)),
    ('sahul', (112.0, -24.0, 152.0, 0.0)),
    ('beringia-west', (155.0, 50.0, 180.0, 72.0)),
    ('beringia-east', (-180.0, 50.0, -150.0, 72.0)),
    ('gulf', (44.0, 21.0, 60.0, 31.0)),
    ('adriatic-black', (11.0, 36.0, 42.0, 48.0)),
    ('patagonia', (-72.0, -56.0, -52.0, -36.0)),
]
SAND = np.array([206, 198, 156], dtype=np.float32)   # a dry steppe, paler than the sea floor would be: the shelves were grassland and tundra
SAND_WET = np.array([184, 176, 142], dtype=np.float32)

def rasterise(sf, bounds, size):
    west, south, east, north = bounds
    w, h = size
    img = Image.new('L', (w, h), 0)
    d = ImageDraw.Draw(img)
    def px(x, y): return ((x - west) / PX, (north - y) / PX)
    for shape in sf.shapes():
        bb = shape.bbox
        if bb[2] < west or bb[0] > east or bb[3] < south or bb[1] > north:
            continue
        parts = list(shape.parts) + [len(shape.points)]
        for a, b in zip(parts, parts[1:]):
            ring = [px(x, y) for x, y in shape.points[a:b]]
            if len(ring) > 2:
                d.polygon(ring, fill=255)
    return np.asarray(img, dtype=np.float32) / 255

def main(ne):
    land_sf = shapefile.Reader(str(Path(ne) / 'ne_10m_land' / 'ne_10m_land.shp'))
    deep_sf = shapefile.Reader(str(Path(ne) / 'ne_10m_bathymetry_K_200' / 'ne_10m_bathymetry_K_200.shp'))
    for name, bounds in REGIONS:
        west, south, east, north = bounds
        size = (int(round((east - west) / PX)), int(round((north - south) / PX)))
        land = rasterise(land_sf, bounds, size)
        deep = rasterise(deep_sf, bounds, size)
        # (the bathymetry layer's polygons have holes drawn as separate rings here, so a ring inside a
        # ring is filled twice: take the parity by drawing even-odd would be right, but the layer's
        # holes are rare and small; a shelf is water that is not deep)
        shelf = np.clip((1 - land) * (1 - deep), 0, 1)
        if shelf.mean() < 0.002:
            print(f'{name}: no shelf found; check the layer')
        # the rim: the shelf's own edge toward the deep, a little paler, and the shore's edge, wetter
        soft = np.asarray(Image.fromarray((shelf * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(6)), dtype=np.float32) / 255
        wet = np.asarray(Image.fromarray((land * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(10)), dtype=np.float32) / 255
        yy, xx = np.mgrid[0:size[1], 0:size[0]].astype(np.float32)
        grain = (np.sin(xx * 0.37 + np.sin(yy * 0.11) * 3) * np.sin(yy * 0.29 + xx * 0.05) * 0.5 + 0.5) * 0.025
        tone = SAND[None, None, :] * (1 - wet[..., None] * 0.5) + SAND_WET[None, None, :] * (wet[..., None] * 0.5)
        rgb = tone * (1 - grain[..., None]) + grain[..., None] * 255 * 0.3
        alpha = shelf * np.clip(soft * 1.6, 0, 1)
        rgb *= (alpha > 0.01)[..., None]
        out = np.dstack([np.clip(rgb, 0, 255), np.clip(alpha, 0, 1) * 255]).astype(np.uint8)
        f = OUT / f'shelf-{name}.png'
        Image.fromarray(out, 'RGBA').save(f, optimize=True)
        print(f"{name}: {size[0]}x{size[1]}, {f.stat().st_size // 1024} KB, shelf {shelf.mean() * 100:.1f}% of the picture, bounds {list(bounds)}")

if __name__ == '__main__':
    main(sys.argv[1])
