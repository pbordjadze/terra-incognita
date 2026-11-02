#!/usr/bin/env python3
"""The pair's span as a share of the clip's width over a stretch of a journey (R21, Motion; round 20's
critic: keep the pair at 40-60% of the width. R22: at 60 fps, lamps out, and the pair's distance apart on the screen in any direction (also as a share of the width), the camera's lurch).

    python3 tools/adrift/pair_share.py <journey.csv> [--from SECONDS] [--to SECONDS] [--fps 60] [--last S]

Reads the camera trace `adrift-check journey` writes beside its mp4 (`--camera-only` writes it alone,
in seconds) and projects each lamp as `lamp_in_clip.py` does; over the stretch (default: the last 40 s,
`--last` to change) prints the pair's span as a share of the width (min / median / max, and the share
of frames within 40-60%), frames with a lamp off the frame, frames with a lamp in the lower third, and
the pair's distance apart on the screen in any direction (also as a share of the width), the camera's lurch: the largest change, frame to frame, of the zoom's rate (log a second a second)
and of the ground's slide across the screen (points a second a second), both per second so they
read the same at 30 and 60 fps.
"""
import csv, math, sys
sys.path.insert(0, __import__('os').path.dirname(__file__))
from lamp_in_clip import screen, xyz, R1, W, H  # noqa: E402

args = sys.argv[1:]
path = args[0]
opt = lambda k, d: float(args[args.index(k) + 1]) if k in args else d
fps = opt('--fps', 60.0)
rows = list(csv.DictReader(open(path)))
n = len(rows)
lo, hi = int(opt('--from', n / fps - opt('--last', 40.0)) * fps), int(opt('--to', n / fps) * fps)
shares, spreads, out, low = [], [], 0, 0
zr, pv = [], []
prev = None
for k, r in enumerate(rows):
    c, z = xyz(float(r['clip_lon']), float(r['clip_lat'])), float(r['clip_zoom'])
    if prev is not None:
        ang = math.acos(max(-1.0, min(1.0, sum(a * b for a, b in zip(c, prev[0])))))
        zr.append((k, math.log(z / prev[1]) * fps))
        # the ground's slide as a vector on the screen is not kept; its speed's change is the lurch
        pv.append((k, ang * R1 * z * fps))
    prev = (c, z)
    if not (lo <= k < hi):
        continue
    pts = screen(r)
    if not pts:
        continue
    out += any(not f or x < 0 or x > W or y < 0 or y > H for x, y, f in pts)
    low += any(y > H * 2 / 3 for _, y, _ in pts)
    xs = [x for x, _, facing in pts if facing]
    if len(xs) >= 2:
        shares.append((max(xs) - min(xs)) / W)
        f = [(x, y) for x, y, facing in pts if facing]
        spreads.append(max(math.dist(a, b) for a in f for b in f) / W)
shares.sort()
spreads.sort()
def jerk(series):
    s = [v for k, v in series if lo <= k < hi]
    return max((abs(b - a) * fps for a, b in zip(s, s[1:])), default=0.0)
zoom_rate = max((abs(v) for k, v in zr if lo <= k < hi), default=0.0)
pan_rate = max((v for k, v in pv if lo <= k < hi), default=0.0)
inside = sum(1 for s in shares if 0.4 <= s <= 0.6) / max(1, len(shares))
q = lambda f: shares[min(len(shares) - 1, int(f * len(shares)))] * 100
print(f"{path}: {lo / fps:.1f}-{hi / fps:.1f} s: pair's span {shares[0]*100:.0f}% / p25 {q(0.25):.0f}% / median {q(0.5):.0f}% / {shares[-1]*100:.0f}% of the width; "
      f"within 40-60% in {inside*100:.0f}%; their distance apart on the screen median {spreads[len(spreads)//2]*100:.0f}% of the width; lamp out in {out}, in the lower third in {low}; "
      f"zoom rate most {zoom_rate:.2f} log/s, its change most {jerk(zr):.2f} log/s2; pan most {pan_rate:.0f} pt/s, its change most {jerk(pv):.0f} pt/s2")
