#!/usr/bin/env python3
"""A journey's framing, second by second (R22, Motion): the clip's zoom, the pair's distance (km) and
span (share of the width), and whether a lamp is off the frame.

    python3 tools/adrift/journey_timeline.py <journey.csv> [--every SECONDS] [--fps 60]
"""
import csv, math, sys
sys.path.insert(0, __import__('os').path.dirname(__file__))
from lamp_in_clip import screen, W, H  # noqa: E402

a = sys.argv[1:]
opt = lambda k, d: float(a[a.index(k) + 1]) if k in a else d
fps, every = opt('--fps', 60.0), opt('--every', 2.0)
rows = list(csv.DictReader(open(a[0])))
for k in range(0, len(rows), int(every * fps)):
    r = rows[k]
    ps = [tuple(map(float, x.split(':')[1:])) for x in r['lamps'].split()]
    d = 0.0
    if len(ps) >= 2:
        (o1, a1), (o2, a2) = ps[:2]
        l1, l2 = math.radians(a1), math.radians(a2)
        d = 6371 * math.acos(min(1, math.sin(l1) * math.sin(l2) + math.cos(l1) * math.cos(l2) * math.cos(math.radians(o1 - o2))))
    pts = screen(r)
    xs = [x for x, _, f in pts if f]
    span = (max(xs) - min(xs)) / W if len(xs) >= 2 else 0
    out = any(not f or x < 0 or x > W or y < 0 or y > H for x, y, f in pts)
    ys = ' '.join(f"{x / W:.2f},{y / H:.2f}" for x, y, _ in pts)
    print(f"{k / fps:5.1f}s zoom {float(r['clip_zoom']):6.1f} dist {d:6.1f} km span {span:4.0%} {'OUT ' if out else ''}{ys}")
