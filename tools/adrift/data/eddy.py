"""Builds the eddy wander: how far, by place, a float strays from the mean current each day,
measured from the drifters themselves.

    python3 tools/adrift/data/eddy.py <cache> [years...]

The random part of a float's step is a random walk whose spread grows as 2 K t: K is the lateral
eddy diffusivity. By Taylor (1921), K = <u'^2> T, the velocity variance about the mean times the
Lagrangian integral time scale (the time the current's wanderings stay correlated). Both come from
the NOAA Global Drifter Program (CC BY 4.0):

  <u'^2>, <v'^2>  the climatology's own current variance (`drifter_currentvariance`, Laurindo,
                  Mariano and Lumpkin 2017: the residual about the time-mean, seasonal, spatial
                  gradient and SOI-related currents, quarter-degree cells; with N, the drifter-days
                  that measured each)
  T (zonal, meridional)  measured here from the 6-hourly interpolated trajectories (tracks.py;
                  drogued drifters only; the years given, 2016-2023 by default) in 10 x 10 degree
                  boxes: each drifter's velocity about the climatology's monthly mean at its place,
                  its autocorrelation over lags of 6 hours, integrated to the first zero crossing
                  (Davis 1991; the method of Zhurbas and Oh 2003, 2004), at most 10 days; a box with
                  fewer than 400 drifter-days takes its neighbours' value. The boxes are smoothed
                  over about 15 degrees and multiplied into the variance cell by cell.

Out, adrift/data/eddy.png, 722 x 362 RGB, one tile (sheets.py) of half-degree cells (the variance
is for the whole year; the seasons' change is in the mean current, not here):
      R  the zonal K, G the meridional: 255 sqrt(K / 40000 m^2/s)
      B  how measured: 255 where the variance has a value, falling to 0 within about 3 degrees of
         the nearest one; the values there are smoothed in from the measured cells
and the time scales as `eddy-time.json` beside it (the 10-degree boxes' T, days, for the tests and
the record), and as a sheet, adrift/data/eddy-time.png, 362 x 182 RGB, one tile of 1-degree cells:
      R  the zonal time scale, G the meridional: 255 T / 10 days (the boxes, smoothed, bilinear)
      B  0
so a random flight (a velocity with memory) can read T where it is.

Reading K in a step: a float moving dt days takes a Gaussian step of standard deviation
sqrt(2 K dt) metres on each axis (2 K t is a variance, so the 30-day cloud of a float's maybes has
a root-mean-square radius of sqrt(4 K t) over both axes). Typical K is a few thousand m^2/s: in 30
days a float wanders about 100-200 km from where the mean current alone would carry it.
"""
import json, sys, urllib.request
from pathlib import Path
import numpy as np
import netCDF4
from scipy import ndimage
import tracks
from sheets import OUT, save, tile

ERDDAP = 'https://erddap.aoml.noaa.gov/gdp/erddap/griddap/'
Q2 = '%5B(-179.875):1:(179.875)%5D%5B(-72.875):1:(84.875)%5D'
Q3 = '%5B(0):1:(11)%5D' + Q2
W, H = 720, 360
KMAX = 40000.0
BOX = 10
LAGS = 40                                  # of 6 hours: 10 days
TMAX_DAYS = 10.0
MIN_DAYS = 400
R_EARTH = 6371000.0
SECONDS = 86400.0


def fetch(cache, name, url):
    path = cache / name
    if not path.exists():
        print('fetching', url[:90] + '...', flush=True)
        urllib.request.urlretrieve(url, path)
    return path


def grid(path, names):
    """Quarter-degree fields as (lat descending from 89.875, lon from -179.875), NaN unmeasured."""
    f = netCDF4.Dataset(path)
    lat = np.array(f.variables['latitude'][:])
    out = []
    for n in names:
        a = np.array(f.variables[n][:], dtype=np.float64)
        a = np.moveaxis(a, -1, -2)[..., ::-1, :]                     # (..., lat descending, lon)
        full = np.full(a.shape[:-2] + (720, 1440), np.nan)
        top = int(round((89.875 - lat.max()) / 0.25))
        full[..., top:top + a.shape[-2], :] = a
        out.append(full)
    f.close()
    return out


def sample(field, lat, lon):
    """Bilinear from a quarter-degree (lat descending, lon) field; NaN where any corner is."""
    fy = (89.875 - lat) / 0.25
    fx = (lon + 179.875) / 0.25
    y0 = np.clip(np.floor(fy).astype(int), 0, 718)
    x0 = np.floor(fx).astype(int) % 1440
    ty, tx = fy - y0, fx - np.floor(fx)
    x1 = (x0 + 1) % 1440
    return ((field[..., y0, x0] * (1 - tx) + field[..., y0, x1] * tx) * (1 - ty)
            + (field[..., y0 + 1, x0] * (1 - tx) + field[..., y0 + 1, x1] * tx) * ty)


def residuals(df, U, V):
    """Each 6-hourly point's velocity about the climatology's monthly mean (m/s), zonal and
    meridional; NaN where the point can't be used (a gap, a drogue off, no climatology)."""
    lat, lon, day = df['lat'].values, df['lon'].values, df['day'].values
    same = np.zeros(len(df), bool)
    ids = df['id'].values
    same[1:-1] = (ids[:-2] == ids[1:-1]) & (ids[2:] == ids[1:-1])
    gap = np.zeros(len(df), bool)
    gap[1:-1] = (np.abs(day[2:] - day[:-2] - 0.5) < 1e-3)
    ok = same & gap & df['drogued'].values
    dlon = (np.roll(lon, -1) - np.roll(lon, 1) + 540) % 360 - 180
    dlat = np.roll(lat, -1) - np.roll(lat, 1)
    dt = 0.5 * SECONDS
    u = np.deg2rad(dlon) * R_EARTH * np.cos(np.deg2rad(lat)) / dt
    v = np.deg2rad(dlat) * R_EARTH / dt
    # the month, by the day of a typical year, interpolated between the months' means
    m = ((day % 365.25) - 15.2) / 30.4375          # month m's mean sits at its middle
    m0 = np.floor(m).astype(int) % 12
    m1 = (m0 + 1) % 12
    t = m - np.floor(m)
    um = np.full(len(df), np.nan)
    vm = np.full(len(df), np.nan)
    for m in range(12):
        for mm, weight in ((m0, 1 - t), (m1, t)):
            sel = mm == m
            if not sel.any():
                continue
            a = sample(U[m], lat[sel], lon[sel]) * weight[sel]
            b = sample(V[m], lat[sel], lon[sel]) * weight[sel]
            um[sel] = np.where(np.isnan(um[sel]), a, um[sel] + a)
            vm[sel] = np.where(np.isnan(vm[sel]), b, vm[sel] + b)
    up, vp = u - um, v - vm
    ok &= np.isfinite(up) & np.isfinite(vp)
    up[~ok], vp[~ok] = np.nan, np.nan
    return up, vp


def timescales(df, up, vp):
    """The Lagrangian integral time scale (days), zonal and meridional, in BOX-degree boxes
    (rows north to south), and the drifter-days that measured each."""
    ny, nx = 180 // BOX, 360 // BOX
    by = np.clip(((90 - df['lat'].values) // BOX).astype(int), 0, ny - 1)
    bx = np.clip(((df['lon'].values + 180) // BOX).astype(int), 0, nx - 1)
    box = by * nx + bx
    ids = df['id'].values
    out = {}
    for name, w in (('zonal', up), ('meridional', vp)):
        cov = np.zeros((LAGS + 1, ny * nx))
        n = np.zeros((LAGS + 1, ny * nx))
        for lag in range(LAGS + 1):
            a, b = w[:len(w) - lag], w[lag:]
            good = np.isfinite(a) & np.isfinite(b) & (ids[:len(w) - lag] == ids[lag:])
            p = np.where(good, a * b, 0)
            cov[lag] = np.bincount(box[:len(w) - lag], p, ny * nx)
            n[lag] = np.bincount(box[:len(w) - lag], good, ny * nx)
        with np.errstate(invalid='ignore', divide='ignore'):
            r = (cov / np.maximum(n, 1)) / (cov[0] / np.maximum(n[0], 1))
        T = np.zeros(ny * nx)
        for j in range(ny * nx):
            acc, prev = 0.0, 1.0
            for lag in range(1, LAGS + 1):
                cur = r[lag, j]
                if not np.isfinite(cur) or cur <= 0:
                    acc += 0.5 * prev * 0.25 * (prev / (prev - min(cur, 0))) if np.isfinite(cur) else 0
                    break
                acc += 0.5 * (prev + cur) * 0.25
                prev = cur
            T[j] = min(acc, TMAX_DAYS)
        days = n[0] / 4
        out[name] = (T.reshape(ny, nx), days.reshape(ny, nx))
    return out


def spread(T, days):
    """Boxes with too few drifter-days take the measured ones' smoothed value; all are smoothed
    over about 1.5 boxes, wrapping east-west."""
    w = (days >= MIN_DAYS).astype(float)
    num = ndimage.gaussian_filter(np.where(w > 0, T, 0), 1.5, mode=('nearest', 'wrap'))
    den = ndimage.gaussian_filter(w, 1.5, mode=('nearest', 'wrap'))
    filled = np.where(w > 0, T, num / np.maximum(den, 1e-6))
    filled = np.where(den > 1e-3, filled, np.nanmean(T[w > 0]))
    return ndimage.gaussian_filter(filled, 1.0, mode=('nearest', 'wrap'))


def main():
    cache = Path(sys.argv[1])
    cache.mkdir(parents=True, exist_ok=True)
    years = [int(y) for y in sys.argv[2:]] or list(range(2016, 2024))
    means = fetch(cache, 'drifter_monthlymeans.nc', f'{ERDDAP}drifter_monthlymeans.nc?U{Q3},V{Q3}')
    var = fetch(cache, 'drifter_currentvariance.nc',
                f'{ERDDAP}drifter_currentvariance.nc?Up2bar{Q2},Vp2bar{Q2},N{Q2}')
    U, V = grid(means, ['U', 'V'])
    Up2, Vp2, N = grid(var, ['Up2bar', 'Vp2bar', 'N'])
    df = tracks.load(cache, years)
    up, vp = residuals(df, U, V)
    print(f'{np.isfinite(up).sum() / 4:.0f} drogued drifter-days with a residual; '
          f'rms u\' {np.nanstd(up):.3f}, v\' {np.nanstd(vp):.3f} m/s', flush=True)
    ts = timescales(df, up, vp)
    record = {'box_degrees': BOX, 'years': years, 'lags_hours': 6, 'max_days': TMAX_DAYS}
    T = {}
    for name, (t, days) in ts.items():
        good = days >= MIN_DAYS
        print(f'{name}: T in measured boxes {np.percentile(t[good], 10):.2f} / '
              f'{np.median(t[good]):.2f} / {np.percentile(t[good], 90):.2f} days (10/50/90%), '
              f'{good.sum()} boxes of {good.size}')
        record[name] = {'days': np.round(t, 3).tolist(), 'drifter_days': np.round(days).tolist()}
        T[name] = spread(t, days)
    (OUT / 'eddy-time.json').write_text(json.dumps(record) + '\n')
    # the variance at half a degree, filled where unmeasured as Terra fills its currents
    sys.path.insert(0, str(Path(__file__).resolve().parents[3] / 'tools' / 'ocean'))
    import build
    u2, v2, measured = build.fill(Up2, Vp2)
    u2, v2 = build.smooth(u2, 1), build.smooth(v2, 1)
    u2 = u2.reshape(H, 2, W, 2).mean(axis=(1, 3))
    v2 = v2.reshape(H, 2, W, 2).mean(axis=(1, 3))
    measured = measured.reshape(H, 2, W, 2).max(axis=(1, 3))
    # the time scales onto the half-degree cells (boxes' centres, bilinear)
    lat = 90 - (np.arange(H) + 0.5) * 180 / H
    lon = -180 + (np.arange(W) + 0.5) * 360 / W
    by = np.clip((90 - lat) / BOX - 0.5, 0, 180 / BOX - 1)
    bx = (lon + 180) / BOX - 0.5
    y0 = np.floor(by).astype(int)
    ty = (by - y0)[:, None]
    y1 = np.minimum(y0 + 1, 180 // BOX - 1)
    x0 = np.floor(bx).astype(int) % (360 // BOX)
    tx = (bx - np.floor(bx))[None, :]
    x1 = (x0 + 1) % (360 // BOX)
    def onto(t):
        return ((t[y0][:, x0] * (1 - tx) + t[y0][:, x1] * tx) * (1 - ty)
                + (t[y1][:, x0] * (1 - tx) + t[y1][:, x1] * tx) * ty)
    Ku = u2 * onto(T['zonal']) * SECONDS
    Kv = v2 * onto(T['meridional']) * SECONDS
    sea = measured > 0.5
    print(f'K zonal {np.percentile(Ku[sea], 10):.0f} / {np.median(Ku[sea]):.0f} / {np.percentile(Ku[sea], 90):.0f}, '
          f'meridional {np.percentile(Kv[sea], 10):.0f} / {np.median(Kv[sea]):.0f} / {np.percentile(Kv[sea], 90):.0f} m^2/s '
          f'(10/50/90%), max {max(Ku.max(), Kv.max()):.0f}')
    def encode(k):
        return np.clip(np.round(255 * np.sqrt(np.clip(k, 0, KMAX) / KMAX)), 0, 255).astype(np.uint8)
    rgb = np.stack([encode(Ku), encode(Kv), np.round(255 * measured).astype(np.uint8)], -1)
    lat1 = 90 - (np.arange(180) + 0.5)
    lon1 = -180 + (np.arange(360) + 0.5)
    def onto1(t):
        by = np.clip((90 - lat1) / BOX - 0.5, 0, 180 / BOX - 1)
        bx = (lon1 + 180) / BOX - 0.5
        y0 = np.floor(by).astype(int)
        ty = (by - y0)[:, None]
        y1 = np.minimum(y0 + 1, 180 // BOX - 1)
        x0 = np.floor(bx).astype(int) % (360 // BOX)
        tx = (bx - np.floor(bx))[None, :]
        x1 = (x0 + 1) % (360 // BOX)
        return ((t[y0][:, x0] * (1 - tx) + t[y0][:, x1] * tx) * (1 - ty)
                + (t[y1][:, x0] * (1 - tx) + t[y1][:, x1] * tx) * ty)
    tz, tm = onto1(T['zonal']), onto1(T['meridional'])
    time_rgb = np.stack([np.clip(np.round(255 * tz / TMAX_DAYS), 0, 255).astype(np.uint8),
                         np.clip(np.round(255 * tm / TMAX_DAYS), 0, 255).astype(np.uint8),
                         np.zeros((180, 360), np.uint8)], -1)
    save('eddy-time.png', tile(time_rgb), 'RGB', {
        'what': 'the Lagrangian integral time scale, zonal in R and meridional in G',
        'cells': [360, 180], 'tiles': [1, 1], 'border': 1,
        'decode': 'byte / 255 * max', 'max': TMAX_DAYS, 'unit': 'days',
        'source': 'measured from GDP 6-hourly trajectories ' + f'{years[0]}-{years[-1]}' + ' in 10-degree boxes, smoothed',
        'credit': 'This study used data collected and made freely available by the NOAA Global Drifter '
                  'Program (https://www.aoml.noaa.gov/phod/gdp/) accessed from '
                  'https://erddap.aoml.noaa.gov/gdp/erddap/index.html'})
    save('eddy.png', tile(rgb), 'RGB', {
        'what': 'lateral eddy diffusivity, zonal in R and meridional in G, how measured in B',
        'cells': [W, H], 'tiles': [1, 1], 'border': 1,
        'decode': '(byte / 255)^2 * max', 'max': KMAX, 'unit': 'm^2/s',
        'step': 'a Gaussian step of standard deviation sqrt(2 K dt) metres on each axis',
        'source': 'NOAA GDP current variance (Laurindo, Mariano and Lumpkin 2017) times the Lagrangian '
                  'integral time scale measured from GDP 6-hourly trajectories ' + f'{years[0]}-{years[-1]}',
        'credit': 'This study used data collected and made freely available by the NOAA Global Drifter '
                  'Program (https://www.aoml.noaa.gov/phod/gdp/) accessed from '
                  'https://erddap.aoml.noaa.gov/gdp/erddap/index.html'})


if __name__ == '__main__':
    main()
