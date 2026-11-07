#!/usr/bin/env python3
"""The ground round a toy seen from above, so the descent onto it has real ground all the way down
(painting-company row 258): a 200 km square of Copernicus Sentinel-2 true colour at about 100 m a pixel,
centred on the toy's own sheet. At its middle it is the toy's own picture (the same scenes, the toy's own
colour lift), so the toy's sheet has no edge in it; toward its own edge its broad colour eases into the
month's ground as Terra draws it (Blue Marble in Terra's light key, August), keeping its detail, so it has
no edge on the Earth either.

    python3 tools/toys/terra/region.py <cache-dir> [place...]    # every place by default

Places (PLACES): beni (Meander's floodplain: beni.jpg), hamwall (the starlings' reed beds: hamwall.jpg and,
finer, hamwall-near.jpg). A place
whose toy has a sheet of its own (Meander) has no near sheet; the others' near sheet is 20 km at 10 m.

Needs rasterio (GDAL), numpy, scipy, Pillow, and the network (AWS's open sentinel-cogs bucket; the scenes
found with Element 84's earth-search: cloud under 5 %, nearest Meander's own date first).

Each picture is 2048 px square, north up by its UTM grid (within a degree or two of true north at these
places, not corrected), its side about the place's middle (Meander's: the same middle as
toys/meander/data/plain.jpg).
"""
import math
import os
import sys
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[3]
# name: (UTM zone, south, the middle in that zone's metres or None for (lon, lat), lift, near sheet)
PLACES = {
    "beni": (19, True, (711800.0, 8566130.0), None, (0.06, 1.18, 1.15), False),
    "hamwall": (30, False, None, (-2.787, 51.153), (0.0, 1.05, 1.05), True),
}
PX = 2048
# the scenes: every one Element 84's earth-search lists over the square from mid-July to mid-September 2024
# with cloud under 5 %, Meander's own date (15 August) first, then nearest it (a tile's scene covers only
# the part of it its orbit's swath crossed, so most tiles need two)
STAC = "https://earth-search.aws.element84.com/v1/search"
WHEN = "2024-07-10T00:00:00Z/2024-09-20T00:00:00Z"
ON = "2024-08-15"


# (Britain is seldom clear: a wider season, a little more cloud, for Ham Wall)
SEASON = {"hamwall": ("2023-04-01T00:00:00Z/2024-09-30T00:00:00Z", "2024-06-15", 12)}


def scenes(bbox, place=""):
    when, on_day, cloud = SEASON.get(place.replace("-near", ""), (WHEN, ON, 5))
    import json
    import urllib.request
    body = json.dumps({"collections": ["sentinel-2-l2a"], "bbox": bbox, "datetime": when, "query": {"eo:cloud_cover": {"lt": cloud}}, "limit": 200}).encode()
    req = urllib.request.Request(STAC, data=body, headers={"Content-Type": "application/json"})
    feats = json.load(urllib.request.urlopen(req, timeout=60))["features"]
    from datetime import date
    on = date.fromisoformat(on_day)
    def key(f):
        d = date.fromisoformat(f["properties"]["datetime"][:10])
        return (abs((d - on).days), f["properties"]["eo:cloud_cover"])
    return [f["assets"]["visual"]["href"] for f in sorted(feats, key=key)]


# a place's lift: (c - lift[0]) * lift[1], then saturation lift[2] about the luminance (Meander's own,
# toys/meander/src/toy.rs, for its floodplain)


def utm_to_lonlat(x, y, zone=19, south=True):
    """Inverse UTM (WGS84), arrays in metres to degrees."""
    a, f, k0 = 6378137.0, 1 / 298.257223563, 0.9996
    e2 = f * (2 - f)
    ep2 = e2 / (1 - e2)
    x = x - 500000.0
    y = y - (10000000.0 if south else 0.0)
    mu = y / k0 / (a * (1 - e2 / 4 - 3 * e2 ** 2 / 64 - 5 * e2 ** 3 / 256))
    e1 = (1 - math.sqrt(1 - e2)) / (1 + math.sqrt(1 - e2))
    p1 = mu + (3 * e1 / 2 - 27 * e1 ** 3 / 32) * np.sin(2 * mu) + (21 * e1 ** 2 / 16 - 55 * e1 ** 4 / 32) * np.sin(4 * mu) + (151 * e1 ** 3 / 96) * np.sin(6 * mu)
    c1 = ep2 * np.cos(p1) ** 2
    t1 = np.tan(p1) ** 2
    n1 = a / np.sqrt(1 - e2 * np.sin(p1) ** 2)
    r1 = a * (1 - e2) / (1 - e2 * np.sin(p1) ** 2) ** 1.5
    d = x / (n1 * k0)
    lat = p1 - (n1 * np.tan(p1) / r1) * (d ** 2 / 2 - (5 + 3 * t1 + 10 * c1 - 4 * c1 ** 2 - 9 * ep2) * d ** 4 / 24)
    lon = (d - (1 + 2 * t1 + c1) * d ** 3 / 6) / np.cos(p1)
    return np.degrees(lon) + (zone * 6 - 183), np.degrees(lat)


def read(cache, zone, south, centre, side, place=""):
    import rasterio
    from rasterio.enums import Resampling
    from rasterio.windows import from_bounds
    f = Path(cache) / f"region_{zone}_{int(centre[0])}_{int(centre[1])}_{int(side)}_{PX}.npy"
    if f.exists():
        return np.load(f)
    h = side / 2
    x0, y0, x1, y1 = centre[0] - h, centre[1] - h, centre[0] + h, centre[1] + h
    total = np.zeros((PX, PX, 3), np.float32)
    weight = np.zeros((PX, PX), np.float32)
    lo0, la0 = utm_to_lonlat(np.array([x0, x1, x0, x1]), np.array([y0, y0, y1, y1]), zone, south)
    from rasterio.warp import transform_bounds
    feather = 40.0  # pixels: each scene eases in over this far from its edge, so the dates meet without a seam
    for url in scenes([float(lo0.min()), float(la0.min()), float(lo0.max()), float(la0.max())], place):
        if weight.min() >= 1.0:
            break
        print("reading", url, flush=True)
        with rasterio.open(url) as d:
            # (a scene in a neighbouring zone: the square's bounds in its own grid, a little skewed)
            b = (x0, y0, x1, y1) if d.crs.to_epsg() == (32700 if south else 32600) + zone else transform_bounds(f"EPSG:{(32700 if south else 32600) + zone}", d.crs, x0, y0, x1, y1)
            win = from_bounds(*b, d.transform)
            a = d.read(window=win, out_shape=(3, PX, PX), boundless=True, fill_value=0, resampling=Resampling.average)
        a = np.moveaxis(a, 0, -1).astype(np.float32)
        ok = a.min(-1) > 0  # (a tile's edge averages in its no-data)
        if ok.sum() == 0:
            continue
        # matched to what is laid where they overlap (another day's light and haze), a gain and an offset a
        # channel, then laid with a weight that rises from its edge, so the dates show as neither
        # rectangles nor seams
        have = weight > 0.5
        both = ok & have
        if both.sum() > 2000:
            est = total / np.maximum(weight, 1e-6)[..., None]
            for c in range(3):
                x, y = a[..., c][both], est[..., c][both]
                a[..., c] = (a[..., c] - x.mean()) * (y.std() / max(x.std(), 1e-3)) + y.mean()
        w = np.minimum(ndimage.distance_transform_edt(ok), feather) / feather
        # (where nothing is laid yet a scene counts whole, so the first one has no fade into the void)
        w = np.where(have, w, ok.astype(np.float32))
        total += a * w[..., None]
        weight += w
    acc = np.where(weight[..., None] > 0, np.clip(total / np.maximum(weight, 1e-6)[..., None], 1, 255), 0).astype(np.uint8)
    os.makedirs(cache, exist_ok=True)
    np.save(f, acc)
    return acc


def lonlat_to_utm(lon, lat, zone, south):
    from rasterio.warp import transform
    xs, ys = transform("EPSG:4326", f"EPSG:{(32700 if south else 32600) + zone}", [lon], [lat])
    return xs[0], ys[0]


def build(cache, name):
    zone, south, centre, lonlat, lift, near = PLACES[name]
    if centre is None:
        centre = lonlat_to_utm(lonlat[0], lonlat[1], zone, south)
    print(name, "middle", utm_to_lonlat(np.array([centre[0]]), np.array([centre[1]]), zone, south))
    one(cache, name, zone, south, centre, 200000.0, lift, edge=True)
    if near:
        one(cache, name + "-near", zone, south, centre, 20000.0, lift, edge=False)


def one(cache, name, zone, south, centre, side, lift, edge):
    s = read(cache, zone, south, centre, side, name).astype(np.float32) / 255.0
    missing = s.sum(-1) == 0
    lum = s @ np.array([0.3, 0.59, 0.11], np.float32)
    s = np.clip((s - lift[0]) * lift[1], 0, 1)
    s = np.clip(lum[..., None] + (s - lum[..., None]) * lift[2], 0, 1)
    # the month's ground as Terra draws it, at each pixel's place
    m = (np.arange(PX) + 0.5) / PX - 0.5
    gx, gy = np.meshgrid(centre[0] + m * side, centre[1] - m * side)
    lon, lat = utm_to_lonlat(gx, gy, zone, south)
    bm = np.asarray(Image.open(ROOT / "Globe" / "ground" / "08.jpg").convert("RGB"), np.float32) / 255.0
    bx = np.clip(((lon + 180) / 360 * bm.shape[1]).astype(int), 0, bm.shape[1] - 1)
    by = np.clip(((90 - lat) / 180 * bm.shape[0]).astype(int), 0, bm.shape[0] - 1)
    ground = bm[by, bx]
    sigma = 8000.0 / (side / PX)
    low = lambda im: np.stack([ndimage.gaussian_filter(im[..., c], sigma) for c in range(3)], -1)
    s[missing] = ground[missing]
    # toward the edge, the broad colour the ground's, the detail the satellite's
    r = np.maximum(np.abs(gx - centre[0]), np.abs(gy - centre[1]))
    w = np.clip((r - 0.15 * side) / (0.46 * side - 0.15 * side), 0, 1)
    w = (w * w * (3 - 2 * w))[..., None] * (1.0 if edge else 0.0)
    out = np.clip(s + w * (low(ground) - low(s)), 0, 1)
    dst = ROOT / "toys" / "terra" / "data" / f"{name}.jpg"
    Image.fromarray((out * 255 + 0.5).astype(np.uint8)).save(dst, quality=88)
    print("wrote", dst, os.path.getsize(dst) // 1024, "KB; missing", int(missing.sum()), "px")


if __name__ == "__main__":
    for name in sys.argv[2:] or PLACES:
        build(sys.argv[1], name)
