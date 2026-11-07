#!/usr/bin/env python3
"""Builds the whales' year (toys/whales/data/routes.json): where each one is, day by day, and when it sings.

    pip install numpy pillow
    python3 tools/toys/whales/routes.py        # writes the file and checks every day's place is at sea

Each whale is a route of dated places, a year round (day of the year, 0 = January 1st), drawn from
the published tracks and sightings of its population (toys/whales/SOURCES.md):

- humpback: the California-Oregon feeding group, which breeds off mainland Mexico (SPLASH:
  Calambokidis et al. 2008; Urbán et al. 2000). Feeds in Monterey Bay summer to late autumn (where its
  song was recorded, in November), goes south along Baja California in December, spends January to
  March round Bahía de Banderas and the Islas Marías, and comes north in April and May.
- blue: the Eastern North Pacific population, from 159 satellite-tagged whales (Bailey et al. 2009,
  Endangered Species Research 10:93-106; Mate et al. 1999): feeds off central and southern California
  July to October, goes south off Baja and Mexico in November and December to the Costa Rica Dome
  (about 9 N, 89 W), winters there January to March, and comes back north April to June.

`sings` is each whale's song through the year, month by month, 0..1 (how much of the time it is heard
at its season's peak), from passive acoustic records: humpback song in Monterey Bay rises in September,
peaks November-December and lasts on the breeding grounds into April (Ryan et al. 2019, PLoS ONE
14:e0222456; song is heard there all winter); blue whale song rises in August and peaks
September-November, fading by January (Oestreich et al. 2020, eLife 9:e53740; Stafford et al. 2001).
The months between are never silent in the toy: a whale's calls are heard at a low floor, which the
toy draws (src/lib.rs says so).

Between two dated places the way is drawn, here: along the great circle, eased, with a slow wander of
the toy's own, written as a track of four places a day that the toy follows (src/route.rs). The way
must keep 25 km from the globe's own coast (Globe/geo.png) and the wander is shrunk until it does, so
a whale never swims ashore.
"""
import json
import math
import os
import sys

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.normpath(os.path.join(HERE, "../../.."))
OUT = os.path.join(ROOT, "toys/whales/data/routes.json")


def doy(m, d):
    return [0, 31, 59, 90, 120, 151, 181, 212, 243, 273, 304, 334][m - 1] + d - 1


WHALES = {
    "humpback": {
        "route": [
            (doy(1, 12), -105.85, 20.75),  # Bahía de Banderas, at its mouth
            (doy(2, 5), -106.45, 21.25),  # toward the Islas Marías
            (doy(2, 28), -105.85, 20.85),
            (doy(3, 22), -106.75, 21.15),
            (doy(4, 4), -110.15, 22.45),  # off Cabo San Lucas
            (doy(4, 20), -113.50, 25.10),  # off Bahía Magdalena
            (doy(5, 4), -116.90, 29.40),
            (doy(5, 14), -119.00, 32.20),
            (doy(5, 21), -120.60, 33.70),  # the Channel Islands, outside them
            (doy(5, 27), -122.10, 35.30),  # off Big Sur
            (doy(6, 2), -122.45, 36.55),  # Monterey Bay, at the canyon's mouth
            (doy(7, 10), -122.60, 36.90),
            (doy(8, 20), -122.40, 36.70),
            (doy(10, 1), -122.45, 36.45),
            (doy(11, 28), -122.35, 36.75),  # the song: Monterey Bay, November 28th
            (doy(12, 5), -122.30, 35.60),
            (doy(12, 9), -120.70, 33.75),
            (doy(12, 17), -117.95, 31.40),
            (doy(12, 22), -116.00, 27.50),  # off Punta Eugenia
            (doy(12, 28), -113.40, 25.00),
            (doy(1, 4) + 365, -110.10, 22.40),
        ],
        "sings": [0.95, 0.9, 0.85, 0.6, 0.2, 0.08, 0.08, 0.15, 0.4, 0.7, 0.9, 1.0],
    },
    "blue": {
        "route": [
            (doy(1, 2), -97.0, 12.6),  # off the Gulf of Tehuantepec
            (doy(1, 25), -90.2, 9.4),  # the Costa Rica Dome
            (doy(2, 20), -88.6, 8.6),
            (doy(3, 18), -89.6, 9.8),
            (doy(4, 8), -95.5, 12.6),
            (doy(4, 26), -104.8, 17.0),
            (doy(5, 14), -111.2, 22.0),
            (doy(6, 2), -115.6, 27.0),
            (doy(6, 22), -119.6, 32.6),
            (doy(7, 12), -120.6, 33.4),  # the Channel Islands, outside them
            (doy(7, 20), -121.3, 34.45),
            (doy(7, 28), -121.9, 35.3),
            (doy(8, 18), -122.35, 36.55),  # Monterey Bay
            (doy(9, 25), -121.9, 35.4),
            (doy(10, 25), -120.8, 33.5),
            (doy(11, 3), -119.0, 32.2),
            (doy(11, 15), -117.3, 30.4),
            (doy(12, 2), -114.0, 25.6),
            (doy(12, 18), -108.6, 19.6),
        ],
        "sings": [0.15, 0.08, 0.06, 0.05, 0.05, 0.08, 0.2, 0.55, 0.9, 1.0, 0.95, 0.5],
    },
}

WANDER_DEG = 0.45  # src/route.rs's wander: its greatest reach from the route, degrees


def xyz(lon, lat):
    la, lo = math.radians(lat), math.radians(lon)
    return np.array([math.cos(la) * math.sin(lo), math.sin(la), math.cos(la) * math.cos(lo)])


def lonlat(v):
    v = v / np.linalg.norm(v)
    return math.degrees(math.atan2(v[0], v[2])), math.degrees(math.asin(max(-1, min(1, v[1]))))


def main():
    geo = np.asarray(Image.open(os.path.join(ROOT, "Globe/geo.png")))
    H, W = geo.shape[:2]

    def coast_km(lon, lat):
        x = int((lon / 360 + 0.5) * W) % W
        y = min(max(int((0.5 - lat / 180) * H), 0), H - 1)
        fine, coarse = (geo[y, x, 0] - 128.0) / 8.0, (geo[y, x, 1] - 128.0) * 2.0
        d = coarse if abs(coarse) > 14 else fine
        return -d * 40075.0 / W * max(math.cos(math.radians(lat)), 0.2)  # positive at sea

    # the year, four places a day: the route along great circles (eased into and out of each dated
    # place), plus a slow wander of the toy's own (a few sines, seeded), halved until it is 25 km at sea
    rng = np.random.default_rng(201)
    bad = 0
    days = {}
    for name, w in WHALES.items():
        r = sorted(w["route"])
        r = [(d % 365, lo, la) for d, lo, la in r]
        r.sort()
        waves = [(rng.uniform(0.25, 0.8) / 30.0, rng.uniform(0, 2 * math.pi), rng.uniform(0, 2 * math.pi)) for _ in range(4)]
        track = []
        for i in range(365 * 4):
            t = i / 4.0
            j = max([k for k in range(len(r)) if r[k][0] <= t], default=len(r) - 1)
            a, b = r[j], r[(j + 1) % len(r)]
            span = (b[0] - a[0]) % 365 or 365
            k = ((t - a[0]) % 365) / span
            k = k * k * (3 - 2 * k)
            va, vb = xyz(a[1], a[2]), xyz(b[1], b[2])
            ang = math.acos(max(-1, min(1, va @ vb)))
            v = va if ang < 1e-9 else (math.sin((1 - k) * ang) * va + math.sin(k * ang) * vb) / math.sin(ang)
            p = lonlat(v)
            if coast_km(*p) < 25:
                bad += 1
                print(f"{name}: day {t:.1f} on the route at {p[0]:.2f},{p[1]:.2f} is {coast_km(*p):.0f} km from the coast")
            dx = sum(math.sin(2 * math.pi * f * t + ph) for f, ph, _ in waves) / 2.0
            dy = sum(math.sin(2 * math.pi * f * t + ph) for f, _, ph in waves) / 2.0
            s_ = WANDER_DEG
            q = p
            while s_ > 0.01:
                q = (p[0] + dx * s_ / max(math.cos(math.radians(p[1])), 0.3), p[1] + dy * s_)
                if coast_km(*q) >= 25:
                    break
                s_ /= 2
            else:
                q = p
            track.append([round(q[0], 4), round(q[1], 4)])
        days[name] = track
    out = {
        "_": "Built by tools/toys/whales/routes.py; never hand-edit. Days of the year (0 = Jan 1), lon, lat.",
        "per_day": 4,
        "whales": {n: {"route": [list(p) for p in sorted(w["route"])], "sings": w["sings"], "track": days[n]} for n, w in WHALES.items()},
    }
    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    with open(OUT, "w") as f:
        json.dump(out, f, separators=(",", ":"))
    print(f"routes.json written; {bad} places too near the coast")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main())
