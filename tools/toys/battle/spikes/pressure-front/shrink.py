#!/usr/bin/env python3
"""Shrinks out/'s PNGs (moments to 1376 px wide, everything to 256 colours) so the folder stays small.
Run: python3 shrink.py   (after render.cjs; needs Pillow)"""
import glob, os
from PIL import Image
here = os.path.join(os.path.dirname(os.path.abspath(__file__)), "out")
for f in glob.glob(os.path.join(here, "*.png")):
    im = Image.open(f).convert("RGB")
    if im.width > 1376:
        im = im.resize((1376, round(im.height * 1376 / im.width)), Image.LANCZOS)
    im.quantize(256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(f, optimize=True)
