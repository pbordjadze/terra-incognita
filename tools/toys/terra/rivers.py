#!/usr/bin/env python3
"""The world's rivers for the kit's globe (painting-company row 258: discovery through scale): every
river reach of HydroSHEDS' HydroRIVERS v1.0 whose upstream area is at least 300 km2, simplified and
packed so the globe can show the great rivers from afar and, closer, more and more of their tributaries.

    python3 tools/toys/terra/rivers.py <cache-dir>      # writes toys/terra/data/rivers.bin

The cache holds HydroRIVERS_v10_shp/ (unzipped from
https://data.hydrosheds.org/file/HydroRIVERS/HydroRIVERS_v10_shp.zip, 544 MB; fetched if missing).
Needs pyshp and numpy.

The file (little-endian), read by toy-kit's globe/rivers.rs:

    b"TRV1"
    u32 FINE_W, u32 FINE_H        the fine tiles, 5 degrees: 72 x 36
    u32 COARSE_W, u32 COARSE_H    the coarse tiles, 10 degrees: 36 x 18
    u32 Q_COARSE                  the least importance in the coarse layer
    coarse table: COARSE_W*COARSE_H x (u32 offset, u32 bytes)   offsets from the start of the file
    fine table:   FINE_W*FINE_H x (u32 offset, u32 bytes)
    the tiles' reaches

A tile (row from the north, column from 180 W) is its reaches, most important first, each:

    u8 q          importance: round(8 log2(upstream area, km2))
    varint n      points (>= 2)
    the points: zigzag varints in units of 2e-4 degree (lon, lat), each from the one before; the first
                from the tile's previous reach's first point (from 0, 0 for the tile's first)

A reach belongs to the tile of its first point. The coarse layer holds the reaches of q >= Q_COARSE
simplified to 0.012 degree (they are seen from afar); the fine tiles hold every reach (q >= 8 log2 300)
simplified to 0.002 degree (about 220 m, a point at the globe's closest). Every line is first smoothed
by three rounds of corner cutting (HydroSHEDS traces a 15-second grid: unsmoothed, its rivers zig-zag).
"""
import struct
import sys
import urllib.request
import zipfile
from pathlib import Path

import numpy as np
import shapefile

ROOT = Path(__file__).resolve().parents[3]
URL = 'https://data.hydrosheds.org/file/HydroRIVERS/HydroRIVERS_v10_shp.zip'
MIN_AREA = 300.0
COARSE_AREA = 2000.0
FINE_EPS, COARSE_EPS = 0.002, 0.012
UNIT = 2e-4
FW, FH, CW, CH = 72, 36, 36, 18


def q_of(area):
    return round(float(8 * np.log2(max(area, 1.0))))


def chaikin(p, n=3):
    """Corner cutting, the ends kept: HydroSHEDS' lines follow a 15-second grid and zig-zag along it."""
    for _ in range(n):
        if len(p) < 3:
            return p
        a, b = p[:-1], p[1:]
        q = np.empty((2 * len(a), 2))
        q[0::2] = 0.75 * a + 0.25 * b
        q[1::2] = 0.25 * a + 0.75 * b
        q[0], q[-1] = p[0], p[-1]
        p = q
    return p


def rdp(p, eps):
    """Douglas-Peucker on lon/lat, longitude scaled by the cosine of the latitude."""
    if len(p) <= 2:
        return p
    s = p.copy()
    s[:, 0] *= np.cos(np.radians(p[:, 1].mean()))
    keep = np.zeros(len(p), bool)
    keep[0] = keep[-1] = True
    stack = [(0, len(p) - 1)]
    while stack:
        a, b = stack.pop()
        if b <= a + 1:
            continue
        d = s[b] - s[a]
        L = np.hypot(*d)
        q = s[a + 1:b] - s[a]
        dist = np.abs(q[:, 0] * d[1] - q[:, 1] * d[0]) / L if L > 0 else np.hypot(q[:, 0], q[:, 1])
        i = int(np.argmax(dist))
        if dist[i] > eps:
            m = a + 1 + i
            keep[m] = True
            stack += [(a, m), (m, b)]
    return p[keep]


def varint(out, v):
    while True:
        b = v & 0x7F
        v >>= 7
        if v:
            out.append(b | 0x80)
        else:
            out.append(b)
            return


def zz(v):
    return (v << 1) ^ (v >> 63)


def encode(out, q, pts, start):
    """A reach; its first point from `start` (the tile's previous reach's first point), returned."""
    out.append(q)
    varint(out, len(pts))
    u = np.round(pts / UNIT).astype(np.int64)
    prev = start
    first = (int(u[0, 0]), int(u[0, 1]))
    for x, y in u:
        varint(out, zz(int(x) - prev[0]))
        varint(out, zz(int(y) - prev[1]))
        prev = (int(x), int(y))
    return first


def tile_of(lon, lat, w, h):
    c = min(int((lon + 180) / 360 * w), w - 1)
    r = min(int((90 - lat) / 180 * h), h - 1)
    return r * w + c


def main():
    cache = Path(sys.argv[1])
    shp = cache / 'HydroRIVERS_v10_shp' / 'HydroRIVERS_v10'
    if not shp.with_suffix('.shp').exists():
        z = cache / 'HydroRIVERS_v10_shp.zip'
        if not z.exists():
            print('fetching', URL, flush=True)
            urllib.request.urlretrieve(URL, z)
        zipfile.ZipFile(z).extractall(cache)
    r = shapefile.Reader(str(shp))
    areas = np.array([rec[0] for rec in r.iterRecords(fields=['UPLAND_SKM'])], np.float64)
    chosen = np.nonzero(areas >= MIN_AREA)[0]
    print(len(areas), 'reaches,', len(chosen), 'of at least', MIN_AREA, 'km2', flush=True)
    fine = [[] for _ in range(FW * FH)]
    coarse = [[] for _ in range(CW * CH)]
    for k, i in enumerate(chosen):
        if k % 200000 == 0:
            print(k, flush=True)
        pts = np.array(r.shape(int(i)).points, np.float64)
        if len(pts) < 2:
            continue
        q = q_of(areas[i])
        pts = chaikin(pts)
        f = rdp(pts, FINE_EPS)
        fine[tile_of(f[0, 0], f[0, 1], FW, FH)].append((q, f))
        if areas[i] >= COARSE_AREA:
            c = rdp(pts, COARSE_EPS)
            coarse[tile_of(c[0, 0], c[0, 1], CW, CH)].append((q, c))
    head = bytearray(b'TRV1') + struct.pack('<5I', FW, FH, CW, CH, q_of(COARSE_AREA))
    table_at = len(head)
    body = bytearray()
    base = table_at + 8 * (CW * CH + FW * FH)
    table = []
    npts = 0
    for tiles in (coarse, fine):
        for t in tiles:
            t.sort(key=lambda e: (-e[0], e[1][0, 0]))
            start = len(body)
            first = (0, 0)
            for q, p in t:
                first = encode(body, q, p, first)
                npts += len(p)
            table.append((base + start, len(body) - start))
    data = head + b''.join(struct.pack('<2I', o, n) for o, n in table) + body
    out = ROOT / 'toys' / 'terra' / 'data' / 'rivers.bin'
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(data)
    print(f'{out}: {len(data) / 1e6:.1f} MB, {npts} points', flush=True)


if __name__ == '__main__':
    main()
