#!/usr/bin/env python3
"""Small derived data for the Aisling's Forest plates, written to data/ beside this file.

    python3 tools/plates/kells-forest/prep.py <cache-dir>

Downloads into <cache-dir> (kept out of the repository), all public domain:
  NASA Visible Earth, Blue Marble land_shallow_topo_2048.jpg
    https://eoimages.gsfc.nasa.gov/images/imagerecords/57000/57752/land_shallow_topo_2048.jpg
  Natural Earth 50 m geography regions and rivers
    https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_50m_geography_regions_polys.geojson
    https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/ne_50m_rivers_lake_centerlines.geojson
and writes
  biome.png             where it is wooded (R), dry (G) and frozen (B), 1024 x 512, read from the Blue Marble
  world-extra.json      mountain ranges (polygons) and the great rivers (lines), for the world plate
  na-borders.geojson    the land borders between the USA, Canada and Mexico, as lines (from ../data/america-states)
  columbus-ring.geojson I-270 as one closed centre line (from ../data/columbus-roads)
"""
import json
import os
import sys
import urllib.request

import numpy as np
from PIL import Image
from shapely.geometry import LineString, Polygon, mapping, shape
from shapely.geometry.polygon import orient
from shapely.ops import unary_union

HERE = os.path.dirname(os.path.abspath(__file__))
OUT, DATA = os.path.join(HERE, 'data'), os.path.join(HERE, '..', 'data')
NE = 'https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/'
SOURCES = {
    'bluemarble.jpg': 'https://eoimages.gsfc.nasa.gov/images/imagerecords/57000/57752/land_shallow_topo_2048.jpg',
    'regions.geojson': NE + 'ne_50m_geography_regions_polys.geojson',
    'rivers.geojson': NE + 'ne_50m_rivers_lake_centerlines.geojson',
}


def rounded(g, nd):
    def r(c):
        return [round(c[0], nd), round(c[1], nd)] if isinstance(c[0], (int, float)) else [r(x) for x in c]
    m = mapping(g)
    return {'type': m['type'], 'coordinates': r(m['coordinates'])}


def save(name, obj):
    with open(os.path.join(OUT, name), 'w') as f:
        json.dump(obj, f, separators=(',', ':'))


def main():
    cache = sys.argv[1]
    os.makedirs(cache, exist_ok=True)
    os.makedirs(OUT, exist_ok=True)
    src = {}
    for name, url in SOURCES.items():
        src[name] = os.path.join(cache, name)
        if not os.path.exists(src[name]):
            urllib.request.urlretrieve(url, src[name])

    # where things grow, are dry or frozen
    im = np.asarray(Image.open(src['bluemarble.jpg']).convert('RGB'), dtype=np.float32) / 255
    r, g, b = im[..., 0], im[..., 1], im[..., 2]
    bright, sat = im.mean(-1), im.max(-1) - im.min(-1)
    clamp = lambda a: np.clip(a, 0, 1)
    ice = clamp((bright - 0.5) / 0.12) * clamp((0.12 - sat) / 0.06)
    forest = clamp((0.36 - bright) / 0.14) * clamp((g - r) / 0.06)
    grass = clamp((g - r + 0.03) / 0.03) * clamp((g - b - 0.1) / 0.06) * (1 - forest)
    veg = clamp(forest + 0.5 * grass) * (1 - ice)
    arid = clamp((r - g) / 0.08) * clamp((bright - 0.25) / 0.12) * (1 - ice)
    rgb = (np.stack([veg, arid, ice], -1) * 255).astype(np.uint8)
    Image.fromarray(rgb).resize((1024, 512), Image.BOX).save(os.path.join(OUT, 'biome.png'), optimize=True)

    ranges = []
    for f in json.load(open(src['regions.geojson']))['features']:
        p = f['properties']
        if p['FEATURECLA'] == 'Range/mtn' and (p.get('SCALERANK') or 9) <= 4:
            geom = shape(f['geometry']).simplify(0.04, preserve_topology=True)
            geom = orient(geom, -1.0) if geom.geom_type == 'Polygon' else geom
            if geom.geom_type == 'MultiPolygon':
                from shapely.geometry import MultiPolygon
                geom = MultiPolygon([orient(q, -1.0) for q in geom.geoms])
            ranges.append({'name': p['NAME'].title(), 'rank': p['SCALERANK'], 'geometry': rounded(geom, 2)})
    rivers = []
    for f in json.load(open(src['rivers.geojson']))['features']:
        p = f['properties']
        if p.get('featurecla') == 'River' and (p.get('scalerank') or 9) <= 3:
            rivers.append({'rank': p['scalerank'], 'geometry': rounded(shape(f['geometry']).simplify(0.03), 2)})
    save('world-extra.json', {'ranges': ranges, 'rivers': rivers})

    states = json.load(open(os.path.join(DATA, 'america-states.geojson')))['features']
    countries = {c: unary_union([shape(f['geometry']).buffer(0) for f in states if f['properties'].get('country') == c])
                 for c in ('USA', 'CAN', 'MEX')}
    borders = []
    for a, bb in (('USA', 'CAN'), ('USA', 'MEX')):
        line = countries[a].boundary.intersection(countries[bb].buffer(0.02)).simplify(0.01)
        borders.append({'type': 'Feature', 'properties': {'between': f'{a}-{bb}'}, 'geometry': rounded(line, 3)})
    save('na-borders.geojson', {'type': 'FeatureCollection', 'features': borders})

    ring = [shape(f['geometry']) for f in json.load(open(os.path.join(DATA, 'columbus-roads.geojson')))['features']
            if f['properties'].get('class') == 'interstate' and f['properties'].get('name') in ('Outerbelt', 'I- 270', 'Jack Nicklaus Fwy')]
    band = unary_union(ring).buffer(0.006)            # both carriageways and every name, as one band
    hole = max(band.interiors, key=lambda h: Polygon(h).area)
    centre = LineString(Polygon(hole).buffer(0.006).exterior.coords).simplify(0.0004)
    save('columbus-ring.geojson', {'type': 'FeatureCollection', 'features': [
        {'type': 'Feature', 'properties': {'name': 'I-270'}, 'geometry': rounded(centre, 5)}]})
    for n in sorted(os.listdir(OUT)):
        print(n, os.path.getsize(os.path.join(OUT, n)))


if __name__ == '__main__':
    main()
