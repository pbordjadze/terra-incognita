#!/usr/bin/env python3
"""A contact sheet of a played plan's shots in time order (painting-company row 246), for a critic to see a
minute in time rather than in chosen stills: each shot at a third of a point a pixel... small, labelled by
its time.

    python3 tools/toys/newcomer/sheet.py <out-dir> <sheet.png> [columns]
"""
import sys, re
from pathlib import Path
from PIL import Image, ImageDraw
out = Path(sys.argv[1]); cols = int(sys.argv[3]) if len(sys.argv) > 3 else 8
fs = sorted([f for f in out.glob('*.png') if '.shell' not in f.name and '.stretch' not in f.name],
            key=lambda f: (int(re.sub(r'\D', '', f.stem) or 0)))
fs = [f.with_suffix('.shell.png') if f.with_suffix('.shell.png').exists() else f for f in fs]
ims = [Image.open(f).convert('RGB') for f in fs]
w = 260; h = int(ims[0].height * w / ims[0].width)
rows = (len(ims) + cols - 1) // cols
sheet = Image.new('RGB', (cols * w, rows * (h + 18)), (20, 20, 20))
d = ImageDraw.Draw(sheet)
for i, (f, im) in enumerate(zip(fs, ims)):
    x, y = (i % cols) * w, (i // cols) * (h + 18)
    sheet.paste(im.resize((w, h)), (x, y + 18)); d.text((x + 4, y + 3), f.stem.replace('.shell', ''), fill=(230, 230, 230))
sheet.save(sys.argv[2])
