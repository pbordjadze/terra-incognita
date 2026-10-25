"""Build the map textures the renderer samples, from public-domain sources.

    python3 tools/build_textures.py <cache-dir>

Everything is equirectangular (longitude across, latitude down), so the renderer can draw any
projection by inverting it per character cell. Writes Web/data/:

  geo.png   4096 x 2048  R  signed distance to the coast, fine: 128 + 8 d, for |d| <= 16 px
                         G  the same, coarse: 128 + d / 2, for |d| <= 254 px (land positive;
                            the great lakes are water)
                         B  distance to a land border between countries: 8 d, up to 32 px
                         A  distance to a river: 16 d, up to 16 px
  geo2.png  2048 x 1024  R  signed distance to a named mountain range: 128 + 4 d (px at 2048)
                         G  the country's wash (1-4, so no neighbours share one, times 60)
                         B  signed distance to the island of Ireland's coast: 128 + 2 d
                         A  signed distance to a named desert: 128 + 4 d
  aux.png   2048 x 1024  R forest, G desert, B ice (from NASA's Blue Marble), A elevation (GEBCO)
  aux2.png  2048 x 1024  R unused, G unused, B night lights (NASA Black Marble),
                         A sea depth (GEBCO; 255 is the shore, darker is deeper)
  places.json            label points: seas, countries, capitals

Distances are in pixels of the texture they live in, linear in each channel; cell.glsl decodes
them with the same constants (its `dec*` functions).
"""
import json, sys, urllib.request
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

Image.MAX_IMAGE_PIXELS = None
ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / 'Web' / 'data'
NE = 'https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/'
NASA = 'https://eoimages.gsfc.nasa.gov/images/imagerecords/'
SOURCES = {
    'land.geojson': NE + 'ne_50m_land.geojson',
    'lakes.geojson': NE + 'ne_50m_lakes.geojson',
    'borders.geojson': NE + 'ne_50m_admin_0_boundary_lines_land.geojson',
    'rivers.geojson': NE + 'ne_50m_rivers_lake_centerlines.geojson',
    'regions.geojson': NE + 'ne_50m_geography_regions_polys.geojson',
    'marine.geojson': NE + 'ne_50m_geography_marine_polys.geojson',
    'admin0.geojson': NE + 'ne_50m_admin_0_countries.geojson',
    'places.geojson': NE + 'ne_110m_populated_places_simple.geojson',
    'bluemarble.jpg': NASA + '57000/57752/land_shallow_topo_2048.jpg',
    'elevation.png': NASA + '73000/73934/gebco_08_rev_elev_21600x10800.png',
    'bathymetry.png': NASA + '73000/73963/gebco_08_rev_bath_21600x10800.png',
    'lights.jpg': NASA + '79000/79765/dnb_land_ocean_ice.2012.3600x1800.jpg',
}

HI = (8192, 4096)      # rasterised here, then halved: distances are exact to a quarter pixel
PAD = 256              # columns wrapped around the antimeridian before measuring distance


def fetch(cache):
    cache.mkdir(parents=True, exist_ok=True)
    for name, url in SOURCES.items():
        p = cache / name
        if not p.exists():
            print('fetching', url, flush=True)
            urllib.request.urlretrieve(url, p)
    return {k: cache / k for k in SOURCES}


def xy(lon, lat, size):
    w, h = size
    return ((lon + 180) / 360 * w, (90 - lat) / 180 * h)


def rings(geom):
    t, c = geom['type'], geom['coordinates']
    polys = [c] if t == 'Polygon' else c if t == 'MultiPolygon' else []
    for poly in polys:
        yield poly


def lines(geom):
    t, c = geom['type'], geom['coordinates']
    if t == 'LineString':
        yield c
    elif t == 'MultiLineString':
        yield from c


def fill(draw, features, size, value=255, keep=lambda p: True):
    for f in features:
        if not keep(f['properties']):
            continue
        for poly in rings(f['geometry']):
            draw.polygon([xy(*pt, size) for pt in poly[0]], fill=value)
            for hole in poly[1:]:
                draw.polygon([xy(*pt, size) for pt in hole], fill=0)


def stroke(draw, features, size, width, keep=lambda p: True):
    for f in features:
        if not keep(f['properties']):
            continue
        for ln in lines(f['geometry']):
            pts = [xy(*pt, size) for pt in ln]
            # split where a line crosses the antimeridian
            seg = [pts[0]]
            for a, b in zip(pts, pts[1:]):
                if abs(a[0] - b[0]) > size[0] / 2:
                    draw.line(seg, fill=255, width=width); seg = []
                seg.append(b)
            if len(seg) > 1:
                draw.line(seg, fill=255, width=width)


def raster(size, fn):
    im = Image.new('L', size, 0)
    fn(ImageDraw.Draw(im))
    return np.asarray(im) > 127


def wrapped_edt(mask):
    """Distance (px) from every pixel to the nearest True pixel, wrapping in longitude."""
    padded = np.concatenate([mask[:, -PAD:], mask, mask[:, :PAD]], axis=1)
    d = ndimage.distance_transform_edt(~padded).astype(np.float32)
    return d[:, PAD:-PAD]


def signed(mask):
    """Signed distance: positive inside `mask`, measured to the boundary between the two."""
    return wrapped_edt(~mask) - wrapped_edt(mask)


def halve(a):
    return a.reshape(a.shape[0] // 2, 2, a.shape[1] // 2, 2).mean(axis=(1, 3))


def enc(d, offset, k):
    """Linear: offset + k * d, clamped to a byte. Mirrored by cell.glsl's dec* functions."""
    return np.clip(np.round(offset + k * d), 0, 255).astype(np.uint8)


def resample(path, size, mode='L'):
    im = Image.open(path)
    if im.mode == 'P':
        im = im.convert('RGB')
    return np.asarray(im.convert(mode).resize(size, Image.BOX), dtype=np.float32) / 255


def four_colour(features):
    """A wash per country (0-3) so that no two touching countries share one: greedy, most
    neighbours first, on the countries' outlines at 1:50m."""
    from shapely.geometry import shape
    from shapely.strtree import STRtree
    geoms = [shape(f['geometry']).buffer(0) for f in features]
    tree = STRtree(geoms)
    neigh = [set() for _ in geoms]
    for i, g in enumerate(geoms):
        for j in tree.query(g.buffer(0.05)):
            j = int(j)
            if j != i and g.buffer(0.05).intersects(geoms[j]):
                neigh[i].add(j); neigh[j].add(i)
    colour = [-1] * len(geoms)
    for i in sorted(range(len(geoms)), key=lambda k: -len(neigh[k])):
        used = {colour[j] for j in neigh[i]}
        colour[i] = next(c for c in range(4) if c not in used) if len(used) < 4 else 0
    return colour


def main():
    src = fetch(Path(sys.argv[1]))
    OUT.mkdir(parents=True, exist_ok=True)
    load = lambda k: json.loads(src[k].read_text())['features']
    land, lakes, borders, rivers, regions = (load(k) for k in ('land.geojson', 'lakes.geojson', 'borders.geojson', 'rivers.geojson', 'regions.geojson'))

    print('coast', flush=True)
    land_m = raster(HI, lambda d: (fill(d, land, HI), fill(d, lakes, HI, 0, lambda p: (p.get('scalerank') or 9) <= 4)))
    coast = halve(signed(land_m)) / 2            # px at 4096
    print('borders, rivers', flush=True)
    border_m = raster(HI, lambda d: stroke(d, borders, HI, 1))
    border = halve(wrapped_edt(border_m)) / 2
    river_m = raster(HI, lambda d: stroke(d, rivers, HI, 1, lambda p: p.get('featurecla') == 'River' and (p.get('scalerank') or 9) <= 6))
    river = halve(wrapped_edt(river_m)) / 2
    print('mountains', flush=True)
    mount_m = raster(HI, lambda d: fill(d, regions, HI, keep=lambda p: p['FEATURECLA'] == 'Range/mtn' and (p.get('SCALERANK') or 9) <= 4))
    mount = halve(signed(mount_m)) / 2
    geo = np.stack([enc(coast, 128, 8), enc(coast, 128, 0.5), enc(border, 0, 8), enc(river, 0, 16)], -1)
    Image.fromarray(geo, 'RGBA').save(OUT / 'geo.png', optimize=True)
    print('Ireland', flush=True)
    from shapely.geometry import shape, Point
    eire = [f for f in land if shape(f['geometry']).buffer(0).contains(Point(-8.0, 53.4))]
    eire_m = raster(HI, lambda d: fill(d, eire, HI))
    eire_d = halve(halve(signed(eire_m))) / 4             # px at 2048
    mount_lo = halve(mount) / 2
    del land_m, border_m, river_m, mount_m, eire_m

    LO = (2048, 1024)
    print('biome', flush=True)
    im = np.asarray(Image.open(src['bluemarble.jpg']).convert('RGB').resize(LO, Image.BICUBIC), dtype=np.float32) / 255
    r, g, b = im[..., 0], im[..., 1], im[..., 2]
    hi, lo = im.max(-1), im.min(-1)
    bright, sat = im.mean(-1), hi - lo
    cl = lambda a: np.clip(a, 0, 1)
    ice = cl((bright - 0.5) / 0.12) * cl((0.12 - sat) / 0.06)
    forest = cl((0.36 - bright) / 0.14) * cl((g - r) / 0.06)
    grass = cl((g - r + 0.03) / 0.03) * cl((g - b - 0.1) / 0.06) * (1 - forest)
    veg = cl(forest + 0.5 * grass) * (1 - ice)
    arid = cl((r - g) / 0.08) * cl((bright - 0.25) / 0.12) * (1 - ice)
    lat = np.linspace(90, -90, LO[1])[:, None] * np.ones((1, LO[0]))
    ice = np.maximum(ice, (lat < -62).astype(np.float32))
    elev = resample(src['elevation.png'], LO)
    aux = np.stack([veg, arid, ice, elev], -1)
    Image.fromarray((cl(aux) * 255).astype(np.uint8), 'RGBA').save(OUT / 'aux.png', optimize=True)

    print('washes, deserts, lights, depth', flush=True)
    admin = load('admin0.geojson')
    colour = four_colour(admin)
    wash_im = Image.new('L', LO, 0)
    dr = ImageDraw.Draw(wash_im)
    for f, c in zip(admin, colour):
        for poly in rings(f['geometry']):
            dr.polygon([xy(*pt, LO) for pt in poly[0]], fill=(c + 1) * 60)
    wash = np.asarray(wash_im)
    desert_m = raster(HI, lambda d: fill(d, regions, HI, keep=lambda p: p['FEATURECLA'] == 'Desert' and (p.get('SCALERANK') or 9) <= 3))
    desert = halve(halve(signed(desert_m))) / 4  # px at 2048
    lights = resample(src['lights.jpg'], LO)
    depth = resample(src['bathymetry.png'], LO)
    geo2 = np.stack([enc(mount_lo, 128, 4), wash.astype(np.uint8), enc(eire_d, 128, 2), enc(desert, 128, 4)], -1)
    Image.fromarray(geo2, 'RGBA').save(OUT / 'geo2.png', optimize=True)
    zero = np.zeros(LO[::-1], np.uint8)
    aux2 = np.stack([zero, zero, (cl(lights) * 255).astype(np.uint8), (cl(depth) * 255).astype(np.uint8)], -1)
    Image.fromarray(aux2, 'RGBA').save(OUT / 'aux2.png', optimize=True)

    print('places', flush=True)
    seas = []
    from shapely.geometry import shape
    for f in load('marine.geojson'):
        p = f['properties']
        if p['scalerank'] > 3 or p['featurecla'] == 'ocean':
            continue
        pt = shape(f['geometry']).representative_point()
        seas.append({'n': p['name'], 'r': p['scalerank'], 'c': [round(pt.x, 2), round(pt.y, 2)]})
    countries = []
    for f in admin:
        p = f['properties']
        countries.append({'n': p['NAME'], 'rank': p['LABELRANK'], 'min': p['MIN_LABEL'], 'c': [round(p['LABEL_X'], 2), round(p['LABEL_Y'], 2)]})
    capitals = []
    for f in load('places.geojson'):
        p = f['properties']
        if p['featurecla'] in ('Admin-0 capital', 'Admin-0 capital alt'):
            capitals.append({'n': p['name'], 'c': [round(p['longitude'], 2), round(p['latitude'], 2)], 'pop': p['pop_max']})
    (OUT / 'places.json').write_text(json.dumps({'seas': seas, 'countries': countries, 'capitals': capitals}, separators=(',', ':')))
    for p in sorted(OUT.iterdir()):
        print(p.name, p.stat().st_size)


if __name__ == '__main__':
    main()
