"""Builds the globe's data (Globe/) from public-domain sources.

    python3 tools/globe/build.py <cache-dir> [<libertinus.zip>]

Everything is equirectangular, 4096 x 2048 (longitude across from -180, latitude down from 90), so
the shader (App/TerraIncognita/Globe/Globe.metal) can draw the orthographic globe by inverting the
projection per pixel. The app uploads each image's decoded bytes as they are (no colour management,
which would bend the distances): geo and terrain are RGBA with alpha 255 (so premultiplying changes
nothing), ids is grey.

  geo.png      R  signed distance to the coast, fine: 128 + 8 d, for |d| <= 16 px (land positive;
                  the larger lakes are water)
               G  the same, coarse: 128 + d / 2, for |d| <= 254 px
               B  signed distance to a land border between countries: 128 + 16 d, for |d| < 8 px
  terrain.png  R  hillshade, 128 flat, lit from the north-west
               G  height: land 128 + elevation / 50 m, sea 127 - depth / 87 m
               B  signed distance to a river: 128 + 16 d, for |d| < 8 px
  ids.png      grey: the country under each pixel, 1 + its index in globe.json (0: none)
  globe.json   countries (name, long name, continent, subregion, population, capital, label point,
               label rank), seas and oceans (name, rank, label point), capitals (name, point)
  fonts/       Libertinus Serif (the captions and names) and Uncial Antiqua (the seas), with their
               licences

Sources (fetched into the cache if missing): Natural Earth 1:50m land, lakes, countries, boundary
lines, rivers and marine areas, and 1:110m populated places; NASA's GEBCO elevation and
bathymetry. Distances are in pixels at 4096, wrapping around the antimeridian; the shader decodes
them with the same constants. The coast's is measured on land rasterised at 8192 and halved; a line's
(borders, rivers) to the line itself, signed by which side of it a pixel lies, so that like the
coast's it crosses zero on the line and stays sharp however close the camera. (Its sign flips where
the nearest line changes side, far from any line: the shader tells those jumps from lines by their
steepness.)
"""
import json, shutil, sys, urllib.request, zipfile
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

Image.MAX_IMAGE_PIXELS = None
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Globe'
NE = 'https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/'
NASA = 'https://eoimages.gsfc.nasa.gov/images/imagerecords/'
SOURCES = {
    'ne_50m_land.geojson': NE + 'ne_50m_land.geojson',
    'lakes.geojson': NE + 'ne_50m_lakes.geojson',
    'admin0.geojson': NE + 'ne_50m_admin_0_countries.geojson',
    'ne_50m_boundary_lines.geojson': NE + 'ne_50m_admin_0_boundary_lines_land.geojson',
    'rivers.geojson': NE + 'ne_50m_rivers_lake_centerlines.geojson',
    'marine.geojson': NE + 'ne_50m_geography_marine_polys.geojson',
    'places.geojson': NE + 'ne_110m_populated_places_simple.geojson',
    'gebco_elev_21600.png': NASA + '73000/73934/gebco_08_rev_elev_21600x10800.png',
    'gebco_bath_21600.png': NASA + '73000/73963/gebco_08_rev_bath_21600x10800.png',
}
HI = (8192, 4096)
SIZE = (4096, 2048)
PAD = 256


def fetch(cache):
    cache.mkdir(parents=True, exist_ok=True)
    for name, url in SOURCES.items():
        if not (cache / name).exists():
            print('fetching', url, flush=True)
            urllib.request.urlretrieve(url, cache / name)
    return {k: cache / k for k in SOURCES}


def rank(p, default=9):
    """A feature's scalerank (0 is the most important, so not `or`)."""
    r = p.get('scalerank')
    return default if r is None else r


def xy(lon, lat, size):
    return ((lon + 180) / 360 * size[0], (90 - lat) / 180 * size[1])


def polygons(geom):
    t, c = geom['type'], geom['coordinates']
    return [c] if t == 'Polygon' else c if t == 'MultiPolygon' else []


def lines(geom):
    t, c = geom['type'], geom['coordinates']
    return [c] if t == 'LineString' else c if t == 'MultiLineString' else []


def fill(draw, features, size, value=255, keep=lambda p: True):
    for f in features:
        if keep(f['properties']):
            for poly in polygons(f['geometry']):
                draw.polygon([xy(*pt, size) for pt in poly[0]], fill=value)
                for hole in poly[1:]:
                    draw.polygon([xy(*pt, size) for pt in hole], fill=0)


def stroke(draw, features, size, keep=lambda p: True):
    for f in features:
        if not keep(f['properties']):
            continue
        for ln in lines(f['geometry']):
            pts = [xy(*pt, size) for pt in ln]
            seg = [pts[0]]
            for a, b in zip(pts, pts[1:]):
                if abs(a[0] - b[0]) > size[0] / 2:      # crossing the antimeridian
                    if len(seg) > 1:
                        draw.line(seg, fill=255, width=1)
                    seg = []
                seg.append(b)
            if len(seg) > 1:
                draw.line(seg, fill=255, width=1)


def raster(size, fn, mode='L'):
    im = Image.new(mode, size, 0)
    fn(ImageDraw.Draw(im))
    return np.asarray(im)


def wrapped_edt(mask):
    padded = np.concatenate([mask[:, -PAD:], mask, mask[:, :PAD]], axis=1)
    return ndimage.distance_transform_edt(~padded).astype(np.float32)[:, PAD:-PAD]


def signed(mask):
    return wrapped_edt(~mask) - wrapped_edt(mask)


def signed_lines(features, size, keep=lambda p: True, spacing=0.1, reach=10.0):
    """Distance (px) from every pixel's centre to the nearest line, signed by the side of the line's
    direction it lies on, clamped to `reach`; wrapping in longitude."""
    from scipy.spatial import cKDTree
    pts, dirs = [], []
    for f in features:
        if not keep(f['properties']):
            continue
        for ln in lines(f['geometry']):
            p = np.array([xy(*pt, size) for pt in ln], np.float64)
            for a, b in zip(p[:-1], p[1:]):
                d = b - a
                n = np.hypot(*d)
                if n == 0 or abs(d[0]) > size[0] / 2:      # crossing the antimeridian
                    continue
                k = max(int(np.ceil(n / spacing)), 1)
                t = (np.arange(k) + 0.5)[:, None] / k
                pts.append(a + t * d)
                dirs.append(np.repeat((d / n)[None], k, 0))
    pts, dirs = np.concatenate(pts), np.concatenate(dirs)
    w = size[0]
    edge = (pts[:, 0] < reach + 1) | (pts[:, 0] > w - reach - 1)
    pts = np.concatenate([pts, pts[edge] + [w, 0], pts[edge] - [w, 0]])
    dirs = np.concatenate([dirs, dirs[edge], dirs[edge]])
    tree = cKDTree(pts)
    ys, xs = np.mgrid[0:size[1], 0:size[0]]
    q = np.stack([xs.ravel() + 0.5, ys.ravel() + 0.5], -1)
    dist, idx = tree.query(q, distance_upper_bound=reach, workers=-1)
    out = np.full(len(q), reach, np.float32)
    hit = np.isfinite(dist)
    v = q[hit] - pts[idx[hit]]
    t = dirs[idx[hit]]
    side = np.sign(t[:, 0] * v[:, 1] - t[:, 1] * v[:, 0])
    out[hit] = np.where(side == 0, 1, side) * np.minimum(dist[hit], reach)
    return out.reshape(size[1], size[0])


def halve(a):
    return a.reshape(a.shape[0] // 2, 2, a.shape[1] // 2, 2).mean(axis=(1, 3))


def enc(d, offset, k):
    return np.clip(np.round(offset + k * d), 0, 255).astype(np.uint8)


def resample(path, size):
    im = Image.open(path)
    if im.mode == 'P':
        im = im.convert('RGB')
    return np.asarray(im.convert('L').resize(size, Image.BOX), dtype=np.float32)


def hillshade(elev_m):
    """Lambertian shade of the relief, lit from the north-west 45 degrees up, exaggerated so the
    great ranges read at a globe's scale; 128 is flat ground."""
    h, w = elev_m.shape
    lat = np.radians(np.linspace(90, -90, h))[:, None]
    px = 40075016.0 / w                                   # metres per pixel along the equator
    z = ndimage.gaussian_filter(elev_m, 1.2) * 18         # exaggeration
    dzdx = (np.roll(z, -1, 1) - np.roll(z, 1, 1)) / (2 * px * np.maximum(np.cos(lat), 0.05))
    dzdy = (np.vstack([z[1:], z[-1:]]) - np.vstack([z[:1], z[:-1]])) / (2 * px)
    nx, ny, nz = -dzdx, dzdy, np.ones_like(z)
    n = np.sqrt(nx * nx + ny * ny + nz * nz)
    az, alt = np.radians(315), np.radians(45)
    lx, ly, lz = np.cos(alt) * np.sin(az), np.cos(alt) * np.cos(az), np.sin(alt)
    shade = (nx * lx + ny * ly + nz * lz) / n
    return enc((shade - np.sin(alt)) / np.sin(alt), 128, 127)


def main():
    src = fetch(Path(sys.argv[1]))
    OUT.mkdir(parents=True, exist_ok=True)
    load = lambda k: json.loads(src[k].read_text())['features']
    land, lakes, admin = load('ne_50m_land.geojson'), load('lakes.geojson'), load('admin0.geojson')

    print('coast and borders', flush=True)
    land_m = raster(HI, lambda d: (fill(d, land, HI), fill(d, lakes, HI, 0, lambda p: rank(p) <= 4))) > 127
    coast = halve(signed(land_m)) / 2
    border = signed_lines(load('ne_50m_boundary_lines.geojson'), SIZE)
    opaque = np.full(coast.shape, 255, np.uint8)
    geo = np.stack([enc(coast, 128, 8), enc(coast, 128, 0.5), enc(border, 128, 16), opaque], -1)
    Image.fromarray(geo, 'RGBA').save(OUT / 'geo.png', optimize=True)

    print('terrain', flush=True)
    elev_m = resample(src['gebco_elev_21600.png'], SIZE) * 25.0
    depth_m = (255 - resample(src['gebco_bath_21600.png'], SIZE)) * 43.0
    land_lo = halve(land_m.astype(np.float32)) > 0.5
    shade = np.where(land_lo, hillshade(elev_m), 128).astype(np.uint8)
    height = np.where(land_lo, enc(elev_m / 50, 128, 1), enc(-depth_m / 87, 127, 1))
    river = signed_lines(load('rivers.geojson'), SIZE, lambda p: p.get('featurecla') == 'River' and rank(p) <= 6)
    Image.fromarray(np.stack([shade, height, enc(river, 128, 16), opaque], -1), 'RGBA').save(OUT / 'terrain.png', optimize=True)
    del land_m

    print('countries', flush=True)
    order = sorted(range(len(admin)), key=lambda i: admin[i]['properties']['NAME'])
    ids_hi = Image.new('L', HI, 0)
    draw = ImageDraw.Draw(ids_hi)
    for k, i in enumerate(order):
        for poly in polygons(admin[i]['geometry']):
            draw.polygon([xy(*pt, HI) for pt in poly[0]], fill=k + 1)
    for k, i in enumerate(order):        # holes (lakes inside a country, enclaves) drawn after all
        for poly in polygons(admin[i]['geometry']):
            for hole in poly[1:]:
                draw.polygon([xy(*pt, HI) for pt in hole], fill=0)
    # enclaves sit in holes of their surrounding country: draw them again on top
    for k, i in enumerate(order):
        p = admin[i]['properties']
        if p['NAME'] in ('Lesotho', 'Vatican', 'San Marino', 'eSwatini'):
            for poly in polygons(admin[i]['geometry']):
                draw.polygon([xy(*pt, HI) for pt in poly[0]], fill=k + 1)
    ids = np.asarray(ids_hi)[::2, ::2]
    Image.fromarray(ids, 'L').save(OUT / 'ids.png', optimize=True)

    capitals = {}
    capital_list = []
    for f in load('places.geojson'):
        p = f['properties']
        if p['featurecla'] in ('Admin-0 capital', 'Admin-0 capital alt') and p.get('adm0cap'):
            capitals.setdefault(p['adm0_a3'], p['name'])
        if p['featurecla'] in ('Admin-0 capital', 'Admin-0 capital alt'):
            capital_list.append({'n': p['name'], 'c': [round(p['longitude'], 3), round(p['latitude'], 3)],
                                 'r': p['labelrank'], 'pop': p['pop_max']})
    countries = []
    for i in order:
        p = admin[i]['properties']
        countries.append({'n': p['NAME'], 'long': p['NAME_LONG'], 'cont': p['CONTINENT'], 'sub': p['SUBREGION'],
                          'pop': p['POP_EST'], 'cap': capitals.get(p['ADM0_A3']),
                          'c': [round(p['LABEL_X'], 3), round(p['LABEL_Y'], 3)], 'r': p['LABELRANK'],
                          'min': p['MIN_LABEL']})
    from shapely.geometry import shape
    seas = []
    for f in load('marine.geojson'):
        p = f['properties']
        if p['scalerank'] > 3 and p['featurecla'] != 'ocean':
            continue
        g = shape(f['geometry']).buffer(0)
        pt = g.representative_point()
        seas.append({'n': p['name'], 'r': p['scalerank'], 'ocean': p['featurecla'] == 'ocean',
                     'c': [round(pt.x, 2), round(pt.y, 2)], 'a': round(g.area, 1)})
    # (no NaN: Swift's JSONDecoder refuses it, and the globe would lose its names and taps)
    (OUT / 'globe.json').write_text(json.dumps({'countries': countries, 'seas': seas, 'capitals': capital_list},
                                               separators=(',', ':'), ensure_ascii=False, allow_nan=False) + '\n')

    fonts = OUT / 'fonts'
    fonts.mkdir(exist_ok=True)
    if len(sys.argv) > 2:
        z = zipfile.ZipFile(sys.argv[2])
        for name in z.namelist():
            base = name.rsplit('/', 1)[-1]
            if base in ('LibertinusSerif-Regular.otf', 'LibertinusSerif-Italic.otf', 'LibertinusSerif-Semibold.otf'):
                (fonts / base).write_bytes(z.read(name))
            elif base == 'OFL.txt':
                (fonts / 'OFL-Libertinus.txt').write_bytes(z.read(name))
    uncial = ROOT / 'tools' / 'plates' / 'kells-night' / 'fonts'
    shutil.copy(uncial / 'UncialAntiqua-Regular.ttf', fonts)
    shutil.copy(uncial / 'OFL-UncialAntiqua.txt', fonts)
    for p in sorted(OUT.rglob('*')):
        if p.is_file():
            print(p.relative_to(OUT), p.stat().st_size)


if __name__ == '__main__':
    main()
