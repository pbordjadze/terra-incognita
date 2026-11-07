#!/usr/bin/env python3
"""How far two renders of the same plans are apart, for a toy that isn't byte-identical run to run
(Mold: its agents claim cells with GPU atomics, which race). For each pair of PNGs present in both
directories: the share of pixels that differ and the mean absolute difference (0-255), then a
summary. Comparing a run with a second run at the same commit gives the toy's noise; a change is
judged against that, not against zero.

    python3 tools/kit/render_noise.py <out-a> <out-b> [glob=*/*.png]   # numpy and Pillow
"""
import sys
from pathlib import Path

import numpy as np
from PIL import Image

a, b = Path(sys.argv[1]), Path(sys.argv[2])
pattern = sys.argv[3] if len(sys.argv) > 3 else "**/*.png"
shares, means, n, same = [], [], 0, 0
for p in sorted(a.glob(pattern)):
    q = b / p.relative_to(a)
    if not q.exists():
        continue
    x = np.asarray(Image.open(p).convert("RGB"), dtype=np.int16)
    y = np.asarray(Image.open(q).convert("RGB"), dtype=np.int16)
    if x.shape != y.shape:
        print(f"size differs: {p.relative_to(a)}")
        continue
    d = np.abs(x - y)
    n += 1
    share = float((d.max(axis=2) > 0).mean())
    same += share == 0
    shares.append(share)
    means.append(float(d.mean()))
if n:
    print(f"{n} images, {same} identical; pixels differing: median {np.median(shares):.1%}, max {max(shares):.1%}; "
          f"mean |difference|: median {np.median(means):.2f}, max {max(means):.2f} (of 255)")
