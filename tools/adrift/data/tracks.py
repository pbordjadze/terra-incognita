"""Reads the NOAA Global Drifter Program's 6-hourly trajectories for the builders beside it
(eddy.py, drifters.py); not run on its own.

    tracks.load(cache, years) -> a DataFrame: id, day (days since 2000-01-01 00:00 UTC), lat, lon,
                                 drogued (bool), sorted by id then day

The files are `<cache>/traj_<year>.csv`, one calendar year each, downloaded from ERDDAP if missing:
drifter_6hour_qc (Global Drifter Program - 6 Hour Interpolated QC Drifter Data; Lumpkin and
Centurioni, NOAA NCEI, doi:10.25921/7ntx-z961; CC BY 4.0), the columns ID, time, latitude,
longitude and drogue_lost_date. A year is about 140 MB; the fetch takes a minute or two.
"""
import urllib.request
from pathlib import Path
import numpy as np
import pandas as pd

ERDDAP = 'https://erddap.aoml.noaa.gov/gdp/erddap/tabledap/drifter_6hour_qc.csv'
EPOCH = pd.Timestamp('2000-01-01T00:00:00Z')


def fetch(cache, year):
    path = cache / f'traj_{year}.csv'
    if not path.exists():
        url = (f'{ERDDAP}?ID,time,latitude,longitude,drogue_lost_date'
               f'&time%3E={year}-01-01T00:00:00Z&time%3C{year + 1}-01-01T00:00:00Z')
        print('fetching', url, flush=True)
        urllib.request.urlretrieve(url, path.with_suffix('.part'))
        path.with_suffix('.part').rename(path)
    return path


def load(cache, years):
    cache = Path(cache)
    parts = []
    for y in years:
        df = pd.read_csv(fetch(cache, y), skiprows=[1], dtype={'ID': np.int64},
                         parse_dates=['time', 'drogue_lost_date'])
        day = (df['time'] - EPOCH) / pd.Timedelta(days=1)
        lost = (df['drogue_lost_date'] - EPOCH) / pd.Timedelta(days=1)
        # the drogue is on while no loss is recorded (NaT) and before the loss; 0 (the epoch of
        # 1970, "uncertain from the beginning") counts as off
        drogued = lost.isna() | (day < lost)
        drogued &= ~(df['drogue_lost_date'] == pd.Timestamp('1970-01-01T00:00:00Z'))
        parts.append(pd.DataFrame({'id': df['ID'].values, 'day': day.values.astype(np.float64),
                                   'lat': df['latitude'].values.astype(np.float64),
                                   'lon': df['longitude'].values.astype(np.float64),
                                   'drogued': drogued.values}))
        print(f'{y}: {len(df)} rows, {df["ID"].nunique()} drifters', flush=True)
    out = pd.concat(parts, ignore_index=True)
    out.sort_values(['id', 'day'], inplace=True, kind='stable')
    out.reset_index(drop=True, inplace=True)
    return out
