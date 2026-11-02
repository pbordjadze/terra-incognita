#!/usr/bin/env python3
"""How an exported clip frames its lamps (R11, R14; Motion).

    python3 tools/adrift/lamp_in_clip.py <clip-camera.csv> [--every N] [--max-step 0.02] [--max-accel 0.002]

Reads the camera trace an export writes (`adrift-check play` beside its mp4 as `<name>-camera.csv`,
`adrift-check journey` as `<out>.csv`): a row a frame with the clip's camera and every lamp out
(`id:lon:lat`). Each lamp is projected exactly as the clip draws it (`Camera::rotation`, roll included,
the 360 x 640 point clip, disc radius 166 x zoom about its centre) and the script reports, over the
clip and every N frames:

- out: frames with a lamp off the frame or behind the globe;
- fill: the lamps' horizontal span as a share of the width (the critic's aim: about 60%);
- centre: the lamps' mean place as shares of width and height (aim: x 0.5, y a little above 0.5);
- low: frames with a lamp in the lower third (where a platform's captions lie);
- pan: how fast the ground slides across the screen, points a second at 30 frames a second (the
  camera's centre moving at its zoom; R15: the critic saw hard pans at a close zoom), median and most;
- step: the largest frame-to-frame change of the zoom, in its log (R19: the critic found a 21% pop);
  with `--max-step S` the script fails (exit 2) if any frame steps more than S;
- accel: the largest frame-to-frame change of that step (R20: the critic saw the zoom's rate stop dead),
  `--max-accel A` fails a trace past it;
- start: frames where a trail's start (each lamp's place in the first frame) lies off the frame or
  within 5% of its edge (R20: the critic saw the trail's start slide off in the middle).
"""
import csv
import math
import sys

W, H, R1 = 360.0, 640.0, 166.0


def rotation(lon, lat, roll):
    a, b = math.radians(lon), -math.radians(lat)
    c0 = [math.cos(a), 0.0, -math.sin(a)]
    c1 = [math.sin(a) * math.sin(b), math.cos(b), math.cos(a) * math.sin(b)]
    c2 = [math.sin(a) * math.cos(b), -math.sin(b), math.cos(a) * math.cos(b)]
    if roll == 0:
        return c0, c1, c2
    sr, cr = math.sin(math.radians(roll)), math.cos(math.radians(roll))
    return ([cr * c0[i] + sr * c1[i] for i in range(3)], [-sr * c0[i] + cr * c1[i] for i in range(3)], c2)


def xyz(lon, lat):
    lo, la = math.radians(lon), math.radians(lat)
    return (math.cos(la) * math.sin(lo), math.sin(la), math.cos(la) * math.cos(lo))


def screen(row):
    m = rotation(float(row["clip_lon"]), float(row["clip_lat"]), float(row["clip_roll"]))
    r = R1 * float(row["clip_zoom"])
    out = []
    for item in (row.get("lamps") or "").split():
        _, lon, lat = item.split(":")
        w = xyz(float(lon), float(lat))
        v = [sum(m[i][j] * w[j] for j in range(3)) for i in range(3)]
        out.append((W / 2 + r * v[0], H / 2 - r * v[1], v[2] > 0))
    return out


def main(argv):
    every = int(argv[argv.index("--every") + 1]) if "--every" in argv else 0
    rows = list(csv.DictReader(open(argv[0])))
    stats = {"out": 0, "low": 0, "fill": [], "cx": [], "cy": [], "pan": []}
    prev = None
    max_step = float(argv[argv.index("--max-step") + 1]) if "--max-step" in argv else None
    step, step_at, last_zoom = 0.0, 0, None
    max_accel = float(argv[argv.index("--max-accel") + 1]) if "--max-accel" in argv else None
    accel, accel_at, last_dz = 0.0, 0, None
    starts, start_off = None, 0
    for n, row in enumerate(rows):
        c = xyz(float(row["clip_lon"]), float(row["clip_lat"]))
        if prev is not None:
            ang = math.acos(max(-1.0, min(1.0, sum(a * b for a, b in zip(c, prev)))))
            stats["pan"].append(ang * R1 * float(row["clip_zoom"]) * 30.0)
        prev = c
        z = float(row["clip_zoom"])
        if last_zoom is not None:
            dz = math.log(z / last_zoom)
            if abs(dz) > step:
                step, step_at = abs(dz), n
            if last_dz is not None and abs(dz - last_dz) > accel:
                accel, accel_at = abs(dz - last_dz), n
            last_dz = dz
        last_zoom = z
        if starts is None and row.get("lamps"):
            starts = " ".join(row["lamps"].split())
        if starts:
            pts0 = screen({**row, "lamps": starts})
            m = 0.05
            start_off += any(not f or x < m * W or x > (1 - m) * W or y < m * H or y > (1 - m) * H for x, y, f in pts0)
        pts = screen(row)
        if not pts:
            continue
        out = any(not f or x < 0 or x > W or y < 0 or y > H for x, y, f in pts)
        stats["out"] += out
        stats["low"] += any(y > H * 2 / 3 for _, y, _ in pts)
        xs, ys = [p[0] for p in pts], [p[1] for p in pts]
        fill = (max(xs) - min(xs)) / W
        cx, cy = sum(xs) / len(xs) / W, sum(ys) / len(ys) / H
        stats["fill"].append(fill)
        stats["cx"].append(cx)
        stats["cy"].append(cy)
        if every and n % every == 0:
            print(f"{n:5d} zoom {float(row['clip_zoom']):5.1f} fill {fill:4.0%} centre {cx:.2f},{cy:.2f}{' OUT' if out else ''}")
    k = len(stats["fill"])
    if not k:
        print("no lamps in the trace")
        return 1
    med = lambda v: sorted(v)[len(v) // 2]
    print(f"{len(rows)} frames: lamp out of frame in {stats['out']}, in the lower third in {stats['low']}; "
          f"fill median {med(stats['fill']):.0%} ({min(stats['fill']):.0%}..{max(stats['fill']):.0%}); "
          f"centre median {med(stats['cx']):.2f},{med(stats['cy']):.2f} (y {min(stats['cy']):.2f}..{max(stats['cy']):.2f}); "
          f"pan median {med(stats['pan'] or [0]):.0f} pt/s, most {max(stats['pan'] or [0]):.0f}; "
          f"largest zoom step {step:.1%} at frame {step_at}, its largest change {accel:.2%} at frame {accel_at}; "
          f"a trail's start off the frame (5% margin) in {start_off}")
    if max_step is not None and step > max_step:
        print(f"FAIL: a zoom step of {step:.1%} exceeds {max_step:.1%}")
        return 2
    if max_accel is not None and accel > max_accel:
        print(f"FAIL: a change of the zoom's step of {accel:.2%} exceeds {max_accel:.2%}")
        return 2
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
