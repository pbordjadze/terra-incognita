"""Builds the real drifters a float is measured against: a subsample of the NOAA Global Drifter
Program's tracks, small enough for a phone, spread so that wherever a float is put, a real drifter
usually passed near.

    python3 tools/adrift/data/drifters.py <cache> [years...]

Source: the 6-hourly interpolated trajectories (tracks.py; drogued or not: a drifter that lost
its drogue still went where the sea took it, a little more with the wind), the years given
(2016-2023 by default). Each track is kept at one point a day (00:00 UTC), as a run of whole days
with no gap (a gap starts a new run). Runs are chosen greedily by what they add: the 2-degree
cells they pass that fewer than COVER chosen runs have passed, until the budget BUDGET bytes is
spent; so the busy North Atlantic gives way to the lonely South Pacific once each is covered.

Out, adrift/data/drifters.bin, little-endian:
      'ADRF' u32 version (1) u32 count
      then per run: u64 id (the GDP buoy ID), f32 day0 (the first point's day, days since
      2000-01-01 00:00 UTC, always a whole day), u16 n (points), u16 flags (bit 0: the drogue
      was on at the first point), then n x (i16 lon, i16 lat) in 1/180 degree (about 600 m)
and `drifters.json` beside it (count, points, the years, the coverage reached), for the record.
"""
import json, struct, sys
from pathlib import Path
import numpy as np
import tracks
import sheets
from sheets import OUT

BUDGET = 4_000_000
COVER = 3
CELL = 2
SCALE = 180.0


def runs(df):
    """Daily runs: (id, day0, drogued0, lon[], lat[]) for every gap-free stretch of midnights."""
    at = np.abs(df['day'].values - np.round(df['day'].values)) < 1e-6
    d = df[at]
    ids, day, lon, lat, dr = (d['id'].values, np.round(d['day'].values).astype(np.int64),
                              d['lon'].values, d['lat'].values, d['drogued'].values)
    brk = np.ones(len(d), bool)
    brk[1:] = (ids[1:] != ids[:-1]) | (day[1:] != day[:-1] + 1)
    starts = np.flatnonzero(brk)
    ends = np.append(starts[1:], len(d))
    out = []
    for s, e in zip(starts, ends):
        if e - s >= 2:
            out.append((int(ids[s]), int(day[s]), bool(dr[s]), lon[s:e], lat[s:e]))
    return out


def cells(lon, lat):
    cx = ((lon + 180) // CELL).astype(int) % (360 // CELL)
    cy = np.clip(((90 - lat) // CELL).astype(int), 0, 180 // CELL - 1)
    return np.unique(cy * (360 // CELL) + cx)


def choose(all_runs):
    """Greedy by new coverage within the budget (lazy: a run's gain only falls as others are chosen)."""
    count = np.zeros((180 // CELL) * (360 // CELL), np.int32)
    gains = np.array([len(c) for c in all_runs], float)
    order = np.argsort(-gains)
    chosen, spent = [], 12
    import heapq
    heap = [(-g, i) for i, g in enumerate(gains)]
    heapq.heapify(heap)
    while heap:
        g, i = heapq.heappop(heap)
        c = all_runs[i][5]
        fresh = float(np.sum(count[c] < COVER))
        if fresh < -g - 1e-9:
            heapq.heappush(heap, (-fresh, i))
            continue
        if fresh <= 0:
            break
        size = 16 + 4 * len(all_runs[i][3])
        if spent + size > BUDGET:
            continue
        spent += size
        count[c] += 1
        chosen.append(i)
    return chosen, count, spent


def main():
    cache = Path(sys.argv[1])
    years = [int(y) for y in sys.argv[2:]] or list(range(2016, 2024))
    df = tracks.load(cache, years)
    rs = runs(df)
    rs = [r + (cells(r[3], r[4]),) for r in rs]
    print(f'{len(rs)} runs, {sum(len(r[3]) for r in rs)} daily points', flush=True)
    chosen, count, spent = choose(rs)
    chosen.sort(key=lambda i: (rs[i][0], rs[i][1]))
    OUT.mkdir(parents=True, exist_ok=True)
    with open(OUT / 'drifters.bin', 'wb') as f:
        f.write(b'ADRF' + struct.pack('<II', 1, len(chosen)))
        points = 0
        for i in chosen:
            id_, day0, drogued, lon, lat, _ = rs[i]
            x = np.round(((lon + 180) % 360 - 180) * SCALE).astype(np.int16)
            y = np.round(lat * SCALE).astype(np.int16)
            f.write(struct.pack('<QfHH', id_, float(day0), len(lon), 1 if drogued else 0))
            f.write(np.stack([x, y], -1).astype('<i2').tobytes())
            points += len(lon)
    sea = count > 0
    record = {'runs': len(chosen), 'points': points, 'bytes': spent, 'years': years,
              'drifters': len({rs[i][0] for i in chosen}),
              'cell_degrees': CELL, 'cover': COVER,
              'cells_passed_by_any_run': int((count > 0).sum()),
              'cells_covered': int((count >= COVER).sum()),
              'day_scale': SCALE}
    (OUT / 'drifters.json').write_text(json.dumps(record, indent=1) + '\n')
    sheets.record('drifters.bin', {
        'what': 'real GDP drifter runs at a point a day (tools/adrift/data/drifters.py has the layout)',
        'runs': record['runs'], 'points': record['points'], 'years': years,
        'source': 'NOAA GDP 6-hourly interpolated trajectories (Lumpkin and Centurioni, doi:10.25921/7ntx-z961), CC BY 4.0',
        'credit': 'This study used data collected and made freely available by the NOAA Global Drifter '
                  'Program (https://www.aoml.noaa.gov/phod/gdp/) accessed from '
                  'https://erddap.aoml.noaa.gov/gdp/erddap/index.html'})
    print(record)
    print(f'adrift/data/drifters.bin {(OUT / "drifters.bin").stat().st_size / 1e6:.2f} MB')


if __name__ == '__main__':
    main()
