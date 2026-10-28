#!/usr/bin/env python3
"""Draws the glint that a pack's doorway shows from afar (docs/runtime3.md › Many at once): a small
warm light with four fine rays, on nothing, the same mark for every experience (the app draws the
natives' in Swift, `Catalog.swift`'s Glint, from the same recipe). Rerun to regenerate them; never
edit the PNGs.

    python3 tools/playground/glint_png.py [Prototypes/<id>/doorway-glint.png ...]

With no paths, writes doorway-glint.png into every bundled pack that has a doorway. Standard library
only (a PNG written by hand): 44 px for an 11 pt mark.
"""
import json
import math
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SIZE = 44
GOLD = (176, 138, 62)       # the atlas's gold (the ring's B08A3E), deep enough to read on the pale land
LIGHT = (232, 208, 140)     # the same gold lifted, for the glow round it
WHITE = (255, 255, 255)
SAMPLES = 3


def png(width, height, rgba_rows):
    def chunk(kind, data):
        body = kind + data
        return struct.pack('>I', len(data)) + body + struct.pack('>I', zlib.crc32(body) & 0xFFFFFFFF)
    raw = b''.join(b'\x00' + bytes(row) for row in rgba_rows)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def glint():
    """The core: a radial fall-off from the gold (opaque at the centre) through its lifted glow (0.55 a
    third of the way out) to nothing at 0.34 of the side. The rays: white, round-capped, two long (0.46
    of the side each way, 1.8 px) and two short diagonals (0.22, 1.2 px), as Catalog.swift's Glint
    draws them."""
    c = SIZE / 2
    rays = [((1, 0), 0.46, 1.8), ((0, 1), 0.46, 1.8), ((0.7071, 0.7071), 0.22, 1.2), ((-0.7071, 0.7071), 0.22, 1.2)]

    def core(dx, dy):
        """Alpha and colour of the core at a point: the gold at the centre, its lifted glow outward."""
        r = math.hypot(dx, dy) / (SIZE * 0.34)
        if r >= 1:
            return 0.0, GOLD
        if r < 0.35:
            k = r / 0.35
            return 1.0 + (0.55 - 1.0) * k, tuple(round(GOLD[i] + (LIGHT[i] - GOLD[i]) * k) for i in range(3))
        return 0.55 * (1 - (r - 0.35) / 0.65), LIGHT

    def ray(dx, dy):
        a = 0.0
        for (ux, uy), reach, width in rays:
            along = dx * ux + dy * uy
            across = abs(-dx * uy + dy * ux)
            half = width / 2
            if abs(along) <= reach * SIZE and across <= half:
                a = max(a, 0.92)
            elif abs(along) <= reach * SIZE + half and math.hypot(max(0, abs(along) - reach * SIZE), across) <= half:
                a = max(a, 0.92)
        return a

    rows = []
    for y in range(SIZE):
        row = []
        for x in range(SIZE):
            cr = gr = br = ar = 0.0
            for i in range(SAMPLES):
                for j in range(SAMPLES):
                    dx = x + (i + .5) / SAMPLES - c
                    dy = y + (j + .5) / SAMPLES - c
                    (ca, colour), ra = core(dx, dy), ray(dx, dy)
                    # the ray over the core (premultiplied, then unmultiplied below)
                    a = ra + ca * (1 - ra)
                    if a <= 0:
                        continue
                    r = (WHITE[0] * ra + colour[0] * ca * (1 - ra)) / a
                    g = (WHITE[1] * ra + colour[1] * ca * (1 - ra)) / a
                    b = (WHITE[2] * ra + colour[2] * ca * (1 - ra)) / a
                    cr += r * a; gr += g * a; br += b * a; ar += a
            n = SAMPLES * SAMPLES
            if ar <= 0:
                row += [0, 0, 0, 0]
            else:
                row += [round(cr / ar), round(gr / ar), round(br / ar), round(255 * ar / n)]
        rows.append(row)
    return png(SIZE, SIZE, rows)


def targets():
    out = []
    for folder in sorted((ROOT / 'Prototypes').iterdir()):
        manifest = folder / 'prototype.json'
        if not manifest.is_file():
            continue
        try:
            m = json.loads(manifest.read_text(encoding='utf-8'))
        except ValueError:
            continue
        if m.get('doorway') and m.get('shelf', 'current') != 'parked':
            out.append(folder / 'doorway-glint.png')
    return out


if __name__ == '__main__':
    paths = [Path(p) for p in sys.argv[1:]] or targets()
    data = glint()
    for path in paths:
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        try:
            shown = path.relative_to(ROOT)
        except ValueError:
            shown = path
        print(f'{shown}  {len(data)} bytes', file=sys.stderr)
