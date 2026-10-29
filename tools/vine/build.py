#!/usr/bin/env python3
"""The vine's long walk (Prototypes/vine/data.js): the grapevine's spread from the South Caucasus over
eight thousand years as branches of one vine, each a way with a year at each vertex, generalised after
the archaeological and historical record (SOURCES.md in the pack: McGovern 2003 and 2017, Unwin 1991,
and the dates of the colonial plantings). Between vertices the way is densified along the great circle
every ~70 km with the years interpolated and a vine's sway either side of the straight way, and leaves
are placed along it every ~110 km, alternating sides. The named places are the nodes where a cluster hangs when the vine arrives.

    python3 tools/vine/build.py

Rebuild, never hand-edit. The ways are drawn, not records of any one vine's travels; the years are
the record's earliest evidence at or near the place, rounded.
"""
import json, math
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Prototypes' / 'vine' / 'data.js'
D = math.pi / 180

ORIGIN = (44.75, 41.45)   # Gadachrili Gora, Kvemo Kartli, Georgia: the earliest wine, about 6000 BCE

# Each branch: its name, where it starts from (a place on an earlier branch, by coordinates), and its
# vertices (lon, lat, year, name or None). Years negative are BCE.
BRANCHES = [
    ('The first vine', [(44.75, 41.45, -6000, 'Gadachrili Gora'), (44.2, 41.7, -5900, 'Shulaveris Gora'), (43.5, 42.0, -5700, None), (42.0, 42.2, -5500, 'Colchis')]),
    ('Down the Zagros', [(44.75, 41.45, -6000, None), (45.2, 40.3, -5600, None), (45.2, 39.73, -5400, 'Areni'), (45.47, 36.98, -5000, 'Hajji Firuz'),
                         (47.0, 35.5, -4300, None), (48.07, 34.52, -3500, 'Godin Tepe'), (50.5, 31.5, -3000, None), (52.5, 29.6, -2500, 'Shiraz')]),
    ('Into Anatolia', [(44.75, 41.45, -6000, None), (42.5, 40.4, -5500, None), (39.5, 39.6, -5000, 'Eastern Anatolia'), (35.5, 39.2, -4300, None),
                       (31.0, 39.0, -3600, None), (28.0, 39.4, -3200, None), (26.24, 39.96, -3000, 'Troy')]),
    ('To the Levant and Egypt', [(39.5, 39.6, -5000, None), (37.0, 37.2, -4600, None), (35.9, 34.5, -4000, 'Byblos'), (35.0, 32.0, -3600, None),
                                 (34.3, 31.3, -3300, None), (31.4, 30.7, -3100, 'The Nile Delta'), (31.0, 29.0, -3000, None), (31.92, 26.18, -2900, 'Abydos')]),
    ('Over the sea to Crete and Greece', [(35.9, 34.5, -4000, None), (33.2, 34.9, -3500, 'Cyprus'), (29.0, 35.5, -3000, None), (25.1, 35.3, -2500, 'Crete'),
                                          (23.0, 36.6, -2000, None), (22.75, 37.7, -1800, 'Mycenae'), (23.7, 38.0, -1400, 'Athens')]),
    ('The Phoenicians, west', [(35.9, 34.5, -4000, None), (35.2, 33.3, -1100, 'Tyre'), (31.0, 34.5, -1000, None), (22.0, 35.0, -900, None), (15.0, 36.5, -850, None),
                               (10.33, 36.85, -800, 'Carthage'), (9.0, 39.5, -700, 'Sardinia'), (1.4, 38.9, -650, 'Ibiza'), (-6.3, 36.5, -600, 'Cádiz'), (-9.1, 38.7, -400, 'Lisbon')]),
    ('The Greeks, west', [(23.7, 38.0, -1400, None), (22.9, 37.9, -800, 'Corinth'), (19.0, 37.0, -750, None), (15.3, 37.1, -730, 'Sicily'), (16.6, 39.3, -700, 'Oenotria'),
                          (14.3, 40.8, -600, 'Campania'), (12.3, 42.4, -500, 'Etruria')]),
    ('Massalia and the Rhône', [(15.3, 37.1, -730, None), (10.0, 41.0, -650, None), (5.37, 43.3, -600, 'Massalia'), (4.8, 44.5, -100, 'The Rhône'),
                                (4.85, 45.75, 1, 'Lyon'), (4.85, 47.0, 100, 'Burgundy'), (4.0, 49.0, 300, 'Champagne'), (2.35, 48.85, 400, 'Paris'), (0.5, 50.9, 450, None), (-0.1, 51.5, 500, 'Britain')]),
    ('To Bordeaux', [(5.37, 43.3, -600, None), (3.0, 43.2, -100, 'Narbonne'), (1.44, 43.6, 1, 'Toulouse'), (-0.58, 44.84, 50, 'Bordeaux'), (-1.0, 46.5, 300, None), (-1.5, 47.2, 500, 'The Loire')]),
    ('Over the Alps to the Mosel and the Rhine', [(12.3, 42.4, -500, None), (11.5, 44.8, -200, 'The Po'), (9.0, 45.5, -100, None), (8.5, 47.5, 100, None),
                                                 (6.64, 49.75, 200, 'Trier, on the Mosel'), (8.27, 50.0, 300, 'Mainz, on the Rhine'), (8.0, 50.05, 800, 'The Rheingau')]),
    ('Up the Danube', [(11.5, 44.8, -200, None), (14.5, 46.0, -50, None), (18.0, 47.5, 100, 'Pannonia'), (21.4, 48.1, 1100, 'Tokaj')]),
    ('Inland in Iberia', [(-6.3, 36.5, -600, None), (-6.0, 37.4, -200, 'Seville'), (-4.0, 39.5, 1, None), (-2.5, 42.4, 100, 'Rioja'), (-7.5, 41.2, 200, 'The Douro')]),
    ('The Silk Road, east', [(52.5, 29.6, -2500, None), (56.0, 33.0, -1500, None), (59.0, 36.0, -1000, 'Khorasan'), (62.2, 37.65, -500, 'Merv'), (66.97, 39.65, -300, 'Samarkand'),
                             (71.0, 40.4, -200, 'Ferghana'), (76.0, 39.5, -150, 'Kashgar'), (85.0, 41.5, -140, None), (94.7, 40.1, -130, 'Dunhuang'), (103.8, 36.0, -125, None), (108.9, 34.27, -120, "Chang'an")]),
    ('On to Japan', [(108.9, 34.27, -120, None), (117.0, 36.5, 300, None), (126.0, 35.5, 500, None), (132.0, 34.5, 650, None), (138.7, 35.65, 718, 'Katsunuma')]),
    ('Out to Madeira and the Canaries', [(-9.1, 38.7, -400, None), (-12.0, 36.0, 1400, None), (-16.9, 32.65, 1425, 'Madeira'), (-16.5, 28.3, 1480, 'The Canaries')]),
    ('Across the ocean to Mexico', [(-6.3, 36.5, -600, None), (-15.0, 30.0, 1493, None), (-40.0, 22.0, 1496, None), (-60.0, 19.0, 1498, None), (-70.0, 18.5, 1500, 'Hispaniola'),
                                    (-85.0, 20.5, 1520, None), (-96.1, 19.2, 1522, 'Veracruz'), (-99.1, 19.4, 1524, 'Mexico')]),
    ('South along the Andes', [(-70.0, 18.5, 1500, None), (-79.5, 9.0, 1520, 'Panama'), (-78.5, 0.0, 1535, None), (-77.0, -12.05, 1540, 'Lima'), (-75.7, -14.07, 1551, 'Ica'),
                               (-71.5, -22.0, 1553, None), (-70.67, -33.45, 1555, 'Santiago'), (-68.84, -32.89, 1561, 'Mendoza')]),
    ('To the Cape', [(-16.5, 28.3, 1480, None), (-20.0, 10.0, 1600, None), (-10.0, -10.0, 1640, None), (5.0, -25.0, 1650, None), (18.42, -33.93, 1655, 'The Cape')]),
    ('North to California', [(-99.1, 19.4, 1524, None), (-105.0, 23.0, 1600, None), (-110.5, 27.5, 1700, None), (-114.7, 30.7, 1750, 'Baja California'), (-117.16, 32.72, 1769, 'San Diego'),
                             (-121.9, 36.6, 1800, 'Monterey'), (-122.5, 38.3, 1823, 'Sonoma'), (-122.3, 38.4, 1838, 'Napa')]),
    ('Across the Indian Ocean to Australia', [(18.42, -33.93, 1655, None), (60.0, -40.0, 1750, None), (110.0, -42.0, 1780, None), (140.0, -40.0, 1786, None), (151.2, -33.87, 1788, 'Sydney'),
                                              (151.3, -32.7, 1825, 'The Hunter'), (145.0, -35.0, 1835, None), (138.9, -34.5, 1842, 'The Barossa')]),
    ('And to New Zealand', [(151.2, -33.87, 1788, None), (165.0, -35.0, 1810, None), (173.95, -35.22, 1819, 'Kerikeri'), (175.5, -39.5, 1900, None), (173.7, -41.5, 1973, 'Marlborough')]),
]

def unit(lon, lat):
    return (math.cos(lat * D) * math.sin(lon * D), math.sin(lat * D), math.cos(lat * D) * math.cos(lon * D))

def arc(a, b):
    u, v = unit(*a), unit(*b)
    return math.acos(max(-1, min(1, u[0] * v[0] + u[1] * v[1] + u[2] * v[2])))

def along(a, b, t):
    u, v, w = unit(*a), unit(*b), arc(a, b)
    if w < 1e-9:
        return a
    sa, sb = math.sin((1 - t) * w) / math.sin(w), math.sin(t * w) / math.sin(w)
    p = tuple(u[i] * sa + v[i] * sb for i in range(3))
    return (math.atan2(p[0], p[2]) / D, math.asin(max(-1, min(1, p[1]))) / D)

def main():
    branches, leaves, nodes = [], [], []
    leaf_seed = 0
    for bi, (name, verts) in enumerate(BRANCHES):
        pts = []   # [lon, lat, year]
        walked = 0.0   # km along the branch, for the stem's wander
        for i, (lon, lat, year, place) in enumerate(verts):
            if place:
                nodes.append({'name': place, 'lon': lon, 'lat': lat, 'year': year, 'branch': bi})
            if i == 0:
                pts.append([round(lon, 3), round(lat, 3), year])
                continue
            plon, plat, pyear, _ = verts[i - 1]
            km = arc((plon, plat), (lon, lat)) * 6371
            n = max(1, int(km // 70))
            for k in range(1, n + 1):
                t = k / n
                q = along((plon, plat), (lon, lat), t)
                # a vine wanders: the stem sways a little either side of the straight way, most in the
                # middle of a stretch and not at all at the places it reaches
                walked_here = walked + km * t
                sway = math.sin(walked_here / 260.0 + bi * 1.7) * 0.22 * math.sin(t * math.pi)
                nxt = along((plon, plat), (lon, lat), min(1, t + 0.01))
                dx, dy = nxt[0] - q[0], nxt[1] - q[1]
                L = math.hypot(dx, dy) or 1
                qlon = q[0] + (-dy / L) * sway / max(0.2, math.cos(q[1] * D))
                qlat = q[1] + (dx / L) * sway
                pts.append([round(qlon, 3), round(qlat, 3), round(pyear + (year - pyear) * t, 1)])
            walked += km
        # leaves every ~200 km along the densified way, alternating sides, a little off the stem
        dist = 0.0
        for i in range(1, len(pts)):
            a, b = pts[i - 1], pts[i]
            seg = arc((a[0], a[1]), (b[0], b[1])) * 6371
            dist += seg
            if dist >= 110:
                dist = 0
                leaf_seed += 1
                side = 1 if leaf_seed % 2 else -1
                # perpendicular to the way, about 60 km off it
                dx, dy = b[0] - a[0], b[1] - a[1]
                L = math.hypot(dx, dy) or 1
                off = 0.42
                lon = b[0] + (-dy / L) * off * side / max(0.2, math.cos(b[1] * D))
                lat = b[1] + (dx / L) * off * side
                leaves.append([round(lon, 3), round(lat, 3), b[2], leaf_seed % 3])
        branches.append({'name': name, 'way': pts, 'start': verts[0][2], 'end': verts[-1][2]})
    data = {'origin': {'lon': ORIGIN[0], 'lat': ORIGIN[1], 'year': -6000, 'name': 'Gadachrili Gora'}, 'branches': branches, 'leaves': leaves, 'nodes': nodes}
    OUT.write_text('/* The vine\'s branches, each a way with a year at each point (negative BCE), densified along the great\n'
                   '   circle; the leaves along them ([lon, lat, year, kind]); the named places where clusters hang. Built\n'
                   '   by tools/vine/build.py, generalised after the record in SOURCES.md; the ways are drawn. */\n'
                   'window.VINE = ' + json.dumps(data, separators=(',', ':')) + ';\n', encoding='utf-8')
    print(f'{OUT}: {len(branches)} branches, {sum(len(b["way"]) for b in branches)} points, {len(leaves)} leaves, {len(nodes)} places, {OUT.stat().st_size // 1024} KB')

if __name__ == '__main__':
    main()
