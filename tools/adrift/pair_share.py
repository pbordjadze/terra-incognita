#!/usr/bin/env python3
"""The pair's span as a share of the clip's width over a stretch of a journey (R21, Motion; round 20's
critic: keep the pair at 40-60% of the width).

    python3 tools/adrift/pair_share.py <journey.csv> [--from SECONDS] [--to SECONDS]

Reads the camera trace `adrift-check journey` writes beside its mp4 and projects each lamp as
`lamp_in_clip.py` does; prints the share's min, median and max over the stretch (default: the last
15 s), and the share of frames within 40-60%.
"""
import csv, math, sys
sys.path.insert(0, __import__('os').path.dirname(__file__))
from lamp_in_clip import screen  # noqa: E402

args = sys.argv[1:]
path = args[0]
opt = lambda k, d: float(args[args.index(k) + 1]) if k in args else d
rows = list(csv.DictReader(open(path)))
n = len(rows)
lo, hi = int(opt('--from', n / 30.0 - 15.0) * 30), int(opt('--to', n / 30.0) * 30)
shares = []
for r in rows[max(0, lo):hi]:
    xs = [x for x, _, facing in screen(r) if facing]
    if len(xs) >= 2:
        shares.append((max(xs) - min(xs)) / 360.0)
shares.sort()
inside = sum(1 for s in shares if 0.4 <= s <= 0.6) / max(1, len(shares))
print(f"{path}: frames {lo}-{hi}: pair's span {shares[0]*100:.0f}% / median {shares[len(shares)//2]*100:.0f}% / {shares[-1]*100:.0f}% of the width; within 40-60% in {inside*100:.0f}% of frames")
