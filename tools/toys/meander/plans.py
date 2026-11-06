#!/usr/bin/env python3
"""Writes Meander's stroke plans (toys/meander/plans/push.json, ipad.json): a finger's push as raw
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
    "push": {
        "_": "The one gesture: a finger pushes through the river sideways (left to right, a little wavy); the reach it passes is carried along and stretched, then keeps swelling by itself and pinches off a lake away from the finger. Then a tap by the river (a ring, the bank nudged). Written by tools/toys/meander/plans.py.",
        "size": [1179, 2556], "ppp": 3, "seconds": 20, "fps": 30,
        "events": stroke(1.0, 0.9, [[40, 420], [200, 440], [340, 470]], n=30) + [{"t": 17.0, "tap": [200, 700]}],
        "shots": [{"t": 0.9, "name": "before"}, {"t": 1.5, "name": "pushing"}, {"t": 2.0, "name": "pushed"}, {"t": 4, "name": "4s"}, {"t": 7, "name": "7s"}, {"t": 11, "name": "11s"}, {"t": 17.3, "name": "tap"}],
        "film": {"from": 0, "to": 14, "every": 0.1},
        "audio": True,
    },
    "ipad": {
        "_": "The 13-inch iPad, landscape: the opening, then a push across the river. Written by tools/toys/meander/plans.py.",
        "size": [2752, 2064], "ppp": 2, "seconds": 18, "fps": 30,
        "events": stroke(6.0, 1.0, [[300, 900], [900, 950], [1500, 1000]]),
        "shots": [{"t": 0.5, "name": "open"}, {"t": 5.5, "name": "before"}, {"t": 17.5, "name": "after"}],
    },
}
for name, p in plans.items():
    (ROOT / "toys/meander/plans" / f"{name}.json").write_text(json.dumps(p, indent=1) + "\n")
    print(name)
