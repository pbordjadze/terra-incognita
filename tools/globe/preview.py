"""Draws the globe as the app's shader does (App/TerraIncognita/Globe/Globe.metal mirrors this, line
for line), so its look can be judged without a phone.

    python3 tools/globe/preview.py out.png [--lon -83 --lat 40 --zoom 1 --select Italy --scale 2]
    python3 tools/globe/preview.py out.png --style ground [--month 7]     (--style night: the Earth at night)

Coordinates: the screen in points, y down; the globe's centre and radius come from `layout`, as in
GlobeView's `place`. --select names a country (globe.json); its locator red is drawn at full strength.
Labels are drawn here with PIL in the app's fonts, roughly where the app's label layer puts them.
"""
import argparse, json
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
G = ROOT / 'Globe'
TYPE = json.loads((G / 'type.json').read_text())       # the type and its scale (GlobeType)


def reading(w, h):
    """The scale lines and names are drawn at on a w x h point screen (GlobeType.readingScale)."""
    (s0, s1), (f0, f1) = TYPE['reading']['shortSide'], TYPE['reading']['factor']
    return f0 + (f1 - f0) * min(max((min(w, h) - s0) / (s1 - s0), 0), 1)


def hexc(s):
    return np.array([int(s[i:i + 2], 16) for i in (1, 3, 5)], np.float32) / 255


# The palette: WikiProject Maps' location-map colours, warmed a little.
PAPER = hexc('#F6F2E7')
OCEAN_SHALLOW = hexc('#DDF3FD')
OCEAN_DEEP = hexc('#BCE3F6')
COAST = hexc('#1C7AB0')
LAND = hexc('#F5EED8')
HIGHLAND = hexc('#E4D3AE')
BORDER = hexc('#8C8478')
RIVER = hexc('#4E9CCB')
LOCATOR = hexc('#C12838')
GOLD = hexc('#B08A3E')
SHADOW = hexc('#6E6352')

# The Persian style, every colour traced (Globe.metal's P_ palette, line for line; CLAUDE.md › The
# Persian style's sources): the manuscript's colours sampled from LACMA's scans of the Khamsa of Nizami
# (M.85.237.16), white-balanced so its lead white is neutral; the map's tints Wikipedia's lightness in
# the manuscript's hues.
P_PAPER = hexc('#F2E4C8')
P_SEA = hexc('#E1EFFF')
P_SEA_DEEP = hexc('#C1D6F0')
P_COAST = hexc('#6F6995')
P_MEADOW = hexc('#DDE7D9')
P_ROCK = hexc('#EDDEEC')
P_PEAK = hexc('#F7F1FF')
P_SHADE = hexc('#A298A7')
P_WHITE = hexc('#F5F5F5')
P_BORDER = hexc('#825947')
P_LAPIS = hexc('#5F6CA7')
P_GOLD = hexc('#C5AA6E')
P_PINK = hexc('#D1A38F')
P_GREEN = hexc('#79A18B')
P_LIGHT_BLUE = hexc('#A2B7D0')
P_SALMON = hexc('#C47C66')
P_INK = hexc('#251F14')
P_SEL = hexc('#C2A862')
SEL_ALPHA = 0.85

# The ground style (Globe.metal's G_ palette, line for line; the land is baked by tools/globe/ground.py).
G_SHELF = hexc('#D3F0EE')
G_OPEN = hexc('#C2E4F4')
G_DEEP = hexc('#A9D2EC')
G_COAST = hexc('#5E9CC0')
G_RIVER = hexc('#6FAED3')
G_ICE = hexc('#F1F5F8')
G_ATMOS = hexc('#DCEEFB')
G_CHOSEN = hexc('#9E1F2C')

# The night (Globe.metal's N_ palette, line for line; the lights by tools/globe/night.py).
N_LAND = hexc('#1E2430')
N_SEA = hexc('#101828')
N_DEEP = hexc('#09101E')
N_ICE = hexc('#444E60')
N_COAST = hexc('#3C5874')
N_LIGHT = hexc('#FFD686')
N_CORE = hexc('#FFF4DC')
N_ATMOS = hexc('#5C82B8')
N_CHOSEN = hexc('#7E303A')


def layout(w, h, persian=False):
    """The globe's centre and resting radius in points (GlobeView's `place`): as large as its room
    (between the buttons and the caption) allows with its frame round it, 14 pt from the room's edges."""
    top = buttons_box(w, h)[3]
    bottom = caption_box(w, h)[1] if persian else h - (20 if min(w, h) >= 700 else 34) - 150
    half = min(w, bottom - top) / 2 - 14
    return w / 2, (top + bottom) / 2, half / 1.1833 if persian else half - 11 * reading(w, h)


def rotation(lon0, lat0):
    """World from view: Ry(lon0) . Rx(-lat0) (GlobeView.rotation)."""
    a, b = np.radians(lon0), np.radians(-lat0)
    ry = np.array([[np.cos(a), 0, np.sin(a)], [0, 1, 0], [-np.sin(a), 0, np.cos(a)]])
    rx = np.array([[1, 0, 0], [0, np.cos(b), -np.sin(b)], [0, np.sin(b), np.cos(b)]])
    return ry @ rx


def hash2(x, y):
    """Globe.metal's hash, in 32-bit floats as there."""
    x, y = np.float32(x), np.float32(y)
    fr = lambda a: (a - np.floor(a)).astype(np.float32)
    p0, p1, p2 = fr(x * np.float32(0.1031)), fr(y * np.float32(0.1031)), fr(x * np.float32(0.1031))
    dd = p0 * (p1 + np.float32(33.33)) + p1 * (p2 + np.float32(33.33)) + p2 * (p0 + np.float32(33.33))
    p0, p1, p2 = p0 + dd, p1 + dd, p2 + dd
    return fr((p0 + p1) * p2)


def vnoise(x, y):
    xi, yi = np.floor(x), np.floor(y)
    xf, yf = x - xi, y - yi
    u, v = xf * xf * (3 - 2 * xf), yf * yf * (3 - 2 * yf)
    a, b = hash2(xi, yi), hash2(xi + 1, yi)
    c, d = hash2(xi, yi + 1), hash2(xi + 1, yi + 1)
    return (a + (b - a) * u) * (1 - v) + (c + (d - c) * u) * v


def vnoise3(x, y, z):
    """Globe.metal's vnoise3: value noise in three dimensions (no seam, no pole on the sphere)."""
    xi, yi, zi = np.floor(x), np.floor(y), np.floor(z)
    f = lambda a: a * a * (3 - 2 * a)
    u, v, w = f(x - xi), f(y - yi), f(z - zi)
    n = 0
    for dz in (0, 1):
        for dy in (0, 1):
            for dx in (0, 1):
                cx, cy, cz = xi + dx, yi + dy, zi + dz
                h = hash2(hash2(cx, cy) * np.float32(157.0) + cz * np.float32(1.37), cz * np.float32(0.71) + hash2(cy, cx) * np.float32(89.0))
                n = n + h * (u if dx else 1 - u) * (v if dy else 1 - v) * (w if dz else 1 - w)
    return n


def worley(x, y):
    """Distance to the nearest of one jittered point per unit cell (Globe.metal's worley)."""
    xi, yi = np.floor(x), np.floor(y)
    best = np.full(np.shape(x), 9.0, np.float32)
    for oy in (-1, 0, 1):
        for ox in (-1, 0, 1):
            cx, cy = xi + ox, yi + oy
            jx, jy = hash2(cx, cy), hash2(cy + 17.0, cx + 31.0)
            best = np.minimum(best, np.hypot(cx + jx - x, cy + jy - y))
    return best


def fwidth(f):
    gy, gx = np.gradient(f)
    return np.abs(gx) + np.abs(gy)


def smoothstep(a, b, x):
    t = np.clip((x - a) / (b - a), 0, 1)
    return t * t * (3 - 2 * t)


def line(d, width, fw=None):
    """Coverage of a line of `width` pixels along d == 0, d a field in any unit; fw its change per
    pixel when d itself wraps (a wrapped field jumps where it wraps, and its own fwidth with it)."""
    fw = fwidth(d) if fw is None else fw
    return 1 - smoothstep(width / 2 - 0.5, width / 2 + 0.5, np.abs(d) / np.maximum(fw, 1e-5))


def sample(img, u, v):
    h, w = img.shape[:2]
    x, y = u * w - 0.5, v * h - 0.5
    x0, y0 = np.floor(x).astype(int), np.floor(y).astype(int)
    fx, fy = (x - x0)[..., None], (y - y0)[..., None]
    if img.ndim == 2:
        img = img[..., None]
    g = lambda yy, xx: img[np.clip(yy, 0, h - 1), xx % w].astype(np.float32)
    top = g(y0, x0) * (1 - fx) + g(y0, x0 + 1) * fx
    bot = g(y0 + 1, x0) * (1 - fx) + g(y0 + 1, x0 + 1) * fx
    return top * (1 - fy) + bot * fy


def mix(a, b, t):
    t = np.asarray(t)[..., None] if np.ndim(t) else t
    return a * (1 - t) + b * t


# The Persian frame, outward from the globe's edge in units of its radius (Globe.metal's): the
# rulings in the order and proportions measured across the opening spread's frame, the braid and the
# border band between them, then on the paper a ruled line with the margin's finials on it.
P_RULES = (                       # (from, to, colour)
    (1.0000, 1.0018, P_PINK), (1.0018, 1.0055, P_GOLD), (1.0055, 1.0091, P_LIGHT_BLUE),
    (1.0307, 1.0314, P_GOLD), (1.0314, 1.0373, P_GREEN), (1.0373, 1.0416, P_GOLD), (1.0416, 1.0453, P_PINK),
    (1.0453, 1.0471, P_GOLD), (1.0471, 1.0486, P_LAPIS), (1.0486, 1.0635, P_GOLD), (1.0635, 1.0650, P_INK),
    (1.1150, 1.1161, P_INK), (1.1161, 1.1274, P_GOLD), (1.1274, 1.1303, P_PINK), (1.1376, 1.1383, P_COAST))
P_BRAID = (1.0091, 1.0307, 74)    # inner, outer, repeats round
P_BAND = (1.0650, 1.1150, 30)
P_FLEURON = (1.1383, 0.045, 48)   # base, height, how many


def picture(name, size, fallback):
    """A bundled ornament (tools/globe/ornament.py), or a stand-in, resized to `size` (w, h) pixels on
    screen: roughly what the shader's mipmaps do (it filters per pixel, by the footprint, and reads
    transparent past a sprite's edge; this is close enough to judge the look by)."""
    path = G / name
    img = Image.open(path).convert('RGBA') if path.exists() else fallback()
    a = np.asarray(img).astype(np.float32) / 255
    a[..., :3] *= a[..., 3:]                                 # premultiplied, as the app uploads it
    out = [np.asarray(Image.fromarray(a[..., c]).resize((max(int(size[0]), 2), max(int(size[1]), 2)), Image.LANCZOS)) for c in range(4)]
    return np.clip(np.stack(out, -1), 0, 1)


def stand_in_braid():
    img = Image.new('RGBA', (1024, 256), tuple(int(c * 255) for c in P_GOLD) + (255,))
    d = ImageDraw.Draw(img)
    for k in range(9):
        x = k * 256
        d.line([(x - 256, 40), (x - 128, 216), (x, 40)], fill=(37, 31, 20, 255), width=30)
        d.line([(x - 256, 216), (x - 128, 40), (x, 216)], fill=(37, 31, 20, 255), width=30)
    return img


def stand_in_band():
    img = Image.new('RGBA', (1536, 256), tuple(int(c * 255) for c in P_LAPIS) + (255,))
    d = ImageDraw.Draw(img)
    d.rectangle((0, 0, 144, 256), fill=tuple(int(c * 255) for c in P_SALMON) + (255,))
    d.rectangle((1392, 0, 1536, 256), fill=tuple(int(c * 255) for c in P_SALMON) + (255,))
    xs = np.arange(160, 1380)
    d.line(list(zip(xs, 128 + 56 * np.sin((xs - 160) / 1220 * 8 * np.pi))), fill=(168, 137, 79, 255), width=7)
    for cx in (0, 1536):
        d.ellipse((cx - 60, 68, cx + 60, 188), fill=(42, 33, 25, 255), outline=(239, 231, 218, 255), width=5)
    return img


def stand_in_fleuron():
    img = Image.new('RGBA', (256, 256), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    ink = tuple(int(c * 255) for c in P_LAPIS) + (255,)
    d.line((128, 240, 128, 110), fill=ink, width=7)
    d.ellipse((93, 40, 163, 120), fill=(169, 181, 216, 255), outline=ink, width=7)
    return img


# The page's frame, when the globe is looked at closer than its own frame allows (Globe.metal's): the
# map becomes the text panel, framed as the opening spread frames its panels, by their rulings and
# braid, on a margin of paper at the screen's edges. In reading points from the paper's inner edge,
# outward from the panel: pink, gold, light blue, the braid, gold, green, gold, pink (the measured
# proportions, the braid 8 points deep).
PAGE_MARGIN = 6
PAGE_RULES = ((0, 0.68, 'P_PINK'), (0.68, 2.03, 'P_GOLD'), (2.03, 3.39, 'P_LIGHT_BLUE'), (11.39, 11.66, 'P_GOLD'),
              (11.66, 13.83, 'P_GREEN'), (13.83, 15.46, 'P_GOLD'), (15.46, 16.81, 'P_PINK'))
PAGE_BRAID = (3.39, 11.39)
PAGE_DEPTH = 16.81
PAGE_ROOM = 30                    # what sits on the page keeps this far in from the safe area: inside the frame


def page_frame(col, px, py, W, H, ls, insets, alpha, grain):
    """The page's frame over `col` (the picture so far), `alpha` of the way in. `insets` are the safe
    area's (top, bottom, left, right) in pixels; `ls` pixels per reading point."""
    top, bottom, left, right = insets
    outer = PAGE_MARGIN * ls
    # e: reading points inward from the frame's outer edge, to the nearest of the four edges
    dl, dr, dt, db = px - left - outer, W - right - outer - px, py - top - outer, H - bottom - outer - py
    e = np.minimum(np.minimum(dl, dr), np.minimum(dt, db)) / ls
    horizontal = np.minimum(dt, db) <= np.minimum(dl, dr)    # nearest the top or bottom: the braid runs along x
    paper = P_PAPER * (0.985 + 0.03 * grain)[..., None]
    out = mix(col, paper, alpha * (1 - np.clip(e * ls + 0.5, 0, 1)))
    def band(a, b):                                          # (a, b: reading points out from the panel)
        lo, hi, k = PAGE_DEPTH - b, PAGE_DEPTH - a, 1.0
        if (hi - lo) * ls < 0.8:                             # (never thinner than 0.8 px: as much fainter)
            c = (lo + hi) / 2
            lo, hi, k = c - 0.4 / ls, c + 0.4 / ls, (hi - lo) * ls / 0.8
        return k * np.clip(np.minimum(e - lo, hi - e) * ls + 0.5, 0, 1)
    for a, b, c in PAGE_RULES:
        out = mix(out, globals()[c], alpha * band(a, b))
    tile = 8 * 4 * ls                                        # the braid's tile: 4:1, 8 points deep
    wtile = picture('braid.png', (tile, 8 * ls), stand_in_braid)
    along = np.where(horizontal, px, py) / tile
    across = (PAGE_DEPTH - PAGE_BRAID[0] - e) / (PAGE_BRAID[1] - PAGE_BRAID[0])   # 0 at the panel's side
    wb = sample(wtile, along % 1, np.clip(across, 0, 1))
    return mix(out, wb[..., :3], alpha * band(*PAGE_BRAID) * wb[..., 3])


def page_strips(w, h, insets):
    """The page frame's four strips in points (x0, y0, x1, y1): no name goes under them."""
    top, bottom, left, right = insets
    d = (PAGE_MARGIN + PAGE_DEPTH) * reading(w, h)
    return [(0, 0, w, top + d), (0, h - bottom - d, w, h), (0, 0, left + d, h), (w - right - d, 0, w, h)]


def persian_page(px, py, x, y, rr, R, W, H, grain):
    page = P_PAPER * (0.985 + 0.03 * grain)[..., None]      # (the paper's own texture; no shadow, no vignette)
    def ring(a, b):
        w, k = (b - a) * R, 1.0                                # (never thinner than 0.8 px: as much fainter)
        if w < 0.8:
            c = (a + b) / 2
            a, b, k = c - 0.4 / R, c + 0.4 / R, w / 0.8
        return k * smoothstep(a - 0.5 / R, a + 0.5 / R, rr) * (1 - smoothstep(b - 0.5 / R, b + 0.5 / R, rr))
    for a, b, c in P_RULES:
        page = mix(page, c, ring(a, b))
    theta = np.arctan2(x, y) % (2 * np.pi)                   # clockwise from the top
    w_in, w_out, wn = P_BRAID
    wlength = 2 * np.pi * (w_in + w_out) / 2 * R / wn
    wtile = picture('braid.png', (wlength, (w_out - w_in) * R), stand_in_braid)
    wb = sample(wtile, (theta / (2 * np.pi) * wn) % 1, (rr - w_in) / (w_out - w_in))
    page = mix(page, wb[..., :3], ring(w_in, w_out) * wb[..., 3])
    r_in, r_out, n = P_BAND
    length = 2 * np.pi * (r_in + r_out) / 2 * R / n
    tile = picture('band.png', (length, (r_out - r_in) * R), stand_in_band)
    b = sample(tile, (theta / (2 * np.pi) * n) % 1, (rr - r_in) / (r_out - r_in))
    page = mix(page, b[..., :3], ring(r_in, r_out) * b[..., 3])
    base, tall, count = P_FLEURON
    side = tall * 256 / 224                                  # the sprite's content is 224 of its 256 px
    sprite = picture('fleuron.png', (side * R, side * R), stand_in_fleuron)
    cell = (theta / (2 * np.pi) * count) % 1 - 0.5
    fu = 0.5 + cell * (2 * np.pi * base / count) / side
    fv = (240 - (rr - base) / side * 256) / 256
    f = sample(sprite, fu, np.clip(fv, 0, 1))
    on = (fv > 0) & (fv < 1) & (np.abs(fu - 0.5) < 0.5)
    page = page * (1 - f[..., 3:] * on[..., None]) + f[..., :3] * on[..., None]
    return page


def persian_globe(scale, k, u, v, d, fine, border, hill, hm, river, depth, mott, gl, m, landa, x, y, z, rr, R, lined, historical_reference=False):
    # the sea: a pale lapis wash, its coasts ruled like a panel, a gold hair close in and a lapis one out
    sea = mix(P_SEA, P_SEA_DEEP, depth)                      # (flat washes, as the manuscript's colours are)
    near = smoothstep(1.4, 2.6, k)                           # the rulings come with a closer look
    sea = mix(sea, P_GOLD, 0.6 * near * line(fine + 4.5, 0.6 * scale) * ((fine < -1.5) & (fine > -8)))
    sea = mix(sea, P_LIGHT_BLUE, 0.5 * near * line(fine + 11, 0.6 * scale) * ((fine < -8) & (fine > -15.5)))
    # the land: the meadows' sage low, the rocks' lavender high, their shadows violet
    land = mix(P_MEADOW, P_ROCK, smoothstep(900, 2600, hm))
    land = mix(land, P_PEAK, 0.7 * smoothstep(5000, 6300, hm))
    land = mix(land, P_SHADE, 0.36 * np.clip(-hill, 0, 1))
    land = mix(land, P_WHITE, 0.30 * np.clip(hill, 0, 1))
    # closer in, the heights' contours every 500 m, pale like the rocks' highlights
    # (lobed, as the painters drew rocks: the contours of the height plus a little cellular swell)
    hc = hm / 600 + 0.3 * worley(u * 4096 / 4.5, v * 2048 / 4.5)
    contour = line(((hc + 0.5) % 1) - 0.5, 0.55 * scale, fwidth(hc)) * smoothstep(800, 1300, hm) * smoothstep(1.6, 2.6, k)
    land = mix(land, P_WHITE, 0.7 * contour)
    land = mix(land, P_SHADE, 0.22 * line(((hc + 0.42) % 1) - 0.5, 0.55 * scale, fwidth(hc)) * smoothstep(800, 1300, hm) * smoothstep(1.6, 2.6, k))
    land = mix(land, P_COAST, 0.75 * line(river, 0.65 * scale) * smoothstep(1.2, 2.2, k) * lined(river))
    # the chosen country gilded in the cartouche's matte gold, its speckle and all, and edged in its pink
    gild = P_SEL * (1 + 0.20 * hill + 0.05 * (vnoise(u * 2400, v * 1200) - 0.5))[..., None]
    land = mix(land, gild, SEL_ALPHA * m)
    border_on = lined(border) * (not historical_reference)
    bl = line(border, 0.7 * scale) * border_on
    land = mix(land, P_BORDER, 0.75 * bl)
    land = mix(land, P_PINK, line(border, 1.5 * scale) * border_on * (m > 0.04))
    col = mix(sea, land, landa)
    col = mix(col, P_COAST, line(d, 0.75 * scale * min(k ** 0.3, 1.3)))
    col = mix(col, P_GOLD, gl * np.where(landa > 0.5, 0.14, 0.22))
    col = col * (1 - 0.06 * (1 - z) ** 2)[..., None]
    return col


def ground_maps(month):
    """The ground style's maps for a month (Globe/ground/): the land, its sea ice, the sea floor."""
    g = G / 'ground'
    return (np.asarray(Image.open(g / f'{month:02d}.jpg').convert('RGB')), np.asarray(Image.open(g / f'ice-{month:02d}.png').convert('RGB')),
            np.asarray(Image.open(g / 'floor.png').convert('RGB')))


def render(w, h, scale, lon0, lat0, zoom, selected, persian=False, historical_reference=False, ground=False, month=None, night=False,
           night_time=0.0):
    if historical_reference:
        selected = 0
    geo = np.asarray(Image.open(G / 'geo.png'))[..., :3]
    ter = np.asarray(Image.open(G / 'terrain.png'))[..., :3]
    ids = np.asarray(Image.open(G / 'ids.png'))
    W, H = int(w * scale), int(h * scale)
    ls = scale * reading(w, h)                          # pixels per reading point: lines' widths
    cx, cy, r0 = layout(w, h, persian)
    R = r0 * zoom * scale
    px, py = np.meshgrid(np.arange(W) + 0.5, np.arange(H) + 0.5)
    x, y = (px - cx * scale) / R, (cy * scale - py) / R
    rr = np.sqrt(x * x + y * y)
    # the page: paper with a little grain and a soft vignette, the globe's shadow, the gold ring
    grain = vnoise(px / 1.7, py / 1.7) * 0.5 + vnoise(px / 9, py / 9) * 0.5
    page = PAPER * (0.985 + 0.03 * grain)[..., None]
    vig = np.hypot(px / W - 0.5, py / H - 0.48)
    page = page * (1 - 0.07 * smoothstep(0.35, 0.8, vig))[..., None]
    sh = np.exp(-np.maximum(np.hypot(x, y + 0.035) - 0.985, 0) ** 2 / 0.0035) * (rr > 0.97)
    page = mix(page, SHADOW, 0.16 * sh)
    ring_on = (1 - smoothstep(1.25, 1.7, zoom)) * (0 if ground else 1)
    ringpx = (rr - 1) * R / ls                   # points outside the globe's edge
    gold = np.zeros_like(rr)
    for at, wd in ((6.5, 1.1), (10.0, 0.7)):
        gold = np.maximum(gold, line(ringpx - at, wd * ls))
    ang = np.degrees(np.arctan2(x, y)) % 360
    tick = line(((ang + 5) % 10) - 5, 0.9 * ls, np.degrees(1 / np.maximum(rr * R, 1))) * ((ringpx > 7) & (ringpx < 9.6))
    dots = np.zeros_like(rr)
    for a in range(0, 360, 90):                     # the four quarters: three little dots, Kells-fashion
        for da, dr in ((0, 8.25), (-1.5, 8.25), (1.5, 8.25)):
            t = np.radians(a + da)
            dx = cx * scale + np.sin(t) * (R + dr * ls) - px
            dy = cy * scale - np.cos(t) * (R + dr * ls) - py
            dots = np.maximum(dots, 1 - smoothstep(1.1 * ls, 1.9 * ls, np.hypot(dx, dy)))
    page = mix(page, GOLD, np.clip(np.maximum(np.maximum(gold, tick * 0.8), dots), 0, 1) * ring_on)
    if persian:
        page = persian_page(px, py, x, y, rr, R, W, H, grain)
    if ground:
        # the ground style's air: a thin bright haze just outside the limb, as it is seen edge on (by
        # night, the air's blue glow, a little wider)
        page = mix(page, N_ATMOS if night else G_ATMOS, (0.7 if night else 0.85) * np.exp(-np.maximum(ringpx, 0) / (5.0 if night else 3.5)) * (rr >= 1))

    # the globe
    inside = rr < 1.0 + 2 / R
    z = np.sqrt(np.clip(1 - rr * rr, 0, 1))
    M = rotation(lon0, lat0)
    wx = M[0, 0] * x + M[0, 1] * y + M[0, 2] * z
    wy = M[1, 0] * x + M[1, 1] * y + M[1, 2] * z
    wz = M[2, 0] * x + M[2, 1] * y + M[2, 2] * z
    lat = np.arcsin(np.clip(wy, -1, 1))
    lon = np.arctan2(wx, wz)
    u = (lon / np.pi + 1) / 2
    v = 0.5 - lat / np.pi
    g = sample(geo, u, v) / 255
    t = sample(ter, u, v) / 255
    fine = (g[..., 0] * 255 - 128) / 8
    coarse = (g[..., 1] * 255 - 128) * 2
    d = np.where(np.abs(coarse) > 14, coarse, fine)
    border = (g[..., 2] * 255 - 128) / 16
    hill = (t[..., 0] * 255 - 128) / 127
    height = t[..., 1] * 255 - 128
    river = (t[..., 2] * 255 - 128) / 16
    # a line's distance changes about a texel a texel; where its sign flips (past a line's end,
    # between lines) it jumps: no line there (Globe.metal's perTexel)
    def grad(f):
        gy, gx = np.gradient(f)
        return gx, gy
    ux1, uy1 = grad(u)
    ux2, uy2 = grad((u + 0.5) % 1)                      # the antimeridian's jump, or 0's
    ux = np.where(np.abs(ux1) < np.abs(ux2), ux1, ux2) * 4096
    uy = np.where(np.abs(uy1) < np.abs(uy2), uy1, uy2) * 4096
    vx, vy = grad(v)
    vx, vy = vx * 2048, vy * 2048
    det = ux * vy - vx * uy
    safe = np.where(np.abs(det) < 1e-8, 1e-8, det)

    def lined(f):
        gx, gy = grad(f)
        gt = np.hypot(vy * gx - vx * gy, -uy * gx + ux * gy) / np.abs(safe)
        return (gt < 1.5) & (np.abs(det) >= 1e-8) & (np.abs(f) < 7)
    landa = smoothstep(-0.5, 0.5, d / np.maximum(fwidth(d), 1e-5))
    k = np.clip(zoom, 1, 8)
    depth = np.clip(-np.minimum(height, 0) * 87 / 5500, 0, 1) ** 0.6
    mott = vnoise(u * 90, v * 45) * 0.6 + vnoise(u * 260, v * 130) * 0.4
    hm = np.maximum(height, 0) * 50
    glat, glon = np.degrees(lat), np.degrees(lon)
    fwlat = fwidth(glat)
    fwlon = np.minimum(fwidth(glon), fwidth(glon % 360))     # the antimeridian's jump, or 0's
    gl = np.maximum(line(((glat + 7.5) % 15) - 7.5, 0.6 * ls, fwlat),
                    line(((glon + 7.5) % 15) - 7.5, 0.6 * ls, fwlon) * (np.abs(glat) < 75))
    # the selected country (the shader: gather, then bilinear)
    m = sample((ids == selected).astype(np.float32), u, v)[..., 0] if selected else np.zeros_like(u)
    if persian:
        col = persian_globe(ls, k, u, v, d, fine, border, hill, hm, river, depth, mott, gl, m, landa, x, y, z, rr, R, lined, historical_reference)
    else:
        # the sea: deeper is a little bluer, with a faint watercolour mottle and lines along the coasts
        sea = mix(OCEAN_SHALLOW, OCEAN_DEEP, depth)
        sea = sea * (0.985 + 0.03 * mott)[..., None]
        for at, a in ((-6, 0.15), (-13, 0.08)):
            sea = mix(sea, COAST, a * line(fine - at, 0.7 * ls) * ((fine < -2) & (fine > -15.5)))
        # the land: cream, warmer up high, the relief shaded from the north-west
        land = mix(LAND, HIGHLAND, 0.85 * smoothstep(250, 3200, hm))
        land = land * (1 + 0.30 * hill)[..., None]
        land = mix(land, RIVER, 0.75 * line(river, 0.65 * ls) * smoothstep(1.2, 2.2, k) * lined(river))
        # the selected country over its rivers, as a locator map has it; its borders drawn on top
        land = mix(land, LOCATOR * (1 + 0.12 * hill)[..., None], 0.94 * m)
        land = mix(land, BORDER, 0.8 * line(border, 0.7 * ls) * lined(border) * (not historical_reference))
        col = mix(sea, land, landa)
        col = mix(col, COAST, line(d, 0.9 * ls * min(k ** 0.25, 1.4)))
        # the graticule, every 15 degrees
        col = mix(col, COAST, gl * np.where(landa > 0.5, 0.10, 0.16))
        # roundness: the limb a little shaded; the rim drawn
        col = col * (1 - 0.07 * (1 - z) ** 2)[..., None]
        col = mix(col, COAST, line((rr - 1) * R, 1.2 * ls))
        if ground:
            # the ground (Globe.metal's, line for line): the month's land, shaded from the north-west;
            # rivers from the regional scale; borders a pencil line from about x2 to x3; the chosen
            # country's red let down
            land_m, ice_m, floor_m = ground_maps(month)
            gland = sample(land_m, u, v) / 255 * (1 + 0.20 * hill)[..., None]
            gland = mix(gland, G_RIVER, 0.7 * line(river, 0.6 * ls) * smoothstep(1.4, 2.4, k) * lined(river))
            gland = mix(gland, LOCATOR * (1 + 0.12 * hill)[..., None], 0.75 * m)
            gland = mix(gland, BORDER, 0.55 * smoothstep(2.0, 3.2, zoom) * line(border, 0.6 * ls) * lined(border) * (not historical_reference))
            # the chosen country's land borders a firm line, over its fill's stepped edge
            gland = mix(gland, G_CHOSEN, line(border, 1.6 * ls) * lined(border) * (not historical_reference) * (m > 0.04))
            depth_m = -np.minimum(height, 0) * 87
            gsea = mix(G_SHELF, G_OPEN, smoothstep(60, 900, depth_m))
            gsea = mix(gsea, G_DEEP, smoothstep(2500, 6000, depth_m))
            fl = (sample(floor_m, u, v)[..., 0] - 128) / 127
            gsea = gsea * (1 + 0.04 * fl)[..., None]
            ice = sample(ice_m, u, v)[..., 0] / 255
            o = (np.float32(17.3), np.float32(5.1), np.float32(9.7))
            jag = np.where(ice > 0.02, vnoise3(wx * 220 + o[0], wy * 220 + o[1], wz * 220 + o[2]) * 0.6
                           + vnoise3(wx * 900 + o[0], wy * 900 + o[1], wz * 900 + o[2]) * 0.4, 0.5)
            gsea = mix(gsea, G_ICE * (0.97 + 0.04 * jag)[..., None], smoothstep(0.30, 0.55, ice + 0.25 * (jag - 0.5)))
            g = mix(gsea, gland, landa)
            pale = smoothstep(0.90, 0.97, gland.mean(-1))
            g = mix(g, G_COAST, (0.55 - 0.35 * pale) * line(d, 0.6 * ls * min(k ** 0.3, 1.5)))
            g = g * (1 - 0.05 * (1 - z) ** 2)[..., None]
            col = mix(g, G_ATMOS, 0.55 * (1 - z) ** 4)
            if night:
                # the Earth at night: the measured lights over the drawn land and sea (N_ palette)
                lights = (sample(np.asarray(Image.open(G / 'ground' / 'night.jpg').convert('RGB')), u, v)[..., 0] / 255) ** 0.75
                # (they come on a beat after dusk, fully by now, and each shimmers a little, slowly)
                shimmer = vnoise3(wx * 1400 + np.float32(night_time * 0.45), wy * 1400 + np.float32(night_time * 0.31), wz * 1400 + np.float32(7.9))
                lights = lights * (0.86 + 0.28 * shimmer)
                nland = N_LAND * (1 + 0.25 * hill)[..., None]
                nland = mix(nland, N_CHOSEN, 0.6 * m)
                nland = mix(nland, N_LIGHT, smoothstep(0.0, 0.85, lights))
                nland = mix(nland, N_CORE, 0.7 * smoothstep(0.7, 1.0, lights))
                nsea = mix(N_SEA, N_DEEP, smoothstep(2500, 6000, depth_m)) * (1 + 0.06 * fl)[..., None]
                nsea = mix(nsea, N_ICE * (0.97 + 0.04 * jag)[..., None], smoothstep(0.30, 0.55, ice + 0.25 * (jag - 0.5)))
                n = mix(nsea, nland, landa)
                n = mix(n, N_COAST, 0.5 * line(d, 0.6 * ls * min(k ** 0.3, 1.5)))
                # roundness: the limb darker, a sphere by night; and the air's glow at it
                n = n * (1 - 0.3 * (1 - z) ** 2)[..., None]
                col = mix(n, N_ATMOS, 0.6 * (1 - z) ** 3)
    edge = smoothstep(1 + 0.5 / R, 1 - 0.5 / R, rr)
    out = np.where(inside[..., None], mix(page, col, edge), page)
    if persian:
        # the page's frame comes in as the globe's own frame leaves the screen
        out = page_frame(out, px, py, W, H, ls, tuple(v * scale for v in safe_insets(w, h)), page_alpha(zoom), grain)
    return Image.fromarray((np.clip(out, 0, 1) * 255).astype(np.uint8), 'RGB'), (cx, cy, r0 * zoom, M)


def short(name):
    return {'United States of America': 'United States'}.get(name, name)


def cartouche(name):
    """The name the Persian caption letters in its cartouche (GlobeScreen.cartouche)."""
    special = {'United States of America': 'United States', 'Heard I. and McDonald Is.': 'Heard & McDonald Is.'}
    return special.get(name, name.replace(' and ', ' & '))


def labels(img, w, h, scale, geom, zoom, selected, persian=False, keep_clear=(), ground=False, month=None, night=False):
    """The app's names (GlobeView, CLAUDE.md › Design rules), in the faces of Globe/type.json at the
    screen's reading scale: the selected country, oceans and seas, then countries (larger the more
    prominent) and capitals, in that order of claim; each on a halo of its ground's tone, wholly on
    the globe and on screen, clear of `keep_clear` (x0, y0, x1, y1 in points) and its gap from any
    other. On the ground (GlobeView's makeLabels): no names at the whole Earth, the land's own names
    (regions.json) from the regional scale, countries closer in, each on a halo of the ground under it."""
    cx, cy, R, M = geom
    data = json.loads((G / 'globe.json').read_text())
    faces = TYPE['styles']['persian' if persian else 'night' if night else 'ground' if ground else 'wikipedia']['names']
    if ground:
        # (GlobeView's groundSample: a small copy of the month's land, for the halos; by night the
        # halos are the faces' own, the land being one dark tone)
        small = np.asarray(Image.open(G / 'ground' / f'{month:02d}.jpg').convert('RGB').resize((512, 256), Image.BOX)).astype(np.float32)

        def halo_at(lon, lat):
            x = int(np.floor((lon + 180) / 360 * 512)) % 512
            y = min(max(int(np.floor((90 - lat) / 180 * 256)), 0), 255)
            return np.round(small[y, x] * 0.6 + np.array([0xF6, 0xF2, 0xE7]) * 0.4) / 255
        if night:
            halo_at = None
    k = reading(w, h)
    fonts = {}

    def font(face):
        key = (face['font'], face['size'])
        if key not in fonts:
            path = next(G / 'fonts' / (face['font'] + ext) for ext in ('.otf', '.ttf') if (G / 'fonts' / (face['font'] + ext)).exists())
            fonts[key] = ImageFont.truetype(str(path), int(round(face['size'] * k * scale)))
        return fonts[key]

    def tier(r):
        t = TYPE['countryTiers']
        return 'country1' if r <= t[0] else 'country2' if r <= t[1] else 'country3'
    draw = ImageDraw.Draw(img)
    placed = [tuple(v * scale for v in r) for r in keep_clear]
    halo = TYPE['halo'] * k * scale
    col = lambda c: tuple(int(255 * v) for v in c)

    def put(text, lon, lat, face, panel=False, mark=False, silhouette=False, sampled=False):
        la, lo = np.radians(lat), np.radians(lon)
        wv = np.array([np.cos(la) * np.sin(lo), np.sin(la), np.cos(la) * np.cos(lo)])
        vx, vy, vz = M.T @ wv
        if vz < 0.4:                                    # (the app keeps a shown name down to 0.3)
            return
        f = font(face)
        sx, sy = (cx + vx * R) * scale, (cy - vy * R) * scale
        spacing = face['tracking'] * face['size'] * k * scale
        widths = [draw.textlength(ch, font=f) for ch in text]
        tw = sum(widths) + spacing * (len(text) - 1)
        bb = draw.textbbox((0, 0), text, font=f)
        th = bb[3] - bb[1]
        mk = face['size'] * 0.9 * k * scale            # a capital's rosette, left of its name, on the city
        pad = (5 if panel else 0) * k * scale + halo + scale
        x0 = sx - (halo + scale + mk / 2) if mark else sx - tw / 2 - pad
        box = (x0, sy - th / 2 - pad, x0 + tw + (mk * 1.33 if mark else 0) + 2 * pad, sy + th / 2 + pad)
        far = max(np.hypot(bx - cx * scale, by - cy * scale) for bx in (box[0], box[2]) for by in (box[1], box[3]))
        if far >= R * scale * 0.99 or box[0] < 0 or box[1] < 0 or box[2] > w * scale or box[3] > h * scale:
            return
        gap = TYPE['gap'] * face['size'] * k * scale / 2
        g = (box[0] - gap, box[1] - gap, box[2] + gap, box[3] + gap)
        if any(not (g[2] < b[0] or g[0] > b[2] or g[3] < b[1] or g[1] > b[3]) for b in placed):
            return
        placed.append(g)
        ink, ground = hexc(face['ink']), halo_at(lon, lat) if sampled and halo_at else hexc(face['halo'])
        if panel:                                       # a quiet text panel, ruled in gold and lapis
            draw.rectangle(box, fill=col(ground), outline=col(P_GOLD), width=max(1, int(1.2 * scale)))
            i = 2.2 * scale
            draw.rectangle((box[0] + i, box[1] + i, box[2] - i, box[3] - i), outline=col(P_LAPIS), width=max(1, int(0.5 * scale)))
        if mark:
            rs = picture('rosette.png', (mk, mk), stand_in_rosette)
            rs = np.concatenate([rs[..., :3] / np.maximum(rs[..., 3:], 1e-4), rs[..., 3:]], -1)   # straight again for PIL
            ri = Image.fromarray((np.clip(rs, 0, 1) * 255).astype(np.uint8), 'RGBA')
            img.paste(ri, (int(sx - ri.size[0] / 2), int(sy - ri.size[1] / 2)), ri)
        x = box[0] + pad + (mk * 1.33 if mark else 0)
        ty = sy - th / 2 - bb[1]
        for passes in ((['halo', 'ink'] if not panel else ['ink']) if not silhouette else ['silhouette', 'ink']):
            xx = x
            for ch, cw in zip(text, widths):
                if passes == 'halo':
                    draw.text((xx, ty), ch, font=f, fill=col(ground), stroke_width=int(round(halo)), stroke_fill=col(ground))
                elif passes == 'silhouette':
                    draw.text((xx, ty), ch, font=f, fill=col(ground), stroke_width=int(round(2.2 * k * scale)), stroke_fill=col(ground))
                else:
                    draw.text((xx, ty), ch, font=f, fill=col(ink))
                xx += cw + spacing

    level = np.log2(2 * np.pi * R / 256) + 1.2      # the web-map zoom the globe is drawn at
    for i, c in enumerate(data['countries']):
        if i + 1 == selected:
            put(short(c['n']).upper(), *c['c'], faces['selected'], panel=persian, silhouette=not persian)
    # the ground's zooms, whatever the screen (GlobeView's makeLabels): seas and the great features from
    # x1.5, the lesser from x2.5 and x4, countries from x3.2, capitals from x5
    sea_z, country_z, capital_z = (1.5, 3.2, 5) if ground else (0, 0, 0)
    for s in sorted(data['seas'], key=lambda s: (not s['ocean'], s['r'])):
        if (s['ocean'] or s['r'] <= level - 2.5) and zoom >= sea_z:
            put(s['n'], *s['c'], faces['ocean' if s['ocean'] else 'sea'])
    if ground:
        for r in json.loads((G / 'regions.json').read_text())['regions']:
            if level >= 4 + (r['r'] - 1) * 0.9 and zoom >= (1.5 if r['r'] <= 1 else 2.5 if r['r'] <= 2 else 4):
                put(r['n'].upper(), *r['c'], faces['region'], sampled=True)
    for i, c in sorted(enumerate(data['countries']), key=lambda ic: (ic[1]['min'], ic[1]['r'])):
        if c['min'] <= level - 0.5 and i + 1 != selected and zoom >= country_z:
            put(short(c['n']).upper(), *c['c'], faces[tier(c['r'])], sampled=ground)
    for c in sorted(data['capitals'], key=lambda c: c.get('r') or 9):
        if level >= 5.6 + (c.get('r') or 5) * 0.25 and zoom >= capital_z:
            put(c['n'] if persian else '\u00b7 ' + c['n'], *c['c'], faces['capital'], mark=persian, sampled=ground)


CAPTION_WIDTH = 560                                    # the caption's widest, in points (a readable line)


def caption_box(w, h):
    """Where the Persian caption sits, in points (x0, y0, x1, y1): at the foot, at most CAPTION_WIDTH
    wide, centred."""
    inset = 5
    pw = min(w - 2 * PAGE_ROOM, CAPTION_WIDTH)              # (inside the page frame at the sides too)
    x0 = (w - pw) / 2
    head_h = (pw - 2 * inset) / 8
    cap = TYPE['styles']['persian']['caption']
    body_h = 12 + cap['eyebrow']['size'] * 1.3 + 6 + cap['text']['size'] * 1.3 + 12
    y1 = h - safe_insets(w, h)[1] - PAGE_ROOM             # (inside the page's frame, when it shows)
    return x0, y1 - (2 * inset + head_h + body_h), x0 + pw, y1


def safe_insets(w, h):
    """The safe area's insets in points (top, bottom, left, right): an iPad's or an iPhone's."""
    return (24, 20, 0, 0) if min(w, h) >= 700 else (59, 34, 0, 0)


def page_alpha(zoom):
    """How far the page's frame has come in (GlobeView's pageFrame): as the globe's own frame leaves."""
    return float(smoothstep(1.3, 2.0, zoom))


def buttons_box(w, h):
    """The style and film buttons, top right, inside the page's frame, in points (AtlasScreen's)."""
    return w - PAGE_ROOM - 115, 0, w, safe_insets(w, h)[0] + PAGE_ROOM + 48


def persian_caption(img, w, h, scale, selected):
    """Roughly the app's Persian caption (GlobeScreen's PersianCaption): the headpiece with the name in
    its cartouche, over a ruled text panel, at the foot of the screen."""
    data = json.loads((G / 'globe.json').read_text())
    if selected:
        c = data['countries'][selected - 1]
        title, eyebrow = cartouche(c['n']), (c.get('sub') or c.get('cont') or '')
        facts = []
        if c.get('cap'):
            facts.append('Capital ' + c['cap'])
        if c.get('pop'):
            facts.append(f"{c['pop'] / 1e6:.0f} million people" if c['pop'] >= 1e7 else f"{c['pop'] / 1e6:.1f} million people")
        text = ' \u00b7 '.join(facts)
    else:
        title, eyebrow, text = 'The whole world', 'The globe', 'Spin it, pinch it, tap a country.'
    f = G / 'fonts'
    S = lambda v: int(round(v * scale))
    tf = ImageFont.truetype(str(f / 'LibertinusSerif-Semibold.otf'), S(22))
    cap = TYPE['styles']['persian']['caption']
    ef = ImageFont.truetype(str(f / (cap['eyebrow']['font'] + '.otf')), S(cap['eyebrow']['size']))
    xf = ImageFont.truetype(str(f / (cap['text']['font'] + '.otf')), S(cap['text']['size']))
    col = lambda k: tuple(int(255 * v) for v in k)
    inset = 5
    x0, y0, x1, y1 = caption_box(w, h)
    inner = x1 - x0 - 2 * inset
    head = picture('headpiece.png', (inner * scale, inner / 8 * scale), lambda: Image.new('RGBA', (2048, 256), col(P_LAPIS) + (255,)))
    head_h = inner / 8
    d = ImageDraw.Draw(img)
    d.rectangle((S(x0), S(y0), S(x1), S(y1)), fill=col(hexc(TYPE['styles']['persian']['captionGround'])))
    d.rectangle((S(x0 + 1), S(y0 + 1), S(x1 - 1), S(y1 - 1)), outline=col(P_GOLD), width=S(2))
    d.rectangle((S(x0 + 3.2), S(y0 + 3.2), S(x1 - 3.2), S(y1 - 3.2)), outline=col(P_PINK), width=max(1, S(0.8)))
    d.rectangle((S(x0 + inset), S(y0 + inset), S(x1 - inset), S(y1 - inset)), outline=col(P_LAPIS), width=max(1, S(0.5)))
    hp = np.concatenate([head[..., :3] / np.maximum(head[..., 3:], 1e-4), head[..., 3:]], -1)
    hi = Image.fromarray((np.clip(hp, 0, 1) * 255).astype(np.uint8), 'RGBA')
    img.paste(hi, (S(x0 + inset), S(y0 + inset)), hi)
    # the name in the cartouche (its face and box: type.json), shrunk to fit, to 0.55 of its size
    cart = TYPE['styles']['persian']['cartouche']
    bx, by, bw, bh = cart['box']
    size = cart['size']
    while True:
        tf = ImageFont.truetype(str(f / (cart['font'] + '.otf')), S(size))
        bb = d.textbbox((0, 0), title, font=tf)
        if bb[2] - bb[0] <= bw * inner * scale or size <= cart['size'] * 0.55:
            break
        size -= 0.5
    cxp, cyp = S(x0 + inset + inner * (bx + bw / 2)), S(y0 + inset + head_h * (by + bh / 2))
    d.text((cxp - (bb[2] - bb[0]) / 2, cyp - (bb[3] + bb[1]) / 2), title, font=tf, fill=col(P_INK))
    ty = y0 + inset + head_h + 12
    # the first line a rubric, in the manuscript's vermilion, set large as its rubrics are
    d.text((S(x0 + inset + 16), S(ty)), eyebrow, font=ef, fill=col(hexc(cap['eyebrow']['color'])))
    d.text((S(x0 + inset + 16), S(ty + cap['eyebrow']['size'] * 1.3 + 6)), text, font=xf, fill=col(hexc(cap['text']['color'])))


def stand_in_rosette():
    img = Image.new('RGBA', (96, 96), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    for k in range(8):
        t = k * np.pi / 4
        x, y = 48 + 24 * np.cos(t), 48 + 24 * np.sin(t)
        d.ellipse((x - 14, y - 14, x + 14, y + 14), fill=(239, 231, 218, 255), outline=(168, 137, 79, 255), width=3)
    d.ellipse((40, 40, 56, 56), fill=(168, 137, 79, 255))
    return img


def main():
    a = argparse.ArgumentParser()
    a.add_argument('out')
    a.add_argument('--lon', type=float, default=-83)
    a.add_argument('--lat', type=float, default=32)
    a.add_argument('--zoom', type=float, default=1)
    a.add_argument('--select', default='')
    a.add_argument('--scale', type=float, default=1)
    a.add_argument('--size', default='1032x1376', help='points: 1032x1376 the 13-inch iPad (the default), 402x874 an iPhone')
    a.add_argument('--no-labels', action='store_true')
    a.add_argument('--historical-reference', action='store_true', help='modern geographic reference with political boundaries and labels suppressed')
    a.add_argument('--style', default='wikipedia', choices=('wikipedia', 'persian', 'ground', 'night'))
    a.add_argument('--month', type=int, default=None, help='the ground style\'s month, 1-12 (default: this month)')
    a.add_argument('--night-time', type=float, default=0.0, help="the night's seconds (its lights shimmer by them)")
    o = a.parse_args()
    w, h = map(int, o.size.split('x'))
    names = [c['n'] for c in json.loads((G / 'globe.json').read_text())['countries']]
    selected = names.index(o.select) + 1 if o.select else 0
    import datetime
    month = o.month or datetime.date.today().month
    ground, night = o.style in ('ground', 'night'), o.style == 'night'
    img, geom = render(w, h, o.scale, o.lon, o.lat, o.zoom, selected, o.style == 'persian', o.historical_reference,
                       ground, month, night, o.night_time)
    if not o.no_labels and not o.historical_reference:
        clear = [buttons_box(w, h)] + ([caption_box(w, h)] if o.style == 'persian' else [])
        if o.style == 'persian' and page_alpha(o.zoom) > 0:
            clear += page_strips(w, h, safe_insets(w, h))
        labels(img, w, h, o.scale, geom, o.zoom, selected, o.style == 'persian', clear, ground, month, night)
        if o.style == 'persian':
            persian_caption(img, w, h, o.scale, selected)
    img.save(o.out)


if __name__ == '__main__':
    main()
