#!/usr/bin/env python3
"""Builds Battle Ink's ground: Cannae's plain, the Aufidus and the ridge, on the toy's own sheet.

    python3 tools/toys/battle/build.py [<tile cache dir>]     # writes toys/battle/data/ground.png, ground.json

Terrarium elevation tiles (AWS open data, s3.amazonaws.com/elevation-tiles-prod/terrarium, SRTM and
other sources; metres = R*256 + G + B/256 - 32768) at zoom 14 (about 9.5 m a pixel here), x 8922..8931,
y 6120..6129: the lower Ofanto from below Canosa to the plain north-west of Canne della Battaglia. The
tiles are fetched into the cache directory (default ~/.cache/battle-tiles) when missing.

The river is traced, not drawn: the cheapest path along the valley floor on lightly smoothed elevation
(a Dijkstra path whose cost grows with the square of height above the floor), between the valley's two
ends, then smoothed along its length. It is the modern floodplain's lowest line; the river of 216 BC
is not known to have run there (SOURCES.md).

The sheet is turned as the spikes turned it (painting-company row 206): the river's downstream course
points up the screen, so the Roman front, facing south-west, faces down it with the river on its right
(the screen's left), Cannae's ridge on the screen's right. The toy's world unit is 5200/1376 m (3.78 m),
the spikes' field pixel, so their tuned deployment and rules keep their meaning; the river crosses
x = 0.16 * 1376 at y = 516.

ground.png: 16-bit grey + alpha, a cell 2 world units, x from X0, y from Y0 (ground.json):
  grey   elevation in centimetres
  alpha  the river's signed distance in world units, (d + 320) * 100 (negative: the far bank, left)
ground.json: the grid, metres a unit, the river's line (world units, every ~6 units) and Cannae's hill.

Never hand-edit the output; rebuild it.
"""
import heapq, json, math, os, sys, urllib.request
from pathlib import Path
import numpy as np
from PIL import Image
from scipy import ndimage

ROOT = Path(__file__).resolve().parents[3]
OUT = ROOT / 'toys' / 'battle' / 'data'
Z, X0T, Y0T, NT = 14, 8922, 6120, 10
FW, FH = 1376, 1032                   # the spikes' field, in world units
MPP = 40075016 * math.cos(math.radians(41.3)) / (256 * 2 ** Z)
UNIT_M = 5200 / FW                    # metres a world unit
GX0, GY0, GX1, GY1, CELL = -400, -400, 1500, 1300, 2


def mosaic(cache):
    cache.mkdir(parents=True, exist_ok=True)
    rows = []
    for y in range(Y0T, Y0T + NT):
        r = []
        for x in range(X0T, X0T + NT):
            p = cache / f'{Z}_{x}_{y}.png'
            if not p.exists():
                urllib.request.urlretrieve(f'https://s3.amazonaws.com/elevation-tiles-prod/terrarium/{Z}/{x}/{y}.png', p)
            a = np.asarray(Image.open(p).convert('RGB')).astype(float)
            r.append(a[..., 0] * 256 + a[..., 1] + a[..., 2] / 256 - 32768)
        rows.append(np.hstack(r))
    return np.vstack(rows)


def thalweg(E, a, b):
    S = ndimage.gaussian_filter(E, 4)
    floor = ndimage.minimum_filter(S, 60)
    cost = np.maximum(S - floor, 0) ** 2 + 0.5
    h, w = E.shape
    dist = np.full(E.shape, np.inf)
    prev = -np.ones(E.shape, np.int64)
    dist[a[1], a[0]] = 0
    q = [(0.0, a[0], a[1])]
    while q:
        d, x, y = heapq.heappop(q)
        if (x, y) == b:
            break
        if d > dist[y, x]:
            continue
        for dx in (-1, 0, 1):
            for dy in (-1, 0, 1):
                if dx or dy:
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < w and 0 <= ny < h:
                        nd = d + cost[ny, nx] * math.hypot(dx, dy)
                        if nd < dist[ny, nx]:
                            dist[ny, nx] = nd
                            prev[ny, nx] = y * w + x
                            heapq.heappush(q, (nd, nx, ny))
    path = [b]
    while path[-1] != a:
        k = prev[path[-1][1], path[-1][0]]
        path.append((int(k % w), int(k // w)))
    p = np.array(path[::-1], float)
    return np.stack([ndimage.gaussian_filter1d(p[:, 0], 10, mode='nearest'),
                     ndimage.gaussian_filter1d(p[:, 1], 10, mode='nearest')], 1)


def main():
    cache = Path(sys.argv[1]) if len(sys.argv) > 1 else Path.home() / '.cache' / 'battle-tiles'
    E = mosaic(cache)
    river = thalweg(E, (240, 1800), (2200, 460))
    ang = math.atan2(river[-1, 1] - river[0, 1], river[-1, 0] - river[0, 0])
    rot = -math.pi / 2 - ang                       # downstream points up the screen
    scale = UNIT_M / MPP                           # tile px a world unit
    cannae = np.array([1301.0, 1257.0])            # Canne della Battaglia, 41.2958 N 16.1517 E
    rc = river[np.argmin(np.linalg.norm(river - cannae, axis=1))]
    c, s = math.cos(-rot), math.sin(-rot)
    ax, ay = FW * 0.16, FH * 0.5

    def to_tile(wx, wy):
        u, v = (wx - ax) * scale, (wy - ay) * scale
        return rc[0] + c * u - s * v, rc[1] + s * u + c * v

    ci, si = math.cos(rot), math.sin(rot)

    def to_world(p):
        dx, dy = p[..., 0] - rc[0], p[..., 1] - rc[1]
        return np.stack([(ci * dx - si * dy) / scale + ax, (si * dx + ci * dy) / scale + ay], -1)

    gw, gh = (GX1 - GX0) // CELL + 1, (GY1 - GY0) // CELL + 1
    ys, xs = np.mgrid[0:gh, 0:gw]
    wx, wy = GX0 + xs * CELL, GY0 + ys * CELL
    tx, ty = to_tile(wx, wy)
    elev = ndimage.map_coordinates(ndimage.gaussian_filter(E, 1.2), [ty, tx], order=3)
    # the river in world units, every ~6 units, sorted down the screen
    rw = to_world(river)
    keep = [0]
    for k in range(1, len(rw)):
        if np.hypot(*(rw[k] - rw[keep[-1]])) >= 6:
            keep.append(k)
    rw = rw[keep]
    rw = rw[(rw[:, 1] > GY0 - 200) & (rw[:, 1] < GY1 + 200)]
    rw = rw[np.argsort(rw[:, 1])]
    # its signed distance: unsigned from a dense raster, signed by the side of the line
    mask = np.zeros((gh, gw), bool)
    for k in range(len(rw) - 1):
        for t in np.linspace(0, 1, 12):
            p = rw[k] * (1 - t) + rw[k + 1] * t
            i, j = int(round((p[0] - GX0) / CELL)), int(round((p[1] - GY0) / CELL))
            if 0 <= i < gw and 0 <= j < gh:
                mask[j, i] = True
    d = ndimage.distance_transform_edt(~mask) * CELL
    rx = np.interp(wy[:, 0], rw[:, 1], rw[:, 0])
    d = np.where(wx < rx[:, None], -d, d)
    d = np.clip(d, -319, 319)
    L = np.clip(np.round(elev * 100), 0, 65535).astype(np.uint16)
    A = np.round((d + 320) * 100).astype(np.uint16)
    OUT.mkdir(parents=True, exist_ok=True)
    # 16-bit grey + alpha (PIL writes LA only at 8 bits; numpy -> raw PNG through a tiny writer)
    write_la16(OUT / 'ground.png', L, A)
    cw = to_world(cannae[None])[0]
    meta = dict(source='Terrarium z14 (AWS open data); built by tools/toys/battle/build.py, never hand-edited',
                x0=GX0, y0=GY0, cell=CELL, w=int(gw), h=int(gh), unit_m=round(UNIT_M, 4),
                elev_m=[round(float(elev.min()), 1), round(float(elev.max()), 1)],
                river=[[round(float(x), 1), round(float(y), 1)] for x, y in rw],
                cannae=[round(float(cw[0]), 1), round(float(cw[1]), 1)])
    (OUT / 'ground.json').write_text(json.dumps(meta, separators=(',', ':')) + '\n')
    print('grid', gw, gh, 'elev', elev.min(), elev.max(), 'river pts', len(rw), 'cannae', cw)


def write_la16(path, L, A):
    import struct, zlib
    h, w = L.shape
    px = np.empty((h, w, 2), '>u2')
    px[..., 0], px[..., 1] = L, A
    raw = b''.join(b'\x00' + px[y].tobytes() for y in range(h))

    def chunk(t, data):
        c = struct.pack('>I', len(data)) + t + data
        return c + struct.pack('>I', zlib.crc32(t + data) & 0xffffffff)
    png = b'\x89PNG\r\n\x1a\n' + chunk(b'IHDR', struct.pack('>IIBBBBB', w, h, 16, 4, 0, 0, 0))
    png += chunk(b'IDAT', zlib.compress(raw, 9)) + chunk(b'IEND', b'')
    path.write_bytes(png)


if __name__ == '__main__':
    main()
