#!/usr/bin/env python3
"""Plots meander-probe's dumps: the river's line every two seconds, a strip of panels.

    PROBE_DUMP=<dir> cargo run -p meander --bin meander-probe --release
    python3 tools/toys/meander/probe_plot.py <dir>/<course>.json <out.png>
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw

fr = json.loads(Path(sys.argv[1]).read_text())[: int(sys.argv[3]) if len(sys.argv) > 3 else 8]
S = 0.02  # px a metre
w = 440
im = Image.new("RGB", (w * len(fr), w), (40, 70, 50))
d = ImageDraw.Draw(im)
for k, f in enumerate(fr):
    ox = k * w
    m = lambda p, ox=ox: (ox + w / 2 + p[0] * S, w / 2 + p[1] * S)
    for o in f["ox"]:
        d.line([m(p) for p in o], fill=(30, 30, 60), width=2)
    d.line([m(p) for p in f["line"]], fill=(220, 190, 140), width=2)
    d.text((ox + 4, 4), f"{f['t']}s", fill=(255, 255, 255))
im.save(sys.argv[2])
