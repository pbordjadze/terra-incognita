"""Builds the weather a thing on the wind meets: how the 10 m wind gusts and veers about its month's
mean, how long a gust lasts, how wide a weather system is, and how often it rains; and the
reference the wind's rules are held to: real 10-day paths through five real years of wind.

    python3 tools/adrift/data/weather.py <cache>

Sources (NOAA PSL, public domain; downloaded into <cache> if missing):
  - NCEP/NCAR Reanalysis 1 (Kalnay et al. 1996), the 10 m wind four times a day on its T62
    Gaussian grid (192 x 94), 2016-2020: `uwnd.10m.gauss.<year>.nc`, `vwnd...`, from
    https://downloads.psl.noaa.gov/Datasets/ncep.reanalysis/surface_gauss/ (about 500 MB).
  - Its long-term monthly mean precipitation rate, 1991-2020: `prate.sfc.mon.ltm.1991-2020.nc`
    (ncep.reanalysis.derived/surface_gauss/).

Out, adrift/data/gusts.png, 1448 x 546 RGBA: twelve 362 x 182 tiles (sheets.py) of 1-degree cells,
bilinear from the Gaussian grid:
      R  sigma_u, the standard deviation of the eastward wind about the month's mean, over the
         month's six-hourly samples of five years (each year-month's own mean taken out, so a
         warm year's January is not counted as weather): 255 sqrt(sigma / 12 m/s)
      G  sigma_v, the same northward
      B  T, the gusts' integral time scale at the place: the six-hourly autocorrelation of the
         anomaly (u and v averaged) summed to its first zero, at most 5 days: 255 T / 5 days
      A  the month's mean rain, 255 sqrt(P / 25 mm/day)
and adrift/data/weather.json:
      scale     the weather's width: the distance (km) at which the anomaly's correlation with
                its neighbours east and north falls to 1/e, by latitude band (pooled months)
      paths     the reference: from a grid of starts, every month, ten real 10-day paths (each
                year's 1st and 15th, 2016-2020) carried by the six-hourly wind (RK2, bilinear in
                space, linear in time): their mean displacement and their spread (the RMS distance
                of each path's end from the paths' mean end), km
"""
import json, sys, urllib.request
from pathlib import Path
import numpy as np
import netCDF4
from sheets import OUT, months, regrid, save, tile

PSL = 'https://downloads.psl.noaa.gov/Datasets/'
YEARS = range(2016, 2021)
W, H = 360, 180
SMAX, TMAX, PMAX = 12.0, 5.0, 25.0
LAGS = 40                     # six-hourly lags: 10 days
R = 6371.0
DAYS = 10


def fetch(cache, sub, name):
    path = cache / name
    if not path.exists():
        print('fetching', name, flush=True)
        urllib.request.urlretrieve(PSL + sub + name, path)
    return path


def load(cache, var, year):
    f = netCDF4.Dataset(fetch(cache, 'ncep.reanalysis/surface_gauss/', f'{var}.10m.gauss.{year}.nc'))
    a = np.array(f.variables[var][:], dtype=np.float32)            # (time, lat, lon), lat N to S
    lat, lon = np.array(f.variables['lat'][:]), np.array(f.variables['lon'][:])
    t = netCDF4.num2date(f.variables['time'][:], f.variables['time'].units,
                         only_use_cftime_datetimes=False, only_use_python_datetimes=True)
    f.close()
    return a, lat, lon, np.array([d.month - 1 for d in t]), np.array([d.day for d in t])


def bilinear(field, lat_src, lon, lat):
    """field (lat N->S, lon 0..360 even) at points (lon -180..180, lat)."""
    nlat, nlon = field.shape
    fy = np.interp(-lat, -lat_src, np.arange(nlat))
    y0 = np.clip(np.floor(fy).astype(int), 0, nlat - 2)
    ty = fy - y0
    fx = (lon % 360) / (360 / nlon)
    x0 = np.floor(fx).astype(int) % nlon
    tx = fx - np.floor(fx)
    x1 = (x0 + 1) % nlon
    top = field[y0, x0] * (1 - tx) + field[y0, x1] * tx
    bot = field[y0 + 1, x0] * (1 - tx) + field[y0 + 1, x1] * tx
    return top * (1 - ty) + bot * ty


def main():
    cache = Path(sys.argv[1])
    cache.mkdir(parents=True, exist_ok=True)
    nlat, nlon = 94, 192
    s2 = {c: np.zeros((12, nlat, nlon)) for c in 'uv'}
    n = np.zeros(12)
    acf = np.zeros((12, LAGS + 1, nlat, nlon))
    corr_e = {}          # band -> list of correlations at zonal lags
    corr_n = {}
    years = {}
    for y in YEARS:
        u, lat, lon, mon, day = load(cache, 'uwnd', y)
        v, *_ = load(cache, 'vwnd', y)
        years[y] = (u, v, mon, day)
        for m in range(12):
            sel = mon == m
            au = u[sel] - u[sel].mean(0)
            av = v[sel] - v[sel].mean(0)
            s2['u'][m] += (au ** 2).sum(0)
            s2['v'][m] += (av ** 2).sum(0)
            n[m] += sel.sum()
            for L in range(LAGS + 1):
                acf[m, L] += (au[:len(au) - L] * au[L:]).sum(0) + (av[:len(av) - L] * av[L:]).sum(0)
            if m in (0, 6):   # the weather's width, from January and July
                for b0 in range(-80, 81, 20):
                    rows = np.where((lat >= b0 - 10) & (lat < b0 + 10))[0]
                    a = au[:, rows, :]
                    sd = a.std(0) + 1e-6
                    ce = []
                    for d in range(0, 40):
                        b = np.roll(a, -d, axis=2)
                        ce.append(float(((a * b).mean(0) / (sd * np.roll(sd, -d, axis=1))).mean()))
                    corr_e.setdefault(b0, []).append(ce)
                    cn = []
                    for d in range(0, 12):
                        r2 = rows[(rows + d) < nlat]
                        a1, a2 = u[sel][:, r2, :] - u[sel][:, r2, :].mean(0), u[sel][:, r2 + d, :] - u[sel][:, r2 + d, :].mean(0)
                        cn.append(float(((a1 * a2).mean(0) / (a1.std(0) * a2.std(0) + 1e-6)).mean()))
                    corr_n.setdefault(b0, []).append(cn)
        print('year', y, flush=True)
    sig = {c: np.sqrt(s2[c] / n[:, None, None]) for c in 'uv'}
    r = acf / acf[:, :1]
    # integral time scale: trapezoid of the ACF to its first zero, six-hourly
    pos = np.cumprod(r > 0, axis=1)
    T = 0.25 * (np.sum(r * pos, axis=1) - 0.5)       # days (the lag-0 term counted half)
    T = np.clip(T, 0.25, TMAX)
    # precipitation
    f = netCDF4.Dataset(fetch(cache, 'ncep.reanalysis.derived/surface_gauss/', 'prate.sfc.mon.ltm.1991-2020.nc'))
    P = np.array(f.variables['prate'][:], dtype=np.float64) * 86400.0  # mm/day
    plat, plon = np.array(f.variables['lat'][:]), np.array(f.variables['lon'][:])
    f.close()
    enc = lambda x, mx: np.clip(np.round(255 * np.sqrt(np.clip(x, 0, mx) / mx)), 0, 255).astype(np.uint8)
    tiles = []
    for m in range(12):
        su = regrid(sig['u'][m], lat, lon, W, H)
        sv = regrid(sig['v'][m], lat, lon, W, H)
        tm = regrid(T[m], lat, lon, W, H)
        pm = regrid(P[m], plat, plon, W, H)
        print(f'month {m + 1:2d}: sigma u {su.mean():.2f} v {sv.mean():.2f} (max {max(su.max(), sv.max()):.1f}) m/s, '
              f'T {tm.mean():.2f} d ({tm.min():.2f}-{tm.max():.2f}), rain {pm.mean():.2f} mm/d (max {pm.max():.1f})')
        b = np.clip(np.round(255 * tm / TMAX), 0, 255).astype(np.uint8)
        tiles.append(tile(np.stack([enc(su, SMAX), enc(sv, SMAX), b, enc(pm, PMAX)], -1)))
    save('gusts.png', months(tiles), 'RGBA', {
        'what': 'the 10 m wind\'s weather about its monthly mean: sigma_u in R, sigma_v in G, the gusts\' '
                'integral time scale in B, the month\'s mean rain in A',
        'cells': [W, H], 'tiles': [4, 3], 'border': 1,
        'decode': 'R, G: (byte / 255)^2 * 12 m/s; B: byte / 255 * 5 days; A: (byte / 255)^2 * 25 mm/day',
        'source': 'NCEP/NCAR Reanalysis 1, 10 m wind four times daily 2016-2020; monthly mean precipitation rate 1991-2020 (NOAA PSL)',
        'credit': 'NCEP/NCAR Reanalysis 1 data provided by the NOAA PSL, Boulder, Colorado, USA'})

    def efold(cs, km_per):
        c = np.mean(cs, axis=0)
        for i in range(1, len(c)):
            if c[i] < np.exp(-1):
                f = (c[i - 1] - np.exp(-1)) / (c[i - 1] - c[i])
                return float((i - 1 + f) * km_per)
        return None
    dlon = 360 / nlon
    scale = []
    for b0 in sorted(corr_e):
        ke = efold(corr_e[b0], dlon * np.pi / 180 * R * np.cos(np.radians(b0)))
        kn = efold(corr_n[b0], 1.9 * np.pi / 180 * R)
        scale.append({'lat': b0, 'east_km': ke and round(ke), 'north_km': kn and round(kn)})
        print(f'band {b0:+3d}: the weather\'s width east {ke and round(ke)} km, north {kn and round(kn)} km')

    # the reference paths
    starts = [(lo + 7.5, la + 0.5) for la in range(-60, 71, 10) for lo in range(-180, 180, 15)]
    slon = np.array([s[0] for s in starts])
    slat = np.array([s[1] for s in starts])
    paths = []
    for m in range(12):
        ends = []
        for y in YEARS:
            u, v, mon, day = years[y]
            for d0 in (1, 15):
                i0 = int(np.where((mon == m) & (day == d0))[0][0])
                if i0 + DAYS * 4 >= len(u):
                    continue
                plon, plat = slon.copy(), slat.copy()
                east = np.zeros_like(plon)
                north = np.zeros_like(plon)
                for s in range(DAYS * 4):
                    def vel(i, frac, lo, la):
                        uu = bilinear(u[i], lat, lo, la) * (1 - frac) + bilinear(u[i + 1], lat, lo, la) * frac
                        vv = bilinear(v[i], lat, lo, la) * (1 - frac) + bilinear(v[i + 1], lat, lo, la) * frac
                        return uu, vv
                    dt = 6 * 3600.0
                    u1, v1 = vel(i0 + s, 0.0, plon, plat)
                    mlon = plon + np.degrees(u1 * dt / 2 / (R * 1000 * np.cos(np.radians(plat))))
                    mlat = np.clip(plat + np.degrees(v1 * dt / 2 / (R * 1000)), -89.5, 89.5)
                    u2, v2 = vel(i0 + s, 0.5, mlon, mlat)
                    east += u2 * dt / 1000
                    north += v2 * dt / 1000
                    plon = (plon + np.degrees(u2 * dt / (R * 1000 * np.cos(np.radians(mlat)))) + 180) % 360 - 180
                    plat = np.clip(plat + np.degrees(v2 * dt / (R * 1000)), -89.5, 89.5)
                ends.append(np.stack([east, north], -1))
        ends = np.array(ends)                         # (samples, starts, 2) km
        mean = ends.mean(0)
        spread = np.sqrt(((ends - mean) ** 2).sum(-1).mean(0))
        for i, (lo, la) in enumerate(starts):
            paths.append({'lon': lo, 'lat': la, 'month': m + 1, 'n': len(ends),
                          'east_km': round(float(mean[i, 0])), 'north_km': round(float(mean[i, 1])),
                          'spread_km': round(float(spread[i]))})
        print(f'month {m + 1:2d}: real 10-day paths, median spread {np.median(spread):.0f} km', flush=True)
    (OUT / 'weather.json').write_text(json.dumps({
        '_': 'Built by tools/adrift/data/weather.py from NCEP/NCAR Reanalysis 1 (NOAA PSL): the weather\'s width by '
             'latitude band, and real 10-day paths on the 10 m wind (2016-2020, each year\'s 1st and 15th of the month) '
             'from a grid of starts: their mean displacement and spread in km.',
        'scale': scale, 'days': DAYS, 'paths': paths}, separators=(',', ':')) + '\n')
    print('adrift/data/weather.json')


if __name__ == '__main__':
    main()
