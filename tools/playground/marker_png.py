#!/usr/bin/env python3
"""Draws the two marker pictures that ship as files: the scaffold's template/globe/marker.png and
Prototypes/hello-pack/marker.png. Rerun to regenerate them; never edit the PNGs.

    python3 tools/playground/marker_png.py

Standard library only (a PNG written by hand). 88 px for a 44 pt marker, ink on transparent.
"""
import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
INK = (0x3B, 0x2C, 0x16)
SIZE = 88
SAMPLES = 4  # per axis, for smooth edges


def png(width, height, rgba_rows):
    def chunk(kind, data):
        body = kind + data
        return struct.pack('>I', len(data)) + body + struct.pack('>I', zlib.crc32(body) & 0xFFFFFFFF)
    raw = b''.join(b'\x00' + bytes(row) for row in rgba_rows)
    return (b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', width, height, 8, 6, 0, 0, 0))
            + chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b''))


def draw(shape):
    """shape(dx, dy) -> True where ink goes, in pixels from the centre."""
    c = SIZE / 2
    rows = []
    for y in range(SIZE):
        row = []
        for x in range(SIZE):
            hits = sum(shape(x + (i + .5) / SAMPLES - c, y + (j + .5) / SAMPLES - c)
                       for i in range(SAMPLES) for j in range(SAMPLES))
            row += [*INK, round(255 * hits / SAMPLES ** 2)]
        rows.append(row)
    return png(SIZE, SIZE, rows)


def ring(r_out, r_in):
    return lambda dx, dy: r_in <= (dx * dx + dy * dy) ** .5 <= r_out


def template_marker():
    """A ring around a dot."""
    outer, dot = ring(38, 32), ring(9, 0)
    return draw(lambda dx, dy: outer(dx, dy) or dot(dx, dy))


def ripple_marker():
    """Two rings, like a ripple or a shell's edge."""
    outer, inner = ring(38, 31), ring(20, 14)
    return draw(lambda dx, dy: outer(dx, dy) or inner(dx, dy))


if __name__ == '__main__':
    for path, data in ((ROOT / 'tools/playground/template/globe/marker.png', template_marker()),
                       (ROOT / 'Prototypes/hello-pack/marker.png', ripple_marker())):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)
        print(f'{path.relative_to(ROOT)}  {len(data)} bytes', file=sys.stderr)
