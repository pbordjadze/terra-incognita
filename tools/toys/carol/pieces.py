#!/usr/bin/env python3
"""Draws Carol's pieces (toys/carol/data/pieces.png) from DejaVu Sans's chess glyphs (Bitstream Vera
licence, free to embed): one row of six cells, king queen rook bishop knight pawn, 160 px a cell.
R is the piece's silhouette (the outline glyph filled from outside in), G its rim (the silhouette
less itself worn in by four pixels), B its shadow (the silhouette blurred); the toy's shader colours
them for either side: chunky one-colour pieces, as a wooden set. Never hand-edit the PNG.

    python3 tools/toys/carol/pieces.py [<font.ttf>]
"""
import sys
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

ROOT = Path(__file__).resolve().parents[3]
FONT = sys.argv[1] if len(sys.argv) > 1 else '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'
CELL, UP = 160, 4
GLYPHS = '♔♕♖♗♘♙'


def cell(ch):
    n = CELL * UP
    font = ImageFont.truetype(FONT, int(n * 0.86))
    img = Image.new('L', (n, n), 0)
    d = ImageDraw.Draw(img)
    box = d.textbbox((0, 0), ch, font=font)
    w, h = box[2] - box[0], box[3] - box[1]
    d.text(((n - w) / 2 - box[0], (n - h) / 2 - box[1] + n * 0.01), ch, font=font, fill=255)
    lines = img
    # silhouette: everything the outside's flood can't reach
    b = lines.point(lambda v: 255 if v > 100 else 0)
    flood = b.copy()
    ImageDraw.floodfill(flood, (0, 0), 128)
    sil = flood.point(lambda v: 0 if v == 128 else 255)
    sil = sil.filter(ImageFilter.MaxFilter(3))
    return sil.resize((CELL, CELL), Image.LANCZOS), lines.resize((CELL, CELL), Image.LANCZOS)


def main():
    out = Image.new('RGBA', (CELL * 6, CELL), (0, 0, 0, 255))
    for i, ch in enumerate(GLYPHS):
        s, _l = cell(ch)
        rim = ImageChops.subtract(s, s.filter(ImageFilter.MinFilter(9)))
        shadow = s.filter(ImageFilter.GaussianBlur(7))
        out.paste(Image.merge('RGBA', (s, rim, shadow, Image.new('L', (CELL, CELL), 255))), (i * CELL, 0))
    p = ROOT / 'toys/carol/data/pieces.png'
    out.save(p, optimize=True)
    print(p, p.stat().st_size)


if __name__ == '__main__':
    main()
