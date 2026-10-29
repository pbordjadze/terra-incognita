"""Builds the lamplight pack's pictures (Prototypes/lamplight/) from public sources.

    python3 tools/lamplight/build.py <cache-dir> [step ...] [--fresh]
    steps: land lamps glow detail pool sprites (all of them when none is named)

Lamplight is the Himalaya's edge at night: the globe's own Earth at night (GlobeStyle.night, which a
pack asks for with terra.globe.set({ night: true })) and, over the mountains and the plains below
them, the night drawn closer than the globe's map can: the relief and the snow by moonlight, the
lights of the plains as the satellite saw them, and lamps where people live in the hills, which it
couldn't. Every picture is drawn in Globe.metal's N_ palette, so it lies on the globe's night as part
of it, and on its own equirectangular grid (each column the same span of longitude, each row the same
span of latitude), as terra.images draws it; the bounds go to Prototypes/lamplight/bounds.js.

  land.jpg + land-alpha    72-98E, 25-37N, 1/150 degree (about 700 m): the land in N_LAND shaded by
               moonlight from the south-west (Copernicus GLO-90), snow above about 5100 m lit by it,
               lakes in N_SEA, and NASA's Black Marble 2016 lights (500 m) as the night style draws its
               own (Globe.metal: pow(light, 0.75) towards N_LIGHT, the brightest towards N_CORE). Its
               edges fade over 4 degrees into the globe's own night (land.png carries the alpha).
  lamps-1.png  lamps where people live in the hills and mountains (GHSL's population, 2020) below 1500
  lamps-2.png  m, 1500-3000 m, and above 3000 m: 1/120 degree over the same box. Drawn as an atlas draws
  lamps-3.png  population, a dot for every 150 people where they live; these pictures from further off
               hold a quarter of them, and the detail all of them, so coming closer lights more. The
               satellite sees the plains' lights and almost none of the hills': these are drawn. The
               plains are left to it (a
               square needs 350 m of height and 200 m of relief within 3 km, or 2000 m of height);
               GHSL spreads people onto glaciers where its built-up layer misreads bare rock, so
               nothing above 4900 m is lit, nor anything above 3700 m steeper than 18 degrees.
  glow.png     the setting sun's last light on the snow (alpenglow): the slopes above about 5000 m
  crown.png    facing west, by a sun 6 degrees up in the west-south-west; and those above 7000 m,
               which keep it longest. Drawn, from the elevation model: a moment, not a measurement.
  detail.jpg   the central Himalaya (77.5-92.5E, 26.3-31.5N), 1/240 degree, land, lights and lamps
               together, for zooming in.
  pool.png     for an app whose runtime can't turn the globe to night: the same night, 60-109E,
               12-49N, about 2 km, fading to nothing round a ring 12.5-18 degrees from (84.5E, 30.5N)
               through twilight violet and the sunset's apricot.
  lamp-1..6.png, doorway-glint.png: the sprites (a lamp's flicker, the doorway's glint from afar).

Sources (fetched into the cache if missing): NASA Black Marble 2016, the 500 m colour and grey tiles C1
and D1 (Earth Observatory records 144898 and 144897; images by Joshua Stevens, Suomi NPP VIIRS data from
Miguel Román, NASA GSFC); Copernicus DEM GLO-90 (AWS open data; produced using Copernicus WorldDEM-90,
DLR e.V. 2010-2014 and Airbus Defence and Space GmbH 2014-2018, provided under COPERNICUS by the
European Union and ESA); GHSL GHS-POP R2023A, epoch 2020, 3 arcseconds (European Commission, Joint
Research Centre, CC BY 4.0). Needs numpy, scipy, Pillow and rasterio. About 2.5 GB of downloads.
"""
import json, math, sys, urllib.request, zipfile
from pathlib import Path
import numpy as np
from PIL import Image
from scipy import ndimage

Image.MAX_IMAGE_PIXELS = None
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Prototypes' / 'lamplight'
BM = 'https://eoimages.gsfc.nasa.gov/images/imagerecords/144000/'
DEM = 'https://copernicus-dem-90m.s3.amazonaws.com/'
GHSL = ('https://jeodpp.jrc.ec.europa.eu/ftp/jrc-opendata/GHSL/GHS_POP_GLOBE_R2023A/'
        'GHS_POP_E2020_GLOBE_R2023A_4326_3ss/V1-0/tiles/')
BOX = (72.0, 25.0, 98.0, 37.0)            # where the elevation model and the people are read
DETAIL = (77.5, 26.3, 92.5, 31.5)
CROWN = (74.0, 27.0, 89.0, 36.5)          # every peak above 7000 m in the box
POOL = (60.0, 12.0, 109.0, 49.0)
POOL_CENTRE, POOL_IN, POOL_OUT = (84.5, 30.5), 12.5, 18.0
MOON_AZ, MOON_ALT = 225.0, 28.0           # the moonlight on the relief: from the south-west
SUN_AZ, SUN_ALT = 255.0, 6.0              # the setting sun, for the alpenglow
LAMP_BANDS = ((-1000, 1500), (1500, 3000), (3000, 9000))

# Globe.metal's night (N_), so the pictures are the globe's own night drawn closer
N_LAND = np.array([0x1E, 0x24, 0x30], np.float32) / 255
N_SEA = np.array([0x10, 0x18, 0x28], np.float32) / 255
N_LIGHT = np.array([0xFF, 0xD6, 0x86], np.float32) / 255
N_CORE = np.array([0xFF, 0xF4, 0xDC], np.float32) / 255
SNOW_DARK = np.array([0x3A, 0x44, 0x5A], np.float32) / 255     # snow in the moon's shadow
SNOW_LIT = np.array([0x9C, 0xAA, 0xC2], np.float32) / 255      # and facing it
LAMP = np.array([1.0, 0.82, 0.56], np.float32)                 # a lamp's flame, warmer than N_LIGHT
LAMP_HALO = np.array([1.0, 0.52, 0.22], np.float32)
ALPENGLOW = np.array([1.0, 0.62, 0.52], np.float32)
CROWN_GLOW = np.array([1.0, 0.72, 0.58], np.float32)


def fetch(url, path):
    path = Path(path)
    if not path.exists():
        print('fetching', url, flush=True)
        path.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(url, path)
    return path


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def mix(a, b, t):
    return a + (b - a) * (t[..., None] if np.ndim(t) else t)


# MARK: the sources

def black_marble(cache, bounds, ppd, grey=False):
    """Black Marble 2016 (500 m tiles C1, 0-90E, and D1, 90-180E, 0-90N) over bounds at ppd pixels a
    degree: the colour composite (the night's ground and lights), or the grey one (the lights alone)."""
    w, s, e, n = bounds
    record, suffix, mode = ('144897', '_gray', 'L') if grey else ('144898', '', 'RGB')
    src = 240
    full = Image.new(mode, (round((e - w) * src), round((n - s) * src)))
    for lon0, t in ((0, 'C1'), (90, 'D1')):
        a, b = max(w, lon0), min(e, lon0 + 90)
        if a >= b: continue
        name = f'BlackMarble_2016_{t}{suffix}.jpg'
        im = Image.open(fetch(BM + f'{record}/{name}', cache / 'lights' / name))
        piece = im.crop((round((a - lon0) * src), round((90 - n) * src), round((b - lon0) * src), round((90 - s) * src)))
        full.paste(piece, (round((a - w) * src), 0))
    size = (round((e - w) * ppd), round((n - s) * ppd))
    # the lights are points: a box filter keeps their sum where a Lanczos would ring round them
    out = full.resize(size, Image.BOX if size[0] < full.size[0] else Image.BILINEAR)
    return np.asarray(out).astype(np.float32) / 255


def dem_tile(cache, lat, lon):
    import rasterio
    n = f'Copernicus_DSM_COG_30_N{lat:02d}_00_E{lon:03d}_00_DEM'
    with rasterio.open(fetch(DEM + f'{n}/{n}.tif', cache / 'dem' / f'{n}.tif')) as d:
        return d.read(1).astype(np.float32)


def elevation(cache, bounds, ppd):
    """The elevation model's block means over bounds (whole degrees) at ppd pixels a degree (1200/ppd whole)."""
    w, s, e, n = [int(round(v)) for v in bounds]
    f = 1200 // ppd
    z = np.zeros(((n - s) * ppd, (e - w) * ppd), np.float32)
    for lat in range(s, n):
        for lon in range(w, e):
            t = dem_tile(cache, lat, lon).reshape(ppd, f, ppd, f).mean((1, 3))
            z[(n - 1 - lat) * ppd:(n - lat) * ppd, (lon - w) * ppd:(lon - w + 1) * ppd] = t
    return z


def ghsl_tiles(cache):
    import rasterio
    out = []
    for t in ('R6_C26', 'R6_C27', 'R6_C28', 'R7_C26', 'R7_C27', 'R7_C28'):
        name = f'GHS_POP_E2020_GLOBE_R2023A_4326_3ss_V1_0_{t}'
        tif = cache / 'ghsl' / f'{name}.tif'
        if not tif.exists():
            z = fetch(GHSL + name + '.zip', cache / 'ghsl' / f'{name}.zip')
            with zipfile.ZipFile(z) as zf: zf.extract(f'{name}.tif', cache / 'ghsl')
        out.append(rasterio.open(tif))
    return out


def people_tile(tiles, lat, lon):
    """GHSL's people in the one-degree tile, on the elevation model's own 3-arcsecond grid (their
    pixels share centres)."""
    from rasterio.windows import from_bounds
    p = np.zeros((1200, 1200), np.float32)
    w, s, e, n = lon - 1 / 2400, lat + 1 / 2400, lon + 1 - 1 / 2400, lat + 1 + 1 / 2400
    for g in tiles:
        b = g.bounds
        if b.left < e and w < b.right and b.bottom < n and s < b.top:
            win = from_bounds(w, s, e, n, g.transform).round_offsets().round_lengths()
            a = np.nan_to_num(g.read(1, window=win, boundless=True, fill_value=0).astype(np.float32))[:1200, :1200]
            p = np.maximum(p, a)
    p[p < 0] = 0
    return p


# MARK: drawing

def hillshade(z, ppd, lat0, az, alt, exaggerate=1.0):
    """Lambertian shading of the elevation model (rows from lat0 southwards) by a light at az, alt degrees."""
    lat = lat0 - (np.arange(z.shape[0]) + .5) / ppd
    dy = 111320.0 / ppd
    dx = dy * np.cos(np.radians(lat))[:, None]
    gy, gx = np.gradient(z)
    gx = gx / dx * exaggerate
    gy = -gy / dy * exaggerate          # rows run south: north is up
    a, h = np.radians(az), np.radians(alt)
    lx, ly, lz = math.sin(a) * math.cos(h), math.cos(a) * math.cos(h), math.sin(h)
    nz = 1 / np.sqrt(1 + gx * gx + gy * gy)
    return np.clip((-gx * lx - gy * ly + lz) * nz, 0, 1)


def with_lights(rgb, grey):
    """The night style's lights (Globe.metal): pow(light, 0.75) towards N_LIGHT, the brightest towards N_CORE."""
    g = np.where(grey < 6 / 255, 0, grey)
    lights = np.power(g, 0.75)
    rgb = mix(rgb, N_LIGHT, smoothstep(0.0, 0.85, lights))
    return mix(rgb, N_CORE, 0.7 * smoothstep(0.7, 1.0, lights))


def night_land(z, ppd, lat0, colour, grey):
    """The land by night in the N_ palette: shaded by the moon, the snow lit by it, lakes as the sea,
    and the satellite's lights."""
    s = hillshade(z, ppd, lat0, MOON_AZ, MOON_ALT, 1.6)
    flat = math.sin(math.radians(MOON_ALT))
    shade = np.clip(1 + 1.15 * (s - flat), 0.45, 1.8)
    # as bright on average as the globe's own night land (N_LAND), so only the relief shows where the
    # picture fades into it: the shade divided by its mean over some 30 km
    shade = shade / np.maximum(ndimage.gaussian_filter(shade, 30000 / (111320 / ppd)), 0.3)
    rgb = N_LAND * shade[..., None]
    snow = smoothstep(5100, 6100, z) * smoothstep(0.0, 0.25, s + 0.1)
    rgb = mix(rgb, mix(SNOW_DARK, SNOW_LIT, np.clip(s * 1.3, 0, 1)), snow)
    # lakes: dark in the satellite's composite and flat in the elevation model
    lum = colour @ np.array([0.3, 0.5, 0.2], np.float32)
    flatness = ndimage.maximum_filter(z, 3) - ndimage.minimum_filter(z, 3)
    lake = ndimage.binary_opening((lum < 0.035) & (flatness < 0.6) & (z > 200), iterations=1)
    rgb = mix(rgb, N_SEA, ndimage.gaussian_filter(lake.astype(np.float32), 0.7))
    return with_lights(rgb, grey)


def box_weight(shape, bounds, inner, feather):
    """1 inside inner (west, south, east, north), falling to 0 over feather degrees, on bounds' grid."""
    w, s, e, n = bounds
    lon = w + (np.arange(shape[1]) + .5) / shape[1] * (e - w)
    lat = n - (np.arange(shape[0]) + .5) / shape[0] * (n - s)
    fx = np.minimum(smoothstep(inner[0], inner[0] + feather, lon), 1 - smoothstep(inner[2] - feather, inner[2], lon))
    fy = np.minimum(smoothstep(inner[1], inner[1] + feather, lat), 1 - smoothstep(inner[3] - feather, inner[3], lat))
    return np.outer(fy, fx).astype(np.float32)


def save_png(rgb, alpha, path):
    a = np.dstack([rgb, alpha[..., None]]) if alpha is not None else rgb
    Image.fromarray((np.clip(a, 0, 1) * 255 + .5).astype(np.uint8)).save(path, optimize=True)


def save_jpg(rgb, path, quality=88):
    Image.fromarray((np.clip(rgb, 0, 1) * 255 + .5).astype(np.uint8)).save(path, quality=quality, optimize=True, progressive=True)


# MARK: the lamps

PEOPLE_PER_LAMP = 150       # up close (the detail picture); a quarter of them from further off
FAR = 0.25


def lamps(cache):
    """The lamps, as an atlas draws population: a dot for every PEOPLE_PER_LAMP people, each where they
    live (a 3-arcsecond cell drawn with the chance its people give, then a place inside it), seeded so
    the same build draws the same lamps. (lon, lat, strength, elevation, far): far marks the quarter
    shown from further off, so zooming in adds lamps between the ones already lit. See the module's
    notes on what is left out (the plains, glaciers, steep high ground)."""
    tiles = ghsl_tiles(cache)
    rng = np.random.default_rng(1983)
    out = []
    w, s, e, n = [int(v) for v in BOX]
    for lat in range(s, n):
        for lon in range(w, e):
            z = dem_tile(cache, lat, lon)
            p = people_tile(tiles, lat, lon)
            if p.sum() < 1: continue
            dy = 111320.0 / 1200
            dx = dy * math.cos(math.radians(lat + .5))
            gy, gx = np.gradient(z)
            slope = np.degrees(np.arctan(np.hypot(gx / dx, gy / dy)))
            # the hills and mountains, judged by 300 m squares
            zb = z.reshape(400, 3, 400, 3).mean((1, 3))
            relief = ndimage.maximum_filter(zb, 11, mode='nearest') - ndimage.minimum_filter(zb, 11, mode='nearest')
            hills = np.repeat(np.repeat((zb >= 350) & ((relief >= 200) | (zb >= 2000)), 3, 0), 3, 1)
            keep = hills & (z <= 4900) & ~((z > 3700) & (slope > 18))
            count = rng.poisson(np.where(keep, p, 0) / PEOPLE_PER_LAMP)
            r, c = np.nonzero(count)
            r, c = np.repeat(r, count[r, c]), np.repeat(c, count[r, c])
            jx, jy = rng.random(len(r)), rng.random(len(r))
            out.append(np.column_stack([lon - 1 / 2400 + (c + jx) / 1200, lat + 1 + 1 / 2400 - (r + jy) / 1200,
                                        0.55 + 0.45 * rng.random(len(r)), z[r, c], rng.random(len(r)) < FAR]))
    return np.concatenate(out)


def splat(L, bounds, ppd, gain=1.9, core_sigma=0.5, halo_sigma=1.5, halo_w=0.06):
    """Lamps as a picture: each a point of light at its place (shared between the four nearest pixels),
    a flame-coloured core and a faint amber halo; where lamps crowd (a town) they add up to a glow.
    rgb and alpha."""
    w, s, e, n = bounds
    W, H = round((e - w) * ppd), round((n - s) * ppd)
    x = (L[:, 0] - w) * ppd - .5
    y = (n - L[:, 1]) * ppd - .5
    m = (x >= 0) & (x < W - 1) & (y >= 0) & (y < H - 1)
    x, y, b = x[m], y[m], L[m, 2]
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
    fx, fy = x - x0, y - y0
    img = np.zeros((H, W), np.float32)
    for dx_, dy_, wt in ((0, 0, (1 - fx) * (1 - fy)), (1, 0, fx * (1 - fy)), (0, 1, (1 - fx) * fy), (1, 1, fx * fy)):
        np.add.at(img, (y0 + dy_, x0 + dx_), b * wt)
    core = 1 - np.exp(-gain * ndimage.gaussian_filter(img, core_sigma))
    halo = 1 - np.exp(-halo_w * 2 * np.pi * halo_sigma ** 2 * ndimage.gaussian_filter(img, halo_sigma))
    alpha = np.clip(core + halo * (1 - core), 0, 1)
    with np.errstate(invalid='ignore', divide='ignore'):
        share = np.nan_to_num(core / np.maximum(alpha, 1e-6))
    return mix(LAMP_HALO, LAMP, share), alpha


def lamp_list(cache):
    path = cache / 'lamps.npy'
    if path.exists() and '--fresh' not in sys.argv:
        return np.load(path)
    L = lamps(cache)
    np.save(path, L)
    return L


# MARK: the steps

def step_land(cache):
    ppd = 150
    z = elevation(cache, BOX, ppd)
    rgb = night_land(z, ppd, BOX[3], black_marble(cache, BOX, ppd), black_marble(cache, BOX, ppd, grey=True))
    save_png(rgb, box_weight(z.shape, BOX, BOX, 4.0), OUT / 'land.png')
    return {'land': list(BOX)}


def step_lamps(cache):
    L = lamp_list(cache)
    print(len(L), 'lamps,', int(L[:, 4].sum()), 'from further off')
    meta = {}
    for i, (lo, hi) in enumerate(LAMP_BANDS, 1):
        m = (L[:, 3] >= lo) & (L[:, 3] < hi) & (L[:, 4] > 0)
        rgb, alpha = splat(L[m], BOX, 120)
        save_png(rgb, alpha, OUT / f'lamps-{i}.png')
        meta[f'lamps-{i}'] = list(BOX)
    return meta


def step_glow(cache):
    ppd = 120
    z = elevation(cache, BOX, ppd)
    sun = hillshade(z, ppd, BOX[3], SUN_AZ, SUN_ALT, 1.3)
    glow = smoothstep(4800, 6000, z) * smoothstep(0.12, 0.6, sun)
    glow = np.clip(glow + 0.7 * ndimage.gaussian_filter(glow, 1.5), 0, 1)     # and its bloom
    save_png(np.broadcast_to(ALPENGLOW, z.shape + (3,)), glow, OUT / 'glow.png')
    w, s, e, n = CROWN
    zc = z[round((BOX[3] - n) * ppd):round((BOX[3] - s) * ppd), round((w - BOX[0]) * ppd):round((e - BOX[0]) * ppd)]
    sc = sun[round((BOX[3] - n) * ppd):round((BOX[3] - s) * ppd), round((w - BOX[0]) * ppd):round((e - BOX[0]) * ppd)]
    crown = smoothstep(6600, 7600, zc) * smoothstep(0.1, 0.5, sc)
    crown = np.clip(crown + 1.2 * ndimage.gaussian_filter(crown, 2.0), 0, 1)
    save_png(np.broadcast_to(CROWN_GLOW, zc.shape + (3,)), crown, OUT / 'crown.png')
    return {'glow': list(BOX), 'crown': list(CROWN)}


def step_detail(cache):
    ppd = 240
    w, s, e, n = DETAIL
    z = elevation(cache, (77, 26, 93, 32), ppd)[round((32 - n) * ppd):round((32 - s) * ppd), round((w - 77) * ppd):round((e - 77) * ppd)]
    rgb = night_land(z, ppd, n, black_marble(cache, DETAIL, ppd), black_marble(cache, DETAIL, ppd, grey=True))
    lrgb, la = splat(lamp_list(cache), DETAIL, ppd)
    save_jpg(mix(rgb, lrgb, la), OUT / 'detail.jpg', 90)
    return {'detail': list(DETAIL)}


def step_pool(cache):
    w, s, e, n = POOL
    ppd = 2560 / (e - w)
    colour = black_marble(cache, POOL, ppd)
    grey = black_marble(cache, POOL, ppd, grey=True)
    H, W = grey.shape
    lum = colour @ np.array([0.3, 0.5, 0.2], np.float32)
    sea = ndimage.gaussian_filter((lum < 0.03).astype(np.float32), 1.0)
    rgb = mix(N_LAND, N_SEA, sea) * np.ones((H, W, 1), np.float32)
    # the land's relief where the elevation model reaches, as the land picture draws it
    z40 = Image.fromarray(elevation(cache, BOX, 40)).resize((round((BOX[2] - BOX[0]) * ppd), round((BOX[3] - BOX[1]) * ppd)), Image.BILINEAR)
    x0, y0 = round((BOX[0] - w) * ppd), round((n - BOX[3]) * ppd)
    za = np.asarray(z40)
    sub = (slice(y0, y0 + za.shape[0]), slice(x0, x0 + za.shape[1]))
    land = night_land(za, ppd, BOX[3], colour[sub], np.zeros_like(za))
    wgt = box_weight(za.shape, BOX, BOX, 0.8)
    rgb[sub] = mix(rgb[sub], land, wgt)
    rgb = with_lights(rgb, grey)
    # the ring, by great-circle distance from the centre: twilight violet, then the sunset's apricot
    lon = np.radians(w + (np.arange(W) + .5) / ppd)[None, :]
    lat = np.radians(n - (np.arange(H) + .5) / ppd)[:, None]
    c0, c1 = np.radians(POOL_CENTRE)
    d = np.degrees(np.arccos(np.clip(np.sin(lat) * np.sin(c1) + np.cos(lat) * np.cos(c1) * np.cos(lon - c0), -1, 1)))
    t = smoothstep(POOL_IN, POOL_OUT, d)
    violet = np.array([0.36, 0.30, 0.52], np.float32)
    apricot = np.array([1.0, 0.74, 0.56], np.float32)
    dusk = np.where((t < 0.5)[..., None], mix(rgb, violet, t / 0.5), mix(violet, apricot, (t - 0.5) / 0.5))
    rgb = np.where((t > 0)[..., None], dusk, rgb)
    alpha = 1 - smoothstep(POOL_IN + 0.35 * (POOL_OUT - POOL_IN), POOL_OUT, d) ** 1.4
    save_png(rgb, alpha, OUT / 'pool.png')
    return {'pool': list(POOL)}


def step_sprites(cache):
    """A lamp's flicker in six frames (a flame-coloured point and its halo, 36 px for 12 points at 3x),
    and the doorway's glint from afar (the shared mark, tools/playground/glint_png.py)."""
    size = 36
    yy, xx = np.mgrid[0:size, 0:size] + 0.5
    r = np.hypot(xx - size / 2, yy - size / 2) / (size / 2)
    for i, (k, spread) in enumerate(((1.0, 1.0), (0.82, 0.92), (0.95, 1.05), (0.7, 0.88), (0.9, 1.0), (0.78, 0.95)), 1):
        core = np.exp(-(r / (0.13 * spread)) ** 2) * k
        halo = np.exp(-(r / (0.42 * spread)) ** 2) * 0.55 * k
        alpha = np.clip(core + halo, 0, 1)
        with np.errstate(invalid='ignore', divide='ignore'):
            share = np.nan_to_num(core / np.maximum(alpha, 1e-6))
        rgb = mix(LAMP_HALO, np.array([1.0, 0.95, 0.82], np.float32), np.clip(share * 1.2, 0, 1))
        save_png(rgb, alpha, OUT / f'lamp-{i}.png')
    import subprocess
    subprocess.run([sys.executable, str(ROOT / 'tools' / 'playground' / 'glint_png.py'), str(OUT / 'doorway-glint.png')], check=True)
    return {}


STEPS = {'land': step_land, 'lamps': step_lamps, 'glow': step_glow, 'detail': step_detail,
         'pool': step_pool, 'sprites': step_sprites}

if __name__ == '__main__':
    cache = Path(sys.argv[1])
    OUT.mkdir(parents=True, exist_ok=True)
    meta_path = OUT / 'bounds.js'
    meta = {}
    if meta_path.exists():
        text = meta_path.read_text()
        meta = json.loads(text[text.index('{'):text.rindex('}') + 1])
    for name in [a for a in sys.argv[2:] if not a.startswith('--')] or list(STEPS):
        print('step', name, flush=True)
        meta.update(STEPS[name](cache))
    meta_path.write_text('// written by tools/lamplight/build.py: each picture\'s [west, south, east, north]\n'
                         'window.LAMPLIGHT_BOUNDS = ' + json.dumps(meta, sort_keys=True) + ';\n')
