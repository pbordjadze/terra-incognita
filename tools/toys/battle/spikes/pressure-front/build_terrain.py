#!/usr/bin/env python3
"""Builds terrain.js: the ground of Cannae for the pressure-front spike.

Run:  python3 build_terrain.py <scratch dir>     (needs curl, numpy, Pillow)

Fetches Terrarium elevation tiles (AWS open data, z13, x 4461-4465, y 3060-3064) around
41.30 N 16.13 E, decodes elevation = R*256 + G + B/256 - 32768 m, rotates the mosaic 55 degrees
counter-clockwise so the Aufidus (Ofanto) valley runs up the screen (north-east up: the river's
plain on the left, the scarp of Cannae on the right), crops the battlefield (about 6.0 x 4.5 km)
and writes it as 320x240 cells of decimetres (uint16, little-endian, base64).
"""
import base64, os, subprocess, sys
import numpy as np
from PIL import Image

out = sys.argv[1] if len(sys.argv) > 1 else "."
os.makedirs(out, exist_ok=True)
rows = []
for y in range(3060, 3065):
    r = []
    for x in range(4461, 4466):
        p = os.path.join(out, f"z13_{x}_{y}.png")
        if not os.path.exists(p):
            subprocess.check_call(["curl", "-sS", "-o", p,
                f"https://s3.amazonaws.com/elevation-tiles-prod/terrarium/13/{x}/{y}.png"])
        a = np.asarray(Image.open(p).convert("RGB")).astype(float)
        r.append(a[..., 0] * 256 + a[..., 1] + a[..., 2] / 256 - 32768)
    rows.append(np.hstack(r))
E = np.vstack(rows).astype(np.float32)
rot = np.asarray(Image.fromarray(E, mode="F").rotate(55, resample=Image.BILINEAR, center=(640, 640)))
x0, y0, W, H = 262, 468, 420, 315          # z13 pixels here are ~14.35 m
crop = rot[y0:y0 + H, x0:x0 + W]
a = np.asarray(Image.fromarray(crop, mode="F").resize((320, 240), Image.BILINEAR))
dm = np.clip(np.round(a * 10), 0, 65535).astype("<u2")
s = base64.b64encode(dm.tobytes()).decode()
here = os.path.dirname(os.path.abspath(__file__))
with open(os.path.join(here, "terrain.js"), "w") as f:
    f.write("// Cannae ground: Terrarium (Mapzen/AWS open data) z13 elevation around 41.30N 16.13E,\n"
            "// rotated 55 deg so the Aufidus valley runs up the screen (north-east is up), 320x240 cells,\n"
            "// decimetres, little-endian uint16. Built by build_terrain.py.\n"
            "window.TERRAIN={w:320,h:240,metresPerCell:%.2f,b64:\"%s\"};\n" % (W * 14.35 / 320, s))
