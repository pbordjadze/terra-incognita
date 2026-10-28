"""Builds the ocean's currents: how the sea's surface usually moves, month by month, measured by
drifting buoys.

    python3 tools/ocean/build.py <cache>

The source is the NOAA Global Drifter Program's climatology of near-surface currents (Laurindo,
Mariano and Lumpkin 2017, Deep-Sea Research I 124, doi:10.1016/j.dsr.2017.04.009; version 3.10, data
to February 2023; CC BY 4.0), its monthly means at a quarter of a degree, from NOAA AOML's ERDDAP
(`<cache>/drifter_monthlymeans.nc`, downloaded if missing, about 350 MB). Drogued drifters follow
the water 15 m down; the climatology combines their tracks with satellite altimetry.

Out, two sheets in App/TerraIncognita/Ocean/, decoded by Ocean.metal (and read from there by its web
twin, tools/ocean/web/, whose shader.js is the same line for line).

OceanCurrents.png, 2888 x 1086 RGB:

    a 4 x 3 grid of 722 x 362 tiles, each one month's 720 x 360 half-degree cells (north up, west
    edge 180 W) with a one-cell border repeated from the opposite side (east-west) or the edge
    (north, south), so bilinear sampling wraps; tile (m % 4, m // 4) holds month m (0 = January):
      R  the eastward current, G the northward: 128 + 127 s sqrt(|v| / 2 m/s), s its sign, |v| at
         most 2 m/s (a few cells of the fastest currents, about 2.5 m/s, are clamped)
      B  how measured: 255 where the climatology has a value for the month, falling to 0 within
         about 3 degrees of the nearest one (where no drifter went: under the polar ice, in some
         gulfs); the velocity there is smoothed in from the measured values, so the coasts'
         currents run on to the shore

OceanStream.png, the same grid: the month's stream function, whose contours are the
paths the water takes and lie closer together where it runs faster (the flow's rotational part,
which carries most of it: the currents differ from its gradient's by a correlation of about 0.95
east-west and 0.85 north-south; the rest is the sea rising and sinking, as at the equator):
      R, G  (psi + 0.2) / 0.4 as sixteen bits, high byte then low (psi on the unit sphere, in m/s
            radians: u = -dpsi/dlat, v = dpsi/dlon / cos lat); read four texels and interpolated
            after joining the bytes, never filtered as bytes
      B  0

Each month's quarter-degree means are averaged into half-degree cells (a cell is measured if any
of its four is), after a Gaussian of a quarter of a degree, which takes the grid's speckle out of
the quiet gyres and leaves the jets (the Gulf Stream is about a degree wide).
"""
import sys, urllib.request
from pathlib import Path
import numpy as np
from PIL import Image
from scipy import ndimage
from scipy.io import netcdf_file
from scipy.linalg import solve_banded

ROOT = Path(__file__).resolve().parents[2]
OUT = [ROOT / 'App' / 'TerraIncognita' / 'Ocean' / 'OceanCurrents.png']
STREAM = [ROOT / 'App' / 'TerraIncognita' / 'Ocean' / 'OceanStream.png']
Q = '%5B(0):1:(11)%5D%5B(-179.875):1:(179.875)%5D%5B(-72.875):1:(84.875)%5D'
URL = ('https://erddap.aoml.noaa.gov/gdp/erddap/griddap/drifter_monthlymeans.nc?'
       + ','.join(v + Q for v in ('U', 'V', 'eU', 'eV')))
VMAX = 2.0                     # m/s, the encoding's range
W, H = 720, 360                # a month at half a degree
TILE_W, TILE_H = W + 2, H + 2


def fetch(path):
    if path.exists():
        return
    print('downloading', URL[:80] + '...')
    with urllib.request.urlopen(URL, timeout=900) as r, open(path.with_suffix('.part'), 'wb') as f:
        while chunk := r.read(1 << 20):
            f.write(chunk)
    path.with_suffix('.part').rename(path)


def read(path):
    """U and V, (12, 720, 1440): latitude down from 89.875, longitude from -179.875; NaN unmeasured."""
    f = netcdf_file(path, 'r', mmap=False)
    lat = f.variables['latitude'][:].astype(float)
    out = []
    for name in ('U', 'V'):
        a = f.variables[name][:].astype(np.float32)            # (month, lon, lat ascending)
        a = np.transpose(a, (0, 2, 1))[:, ::-1, :]               # (month, lat descending, lon)
        full = np.full((12, 720, 1440), np.nan, np.float32)
        top = int(round((89.875 - lat.max()) / 0.25))
        full[:, top:top + a.shape[1], :] = a
        out.append(full)
    f.close()
    return out


def smooth(a, sigma):
    """A Gaussian, wrapping east-west."""
    return ndimage.gaussian_filter(a, sigma, mode=('nearest', 'wrap'))


def fill(u, v):
    """Fills unmeasured cells from the measured ones, near first (normalised convolution at widening
    scales); returns the filled fields and how measured each cell is, 0 to 1."""
    known = np.isfinite(u) & np.isfinite(v)
    w = known.astype(np.float32)
    u0, v0 = np.where(known, u, 0), np.where(known, v, 0)
    fu, fv = np.where(known, u, np.nan), np.where(known, v, np.nan)
    for sigma in (1, 2, 4, 8, 16, 32, 64, 128):
        s = smooth(w, sigma)
        ok = np.isnan(fu) & (s > 1e-3)
        fu[ok] = (smooth(u0, sigma) / np.maximum(s, 1e-6))[ok]
        fv[ok] = (smooth(v0, sigma) / np.maximum(s, 1e-6))[ok]
    fu, fv = np.nan_to_num(fu), np.nan_to_num(fv)
    # (distance in quarter degrees to the nearest measured cell, east-west wrapping round)
    wide = np.concatenate([~known[:, -360:], ~known, ~known[:, :360]], axis=1)
    far = ndimage.distance_transform_edt(wide)[:, 360:-360]
    measured = np.clip(1 - far / 12, 0, 1)
    return fu, fv, measured


def stream(u, v):
    """The stream function of a half-degree field on the unit sphere (rows north to south): Poisson's
    equation for the vorticity, spectral in longitude, tridiagonal in latitude (no flux through the
    poles); mean zero."""
    dl, dp = 2 * np.pi / W, np.pi / H
    lat = np.deg2rad(90 - (np.arange(H) + 0.5) * 180 / H)
    cos = np.cos(lat)[:, None]
    dvdl = (np.roll(v, -1, 1) - np.roll(v, 1, 1)) / (2 * dl)
    uc = u * cos
    duc = np.empty_like(uc)
    duc[1:-1] = (uc[:-2] - uc[2:]) / (2 * dp)
    duc[0], duc[-1] = (uc[0] - uc[1]) / dp, (uc[-2] - uc[-1]) / dp
    zeta = (dvdl - duc) / cos
    Z = np.fft.rfft(zeta, axis=1)
    face = np.cos(np.deg2rad(90 - np.arange(H + 1) * 180 / H))
    face[0] = face[-1] = 0
    out = np.zeros_like(Z)
    for k in range(Z.shape[1]):
        band = np.zeros((3, H), complex)
        band[0, 1:] = face[1:H]
        band[1] = -(face[:-1] + face[1:]) - (2 - 2 * np.cos(k * dl)) / dl ** 2 * dp ** 2 / cos[:, 0] - (1e-6 if k == 0 else 0)
        band[2, :-1] = face[1:H]
        out[:, k] = solve_banded((1, 1), band, Z[:, k] * cos[:, 0] * dp ** 2)
    psi = np.fft.irfft(out, n=W, axis=1)
    return psi - psi.mean()


def encode(x):
    s = np.sign(x) * np.sqrt(np.clip(np.abs(x), 0, VMAX) / VMAX)
    return np.clip(np.round(128 + 127 * s), 1, 255).astype(np.uint8)


def tile(a):
    """A month's cells with the one-cell border round them."""
    a = np.concatenate([a[:, -1:], a, a[:, :1]], axis=1)
    return np.concatenate([a[:1], a, a[-1:]], axis=0)


def main():
    cache = Path(sys.argv[1])
    cache.mkdir(parents=True, exist_ok=True)
    path = cache / 'drifter_monthlymeans.nc'
    fetch(path)
    U, V = read(path)
    sheet = np.zeros((TILE_H * 3, TILE_W * 4, 3), np.uint8)
    flow = np.zeros_like(sheet)
    for m in range(12):
        u, v, measured = fill(U[m], V[m])
        u, v = smooth(u, 1), smooth(v, 1)
        # to half a degree
        u = u.reshape(H, 2, W, 2).mean(axis=(1, 3))
        v = v.reshape(H, 2, W, 2).mean(axis=(1, 3))
        measured = measured.reshape(H, 2, W, 2).max(axis=(1, 3))
        rgb = np.stack([encode(u), encode(v), np.round(255 * measured).astype(np.uint8)], axis=-1)
        x, y = (m % 4) * TILE_W, (m // 4) * TILE_H
        sheet[y:y + TILE_H, x:x + TILE_W] = tile(rgb)
        psi = stream(u, v)
        n = np.clip(np.round((psi + 0.2) / 0.4 * 65535), 0, 65535).astype(np.uint32)
        flow[y:y + TILE_H, x:x + TILE_W] = tile(np.stack([n >> 8, n & 255, 0 * n], axis=-1).astype(np.uint8))
        speed = np.hypot(u, v)
        print(f'month {m + 1:2d}: measured {measured.mean():.2f} of the globe, '
              f'speed median {np.median(speed[measured > 0.5]):.2f}, max {speed.max():.2f} m/s, '
              f'stream {psi.min():+.3f} to {psi.max():+.3f}')
    for outs, data in ((OUT, sheet), (STREAM, flow)):
        for out in outs:
            out.parent.mkdir(parents=True, exist_ok=True)
            Image.fromarray(data, 'RGB').save(out, optimize=True)
            print(out.relative_to(ROOT), f'{out.stat().st_size / 1e6:.1f} MB')


if __name__ == '__main__':
    main()
