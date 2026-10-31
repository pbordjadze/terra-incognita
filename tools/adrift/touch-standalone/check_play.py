#!/usr/bin/env python3
"""Checks a played plan on the real build: reads the play.log `adrift-check -- play` wrote for one
of the exported plans (plans/check/<name>.json) against the plan's `expect` steps (each exported as
a shot named expect-NN). What the log carries is checked (the floats' count and places, which is in
hand, the camera); what it does not (the gestures' state, intents seen, riding, the clock running)
is listed as unchecked. Exit 1 on a failure.

    python3 tools/adrift/touch-standalone/check_play.py <plan.json> <out-dir>
"""
import json, re, sys

LINE = re.compile(r"^\s*(\d+) t=\s*([\d.]+)s day=\s*([-\d.]+) cam=([-\d.]+),([-\d.]+) z([\d.]+) (\S+) floats \[(.*)\]$")
FLOAT = re.compile(r"#(\d+) ([-\d.]+),([-\d.]+) (\w+) (\w+) wake (\d+)")

def lon_delta(a, b):
    d = b - a
    if d > 180: d -= 360
    if d < -180: d += 360
    return d

def main(plan_path, out_dir):
    plan = json.load(open(plan_path))
    shots = {}
    for line in open(out_dir + "/play.log"):
        m = LINE.match(line)
        if not m: continue
        name = m.group(7).split("/")[-1].removesuffix(".png")
        floats = {}
        for f in FLOAT.finditer(m.group(8)):
            floats[int(f.group(1))] = {"lon": float(f.group(2)), "lat": float(f.group(3)), "held": f.group(4), "hand": f.group(5)}
        shots[name] = {"t": float(m.group(2)), "day": float(m.group(3)), "camera": {"lon": float(m.group(4)), "lat": float(m.group(5)), "zoom": float(m.group(6))}, "floats": floats}
    cam0 = plan["scene"]["camera"]
    last = cam0
    failures, unchecked, checked = [], set(), 0
    for step in plan["steps"]:
        e = step.get("expect")
        if e is None: continue
        shot = shots.get(step["shot"])
        if shot is None:
            failures.append(f"{step['shot']}: no line in play.log"); continue
        cam = shot["camera"]
        def fail(msg): failures.append(f"{step['shot']} (t={shot['t']}): {msg}")
        for k, v in e.items():
            if k == "floats":
                checked += 1
                if len(shot["floats"]) != v: fail(f"{len(shot['floats'])} floats, expected {v}")
            elif k == "float":
                checked += 1
                f = shot["floats"].get(v["id"])
                if f is None or abs(lon_delta(f["lon"], v["lon"])) > 1.0 or abs(f["lat"] - v["lat"]) > 1.0:
                    fail(f"float {v['id']} at {f}, expected ({v['lon']}, {v['lat']}) (the real current moves it a little)")
            elif k == "held":
                checked += 1
                held = [i for i, f in shot["floats"].items() if f["hand"] != "Free"]
                if (held[0] if held else None) != v: fail(f"held {held}, expected {v}")
            elif k == "camera":
                checked += 1
                if abs(lon_delta(cam["lon"], v["lon"])) > 0.05 or abs(cam["lat"] - v["lat"]) > 0.05 or abs(cam["zoom"] - v["zoom"]) > 0.05:
                    fail(f"camera {cam}, expected {v}")
            elif k == "zoom_gt":
                checked += 1
                if not cam["zoom"] > v: fail(f"zoom {cam['zoom']} not > {v}")
            elif k == "zoom_lt":
                checked += 1
                if not cam["zoom"] < v: fail(f"zoom {cam['zoom']} not < {v}")
            elif k == "day":
                checked += 1
                if abs(shot["day"] - v) > 0.05: fail(f"day {shot['day']}, expected {v}")
            elif k == "camera_moved":
                checked += 1
                moved = abs(lon_delta(cam["lon"], cam0["lon"])) > 0.05 or abs(cam["lat"] - cam0["lat"]) > 0.05 or abs(cam["zoom"] - cam0["zoom"]) > 0.05
                if moved != v: fail(f"camera moved {moved} ({cam} from {cam0}), expected {v}")
            elif k == "camera_same_as_last":
                checked += 1
                same = abs(lon_delta(cam["lon"], last["lon"])) < 0.02 and abs(cam["lat"] - last["lat"]) < 0.02 and abs(cam["zoom"] - last["zoom"]) < 0.02
                if same != v: fail(f"camera same as last {same} ({cam} from {last}), expected {v}")
            elif k == "camera_near_float":
                checked += 1
                f = shot["floats"].get(v[0])
                if f is None: fail(f"no float {v[0]}")
                else:
                    d = (lon_delta(cam["lon"], f["lon"]) ** 2 + (cam["lat"] - f["lat"]) ** 2) ** 0.5
                    if d > v[1]: fail(f"camera {cam} is {d:.2f} deg from float {v[0]}, expected within {v[1]}")
            else:
                unchecked.add(k)
        last = cam
    name = plan_path.split("/")[-1]
    for f in failures: print("FAIL", name, f)
    print(("FAIL" if failures else "ok  "), name, f"{checked} checked", ("unchecked: " + ", ".join(sorted(unchecked))) if unchecked else "")
    return 1 if failures else 0

if __name__ == "__main__":
    sys.exit(main(sys.argv[1], sys.argv[2]))
