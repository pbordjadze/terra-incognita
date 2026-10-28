"""Builds the ground style's data (Globe/ground/, Globe/regions.json) from public sources.

    python3 tools/globe/ground.py <cache-dir>

Everything is on the globe's own equirectangular grid (longitude across from -180, latitude down from
90), as tools/globe/build.py's maps are; the app uploads the decoded bytes as they are.

  ground/MM.jpg    4096 x 2048 RGB, month MM's land as the ground style draws it: NASA's Blue Marble
                   Next Generation (the surface's colour, cloud-free, that month of 2004) in the atlas's
                   light key: in OKLab its lightness lifted and compressed (0.66 + 0.33 L: forest 0.25
                   becomes 0.74, sand 0.70 becomes 0.89, snow stays white), its hue kept, its chroma
                   x0.95 for the greens down to x0.52 for the warm earths. Lakes and reservoirs
                   (Natural Earth's 1:10m, and water the picture plainly shows: very dark, nearly grey)
                   in the shelves' turquoise. Over the sea, the nearest land's colour for 6 pixels, so
                   the shader's coast blends land with land, then the open sea's. June has no plain image at this size: May and July's
                   mean.
  ground/ice-MM.png  1440 x 720 RGB grey: month MM's usual sea-ice concentration (NSIDC Sea Ice
                   Index v4, the month's mean over 2015-2024; 255 all ice; the pole it never sees, ice).
  ground/floor.png 1440 x 720 RGB grey: the sea floor's relief, from the globe's own depths
                   (terrain.png's G), smoothed and shaded from the north-west as the land is; 128 flat.
  regions.json     the land's own names: Natural Earth's physical regions (ranges, deserts,
                   plateaus, basins, plains, steppes...) of rank 3 or less, without the ones it files
                   wrongly, each at its pole of inaccessibility: n name, k class, r rank, c [lon, lat].

Sources (fetched into the cache if missing): NASA Blue Marble NG, plain, 5400 x 2700 (credit NASA
Earth Observatory); NSIDC G02135 v4 monthly concentration GeoTIFFs (polar stereographic, 25 km);
Natural Earth 1:10m lakes and 1:50m geography regions (public domain). Needs numpy, scipy, Pillow,
shapely, pyproj and tifffile.
"""
import json, sys, urllib.request, warnings
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

Image.MAX_IMAGE_PIXELS = None
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Globe'
SIZE = (4096, 2048)
SEA = (1440, 720)
NASA = 'https://eoimages.gsfc.nasa.gov/images/imagerecords/'
BLUE_MARBLE = {1: 73938, 2: 73967, 3: 73992, 4: 74017, 5: 74042, 7: 74092, 8: 74117, 9: 74142, 10: 74167,
               11: 74192, 12: 74218}                      # Visible Earth's records; no plain June at 5400
NE = 'https://raw.githubusercontent.com/nvkelso/natural-earth-vector/master/geojson/'
NSIDC = 'https://noaadata.apps.nsidc.org/NOAA/G02135/'
MONTHS = ('Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec')
YEARS = range(2015, 2025)
# the sea: the shelves (under 60 m) pale turquoise, the open ocean, the abyss (past 6000 m)
SHELF, OPEN, DEEP = (0xD3, 0xF0, 0xEE), (0xC2, 0xE4, 0xF4), (0xA9, 0xD2, 0xEC)
# classes Natural Earth files that the land's names don't want, and the ones it files wrongly
CLASSES = {'Desert', 'Range/mtn', 'Plateau', 'Basin', 'Plain', 'Tundra', 'Lowland', 'Valley'}
WRONG = {'PUNJAB', 'CAATINGAS', 'SELVAS', 'PENÍNSULA IBÉRICA', 'CANADIAN SHIELD'}


def fetch(url, path):
    if not path.exists():
        print('fetching', url, flush=True)
        path.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(url, path)
    return path


def blue_marble(cache, month):
    if month == 6:
        return (blue_marble(cache, 5).astype(np.uint16) + blue_marble(cache, 7)) // 2
    rec = BLUE_MARBLE[month]
    p = fetch(f'{NASA}{rec // 1000 * 1000}/{rec}/world.2004{month:02d}.3x5400x2700.jpg', cache / f'bm{month:02d}.jpg')
    return np.asarray(Image.open(p).convert('RGB').resize(SIZE, Image.LANCZOS))


def linear(c):
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def srgb(c):
    c = np.clip(c, 0, 1)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


def oklab(rgb):
    l = linear(rgb)
    r, g, b = l[..., 0], l[..., 1], l[..., 2]
    L = np.cbrt(0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b)
    M = np.cbrt(0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b)
    S = np.cbrt(0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b)
    return np.stack([0.2104542553 * L + 0.7936177850 * M - 0.0040720468 * S,
                     1.9779984951 * L - 2.4285922050 * M + 0.4505937099 * S,
                     0.0259040371 * L + 0.7827717662 * M - 0.8086757660 * S], -1)


def from_oklab(lab):
    L, a, b = lab[..., 0], lab[..., 1], lab[..., 2]
    l = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3
    m = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3
    s = (L - 0.0894841775 * a - 1.2914855480 * b) ** 3
    return srgb(np.stack([4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s,
                          -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s,
                          -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s], -1))


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def quiet(sat):
    """The satellite's colour in the atlas's light key (the docstring's numbers)."""
    lab = oklab(sat)
    hue = np.degrees(np.arctan2(lab[..., 2], lab[..., 1]))
    k = 0.95 * (0.52 + 0.48 * smoothstep(80, 115, hue))
    return from_oklab(np.stack([0.66 + 0.33 * lab[..., 0], lab[..., 1] * k, lab[..., 2] * k], -1)), lab


def lakes(cache):
    """Natural Earth's 1:10m lakes and reservoirs, rasterised at twice the grid and averaged: 0..1."""
    p = fetch(NE + 'ne_10m_lakes.geojson', cache / 'ne_10m_lakes.geojson')
    w, h = SIZE[0] * 2, SIZE[1] * 2
    im = Image.new('L', (w, h), 0)
    d = ImageDraw.Draw(im)
    xy = lambda lo, la: ((lo + 180) / 360 * w, (90 - la) / 180 * h)
    for f in json.loads(p.read_text())['features']:
        g = f['geometry']
        for poly in ([g['coordinates']] if g['type'] == 'Polygon' else g['coordinates']):
            d.polygon([xy(*pt) for pt in poly[0]], fill=255)
            for hole in poly[1:]:
                d.polygon([xy(*pt) for pt in hole], fill=0)
    return np.asarray(im.resize(SIZE, Image.BOX)).astype(np.float32) / 255


def land_mask():
    """Land, by the globe's own coast (geo.png: the fine distance near it, the coarse further off)."""
    g = np.asarray(Image.open(OUT / 'geo.png'))[..., :2].astype(np.float32)
    fine, coarse = (g[..., 0] - 128) / 8, (g[..., 1] - 128) * 2
    return np.where(np.abs(coarse) > 14, coarse, fine) > 0


def ground(cache):
    land = land_mask()
    # every pixel's nearest land pixel, wrapping in longitude: the sea takes its colour
    pad = 256
    padded = np.concatenate([land[:, -pad:], land, land[:, :pad]], 1)
    dist, (iy, ix) = ndimage.distance_transform_edt(~padded, return_indices=True)
    dist, iy, ix = dist[:, pad:-pad], iy[:, pad:-pad], (ix[:, pad:-pad] - pad) % SIZE[0]
    far = (dist > 6)[..., None]                           # (the shader never reads it there)
    water = lakes(cache)
    (OUT / 'ground').mkdir(parents=True, exist_ok=True)
    for m in range(1, 13):
        sat = blue_marble(cache, m).astype(np.float32) / 255
        col, lab = quiet(sat)
        seen = smoothstep(0.22, 0.16, lab[..., 0]) * smoothstep(0.045, 0.025, np.hypot(lab[..., 1], lab[..., 2]))
        wet = np.maximum(water, seen)[..., None]
        col = col * (1 - wet) + np.array(SHELF, np.float32) / 255 * 0.97 * wet
        col = np.where(far, np.array(OPEN, np.float32) / 255, col[iy, ix])
        Image.fromarray((np.clip(col, 0, 1) * 255 + 0.5).astype(np.uint8), 'RGB').save(
            OUT / 'ground' / f'{m:02d}.jpg', quality=86, optimize=True, progressive=False)
        print(f'ground {m:02d}', (OUT / 'ground' / f'{m:02d}.jpg').stat().st_size, flush=True)


def floor():
    """The sea floor's relief, shaded from the north-west as build.py shades the land: 128 flat."""
    ter = np.asarray(Image.open(OUT / 'terrain.png'))[..., 1].astype(np.float32)
    depth = np.where(ter < 128, (127 - ter) * 87.0, 0.0)
    z = ndimage.gaussian_filter(-depth, 3.0) * 6
    h, w = z.shape
    lat = np.radians(np.linspace(90, -90, h))[:, None]
    px = 40075016.0 / w
    dzdx = (np.roll(z, -1, 1) - np.roll(z, 1, 1)) / (2 * px * np.maximum(np.cos(lat), 0.05))
    dzdy = (np.vstack([z[1:], z[-1:]]) - np.vstack([z[:1], z[:-1]])) / (2 * px)
    nx, ny, nz = -dzdx, dzdy, np.ones_like(z)
    n = np.sqrt(nx * nx + ny * ny + nz * nz)
    az, alt = np.radians(315), np.radians(45)
    lx, ly, lz = np.cos(alt) * np.sin(az), np.cos(alt) * np.cos(az), np.sin(alt)
    shade = np.clip(((nx * lx + ny * ly + nz * lz) / n - np.sin(alt)) / np.sin(alt), -1, 1)
    small = np.asarray(Image.fromarray(shade.astype(np.float32), 'F').resize(SEA, Image.BOX))
    return np.clip(np.round(128 + 127 * small), 0, 255).astype(np.uint8)


def ice(cache, month):
    """The month's mean sea-ice concentration over YEARS on the SEA grid (0..1)."""
    import tifffile
    from pyproj import Transformer
    lon = (np.arange(SEA[0]) + 0.5) / SEA[0] * 360 - 180
    lat = 90 - (np.arange(SEA[1]) + 0.5) / SEA[1] * 180
    LON, LAT = np.meshgrid(lon, lat)
    out = np.zeros((SEA[1], SEA[0]), np.float32)
    for hemi, H, epsg, x0, y0, sel in (('north', 'N', 3411, -3850000, 5850000, LAT > 30),
                                       ('south', 'S', 3412, -3950000, 4350000, LAT < -30)):
        stack = []
        for y in YEARS:
            name = f'{H}_{y}{month:02d}_concentration_v4.0.tif'
            p = fetch(f'{NSIDC}{hemi}/monthly/geotiff/{month:02d}_{MONTHS[month - 1]}/{name}', cache / 'ice' / name)
            a = tifffile.imread(p).astype(np.float32)
            c = np.where(a <= 1000, a / 1000, np.nan)          # 2510 the pole's hole: never seen, ice
            stack.append(np.where(a == 2510, 1.0, c))
        with warnings.catch_warnings():                    # (land and the far sea: no value in any year)
            warnings.simplefilter('ignore', RuntimeWarning)
            c = np.nan_to_num(np.nanmean(np.stack(stack), 0))
        x, yy = Transformer.from_crs('EPSG:4326', f'EPSG:{epsg}', always_xy=True).transform(LON[sel], LAT[sel])
        i, j = ((x - x0) / 25000).astype(int), ((y0 - yy) / 25000).astype(int)
        ok = (i >= 0) & (i < c.shape[1]) & (j >= 0) & (j < c.shape[0])
        v = np.zeros(len(x), np.float32)
        v[ok] = c[j[ok], i[ok]]
        out[sel] = v
    return out


def sea(cache):
    (OUT / 'ground').mkdir(parents=True, exist_ok=True)
    f = floor()
    Image.fromarray(np.stack([f, f, f], -1), 'RGB').save(OUT / 'ground' / 'floor.png', optimize=True)
    for m in range(1, 13):
        r = np.clip(np.round(ice(cache, m) * 255), 0, 255).astype(np.uint8)
        Image.fromarray(np.stack([r, r, r], -1), 'RGB').save(OUT / 'ground' / f'ice-{m:02d}.png', optimize=True)
        print(f'ice {m:02d}', flush=True)


def regions(cache):
    from shapely.geometry import shape
    from shapely.ops import polylabel
    p = fetch(NE + 'ne_50m_geography_regions_polys.geojson', cache / 'ne_50m_geography_regions_polys.geojson')
    out = []
    for f in json.loads(p.read_text())['features']:
        q = f['properties']
        if q['FEATURECLA'] not in CLASSES or q['SCALERANK'] > 3 or q['NAME'] in WRONG:
            continue
        g = shape(f['geometry']).buffer(0)
        if g.geom_type == 'MultiPolygon':
            g = max(g.geoms, key=lambda x: x.area)
        pt = polylabel(g, 0.05)
        out.append({'n': q['NAME_EN'] or q['NAME'].title(), 'k': q['FEATURECLA'], 'r': q['SCALERANK'],
                    'c': [round(pt.x, 2), round(pt.y, 2)], 'a': round(g.area, 1)})
    out.sort(key=lambda r: (r['r'], -r['a']))
    for r in out:
        del r['a']
    (OUT / 'regions.json').write_text(json.dumps({'regions': out}, separators=(',', ':'), ensure_ascii=False, allow_nan=False) + '\n')
    print('regions', len(out))


def main():
    cache = Path(sys.argv[1])
    regions(cache)
    sea(cache)
    ground(cache)


if __name__ == '__main__':
    main()
