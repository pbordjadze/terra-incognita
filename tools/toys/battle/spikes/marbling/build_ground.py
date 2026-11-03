#!/usr/bin/env python3
"""Builds ground.png, Cannae's real ground beneath the marbling tray.

Run: python3 build_ground.py   (fetches 9 Terrarium z13 tiles round 41.30 N 16.13 E; needs numpy, scipy, PIL)

The ground is turned (north is not up) so the account's arrangement reads on screen: the Ofanto's valley
runs top to bottom, the river near the left third, the Cannae rise beyond it at the far left, the plain to
the right where the armies lie. ground.png: R hillshade, G river mask, B elevation (normalised).
The river's line is the valley floor's lowest line (smoothed); its meanders are drawn, not measured.
"""
import io, os, urllib.request
import numpy as np
from PIL import Image
from scipy import ndimage as nd

HERE = os.path.dirname(os.path.abspath(__file__))
rows = []
for y in (3061, 3062, 3063):
    row = []
    for x in (4462, 4463, 4464):
        url = f"https://s3.amazonaws.com/elevation-tiles-prod/terrarium/13/{x}/{y}.png"
        a = np.asarray(Image.open(io.BytesIO(urllib.request.urlopen(url).read())).convert("RGB")).astype(float)
        row.append(a)
    rows.append(np.hstack(row))
a = np.vstack(rows)
e = a[..., 0] * 256 + a[..., 1] + a[..., 2] / 256 - 32768
r = nd.rotate(nd.gaussian_filter(e, 1.2), 45, reshape=False, order=1, mode="nearest")
c = r[210:510, 32:432][::-1, ::-1].copy()  # 300x400 px at ~14 m: ~5.6 x 4.2 km
H, W = c.shape
s = nd.gaussian_filter(c, 1.3)
gy, gx = np.gradient(s, 14.0)
az, alt = np.radians(315), np.radians(40)
slope = np.arctan(np.hypot(gx, gy) * 2.5)
asp = np.arctan2(-gx, gy)
hs = np.clip(np.sin(alt) * np.cos(slope) + np.cos(alt) * np.sin(slope) * np.cos(az - asp), 0, 1)
el = np.clip((c - c.min()) / (c.max() - c.min()), 0, 1)

cs = nd.gaussian_filter(c, 10)
xs = np.array([np.argmin(cs[y, 60:200]) + 60 for y in range(H)], float)
xs = nd.gaussian_filter1d(xs, 35, mode="nearest")
big = (1200, 900)
up = lambda m: np.asarray(Image.fromarray((m * 255).astype("uint8")).resize(big, Image.BICUBIC)).astype(float) / 255
Y, X = np.mgrid[0:900, 0:1200]
yy = Y[:, 0] / 3.0
xr = (np.interp(yy, np.arange(H), xs) + 14 * np.sin(yy * 0.033) + 6 * np.sin(yy * 0.085 + 1.0) + 2 * np.sin(yy * 0.21)) * 3.0
riv = np.clip(1.0 - (np.abs(X - xr[:, None]) - 4) / 3.0, 0, 1)
img = np.dstack([up(hs), riv, up(el)])
Image.fromarray((img * 255).astype("uint8")).resize((688, 516), Image.LANCZOS).save(os.path.join(HERE, "ground.png"))
print("river x (screen px, every 1/10 of height):", (xr[::90] / 1200 * 1376).astype(int))
