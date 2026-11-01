#!/usr/bin/env python3
"""Writes adrift-check plans that conduct a cloud (the conductor spike, painting-company row 199):
a tap, a sweep, a stir and a hold, with shots of the fold and the bloom between them.

    python3 tools/adrift/conduct-plan.py adrift/build/conduct.json [--frames] [--zoom=4.5] [--lines]
    cd adrift && cargo run -q -p adrift-check -- play build/conduct.json build/out/conduct

Each cue prints a line (`cue: ...`: the gesture, how many of the 2048 worlds did it, the world
chosen, and what that frame cost on this machine's CPU and software GPU)."""
import json, math, sys

out = sys.argv[1]
W, H = 393, 852
cx, cy = W / 2, H / 2
steps = [{"tick": 1.5}, {"shot": "01-bloomed"}]
fid = [1]

def stroke(pts, secs, name):
    fid[0] += 1
    i = fid[0]
    n = len(pts) - 1
    steps.append({"down": pts[0], "id": i})
    for p in pts[1:-1]:
        steps.append({"tick": secs / n})
        steps.append({"move": p, "id": i})
    steps.append({"tick": secs / n})
    steps.append({"up": pts[-1], "id": i})
    steps.append({"tick": 0.1})
    steps.append({"shot": name + "-a-folding"})
    steps.append({"tick": 0.3})
    steps.append({"shot": name + "-b-cued"})
    steps.append({"tick": 1.2})
    steps.append({"shot": name + "-c-bloomed"})

stroke([[cx + 40, cy - 30], [cx + 40, cy - 30]], 0.08, "02-point")
stroke([[cx - 80 + 16 * j, cy + 4 * j] for j in range(11)], 0.25, "03-sweep")
stroke([[cx + 60 * math.cos(a), cy + 60 * math.sin(a)] for a in [2 * math.pi * j / 16 for j in range(17)]], 0.6, "04-stir")
fid[0] += 1
steps += [{"down": [cx, cy], "id": fid[0]}, {"tick": 1.0}, {"shot": "05-fermata"}, {"up": [cx, cy], "id": fid[0]}, {"tick": 1.0}, {"shot": "06-after"}]
plan = {"size": [W * 3, H * 3], "ppp": 3, "frames": "--frames" in sys.argv,
        "scene": {"camera": {"lon": -70.0, "lat": 37.0, "zoom": float(next((a.split("=")[1] for a in sys.argv if a.startswith("--zoom=")), 4.5))}, "day": 160.0, "seconds": 5.0, "conduct": [-74.0, 35.6], "conduct_lines": "--lines" in sys.argv},
        "steps": steps}
json.dump(plan, open(out, "w"), indent=1)
print(out, len(steps), "steps")
