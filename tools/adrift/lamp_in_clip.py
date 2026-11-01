#!/usr/bin/env python3
"""Whether a lamp stays in an exported clip's frame (R11, Motion).

    python3 tools/adrift/lamp_in_clip.py <out>/clip-camera.csv LON LAT [LON LAT ...] [--compare take]

Reads the camera trace `adrift-check play` writes beside an export (`<name>-camera.csv`: the clip's
camera and the take's, a row a frame) and, for each lamp place given (a plan's float; lamps drift a few
tenths of a degree in a short take, so this is a close guide, not exact), how far from the clip's centre
it lies as a share of the clip's half-width (360 x 640 points; the disc's radius at zoom 1 is 166):
over 1.0 it is out of frame, and behind the globe counts as out. `--compare take` also reports the
take's own camera drawn at the clip's size, to show what the lean changed.
"""
import csv
import math
import sys

R1, HALF = 166.0, 180.0


def xyz(lon, lat):
    lo, la = math.radians(lon), math.radians(lat)
    return (math.cos(la) * math.sin(lo), math.sin(la), math.cos(la) * math.cos(lo))


def share(cam_lon, cam_lat, zoom, lamp):
    c = xyz(cam_lon, cam_lat)
    d = sum(a * b for a, b in zip(c, lamp))
    if d <= 0:
        return float("inf")
    return math.sqrt(max(0.0, 1 - d * d)) * R1 * zoom / HALF


def main(argv):
    compare = "--compare" in argv
    args = [a for a in argv if not a.startswith("--") and a != "take"]
    rows = list(csv.DictReader(open(args[0])))
    nums = [float(a) for a in args[1:]]
    lamps = [xyz(nums[i], nums[i + 1]) for i in range(0, len(nums), 2)]
    for who in (["clip", "take"] if compare else ["clip"]):
        worst, out, zooms = 0.0, 0, []
        for r in rows:
            lon, lat, z = float(r[f"{who}_lon"]), float(r[f"{who}_lat"]), float(r[f"{who}_zoom"])
            s = max(share(lon, lat, z, l) for l in lamps)
            worst = max(worst, s)
            out += s > 1.0
            zooms.append(z)
        print(f"{who}: {len(rows)} frames, lamps out of frame in {out}, furthest at {worst:.2f} of the half-width, zoom {min(zooms):.2f}..{max(zooms):.2f}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
