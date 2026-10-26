#!/usr/bin/env python3
"""The map data a look's page draws its plates from: one GeoJSON file per scale and layer, clipped
and simplified for the scale it is drawn at, in tools/plates/data/.

    python3 tools/plates/build_data.py <downloads>

<downloads> holds the sources, fetched as they are:
  world-atlas@2.0.2 countries-50m.json and land-50m.json (Natural Earth 50 m, public domain),
    https://cdn.jsdelivr.net/npm/world-atlas@2.0.2/
  Natural Earth 10 m (public domain), https://naciscdn.org/naturalearth/10m/...:
    physical/ne_10m_land, ne_10m_lakes, ne_10m_rivers_lake_centerlines;
    cultural/ne_10m_admin_1_states_provinces_lakes, ne_10m_populated_places_simple, ne_10m_roads
  US Census TIGER/Line 2024 (public domain), https://www2.census.gov/geo/tiger/TIGER2024/...:
    ROADS/tl_2024_<county>_roads, AREAWATER/tl_2024_<county>_areawater, LINEARWATER/tl_2024_<county>_linearwater
    for Franklin County and its neighbours (39049, 39041, 39129, 39045, 39089, 39097, 39159), and
    PLACE/tl_2024_39_place
  OpenStreetMap (ODbL: credited in the app as "© OpenStreetMap contributors"), the API's map call in three
  bands (it refuses one box this full): shortnorth-<south>.osm =
    https://api.openstreetmap.org/api/0.6/map?bbox=-83.0145,<south>,-82.9955,<north> for the bands
    39.9675-39.9780, 39.9780-39.9885 and 39.9885-39.9985
"""
import io
import json
import os
import shutil
import sys
import xml.etree.ElementTree as ET
import zipfile

import shapefile
from shapely.geometry import LineString, MultiPolygon, Point, Polygon, box, mapping, shape
from shapely.geometry.polygon import orient

SRC = sys.argv[1]
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), 'data')
BOXES = {
    'america': (-170, 5, -50, 75),
    'ohio': (-88.5, 35.0, -77.0, 45.5),
    'columbus': (-83.30, 39.55, -82.70, 40.42),
    'shortnorth': (-83.0145, 39.9675, -82.9955, 39.9985),
}


def read_shp(zipname):
    z = zipfile.ZipFile(os.path.join(SRC, zipname))
    base = [n for n in z.namelist() if n.endswith('.shp')][0][:-4]
    r = shapefile.Reader(shp=io.BytesIO(z.read(base + '.shp')), shx=io.BytesIO(z.read(base + '.shx')),
                         dbf=io.BytesIO(z.read(base + '.dbf')), encoding='latin-1')
    names = [f[0] for f in r.fields[1:]]
    for sr in r.iterShapeRecords():
        if sr.shape.shapeType != shapefile.NULL:
            yield shape(sr.shape.__geo_interface__), dict(zip(names, sr.record))


def rounded(geom, digits):
    # d3-geo reads rings on the sphere: an exterior must run clockwise, or it fills the rest of the world
    if isinstance(geom, Polygon):
        geom = orient(geom, sign=-1.0)
    elif isinstance(geom, MultiPolygon):
        geom = MultiPolygon([orient(p, sign=-1.0) for p in geom.geoms])
    g = mapping(geom)

    def walk(x):
        if isinstance(x, (list, tuple)) and x and isinstance(x[0], (int, float)):
            return [round(v, digits) for v in x]
        return [walk(y) for y in x]

    def ring_area(r):
        return sum(a[0] * b[1] - b[0] * a[1] for a, b in zip(r, r[1:])) / 2

    def fix_polygon(rings):
        # rounding can collapse a small ring or turn it over: drop the collapsed, re-turn the rest
        out = []
        for i, r in enumerate(rings):
            a = ring_area(r)
            if len({tuple(c) for c in r}) < 3 or a == 0:
                if i == 0:
                    return None
                continue
            out.append(r[::-1] if (a > 0) == (i == 0) else r)
        return out

    g['coordinates'] = walk(g['coordinates'])
    if g['type'] == 'Polygon':
        g['coordinates'] = fix_polygon(g['coordinates']) or []
    elif g['type'] == 'MultiPolygon':
        g['coordinates'] = [p for p in (fix_polygon(p) for p in g['coordinates']) if p]
    return g


def write(name, feats, digits):
    fc = {'type': 'FeatureCollection', 'features': [
        {'type': 'Feature', 'properties': p, 'geometry': r} for g, p in feats if not g.is_empty
        for r in [rounded(g, digits)] if r['coordinates']]}
    path = os.path.join(OUT, name + '.geojson')
    with open(path, 'w') as f:
        json.dump(fc, f, separators=(',', ':'))
    print(f'{name}: {len(fc["features"])} features, {os.path.getsize(path) // 1024} KB')


COUNTIES = ('39049', '39041', '39129', '39045', '39089', '39097', '39159')


def clipped(zipname, level, tol, keep=lambda p: True, props=lambda p: {}):
    b = box(*BOXES[level])
    out = []
    names = [zipname.replace('39049', c) for c in COUNTIES] if '39049' in zipname else [zipname]
    for g, p in (r for n in names for r in read_shp(n)):
        if keep(p) and g.intersects(b):
            g = g.intersection(b)
            out.append((g.simplify(tol, preserve_topology=True) if tol else g, props(p)))
    return out


def places(level, minpop):
    b = box(*BOXES[level])
    return [(g, {'name': p['name'], 'pop': int(p['pop_max'] or 0)})
            for g, p in read_shp('ne_10m_populated_places_simple.zip') if g.within(b) and (p['pop_max'] or 0) >= minpop]


os.makedirs(OUT, exist_ok=True)
for f in ('countries-50m.json', 'land-50m.json'):
    shutil.copy(os.path.join(SRC, f), os.path.join(OUT, f))

for level, tol, digits, minpop in (('america', 0.04, 3, 1_500_000), ('ohio', 0.005, 4, 40_000)):
    write(f'{level}-lakes', clipped('ne_10m_lakes.zip', level, tol, props=lambda p: {'name': p.get('name') or ''}), digits)
    write(f'{level}-rivers', clipped('ne_10m_rivers_lake_centerlines.zip', level, tol,
                                     keep=lambda p: level == 'ohio' or (p.get('scalerank') or 9) <= 6,
                                     props=lambda p: {'name': p.get('name') or ''}), digits)
    write(f'{level}-states', clipped('ne_10m_admin_1_states_provinces_lakes.zip', level, tol,
                                     keep=lambda p: p.get('adm0_a3') in ('USA', 'CAN', 'MEX'),
                                     props=lambda p: {'name': p.get('name') or '', 'country': p.get('adm0_a3')}), digits)
    write(f'{level}-places', places(level, minpop), digits)
write('america-land', clipped('ne_10m_land.zip', 'america', 0.04), 3)
write('ohio-roads', clipped('ne_10m_roads.zip', 'ohio', 0.002, keep=lambda p: p.get('type') == 'Major Highway',
                            props=lambda p: {'label': p.get('label') or ''}), 4)

ROADCLASS = {'S1100': 'interstate', 'S1200': 'major', 'S1400': 'local'}
write('columbus-roads', clipped('tl_2024_39049_roads.zip', 'columbus', 0.0001, keep=lambda p: p['MTFCC'] in ROADCLASS,
                                props=lambda p: {'name': p['FULLNAME'] or '', 'class': ROADCLASS[p['MTFCC']]}), 5)
write('columbus-water', clipped('tl_2024_39049_areawater.zip', 'columbus', 0.00005, props=lambda p: {'name': p['FULLNAME'] or ''}), 5)
write('columbus-streams', clipped('tl_2024_39049_linearwater.zip', 'columbus', 0.00005, props=lambda p: {'name': p['FULLNAME'] or ''}), 5)
write('columbus-city', clipped('tl_2024_39_place.zip', 'columbus', 0.0002,
                               keep=lambda p: p['NAME'] in ('Columbus', 'Upper Arlington', 'Grandview Heights', 'Bexley', 'Worthington', 'Whitehall'),
                               props=lambda p: {'name': p['NAME']}), 5)

# Short North, from OpenStreetMap
bands = [ET.parse(os.path.join(SRC, f)).getroot() for f in sorted(os.listdir(SRC)) if f.startswith('shortnorth-') and f.endswith('.osm')]
nodes = {n.get('id'): (float(n.get('lon')), float(n.get('lat'))) for r in bands for n in r.iter('node')}
tags = lambda el: {t.get('k'): t.get('v') for t in el.findall('tag')}
# a way crossing two bands comes back in both: keep it once
seen = set()
ways = []
for r in bands:
    for w in r.iter('way'):
        if w.get('id') not in seen:
            seen.add(w.get('id')); ways.append(w)
tree_nodes = {n.get('id'): n for r in bands for n in r.iter('node') if tags(n).get('natural') == 'tree'}
b = box(*BOXES['shortnorth'])
GREEN = {('leisure', 'park'), ('leisure', 'pitch'), ('leisure', 'playground'), ('landuse', 'grass'), ('landuse', 'forest'),
         ('natural', 'wood'), ('landuse', 'meadow'), ('landuse', 'cemetery'), ('natural', 'scrub'), ('leisure', 'garden')}
layers = {k: [] for k in ('roads', 'buildings', 'water', 'green', 'rail', 'trees')}
for w in ways:
    pts = [nodes[nd.get('ref')] for nd in w.findall('nd') if nd.get('ref') in nodes]
    t = tags(w)
    if len(pts) < 2:
        continue
    closed = len(pts) > 3 and pts[0] == pts[-1]
    if 'building' in t and closed:
        layers['buildings'].append((Polygon(pts).buffer(0).intersection(b), {'kind': t.get('building'), 'name': t.get('name', '')}))
    elif 'highway' in t:
        layers['roads'].append((LineString(pts).intersection(b), {'class': t['highway'], 'name': t.get('name', '')}))
    elif t.get('natural') == 'water' or t.get('landuse') == 'reservoir':
        if closed:
            layers['water'].append((Polygon(pts).buffer(0).intersection(b), {'name': t.get('name', '')}))
    elif 'waterway' in t:
        layers['water'].append((LineString(pts).intersection(b), {'name': t.get('name', ''), 'waterway': t['waterway']}))
    elif 'railway' in t:
        layers['rail'].append((LineString(pts).intersection(b), {'kind': t['railway']}))
    elif closed and any((k, t.get(k)) in GREEN for k in ('leisure', 'landuse', 'natural')):
        layers['green'].append((Polygon(pts).buffer(0).intersection(b), {'kind': t.get('leisure') or t.get('landuse') or t.get('natural'), 'name': t.get('name', '')}))
for n in tree_nodes.values():
    p = Point(float(n.get('lon')), float(n.get('lat')))
    if p.within(b):
        layers['trees'].append((p, {}))
for k, feats in layers.items():
    write(f'shortnorth-{k}', feats, 6)
