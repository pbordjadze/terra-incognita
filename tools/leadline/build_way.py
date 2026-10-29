#!/usr/bin/env python3
"""HMS Challenger's way round the world, dated (Prototypes/lead-line/way.js): the expedition's stations
as recorded (the 94 dated stations of Murray and Renard's deposits record, through PANGAEA 849072,
CC BY 3.0, kept from the first version of this study in `stations.js`) joined with her calls at ports
and the turns of her course between them, from the expedition's published itinerary, dated to the day
where the record has one and to the month otherwise (a day in the month is drawn). Between any two
dated points the ship is drawn on the great circle at a steady pace. The way is drawn, not logged:
the stations are the record, the rest an illustration that keeps the ship at sea and in the right
ocean in the right month.

    python3 tools/leadline/build_way.py

Rebuild, never hand-edit. Prints any stretch the ship would have to cover faster than 450 km a day,
which is a date to look at.
"""
import json, math, re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
PACK = ROOT / 'Prototypes' / 'lead-line'
DAY = 86400.0

# Her calls and turns, (date, lon, lat, name): from the expedition's itinerary (Wikipedia, "Challenger
# expedition", after Thomson and Murray's Report), to the day where it says, else a day in the month.
CALLS = [
    ('1872-12-21', -1.09, 50.80, 'Portsmouth'), ('1872-12-24', -5.3, 48.3, None), ('1872-12-29', -9.6, 43.1, None),
    ('1873-01-03', -9.14, 38.70, 'Lisbon'), ('1873-01-12', -9.0, 36.9, None), ('1873-01-18', -5.35, 36.14, 'Gibraltar'),
    ('1873-01-26', -5.6, 36.0, None), ('1873-02-03', -16.91, 32.65, 'Madeira'), ('1873-02-10', -16.25, 28.47, 'Tenerife'),
    ('1873-03-16', -64.93, 18.34, 'St Thomas'), ('1873-04-04', -64.78, 32.30, 'Bermuda'), ('1873-05-09', -63.57, 44.65, 'Halifax'),
    ('1873-05-31', -64.78, 32.30, 'Bermuda'), ('1873-07-01', -28.63, 38.53, 'The Azores'), ('1873-07-16', -16.91, 32.65, 'Madeira'),
    ('1873-07-27', -23.51, 14.92, 'Cape Verde'), ('1873-08-27', -29.35, 0.92, "St Paul's Rocks"), ('1873-09-01', -32.42, -3.85, 'Fernando de Noronha'),
    ('1873-09-14', -38.51, -12.97, 'Bahia'), ('1873-10-15', -12.28, -37.11, 'Tristan da Cunha'), ('1873-10-28', 18.42, -33.92, 'Cape Town'),
    ('1873-12-17', 18.4, -34.3, None), ('1873-12-26', 37.75, -46.6, 'Prince Edward Islands'), ('1874-01-02', 51.8, -46.4, 'The Crozets'),
    ('1874-01-07', 70.2, -49.35, 'Kerguelen'), ('1874-02-06', 73.5, -53.1, 'Heard Island'), ('1874-02-16', 78.4, -66.6, 'The Antarctic Circle'),
    ('1874-02-24', 90.0, -64.0, None), ('1874-03-17', 144.96, -37.81, 'Melbourne'), ('1874-04-01', 146.4, -39.3, None),
    ('1874-04-06', 151.21, -33.87, 'Sydney'), ('1874-06-28', 174.78, -41.29, 'Wellington'), ('1874-07-19', -175.2, -21.14, 'Tonga'),
    ('1874-07-25', 178.0, -19.05, 'Fiji'), ('1874-08-31', 142.5, -10.75, 'Cape York'), ('1874-09-19', 134.5, -6.0, 'The Aru Islands'),
    ('1874-10-01', 129.9, -4.52, 'Banda'), ('1874-10-04', 128.18, -3.70, 'Amboina'), ('1874-10-16', 127.37, 0.79, 'Ternate'),
    ('1874-10-23', 122.07, 6.90, 'Zamboanga'), ('1874-11-04', 120.98, 14.60, 'Manila'), ('1874-11-17', 114.17, 22.30, 'Hong Kong'),
    ('1875-01-06', 114.2, 22.1, None), ('1875-01-11', 120.98, 14.60, 'Manila'), ('1875-01-24', 123.89, 10.31, 'Cebu'),
    ('1875-02-05', 122.07, 6.90, 'Zamboanga'), ('1875-02-10', 125.5, 5.3, None), ('1875-02-23', 140.72, -2.55, 'Humboldt Bay'),
    ('1875-03-04', 147.3, -2.0, 'The Admiralty Islands'), ('1875-03-23', 142.25, 11.4, 'The Mariana Trench'), ('1875-04-11', 139.64, 35.44, 'Yokohama'),
    ('1875-06-16', 139.9, 35.1, None), ('1875-07-27', -157.86, 21.31, 'Honolulu'), ('1875-08-19', -155.08, 19.73, 'Hilo'),
    ('1875-09-18', -149.57, -17.53, 'Tahiti'), ('1875-11-13', -78.84, -33.63, 'Juan Fernández'), ('1875-11-19', -71.63, -33.04, 'Valparaíso'),
    ('1875-12-11', -71.8, -33.2, None), ('1875-12-20', -80.0, -40.0, None), ('1875-12-31', -75.3, -46.8, 'Port Otway'),
    ('1876-01-11', -70.91, -53.16, 'The Strait of Magellan'), ('1876-01-18', -65.5, -54.0, None), ('1876-01-23', -57.85, -51.69, 'The Falklands'),
    ('1876-02-15', -56.16, -34.90, 'Montevideo'), ('1876-02-28', -56.0, -35.0, None), ('1876-03-10', -30.0, -36.0, None),
    ('1876-03-27', -14.36, -7.95, 'Ascension'), ('1876-04-16', -23.51, 14.92, 'Cape Verde'),
    ('1876-05-20', -8.72, 42.24, 'Vigo'), ('1876-05-24', -1.09, 50.80, 'Spithead'),
]

def day(s):
    y, m, d = map(int, s.split('-'))
    import datetime
    return (datetime.datetime(y, m, d, tzinfo=datetime.timezone.utc) - datetime.datetime(1970, 1, 1, tzinfo=datetime.timezone.utc)).total_seconds() / DAY

def km(a, b):
    la, lb = math.radians(a[1]), math.radians(b[1]); dl = math.radians(b[0] - a[0])
    return 6371 * math.acos(max(-1, min(1, math.sin(la) * math.sin(lb) + math.cos(la) * math.cos(lb) * math.cos(dl))))

def main():
    text = (PACK / 'stations.js').read_text(encoding='utf-8')
    data = json.loads(text[text.index('{'):text.rindex('}') + 1])
    points = []
    for o in data['observations']:
        points.append({'d': day(o['date']), 'lon': o['lon'], 'lat': o['lat'], 'kind': 'station', 'name': o['station'].replace('CHA-', ''),
                       'fathoms': o['fathoms'], 'depth': o['depth']})
    for s, lon, lat, name in CALLS:
        p = {'d': day(s), 'lon': lon, 'lat': lat, 'kind': 'port' if name else 'way'}
        if name: p['name'] = name
        points.append(p)
    points.sort(key=lambda p: (p['d'], p['kind'] != 'station'))
    # two points on one day: the station comes first; a port the same day as a station far away is dropped
    cleaned = []
    for p in points:
        if cleaned and abs(p['d'] - cleaned[-1]['d']) < 0.5 and p['kind'] != 'station' and km((p['lon'], p['lat']), (cleaned[-1]['lon'], cleaned[-1]['lat'])) > 150:
            print(f"dropped {p.get('name', p['kind'])} on the same day as station {cleaned[-1].get('name')}")
            continue
        cleaned.append(p)
    for a, b in zip(cleaned, cleaned[1:]):
        dd = max(0.5, b['d'] - a['d']); v = km((a['lon'], a['lat']), (b['lon'], b['lat'])) / dd
        if v > 450:
            print(f"fast: {a.get('name', a['kind'])} -> {b.get('name', b['kind'])}: {v:.0f} km/day over {dd:.1f} days")
    out = PACK / 'way.js'
    out.write_text('/* HMS Challenger\'s way, dated: her stations as recorded (stations.js) and her calls and turns from the\n'
                   '   itinerary, in order of day (days since 1970-01-01 UTC); tools/leadline/build_way.py. Between two\n'
                   '   points she is drawn on the great circle at a steady pace. */\n'
                   'window.WAY = ' + json.dumps({'points': cleaned}, separators=(',', ':')) + ';\n', encoding='utf-8')
    print(f'{out}: {len(cleaned)} points, {sum(1 for p in cleaned if p["kind"] == "station")} stations, {out.stat().st_size // 1024} KB')

if __name__ == '__main__':
    main()
