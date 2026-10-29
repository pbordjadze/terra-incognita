#!/usr/bin/env python3
"""The sea's depth everywhere, for the lead line (Prototypes/lead-line/depths.js): a grid of 1024 x 512
cells (about a third of a degree), each the depth in steps of 50 m (0 land or shallower than 25 m,
255 for 12,750 m and deeper), read from the globe's own terrain map (Globe/terrain.png, whose green
channel is 127 - depth / 87 m below sea level: GEBCO, through tools/globe/build.py), each cell the
mean of its sixteen pixels. Written as base64 so the pack's script
can read it in one go.

    python3 tools/leadline/build_depths.py

Rebuild, never hand-edit. The grid is coarse: the lead finds the depth of a cell, not of a point.
"""
import base64
from pathlib import Path
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
W, H = 1024, 512

def main():
    im = np.asarray(Image.open(ROOT / 'Globe' / 'terrain.png').convert('RGBA'))
    g = im[..., 1].astype(np.int32)
    depth = np.where(g <= 127, (127 - g) * 87, 0)          # metres below sea level (0 on land)
    h, w = depth.shape
    fy, fx = h // H, w // W                                   # 4 x 4 pixels a cell
    cells = depth[:H * fy, :W * fx].reshape(H, fy, W, fx).mean(axis=(1, 3))
    steps = np.clip(np.round(cells / 50), 0, 255).astype(np.uint8)
    out = ROOT / 'Prototypes' / 'lead-line' / 'depths.js'
    out.parent.mkdir(parents=True, exist_ok=True)
    text = base64.b64encode(steps.tobytes()).decode('ascii')
    out.write_text('/* The sea\'s depth in a grid of %d x %d cells, the mean depth in it in steps of 50 m (0: land or\n'
                   '   shallower than 25 m), from the globe\'s own terrain (GEBCO, through tools/globe/build.py and\n'
                   '   tools/leadline/build_depths.py). Row 0 is the north (lat 90), column 0 the west (lon -180). */\n'
                   'window.DEPTHS = { width: %d, height: %d, metresPerStep: 50, data: "%s" };\n' % (W, H, W, H, text))
    sea = steps[steps > 0]
    print(f'{out}: {out.stat().st_size // 1024} KB; deepest {int(steps.max()) * 50} m; mean of the sea {int(sea.mean()) * 50} m')

if __name__ == '__main__':
    main()
