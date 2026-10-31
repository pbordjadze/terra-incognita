"""Builds the wind a float's leeway rides: the month's usual 10 m wind over the sea.

    python3 tools/adrift/data/wind.py <cache>

Source: the NCEP/NCAR Reanalysis 1 (Kalnay et al. 1996, Bull. Amer. Meteor. Soc. 77, 437-471),
its derived long-term monthly means of the 10 m wind over 1991-2020 (`uwnd.10m.mon.ltm.1991-2020.nc`,
`vwnd...`, from NOAA PSL, https://psl.noaa.gov/data/gridded/data.ncep.reanalysis.html; public
domain), on its 1.875 x ~1.9 degree Gaussian grid, downloaded into <cache> if missing.

Out, adrift/data/wind.png, 1448 x 546 RGB: twelve 362 x 182 tiles (sheets.py) of 1-degree cells,
bilinear from the Gaussian grid:
      R  the eastward wind, G the northward: 128 + 127 w / 16 m/s (monthly means never pass
         13 m/s), so a byte is an eighth of a metre a second
      B  0
A monthly mean is the mean of a wind that veers and gusts: a float's leeway on it is the leeway
of a typical month, smoother than any real month's. Over land the wind is drawn too (a float is
never there; the sheet is simply complete).
"""
import sys, urllib.request
from pathlib import Path
import numpy as np
import netCDF4
from sheets import months, regrid, save, tile

PSL = 'https://downloads.psl.noaa.gov/Datasets/ncep.reanalysis.derived/surface_gauss/'
W, H = 360, 180
WMAX = 16.0


def fetch(cache, name):
    path = cache / name
    if not path.exists():
        print('fetching', PSL + name, flush=True)
        urllib.request.urlretrieve(PSL + name, path)
    return path


def encode(x):
    return np.clip(np.round(128 + 127 * x / WMAX), 1, 255).astype(np.uint8)


def main():
    cache = Path(sys.argv[1])
    cache.mkdir(parents=True, exist_ok=True)
    fields = {}
    for v in ('uwnd', 'vwnd'):
        f = netCDF4.Dataset(fetch(cache, f'{v}.10m.mon.ltm.1991-2020.nc'))
        a = np.array(f.variables[v][:], dtype=np.float64)             # (12, lat, lon)
        lat, lon = np.array(f.variables['lat'][:]), np.array(f.variables['lon'][:])
        f.close()
        fields[v] = [regrid(a[m], lat, lon, W, H) for m in range(12)]
    tiles = []
    for m in range(12):
        u, v = fields['uwnd'][m], fields['vwnd'][m]
        speed = np.hypot(u, v)
        print(f'month {m + 1:2d}: mean {speed.mean():.2f}, max {speed.max():.2f} m/s')
        tiles.append(tile(np.stack([encode(u), encode(v), np.zeros_like(u, np.uint8)], -1)))
    save('wind.png', months(tiles), 'RGB', {
        'what': 'the month\'s mean 10 m wind, eastward in R and northward in G',
        'cells': [W, H], 'tiles': [4, 3], 'border': 1,
        'decode': '(byte - 128) / 127 * max', 'max': WMAX, 'unit': 'm/s',
        'source': 'NCEP/NCAR Reanalysis 1, long-term monthly means 1991-2020 (NOAA PSL)',
        'credit': 'NCEP/NCAR Reanalysis 1 data provided by the NOAA PSL, Boulder, Colorado, USA'})


if __name__ == '__main__':
    main()
