"""Builds the clouds pack's sky: where clouds usually are, month by month, from satellites.

    python3 tools/playground/clouds/build.py <cache> [--fetch [--discard]]

The source is ISCCP-H, the International Satellite Cloud Climatology Project's gridded monthly means
(HGM, version 1.0, NOAA NCEI; "no constraints on data access or use"): 1-degree cells, every
satellite's three-hourly views combined. This averages its months from January 2000 to December 2016
into one typical January, February, ..., December. `<cache>` holds the HGM files (named as NCEI names
them); --fetch downloads the missing ones from NCEI first (204 files, about 4 MB each; --discard
deletes each once read). The running sums are kept in `<cache>/sums-2000-2016.npz`, so a second
build reads no files.

Out, the same files twice: for the native prototype (App/TerraIncognita/Clouds/CloudsSky.png and
CloudsLand.png, decoded by Clouds.metal) and for its web twin (Prototypes/clouds-web/sky.png and
land.png, decoded by shader.js, the same line for line). The sky, 1448 x 1092 RGB:

    a 4 x 6 grid of 362 x 182 tiles, each one month's 360 x 180 cells (north up, west edge 180 W)
    with a one-cell border repeated from the opposite side (east-west) or the edge (north, south),
    so bilinear sampling wraps; tile (m % 4, m // 4) holds month m (0 = January) of
      rows 0-2:  R  cloud amount, 0-1 of the time a cell is seen cloudy
                 G  stratocumulus and stratus: low clouds thick enough to hide the ground
                 B  altostratus and nimbostratus: the same, at middle heights
      rows 3-5:  R  cirrostratus: the same, high
                 G  deep convection: thick and reaching high
                 B  cumulus: low and thin
    each a share of the typed clouds; the rest are thin and above the low ones (altocumulus, cirrus).

And the land, 2048 x 1024 grey, 255 on land: the globe's own coast (the sign of
Globe/geo.png's distance to it), so that a shadow falling on land takes the land's shadow colour.

ISCCP types a cloud by the pressure at its top (low below the 680 hPa level, high above 440) and its
optical thickness (thin below 3.55, thick above 22.63). Clouds are typed only in daylight (thickness
needs the visible channel), so the shares are of the typed clouds; the amount counts day and night. Missing cells (a handful) take their
neighbours' mean. Every field is smoothed by a Gaussian of about 110 km (a cell's height: the
grid's blocks and its satellites' seams), widening to about 330 km towards the poles, the same
distance east-west as north-south: cloud over snow and ice is the least certain part of the record,
and the smoothing keeps its blocks and seams from looking like weather.
"""
import argparse, sys, time, urllib.request
from pathlib import Path
import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[3]
SKY = [ROOT / 'App' / 'TerraIncognita' / 'Clouds' / 'CloudsSky.png', ROOT / 'Prototypes' / 'clouds-web' / 'sky.png']
LAND = [ROOT / 'App' / 'TerraIncognita' / 'Clouds' / 'CloudsLand.png', ROOT / 'Prototypes' / 'clouds-web' / 'land.png']
NCEI = ('https://www.ncei.noaa.gov/data/international-satellite-cloud-climate-project-isccp-h-series-data/'
        'access/isccp-basic/hgm/')
YEARS = range(2000, 2017)
TILE_W, TILE_H = 362, 182


def name(y, m):
    return f'ISCCP-Basic.HGM.v01r00.GLOBAL.{y}.{m:02d}.99.9999.GPC.10KM.CS00.EA1.00.nc'


def fetch(path, url):
    for attempt in range(4):
        try:
            with urllib.request.urlopen(url, timeout=180) as r, open(path.with_suffix('.part'), 'wb') as f:
                f.write(r.read())
            path.with_suffix('.part').rename(path)
            return
        except OSError as e:
            if attempt == 3:
                raise SystemExit(f'cannot fetch {url}: {e}')
            time.sleep(2 ** (attempt + 1))


def field(f, key):
    d = f[key]
    raw = d[...]
    a = raw.astype(np.float64) * d.attrs.get('scale_factor', [1])[0] + d.attrs.get('add_offset', [0])[0]
    fill = d.attrs.get('_FillValue', [None])[0]
    if fill is not None:
        a[raw == fill] = np.nan
    return a


def sums(cache, download, discard):
    """Per calendar month: sums of amount, of each typed share's amount, and of log thickness."""
    keep = cache / f'sums-{YEARS[0]}-{YEARS[-1]}.npz'
    if keep.exists():
        return dict(np.load(keep))
    import h5py
    S = {k: np.zeros((12, 180, 360)) for k in ('tot', 'low', 'mid', 'high', 'deep', 'cu', 'typed')}
    N = {k: np.zeros((12, 180, 360), np.int32) for k in ('tot', 'typed')}
    for y in YEARS:
        for m in range(1, 13):
            path = cache / name(y, m)
            if not path.exists():
                if not download:
                    raise SystemExit(f'{path.name} is not in {cache} (--fetch downloads it)')
                fetch(path, NCEI + path.name)
            with h5py.File(path, 'r') as f:
                tot = field(f, 'cldamt')[0] / 100
                types = field(f, 'cldamt_types')[0] / 100
            if discard:
                path.unlink()
            i = m - 1
            ok = ~np.isnan(tot)
            S['tot'][i][ok] += tot[ok]
            N['tot'][i] += ok
            typed = np.nansum(types, 0)
            okt = ~np.all(np.isnan(types), 0) & (typed > 0.005)
            # ISCCP's 18 types: low, middle, high; thin, medium, thick; liquid, then ice (its labels)
            parts = {'low': types[[1, 2, 4, 5]], 'mid': types[[7, 8, 10, 11]], 'high': types[[13, 16]],
                     'deep': types[[14, 17]], 'cu': types[[0, 3]]}
            for k, v in parts.items():
                S[k][i][okt] += np.nansum(v, 0)[okt]
            S['typed'][i][okt] += typed[okt]
            N['typed'][i] += okt
            print(f'{y}-{m:02d}', file=sys.stderr)
    out = {**{'S_' + k: v for k, v in S.items()}, **{'N_' + k: v for k, v in N.items()}}
    np.savez_compressed(keep, **out)
    return out


def fill(a):
    """Missing cells take the mean of their present neighbours, repeatedly, until none are missing."""
    a = a.copy()
    for _ in range(50):
        miss = np.isnan(a)
        if not miss.any():
            break
        p = np.pad(np.where(miss, 0, a), ((1, 1), (0, 0)), mode='edge')
        w = np.pad((~miss).astype(float), ((1, 1), (0, 0)), mode='edge')
        s = sum(np.roll(p, d, 1)[1 + dy:181 + dy] for dy in (-1, 0, 1) for d in (-1, 0, 1))
        n = sum(np.roll(w, d, 1)[1 + dy:181 + dy] for dy in (-1, 0, 1) for d in (-1, 0, 1))
        a[miss] = np.where(n[miss] > 0, s[miss] / np.maximum(n[miss], 1), np.nan)
    return a


def smooth(a):
    """Gaussian of one cell's height (three towards the poles), the same distance east-west: there
    a row's cells are narrower, so it spans more of them (wrapping round)."""
    lat = np.radians(np.arange(89.5, -90, -1))               # north up
    sigma = 1 + 2 * np.clip((np.degrees(np.abs(lat)) - 55) / 15, 0, 1)
    across = np.minimum(sigma / np.maximum(np.cos(lat), 0.02), 120)
    f = np.fft.rfftfreq(360)
    a = np.fft.irfft(np.fft.rfft(a, axis=1) * np.exp(-2 * (np.pi * f[None] * across[:, None]) ** 2), 360, axis=1)
    out = np.empty_like(a)
    p = np.pad(a, ((9, 9), (0, 0)), mode='edge')
    for i in range(180):
        k = np.exp(-0.5 * (np.arange(-9, 10) / sigma[i]) ** 2)
        out[i] = (k[:, None] * p[i:i + 19]).sum(0) / k.sum()
    return out


def tile(a):
    """One month's 180 x 360 field, north up and starting at 180 W, with its wrapping border."""
    t = np.zeros((TILE_H, TILE_W))
    t[1:-1, 1:-1] = a
    t[1:-1, 0], t[1:-1, -1] = a[:, -1], a[:, 0]
    t[0], t[-1] = t[1], t[-2]
    return t


def main():
    p = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    p.add_argument('cache', type=Path)
    p.add_argument('--fetch', action='store_true', help='download missing months from NCEI')
    p.add_argument('--discard', action='store_true', help='delete each downloaded month once read')
    o = p.parse_args()
    o.cache.mkdir(parents=True, exist_ok=True)
    s = sums(o.cache, o.fetch, o.discard)
    div = lambda a, b: np.where(b > 0, a / np.maximum(b, 1e-9), np.nan)
    # ISCCP's rows run south to north and its columns from 0.5 E: north up, from 180 W
    orient = lambda a: np.roll(a[::-1], 180, axis=1)
    atlas = np.zeros((6 * TILE_H, 4 * TILE_W, 3))
    for m in range(12):
        amount = div(s['S_tot'][m], s['N_tot'][m])
        typed = s['S_typed'][m]
        share = {k: div(s['S_' + k][m], typed) for k in ('low', 'mid', 'high', 'deep', 'cu')}
        a = [amount, share['low'], share['mid']]
        b = [share['high'], share['deep'], share['cu']]
        x, y = (m % 4) * TILE_W, (m // 4) * TILE_H
        for c in range(3):
            atlas[y:y + TILE_H, x:x + TILE_W, c] = tile(np.clip(smooth(fill(orient(a[c]))), 0, 1))
            atlas[3 * TILE_H + y:3 * TILE_H + y + TILE_H, x:x + TILE_W, c] = tile(np.clip(smooth(fill(orient(b[c]))), 0, 1))
    for out in SKY:
        out.parent.mkdir(parents=True, exist_ok=True)
        Image.fromarray(np.round(atlas * 255).astype(np.uint8), 'RGB').save(out, optimize=True)
        print(f'{out.relative_to(ROOT)}: {out.stat().st_size:,} bytes', file=sys.stderr)
    # the land: the globe's coarse distance to the coast (G, 128 + d / 2) is above 128 on land
    geo = np.asarray(Image.open(ROOT / 'Globe' / 'geo.png'))
    land = (geo[..., 1] > 128) | ((geo[..., 1] == 128) & (geo[..., 0] > 128))
    half = land.reshape(1024, 2, 2048, 2).mean((1, 3))
    for out in LAND:
        Image.fromarray(np.round(half * 255).astype(np.uint8), 'L').save(out, optimize=True)
        print(f'{out.relative_to(ROOT)}: {out.stat().st_size:,} bytes', file=sys.stderr)


if __name__ == '__main__':
    main()
