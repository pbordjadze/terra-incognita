#!/usr/bin/env python3
"""Writes Meander's stroke plans (toys/meander/plans/push.json, ipad.json, and the several fingers' two.json,
share.json, rub.json, hand.json): a finger's push as raw
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
        "film": {"from": 0, "to": 14, "every": 0.1, "fingers": True},
        "audio": True,
    },
    "ipad": {
        "_": "The 13-inch iPad, landscape: the opening, then a push across the river. Written by tools/toys/meander/plans.py.",
        "size": [2752, 2064], "ppp": 2, "seconds": 18, "fps": 30,
        "events": stroke(6.0, 1.0, [[300, 900], [900, 950], [1500, 1000]]),
        "shots": [{"t": 0.5, "name": "open"}, {"t": 5.5, "name": "before"}, {"t": 17.5, "name": "after"}],
    },
    "two": {
        "_": "Two thumbs at once on two reaches: the upper one pushed left, the lower one, a moment later, pushed left too; each should carry its own reach and swell its own bend. Written by tools/toys/meander/plans.py.",
        "size": [1179, 2556], "ppp": 3, "seconds": 16, "fps": 30,
        "events": stroke(1.0, 1.0, [[281, 340], [200, 345], [140, 352]], n=30, ident=1)
        + stroke(1.15, 1.0, [[124, 681], [80, 686], [40, 700]], n=30, ident=2),
        "shots": [{"t": 0.9, "name": "before"}, {"t": 1.6, "name": "pushing"}, {"t": 2.4, "name": "pushed"}, {"t": 6, "name": "6s"}, {"t": 12, "name": "12s"}],
        "film": {"from": 0, "to": 14, "every": 0.1, "fingers": True},
        "audio": True,
    },
    "share": {
        "_": "Two fingers on one reach pulling it apart (one up and right, one down and left), and while they pull a third finger taps by the river below (a ring, the bank nudged). Written by tools/toys/meander/plans.py.",
        "size": [1179, 2556], "ppp": 3, "seconds": 16, "fps": 30,
        "events": stroke(1.0, 1.2, [[281, 340], [320, 320], [360, 300]], n=36, ident=1)
        + stroke(1.0, 1.2, [[192, 420], [140, 445], [90, 470]], n=36, ident=2)
        + [{"t": 1.5, "down": [200, 800], "id": 3}, {"t": 1.58, "up": [200, 800], "id": 3}],
        "shots": [{"t": 0.9, "name": "before"}, {"t": 1.6, "name": "pulling"}, {"t": 2.4, "name": "pulled"}, {"t": 6, "name": "6s"}, {"t": 12, "name": "12s"}],
        "film": {"from": 0, "to": 14, "every": 0.1, "fingers": True},
        "audio": True,
    },
    "rub": {
        "_": "Five fingers down at once, each sweeping across the valley at its own height for 2.5 s, some left, some right, as a hand rubs: the frame's cost with many fingers, and whether the river stays whole. Written by tools/toys/meander/plans.py.",
        "size": [1179, 2556], "ppp": 3, "seconds": 14, "fps": 60,
        "events": [e for k, (y, x0, x1) in enumerate([(150, 330, 120), (300, 340, 120), (450, 60, 300), (600, 380, 150), (760, 60, 320)])
                   for e in stroke(1.0 + 0.05 * k, 2.5, [[x0, y], [(x0 + x1) / 2, y + 20], [x1, y + 10]], n=150, ident=k + 1)],
        "shots": [{"t": 0.9, "name": "before"}] + [{"t": round(1.3 + 0.2 * i, 2), "name": f"down{i}"} for i in range(11)] + [{"t": 6, "name": "6s"}, {"t": 12, "name": "12s"}],
        "film": {"from": 0, "to": 12, "every": 0.1, "fingers": True},
        "audio": True,
    },
    "hand": {
        "_": "A hand's five fingertips land on the river at five places and push the same way (to the right) together for a little over a second, then lift: five pushed bends at once. Written by tools/toys/meander/plans.py.",
        "size": [1179, 2556], "ppp": 3, "seconds": 16, "fps": 30,
        "events": [e for k, (x, y, dx) in enumerate([(121, 150, 100), (242, 300, 110), (77, 450, 120), (191, 600, 110), (128, 760, 90)])
                   for e in stroke(1.0 + 0.04 * k, 1.2, [[x, y], [x + dx * 0.5, y + 6], [x + dx, y + 4]], n=36, ident=k + 1)],
        "shots": [{"t": 0.9, "name": "before"}, {"t": 1.7, "name": "pushing"}, {"t": 2.6, "name": "pushed"}, {"t": 6, "name": "6s"}, {"t": 12, "name": "12s"}],
        "film": {"from": 0, "to": 14, "every": 0.1, "fingers": True},
        "audio": True,
    },
}
for name, p in plans.items():
    (ROOT / "toys/meander/plans" / f"{name}.json").write_text(json.dumps(p, indent=1) + "\n")
    print(name)
