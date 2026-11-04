#!/usr/bin/env python3
"""A strip of a played plan's shots side by side, for looking at a whole battle at once.

    python3 tools/toys/battle/strip.py <shots dir> <out.png> [scale] [name ...]   # names: shot files without .png
"""
import sys
from pathlib import Path
from PIL import Image

d, out = Path(sys.argv[1]), sys.argv[2]
scale = float(sys.argv[3]) if len(sys.argv) > 3 else 0.2
names = sys.argv[4:] or sorted(p.stem for p in d.glob('*.png'))
ims = [Image.open(d / f'{n}.png').convert('RGB') for n in names]
w, h = int(ims[0].width * scale), int(ims[0].height * scale)
strip = Image.new('RGB', (w * len(ims) + 4 * (len(ims) - 1), h), 'white')
for i, im in enumerate(ims):
    strip.paste(im.resize((w, h), Image.LANCZOS), (i * (w + 4), 0))
strip.save(out)
