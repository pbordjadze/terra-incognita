#!/usr/bin/env python3
"""The vine's long walk (Prototypes/vine/data.js): the grapevine's spread from the South Caucasus over
eight thousand years as one plant, built for the globe's dated geography (docs/runtime3.md, runtime 6).

    python3 tools/vine/build.py

What it writes, all of it dated (a year at every place, negative BCE), so the globe draws it as far as
the date has come and the script sets each layer once:

- `branches`: the trunk ways, each a list of [lon, lat, year] densified along great circles every
  ~60 km with the years interpolated, and `from`, the trunk and point it grows out of (null for the
  first). The ways are generalised after the record (SOURCES.md): they follow the coasts, valleys and
  roads the vine is known to have travelled, not the route of any one vine.
- `twigs`: short ways from the nearest trunk to the world's wine regions, each dated to the earliest
  planting the record gives for the region; they grow out in the years before it.
- `strands`: two thinner stems that twine round each trunk's land runs, offset either side by a sine of
  the distance along the way and crossing the trunk, dated as the trunk is.
- `stems` and `voyages`: every branch and twig cut into runs over land and over sea by the globe's own
  land (Globe/geo.png's coast sign, as the app's shader reads it): on land the vine is a stem that
  thickens with age; over the sea it is a voyage, a fine dashed line (the vine was carried), with the
  same dates, so the two meet at every landfall.
- `leaves`: [lon, lat, year, kind, south] every ~110 km along the land runs, alternating sides, dated
  a little after the stem passes; `south` says which hemisphere's seasons it keeps.
- `clusters`: the named places (the trunks' and the regions') with the year the vine arrives.
- `tips`: for each trunk, its whole way as a dated way for the tendril sprite at the growing tip.
- `ticks`, `pace`: the strip's marks (the trunks' named places) and the uneven pace of the years.

Rebuild, never hand-edit. The years are the record's earliest evidence at or near each place, rounded;
between places they are interpolated and mean nothing exact.
"""
import json, math, random
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Prototypes' / 'vine' / 'data.js'
D = math.pi / 180
KM = 6371.0

ORIGIN = (44.75, 41.45, -6000, 'Gadachrili Gora')   # Kvemo Kartli, Georgia: the earliest wine, about 6000 BCE

# ---- the trunks: name, then vertices (lon, lat, year, name or None). A trunk's first vertex lies on an
# earlier trunk (the place it grows from); the years rise along it. -------------------------------------
TRUNKS = [
    ('The first vine', [ORIGIN, (44.95, 41.58, -5960, None), (45.4, 41.62, -5850, None), (46.0, 41.7, -5700, 'Kakheti'), (46.9, 41.3, -5600, None),
                        (47.8, 40.5, -5300, None), (49.0, 40.3, -5000, None)]),
    ('West to Colchis', [(45.4, 41.62, -5760, None), (45.0, 42.15, -5730, None), (44.3, 42.05, -5700, None), (44.2, 41.7, -5690, 'Shulaveris Gora'), (43.9, 42.0, -5660, None),
                         (42.9, 42.15, -5600, None), (42.0, 42.2, -5400, 'Colchis'), (41.95, 41.8, -5200, None)]),
    ('Down the Zagros', [(45.4, 41.62, -5720, None), (45.55, 41.0, -5650, None), (45.1, 40.5, -5600, None), (45.2, 39.73, -5500, 'Areni'), (45.0, 39.0, -5400, None), (45.47, 36.98, -5000, 'Hajji Firuz'),
                         (46.2, 36.0, -4700, None), (47.0, 35.5, -4300, None), (47.6, 34.9, -3900, None), (48.07, 34.52, -3500, 'Godin Tepe'),
                         (48.8, 33.4, -3200, None), (49.7, 32.6, -3000, None), (51.0, 31.2, -2800, None), (52.0, 30.3, -2600, None), (52.5, 29.6, -2500, 'Shiraz')]),
    ('Into Anatolia', [(44.2, 41.7, -5520, None), (43.9, 41.1, -5460, None), (42.7, 40.5, -5400, None), (41.3, 40.0, -5300, None), (39.5, 39.6, -5000, 'Eastern Anatolia'),
                       (37.5, 39.3, -4600, None), (35.5, 39.2, -4300, None), (33.5, 39.0, -3900, None), (31.0, 39.0, -3600, None), (29.5, 39.2, -3400, None),
                       (28.0, 39.4, -3200, None), (27.0, 39.8, -3100, None), (26.24, 39.96, -3000, 'Troy')]),
    ('To the Levant and Egypt', [(39.5, 39.6, -5000, None), (38.3, 38.4, -4800, None), (37.0, 37.2, -4600, None), (36.6, 36.3, -4400, None), (36.0, 35.2, -4200, None),
                                 (35.9, 34.5, -4000, 'Byblos'), (35.5, 33.9, -3900, None), (35.4, 33.0, -3800, None), (35.0, 32.0, -3600, None), (34.7, 31.6, -3450, None),
                                 (34.3, 31.3, -3300, None), (33.5, 31.1, -3200, None), (32.3, 30.9, -3150, None), (31.4, 30.7, -3100, 'The Nile Delta'),
                                 (31.2, 29.9, -3050, None), (31.0, 29.0, -3000, None), (31.2, 27.8, -2950, None), (31.92, 26.18, -2900, 'Abydos')]),
    ('Over the sea to Crete and Greece', [(35.9, 34.5, -4000, None), (34.6, 34.7, -3800, None), (33.2, 34.9, -3500, 'Cyprus'), (32.4, 34.7, -3400, None),
                                          (30.5, 34.9, -3200, None), (28.5, 35.3, -2900, None), (26.3, 35.3, -2600, None), (25.1, 35.3, -2500, 'Crete'),
                                          (24.0, 35.9, -2300, None), (23.0, 36.6, -2000, None), (22.9, 37.2, -1900, None), (22.75, 37.7, -1800, 'Mycenae'),
                                          (23.3, 37.9, -1600, None), (23.7, 38.0, -1400, 'Athens')]),
    ('The Phoenicians, west', [(35.9, 34.5, -4000, None), (35.6, 33.9, -2500, None), (35.2, 33.3, -1100, 'Tyre'), (34.0, 33.6, -1000, None), (31.0, 34.5, -980, None),
                               (27.0, 34.6, -950, None), (22.0, 35.0, -920, None), (18.0, 35.6, -890, None), (15.0, 36.5, -860, None), (12.5, 37.0, -830, None),
                               (10.33, 36.85, -800, 'Carthage'), (9.5, 38.5, -750, None), (9.0, 39.5, -700, 'Sardinia'), (5.0, 39.3, -680, None),
                               (1.4, 38.9, -650, 'Ibiza'), (-1.5, 37.6, -630, None), (-4.5, 36.3, -610, None), (-6.3, 36.5, -600, 'Cádiz'), (-7.5, 37.0, -500, None),
                               (-8.8, 37.9, -450, None), (-9.1, 38.7, -400, 'Lisbon')]),
    ('The Greeks, west', [(23.7, 38.0, -1400, None), (22.9, 37.9, -800, 'Corinth'), (21.5, 38.2, -780, None), (20.0, 37.8, -760, None), (17.5, 37.3, -745, None),
                          (15.3, 37.1, -730, 'Sicily'), (15.8, 38.3, -715, None), (16.6, 39.3, -700, 'Oenotria'), (15.8, 40.2, -650, None), (14.3, 40.8, -600, 'Campania'),
                          (13.5, 41.6, -550, None), (12.3, 42.4, -500, 'Etruria'), (11.3, 43.6, -400, None), (11.0, 44.5, -250, None), (10.9, 45.3, -150, None),
                          (8.9, 45.5, -100, 'Piedmont'), (7.5, 46.3, 50, 'Valais')]),
    ('Massalia and the Rhône', [(15.3, 37.1, -730, None), (13.5, 39.5, -680, None), (11.0, 41.5, -640, None), (8.0, 43.0, -620, None), (5.37, 43.3, -600, 'Massalia'),
                                (4.8, 43.9, -400, None), (4.8, 44.5, -100, 'The Rhône'), (4.85, 45.75, 1, 'Lyon'), (4.8, 46.5, 60, None), (4.85, 47.0, 100, 'Burgundy'),
                                (4.5, 48.0, 200, None), (4.0, 49.0, 300, 'Champagne'), (2.35, 48.85, 400, 'Paris'), (0.8, 49.5, 500, None), (-0.1, 51.5, 700, 'Britain')]),
    ('To Bordeaux and the Loire', [(5.37, 43.3, -600, None), (3.0, 43.2, -120, 'Narbonne'), (1.44, 43.6, -30, 'Toulouse'), (0.2, 44.2, 20, None),
                                   (-0.58, 44.84, 50, 'Bordeaux'), (-0.7, 45.8, 200, None), (-0.9, 46.6, 300, None), (-1.5, 47.2, 450, 'The Loire'),
                                   (0.7, 47.4, 500, None), (2.8, 47.5, 580, 'Sancerre')]),
    ('Over the Alps to the Mosel and the Rhine', [(10.9, 45.3, -150, None), (10.5, 46.2, -50, None), (9.4, 46.9, 20, None), (8.5, 47.5, 100, None),
                                                  (7.6, 48.6, 150, 'Alsace'), (6.9, 49.3, 190, None), (6.64, 49.75, 250, 'Trier, on the Mosel'),
                                                  (7.4, 50.1, 290, None), (8.27, 50.0, 330, 'Mainz, on the Rhine'), (8.0, 50.05, 800, 'The Rheingau'),
                                                  (9.9, 49.8, 850, 'Franconia')]),
    ('Up the Danube', [(10.9, 45.3, -90, None), (12.5, 45.7, -100, None), (13.7, 46.1, -40, None), (15.0, 46.4, 10, None), (15.5, 48.3, 90, 'The Wachau'),
                       (17.1, 48.1, 110, None), (18.0, 47.5, 130, 'Pannonia'), (17.8, 46.9, 200, 'Balaton'), (19.0, 47.0, 400, None), (20.4, 47.9, 900, 'Eger'),
                       (21.4, 48.1, 1100, 'Tokaj')]),
    ('Inland in Iberia', [(-6.3, 36.5, -600, None), (-6.0, 37.4, -300, 'Seville'), (-5.0, 37.9, -200, None), (-4.2, 38.3, -120, None), (-3.6, 39.3, -40, None),
                          (-3.2, 40.4, 1, None), (-2.9, 41.5, 60, None), (-2.5, 42.4, 100, 'Rioja'), (-4.0, 42.3, 150, None), (-5.8, 41.6, 200, None),
                          (-7.5, 41.2, 250, 'The Douro'), (-8.4, 41.7, 300, None), (-8.7, 42.4, 400, 'Rías Baixas')]),
    ('Thrace and the Black Sea', [(26.24, 39.96, -3000, None), (26.6, 40.6, -2600, None), (26.0, 41.3, -2200, None), (25.6, 41.9, -1800, None), (25.0, 42.3, -1400, None),
                                  (25.9, 42.4, -1000, 'Thrace'), (27.5, 42.5, -700, None), (28.9, 43.4, -600, 'Dionysopolis'), (29.5, 44.8, -550, None),
                                  (30.4, 46.1, -500, 'Olbia'), (30.5, 46.5, -450, None), (29.0, 46.9, -300, 'Moldova')]),
    ('The Greeks to Crimea', [(28.9, 43.4, -600, None), (30.2, 44.2, -560, None), (32.0, 44.8, -500, None), (33.5, 44.6, -422, 'Chersonesus')]),
    ('The Silk Road, east', [(52.5, 29.6, -2500, None), (54.5, 31.5, -2000, None), (56.0, 33.0, -1500, None), (57.5, 34.5, -1200, None), (59.0, 36.0, -1000, 'Khorasan'),
                             (61.0, 36.9, -700, None), (62.2, 37.65, -500, 'Merv'), (64.4, 39.8, -400, 'Bukhara'), (66.97, 39.65, -300, 'Samarkand'),
                             (69.0, 40.6, -250, None), (71.0, 40.4, -200, 'Ferghana'), (73.5, 39.9, -180, None), (76.0, 39.5, -150, 'Kashgar'),
                             (80.0, 41.5, -140, None), (84.0, 42.3, -135, None), (89.2, 42.95, -130, 'Turpan'), (93.0, 41.8, -128, None), (94.7, 40.1, -125, 'Dunhuang'),
                             (98.5, 39.7, -124, None), (103.8, 36.0, -122, None), (106.0, 35.5, -121, None), (108.9, 34.27, -120, "Chang'an")]),
    ('Into India', [(62.2, 37.65, -500, None), (65.7, 34.7, -400, None), (69.2, 34.5, -300, 'Kapisa'), (71.5, 33.6, -250, None), (73.0, 33.0, -200, 'Taxila')]),
    ('On to Korea and Japan', [(108.9, 34.27, -120, None), (113.0, 34.8, 100, None), (117.0, 36.5, 300, None), (121.0, 37.5, 400, None), (124.5, 38.0, 450, None),
                               (126.0, 37.3, 500, None), (127.5, 35.8, 550, None), (129.1, 35.2, 600, None), (131.0, 34.2, 650, None), (134.0, 34.7, 690, None),
                               (136.0, 35.2, 705, None), (138.7, 35.65, 718, 'Katsunuma')]),
    ('Out to Madeira and the Canaries', [(-9.1, 38.7, -400, None), (-10.5, 37.0, 1300, None), (-13.0, 35.0, 1400, None), (-16.9, 32.65, 1425, 'Madeira'),
                                         (-16.5, 30.0, 1460, None), (-16.5, 28.3, 1480, 'The Canaries')]),
    ('Across the ocean to Mexico', [(-16.5, 28.3, 1480, None), (-25.0, 24.0, 1493, None), (-40.0, 21.0, 1495, None), (-55.0, 19.0, 1497, None), (-65.0, 18.6, 1499, None),
                                    (-70.0, 18.5, 1500, 'Hispaniola'), (-74.0, 19.5, 1510, None), (-78.0, 21.0, 1514, None), (-85.0, 20.5, 1519, None),
                                    (-96.1, 19.2, 1522, 'Veracruz'), (-97.5, 19.3, 1523, None), (-99.1, 19.4, 1524, 'Mexico')]),
    ('North by the Rio Grande', [(-99.1, 19.4, 1524, None), (-101.0, 21.0, 1550, None), (-102.6, 22.8, 1570, 'Zacatecas'), (-102.2, 25.4, 1597, 'Parras'),
                                 (-104.5, 28.5, 1615, None), (-106.3, 30.2, 1625, None), (-106.9, 33.4, 1629, 'Senecú, on the Rio Grande'),
                                 (-106.5, 31.75, 1659, 'El Paso del Norte')]),
    ('South along the Andes', [(-70.0, 18.5, 1500, None), (-74.0, 15.0, 1510, None), (-79.5, 9.0, 1520, None), (-80.0, 4.0, 1530, None), (-81.0, -2.0, 1535, None),
                               (-80.5, -7.0, 1538, None), (-77.0, -12.05, 1540, 'Lima'), (-75.7, -14.07, 1551, 'Ica'), (-72.0, -18.0, 1553, None),
                               (-70.4, -23.6, 1554, None), (-70.67, -33.45, 1555, 'Santiago'), (-70.0, -32.8, 1558, None), (-68.84, -32.89, 1561, 'Mendoza'),
                               (-66.0, -32.0, 1600, None), (-64.5, -31.4, 1620, 'Córdoba'), (-60.0, -32.9, 1700, None), (-58.4, -34.6, 1750, 'Buenos Aires'),
                               (-56.2, -34.9, 1870, 'Montevideo')]),
    ('To Brazil', [(-16.9, 32.65, 1425, None), (-20.0, 20.0, 1500, None), (-26.0, 10.0, 1510, None), (-33.0, 0.0, 1520, None), (-38.0, -8.0, 1525, None),
                   (-40.0, -17.0, 1528, None), (-43.0, -22.5, 1530, None), (-46.4, -23.97, 1532, 'São Vicente'), (-47.0, -23.0, 1600, None), (-49.5, -25.5, 1750, None),
                   (-51.5, -29.2, 1875, 'The Serra Gaúcha')]),
    ('To the Cape', [(-16.5, 28.3, 1480, None), (-23.0, 15.0, 1500, None), (-26.0, 5.0, 1520, None), (-22.0, -8.0, 1560, None), (-12.0, -20.0, 1600, None),
                     (0.0, -30.0, 1630, None), (10.0, -35.0, 1645, None), (18.42, -33.93, 1655, 'The Cape'), (18.86, -33.93, 1679, 'Stellenbosch'),
                     (19.2, -33.7, 1700, 'Paarl')]),
    ('North to California', [(-99.1, 19.4, 1524, None), (-101.5, 20.7, 1560, None), (-104.0, 22.0, 1600, None), (-105.5, 23.5, 1650, None), (-107.5, 25.5, 1700, None),
                             (-110.5, 27.5, 1720, None), (-111.7, 28.8, 1735, None), (-114.7, 30.7, 1750, 'Baja California'), (-116.0, 31.9, 1760, None),
                             (-117.16, 32.72, 1769, 'San Diego'), (-118.3, 34.1, 1771, 'San Gabriel'), (-120.5, 35.0, 1790, None), (-121.9, 36.6, 1800, 'Monterey'),
                             (-122.0, 37.5, 1810, None), (-122.5, 38.3, 1823, 'Sonoma'), (-122.3, 38.4, 1838, 'Napa')]),
    ('North to Oregon', [(-122.3, 38.4, 1838, None), (-122.8, 40.5, 1900, None), (-122.9, 42.5, 1950, None), (-123.1, 45.3, 1965, 'The Willamette')]),
    ('Across the Indian Ocean to Australia', [(18.42, -33.93, 1655, None), (30.0, -40.0, 1700, None), (50.0, -42.0, 1740, None), (75.0, -43.0, 1770, None),
                                              (100.0, -43.5, 1780, None), (125.0, -42.0, 1784, None), (140.0, -40.0, 1786, None), (148.0, -37.0, 1787, None),
                                              (151.2, -33.87, 1788, 'Sydney'), (151.3, -32.7, 1828, 'The Hunter'), (149.0, -34.5, 1832, None), (145.0, -35.5, 1835, None),
                                              (141.0, -34.5, 1838, None), (138.9, -34.5, 1842, 'The Barossa'), (138.6, -33.8, 1848, 'The Clare')]),
    ('To Victoria and Tasmania', [(151.2, -33.87, 1788, None), (150.0, -36.0, 1810, None), (147.5, -38.5, 1820, None), (147.1, -41.4, 1823, 'Tasmania'),
                                  (145.4, -37.7, 1838, 'The Yarra')]),
    ('And to New Zealand', [(151.2, -33.87, 1788, None), (158.0, -34.5, 1800, None), (165.0, -35.0, 1810, None), (171.0, -34.8, 1816, None), (173.95, -35.22, 1819, 'Kerikeri'),
                            (175.0, -37.5, 1840, None), (176.9, -39.5, 1851, "Hawke's Bay"), (175.5, -41.0, 1900, None), (173.95, -41.5, 1973, 'Marlborough')]),
]

# ---- the world's wine regions the trunks don't reach, each a twig from the nearest trunk the vine had
# reached by then: name, lon, lat, the earliest planting the record gives (SOURCES.md). ----------------
REGIONS = [
    ('The Alazani valley', 45.7, 41.95, -5000), ('Commandaria', 33.0, 34.8, -2000), ('The Bekaa', 36.0, 33.8, -3000), ('Santorini', 25.43, 36.4, -1600),
    ('Nemea', 22.65, 37.8, -1000), ('Chios', 26.1, 38.4, -700), ('Naxos', 25.4, 37.1, -1000), ('Samos', 26.8, 37.75, -800),
    ('Corsica', 9.3, 42.0, -570), ('Issa', 16.2, 43.05, -400), ('Chianti', 11.3, 43.5, -400), ('Valpolicella', 10.9, 45.5, -100),
    ('Priorat', 0.8, 41.2, 1194), ('Penedès', 1.7, 41.4, -100), ('Jerez', -6.1, 36.7, -600), ('Vinho Verde', -8.4, 41.6, 300),
    ('Languedoc', 3.5, 43.4, -550), ('Beaujolais', 4.7, 46.1, 50), ('The Jura', 5.7, 46.9, 100), ('Savoie', 6.0, 45.6, 100),
    ('Eastern Sicily', 15.0, 37.75, -730), ('Carthage', 10.33, 36.85, -800), ('Dalmatia', 17.0, 43.3, -400), ('Pfalz', 8.2, 49.3, 300),
    ('Württemberg', 9.2, 49.0, 750), ('Slovenia', 15.6, 46.5, -50), ('Eger', 20.4, 47.9, 900), ('Dealu Mare', 26.2, 45.0, -500),
    ('Mavrud', 24.8, 42.1, -1000), ('Kakheti', 45.9, 41.8, -5500), ('Yerevan', 44.5, 40.2, -4100),
    ('Turpan', 89.2, 42.95, -130), ('Ningxia', 106.0, 38.3, 1984), ('Yantai', 121.4, 37.5, 1892), ('Yamanashi', 138.6, 35.7, 1874),
    ('Meknès', -5.55, 33.9, 1912), ('Algiers', 3.0, 36.75, 1830), ('Constantia', 18.42, -34.03, 1685), ('The Swartland', 18.7, -33.3, 1700),
    ('Casa Madero', -102.2, 25.4, 1597), ('Tarija', -64.7, -21.5, 1600), ('Cafayate', -65.97, -26.07, 1850), ('Casablanca valley', -71.4, -33.3, 1982),
    ('Colchagua', -71.3, -34.6, 1560), ('Canelones', -56.3, -34.5, 1870), ('Santa Barbara', -120.0, 34.6, 1782), ('Paso Robles', -120.7, 35.6, 1797),
    ('Lodi', -121.3, 38.1, 1850), ('Coonawarra', 140.8, -37.3, 1890), ('Margaret River', 115.1, -33.95, 1967), ('The Swan valley', 116.0, -31.8, 1829),
    ('Central Otago', 169.3, -45.0, 1864), ('Gisborne', 178.0, -38.65, 1850), ('Rutherglen', 146.5, -36.05, 1851),
]

# ---- the globe's land ---------------------------------------------------------------------------------
_geo = np.array(Image.open(ROOT / 'Globe' / 'geo.png'))
W, H = _geo.shape[1], _geo.shape[0]
LAND = _geo[..., 1].astype(int) > 128          # the coarse coast distance's sign (land positive)


def on_land(lon, lat):
    x = int(((lon + 180) % 360) / 360 * W) % W
    y = min(H - 1, max(0, int((90 - lat) / 180 * H)))
    return bool(LAND[y, x])


# ---- geometry -------------------------------------------------------------------------------------------
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


def cut_corners(verts):
    """One pass of corner cutting at the unnamed vertices (a named place stays exactly where it is): a
    zigzag of waypoints becomes a bend, as a vine's way does."""
    if len(verts) < 3:
        return list(verts)
    out = [verts[0]]
    for i in range(1, len(verts) - 1):
        v = verts[i]
        if len(v) > 3 and v[3]:
            out.append(v)
            continue
        a, b = verts[i - 1], verts[i + 1]
        out.append((0.25 * a[0] + 0.5 * v[0] + 0.25 * b[0], 0.25 * a[1] + 0.5 * v[1] + 0.25 * b[1], v[2]) + tuple(v[3:]))
    out.append(verts[-1])
    return out


def curved(verts):
    """The vertices with a few more between each pair, on a smooth curve through them all (Catmull-Rom
    in longitude and latitude), so the way bends at a place rather than kinking; the years follow the
    curve's parameter. A vertex's name stays on it."""
    verts = cut_corners(cut_corners(verts))
    n = len(verts)
    if n < 3:
        return list(verts)
    out = []
    for i in range(n - 1):
        p0, p1, p2, p3 = verts[max(0, i - 1)], verts[i], verts[i + 1], verts[min(n - 1, i + 2)]
        out.append(verts[i])
        km = arc((p1[0], p1[1]), (p2[0], p2[1])) * KM
        k = max(2, min(24, int(km // 40)))
        for j in range(1, k):
            t = j / k
            t2, t3 = t * t, t * t * t
            def cr(a, b, c, d):
                return 0.5 * (2 * b + (-a + c) * t + (2 * a - 5 * b + 4 * c - d) * t2 + (-a + 3 * b - 3 * c + d) * t3)
            lon = cr(p0[0], p1[0], p2[0], p3[0])
            lat = cr(p0[1], p1[1], p2[1], p3[1])
            out.append((lon, lat, p1[2] + (p2[2] - p1[2]) * t))
    out.append(verts[-1])
    return out


def densify(verts, step_km=45.0, sway=0.3, seed=0):
    """[lon, lat, year] every ~step_km along the great circles between the vertices, the years
    interpolated, the way swaying a little either side of the straight line between places (a vine
    wanders; nothing at the places themselves)."""
    verts = curved(verts)
    pts = [[round(verts[0][0], 3), round(verts[0][1], 3), verts[0][2]]]
    walked = 0.0
    for i in range(1, len(verts)):
        (plon, plat, pyear), (lon, lat, year) = verts[i - 1][:3], verts[i][:3]
        km = arc((plon, plat), (lon, lat)) * KM
        n = max(1, int(km // step_km))
        for k in range(1, n + 1):
            t = k / n
            q = along((plon, plat), (lon, lat), t)
            # (two wavelengths, so the wander is a vine's and not a sine)
            d_here = walked + km * t
            s = (math.sin(d_here / 230.0 + seed * 1.7) + 0.45 * math.sin(d_here / 97.0 + seed * 0.9)) * sway * math.sin(t * math.pi)
            nxt = along((plon, plat), (lon, lat), min(1, t + 0.01))
            dx, dy = nxt[0] - q[0], nxt[1] - q[1]
            L = math.hypot(dx, dy) or 1
            qlon = q[0] + (-dy / L) * s / max(0.2, math.cos(q[1] * D))
            qlat = q[1] + (dx / L) * s
            pts.append([round(qlon, 3), round(qlat, 3), round(pyear + (year - pyear) * t, 1)])
        walked += km
    return pts


def nearest(way, lon, lat, by=None):
    """The index of the way's nearest point (and its distance in km), among those `by` accepts."""
    best, bd = None, 1e9
    for i, p in enumerate(way):
        if by and not by(p):
            continue
        d = arc((p[0], p[1]), (lon, lat)) * KM
        if d < bd:
            best, bd = i, d
    return best, bd


def runs(way):
    """The way cut into runs over land and over sea by the globe's land, each a list of points sharing
    its ends with the next (so stems and voyages meet at every landfall); a run shorter than four points
    (about 160 km) joins its neighbours, so a coast grazed or a lake crossed doesn't flicker between the two."""
    kinds = [on_land(p[0], p[1]) for p in way]
    # the trunks' named places are on land whatever the coarse coast says of a small island or a delta
    for i, p in enumerate(way):
        if len(p) > 3 and p[3]:
            kinds[i] = True
    changed = True
    while changed:
        changed = False
        i = 0
        while i < len(kinds):
            j = i
            while j < len(kinds) and kinds[j] == kinds[i]:
                j += 1
            if j - i < 4 and (i > 0 or j < len(kinds)):
                fill = kinds[i - 1] if i > 0 else kinds[j]
                for k in range(i, j):
                    kinds[k] = fill
                changed = True
            i = j
    out, start = [], 0
    for i in range(1, len(way) + 1):
        if i == len(way) or kinds[i] != kinds[start]:
            # (a run includes the first point of the next, so the line is unbroken)
            end = min(len(way), i + 1)
            if end - start >= 2:
                out.append(('land' if kinds[start] else 'sea', start, end))
            start = i
    return out


def main():
    branches = []
    for ti, (name, verts) in enumerate(TRUNKS):
        way = densify(verts, seed=ti)
        # the vertices' names, kept on the densified way for the land rule and the clusters
        named = {}
        for lon, lat, year, place in verts:
            if place:
                i, d = nearest(way, lon, lat)
                named[i] = place
        for i, place in named.items():
            way[i] = way[i] + [place]
        frm = None
        if ti > 0:
            lon, lat = verts[0][0], verts[0][1]
            cands = [(nearest(branches[b]['way'], lon, lat), b) for b in range(ti)]
            (i, d), b = min(cands, key=lambda c: c[0][1])
            # (corner cutting moves an unnamed waypoint a little: the branch is attached to the nearest trunk
            # point, its own first place moved onto it)
            if d > 80:
                raise SystemExit(f'{name} starts {d:.0f} km from any earlier trunk')
            frm = [b, i]
            way[0] = [branches[b]['way'][i][0], branches[b]['way'][i][1], way[0][2]] + way[0][3:]
            if branches[b]['way'][i][2] > verts[0][2] + 60:
                raise SystemExit(f'{name} starts in {verts[0][2]} where {branches[b]["name"]} arrives only in {branches[b]["way"][i][2]}')
        branches.append({'name': name, 'way': way, 'from': frm, 'twig': False})
    trunks = len(branches)

    # the regions: a twig from the nearest trunk point the vine had reached by the region's year
    for ri, (name, lon, lat, year) in enumerate(REGIONS):
        cands = []
        for b in range(trunks):
            i, d = nearest(branches[b]['way'], lon, lat, by=lambda p: p[2] <= year)
            if i is not None:
                cands.append((d, b, i))
        if not cands:
            raise SystemExit(f'{name} ({year}): no trunk had arrived by then')
        d, b, i = min(cands)
        if d > 1400:
            raise SystemExit(f'{name} ({year}) is {d:.0f} km from the nearest trunk by then ({branches[b]["name"]}): give it a trunk')
        start = branches[b]['way'][i]
        if d < 25:
            # the region lies on the trunk itself: a cluster there, no twig
            branches[b]['way'][i] = start[:3] + [name] if len(start) < 4 else start
            continue
        # it grows out over the years before its planting, never before the trunk arrived
        span = max(15, min(400, d / 4))
        y0 = max(start[2], year - span)
        if y0 >= year:
            y0 = year - 1
        # it leaves along the trunk's own direction for a little, then bends to the region: a branch, not a
        # right angle
        way_t = branches[b]['way']
        nxt = way_t[min(len(way_t) - 1, i + 1)] if i + 1 < len(way_t) else way_t[i - 1]
        tdx, tdy = nxt[0] - start[0], nxt[1] - start[1]
        tl = math.hypot(tdx, tdy) or 1
        dx, dy = lon - start[0], lat - start[1]
        L = math.hypot(dx, dy) or 1
        # (along the trunk the way the region lies, if it lies that way at all)
        sign = 1 if tdx * dx + tdy * dy >= 0 else -1
        lead = min(0.45, d / 2500)
        knee = (start[0] + sign * tdx / tl * lead, start[1] + sign * tdy / tl * lead)
        mid = along(knee, (lon, lat), 0.5)
        side = 1 if ri % 2 else -1
        bend = min(0.5, d / 1500)
        mid = (mid[0] + (-dy / L) * bend * side, mid[1] + (dx / L) * bend * side)
        verts = [(start[0], start[1], y0), (knee[0], knee[1], y0 + (year - y0) * 0.2), (mid[0], mid[1], y0 + (year - y0) * 0.6), (lon, lat, year, name)]
        way = densify(verts, step_km=45, sway=0.04, seed=100 + ri)
        way[-1] = way[-1] + [name]
        branches.append({'name': name, 'way': way, 'from': [b, i], 'twig': True})

    # stems and voyages: the runs over land and sea, as dated line features
    stems, voyages, leaves, clusters, tips, strands = [], [], [], [], [], []
    # where the vine grew, not only passed: the named places; a leaf opens only within reach of one, so
    # the way through a desert or along a coast it was carried past is a bare stem
    grew = [(p[0], p[1]) for b in branches for p in b['way'] if len(p) > 3 and p[3]]
    def near_vines(lon, lat, km=260):
        return any(arc((lon, lat), g) * KM <= km for g in grew)
    leaf_seed = 0
    for bi, br in enumerate(branches):
        way = br['way']
        rnd = random.Random(bi * 7919 + 13)
        gap = rnd.uniform(55, 125)
        for kind, s, e in runs(way):
            feature = {'branch': bi, 'start': s, 'coordinates': [[p[0], p[1], p[2]] for p in way[s:e]]}
            (stems if kind == 'land' else voyages).append(feature)
            if kind == 'land' and not br['twig'] and e - s >= 4:
                # two thinner stems twine round the trunk: each offset to one side by a sine of the distance
                # along the way (in degrees of latitude, a few tens of km), crossing the trunk twice a wave,
                # dated as the trunk is, so they grow with it
                # (the run resampled four times finer first, so the twining is a curve and not a zigzag)
                fine = []
                for i in range(s, e - 1):
                    a, b = way[i], way[i + 1]
                    for k in range(4):
                        t = k / 4
                        fine.append([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t])
                fine.append(list(way[e - 1][:3]))
                for side in (1, -1):
                    pts, dist = [], 0.0
                    for i in range(len(fine)):
                        if i > 0:
                            dist += arc((fine[i - 1][0], fine[i - 1][1]), (fine[i][0], fine[i][1])) * KM
                        a, b = fine[max(0, i - 1)], fine[min(len(fine) - 1, i + 1)]
                        dx, dy = b[0] - a[0], b[1] - a[1]
                        L = math.hypot(dx, dy) or 1
                        off = side * 0.14 * math.sin(dist / 110.0 + bi)
                        pts.append([round(fine[i][0] + (-dy / L) * off / max(0.2, math.cos(fine[i][1] * D)), 3), round(fine[i][1] + (dx / L) * off, 3), round(fine[i][2], 1)])
                    strands.append({'branch': bi, 'coordinates': pts})
            if kind == 'land':
                # leaves along the land at uneven gaps (55 to 125 km), either side, a little off the stem,
                # dated a little after the stem passed (they open as the tip moves on)
                dist = 0.0
                for i in range(s + 1, e):
                    a, b = way[i - 1], way[i]
                    dist += arc((a[0], a[1]), (b[0], b[1])) * KM
                    if dist >= gap * (1 if not br['twig'] else 0.8):
                        dist = 0
                        gap = rnd.uniform(55, 125)
                        if not near_vines(b[0], b[1]):
                            continue
                        leaf_seed += 1
                        side = 1 if rnd.random() < 0.5 else -1
                        dx, dy = b[0] - a[0], b[1] - a[1]
                        L = math.hypot(dx, dy) or 1
                        off = 0.12
                        lon = b[0] + (-dy / L) * off * side / max(0.2, math.cos(b[1] * D))
                        lat = b[1] + (dx / L) * off * side
                        if on_land(lon, lat):
                            lag = max(3, abs(b[2] - a[2]) * 0.5)
                            leaves.append([round(lon, 3), round(lat, 3), round(b[2] + lag, 1), rnd.randrange(3), 1 if lat < 0 else 0])
        # a leaf at every trunk's end, where its tendril stops: a finished stem ends in a leaf, not a cut
        if not br['twig']:
            last = way[-1]
            if on_land(last[0], last[1]):
                leaves.append([round(last[0], 3), round(last[1], 3), round(last[2] + 2, 1), rnd.randrange(3), 1 if last[1] < 0 else 0])
        for p in way:
            if len(p) > 3 and p[3] and not (bi == 0 and p[3] == ORIGIN[3]):
                clusters.append({'name': p[3], 'lon': p[0], 'lat': p[1], 'year': p[2], 'branch': bi})
        if not br['twig']:
            tips.append({'branch': bi, 'name': br['name'], 'coordinates': [[p[0], p[1]] for p in way], 'dates': [p[2] for p in way]})

    # the strip's marks: the record's landmarks, not every place (a tick within a fingertip is landed on)
    # the ships: a sail on each long voyage (six points and more, about 240 km), the longest first, as many
    # as the sprite budget leaves beside the tendrils (64 in all)
    long = sorted([v for v in voyages if len(v['coordinates']) >= 6], key=lambda v: -len(v['coordinates']))[:64 - len(tips) - 4]
    ships = [{'branch': v['branch'], 'coordinates': [[c[0], c[1]] for c in v['coordinates']], 'dates': [c[2] for c in v['coordinates']]} for v in long]
    ticks = [-6000, -5000, -4000, -3000, -2000, -1000, -600, -120, 100, 718, 1425, 1500, 1655, 1788, 1973]
    # the years' pace: deep time quickly, the last five centuries slowly; about three minutes in all
    pace = [[-6000, 0], [-5500, 13], [-3000, 40], [-1000, 68], [-100, 92], [800, 110], [1400, 122], [1600, 144], [1800, 164], [1900, 178], [2026, 196]]
    data = {
        'origin': {'lon': ORIGIN[0], 'lat': ORIGIN[1], 'year': ORIGIN[2], 'name': ORIGIN[3]},
        'branches': [{'name': b['name'], 'from': b['from'], 'twig': b['twig'], 'way': [p[:3] for p in b['way']]} for b in branches],
        'stems': stems, 'voyages': voyages, 'strands': strands, 'leaves': leaves, 'clusters': clusters, 'tips': tips, 'ships': ships,
        'ticks': ticks, 'pace': pace, 'today': 2026,
    }
    OUT.write_text('/* The vine, dated for the globe (runtime 6): its trunks and twigs as ways with a year at each point\n'
                   '   (negative BCE), the stems over land and voyages over sea cut from them, the leaves along the stems,\n'
                   '   the clusters at the named places, the tendrils\' ways. Built by tools/vine/build.py after the record\n'
                   '   in SOURCES.md; the ways are drawn, not the route of any one vine. Never hand-edit. */\n'
                   'window.VINE = ' + json.dumps(data, separators=(',', ':')) + ';\n', encoding='utf-8')
    land_pts = sum(len(f['coordinates']) for f in stems)
    sea_pts = sum(len(f['coordinates']) for f in voyages)
    print(f'{OUT.relative_to(ROOT)}: {trunks} trunks, {len(branches) - trunks} twigs; {len(stems)} stems ({land_pts} points), '
          f'{len(voyages)} voyages ({sea_pts} points), {len(leaves)} leaves, {len(clusters)} clusters, {len(tips)} tips, {len(ships)} ships, {OUT.stat().st_size // 1024} KB')


if __name__ == '__main__':
    main()
