#!/usr/bin/env python3
"""Writes Meander's stroke plans (toys/meander/plans/draw.json, ipad.json): a finger's course as raw
down/move/up events (the check's drag has only two ends), a slightly wavy stroke as a hand draws it.

    python3 tools/toys/meander/plans.py
"""
import json, math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def stroke(t0, secs, pts, n=40, ident=1):
    """Raw events along a polyline through `pts` (points), `n` moves over `secs`."""
    seg = [math.dist(a, b) for a, b in zip(pts, pts[1:])]
    total = sum(seg)
    out = [{"t": round(t0, 3), "down": list(pts[0]), "id": ident}]
    for k in range(1, n + 1):
        s = total * k / n
        i = 0
        while i < len(seg) - 1 and s > seg[i]:
            s -= seg[i]
            i += 1
        f = min(s / seg[i], 1.0)
        a, b = pts[i], pts[i + 1]
        p = [round(a[0] + (b[0] - a[0]) * f + 3 * math.sin(k * 0.7), 1), round(a[1] + (b[1] - a[1]) * f, 1)]
        out.append({"t": round(t0 + secs * k / n, 3), "move": p, "id": ident})
    out.append({"t": round(t0 + secs + 0.02, 3), "up": out[-1]["move"], "id": ident})
    return out


plans = {
    "draw": {
        "_": "The one gesture: a finger lays a new course across the plain (a slightly wavy stroke, bottom left to top right); the old river is left to silt; the new one snakes and cuts its first oxbow. Then a tap on the plain (a ring). Written by tools/toys/meander/plans.py.",
        "size": [1179, 2556], "ppp": 3, "seconds": 24, "fps": 30,
        "events": stroke(1.0, 1.4, [[120, 790], [160, 600], [130, 420], [210, 250], [280, 60]]) + [{"t": 21.0, "tap": [300, 420]}],
        "shots": [{"t": 0.9, "name": "before"}, {"t": 2.0, "name": "drawn"}, {"t": 3.0, "name": "3s"}, {"t": 8, "name": "8s"}, {"t": 12, "name": "12s"}, {"t": 16, "name": "16s"}, {"t": 21.3, "name": "tap"}],
        "film": {"from": 0, "to": 24, "every": 0.1},
        "audio": True,
    },
    "ipad": {
        "_": "The 13-inch iPad, landscape: the opening, then a course drawn across. Written by tools/toys/meander/plans.py.",
        "size": [2752, 2064], "ppp": 2, "seconds": 18, "fps": 30,
        "events": stroke(6.0, 1.2, [[400, 1000], [600, 700], [500, 400], [700, 60]]),
        "shots": [{"t": 0.5, "name": "open"}, {"t": 5.5, "name": "before"}, {"t": 17.5, "name": "after"}],
    },
}
for name, p in plans.items():
    (ROOT / "toys/meander/plans" / f"{name}.json").write_text(json.dumps(p, indent=1) + "\n")
    print(name)
