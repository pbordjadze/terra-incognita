"""Draws the globe as the app's shader does (App/TerraIncognita/Globe/Globe.metal mirrors this, line
for line), so its look can be judged without a phone.

    python3 tools/globe/preview.py out.png [--lon -83 --lat 40 --zoom 1 --select Italy --scale 2]

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

# The Persian style: a study of the illuminated opening and the landscapes of LACMA's Khamsa of
# Nizami (M.85.237.16, Tabriz, 1529), cleaned of the varnish's yellow. The globe is the quiet text
# panel; its frame is the manuscript's: gold, pink, green and lapis rulings around a lapis band.
P_PAPER = hexc('#F3E9D2')
P_SEA = hexc('#E7EBF1')
P_SEA_DEEP = hexc('#D3DAE9')
P_COAST = hexc('#4A5794')
P_MEADOW = hexc('#ECE7CE')       # lowland: the meadows' pale sage, on cream
P_UPLAND = hexc('#DCDCC0')
P_ROCK = hexc('#D5C6D0')         # highland: the lavender rocks
P_PEAK = hexc('#EDE5EA')
P_SHADE = hexc('#8C7C9C')
P_BORDER = hexc('#A8635F')
P_RIVER = hexc('#5C6EAE')
P_LAPIS = hexc('#4E5B9B')
P_GOLD = hexc('#A8894F')
P_GOLD_LIGHT = hexc('#C9AE6A')
P_PINK = hexc('#D59A86')
P_GREEN = hexc('#6F9A7E')
P_SALMON = hexc('#C7735A')
P_INK = hexc('#2A2119')
P_SEL = hexc('#D6BE7E')
SEL_ALPHA = 0.85


def layout(w, h, persian=False):
    """The globe's centre and resting radius in points (GlobeView's `place`); a little smaller in the
    Persian style, whose frame needs the room."""
    return w / 2, h * 0.43, (0.42 if persian else 0.455) * min(w, h * 0.55)


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


# The Persian frame, outward from the globe's edge, in units of its radius (so it grows with the
# globe): the rulings of a manuscript's frame (jadval) around the ornamental band, then on the
# paper a blue ruled circle with blue-ink finials standing on it, as round a medallion.
P_RULES = (                       # (from, to, colour)
    (1.000, 1.0035, P_INK), (1.0035, 1.0115, P_GOLD), (1.0115, 1.0155, P_PINK),
    (1.0705, 1.0745, P_PINK), (1.0745, 1.0805, P_GREEN), (1.0805, 1.0905, P_GOLD), (1.0905, 1.093, P_INK),
    (1.100, 1.1025, P_LAPIS))
P_BAND = (1.0155, 1.0705, 20)     # inner, outer, repeats round
P_FLEURON = (1.1025, 0.045, 48)   # base, height, how many


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


def persian_page(px, py, x, y, rr, R, W, H, grain):
    page = P_PAPER * (0.985 + 0.03 * grain)[..., None]
    vig = np.hypot(px / W - 0.5, py / H - 0.48)
    page = page * (1 - 0.06 * smoothstep(0.35, 0.8, vig))[..., None]
    sh = np.exp(-np.maximum(np.hypot(x, y + 0.03) - 1.10, 0) ** 2 / 0.003) * (rr > 1.08)
    page = mix(page, SHADOW, 0.10 * sh)
    def ring(a, b):
        return smoothstep(a - 0.5 / R, a + 0.5 / R, rr) * (1 - smoothstep(b - 0.5 / R, b + 0.5 / R, rr))
    for a, b, c in P_RULES:
        page = mix(page, c, ring(a, b))
    theta = np.arctan2(x, y) % (2 * np.pi)                   # clockwise from the top
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


def persian_globe(scale, k, u, v, d, fine, border, hill, hm, river, depth, mott, gl, m, landa, x, y, z, rr, R, lined):
    # the sea: a pale lapis wash, its coasts ruled like a panel, a gold hair close in and a lapis one out
    sea = mix(P_SEA, P_SEA_DEEP, depth)
    sea = sea * (0.985 + 0.03 * mott)[..., None]
    near = smoothstep(1.4, 2.6, k)                           # the rulings come with a closer look
    sea = mix(sea, P_GOLD, 0.6 * near * line(fine + 4.5, 0.6 * scale) * ((fine < -1.5) & (fine > -8)))
    sea = mix(sea, P_COAST, 0.14 * near * line(fine + 11, 0.6 * scale) * ((fine < -8) & (fine > -15.5)))
    # the land: the meadows' sage low, the rocks' lavender high, their shadows violet
    land = mix(P_MEADOW, P_UPLAND, smoothstep(200, 1100, hm))
    land = mix(land, P_ROCK, smoothstep(900, 2600, hm))
    land = mix(land, P_PEAK, 0.7 * smoothstep(5000, 6300, hm))
    land = mix(land, P_SHADE, 0.36 * np.clip(-hill, 0, 1))
    land = mix(land, np.ones(3, np.float32), 0.30 * np.clip(hill, 0, 1))
    # closer in, the heights' contours every 500 m, pale like the rocks' highlights
    # (lobed, as the painters drew rocks: the contours of the height plus a little cellular swell)
    hc = hm / 600 + 0.3 * worley(u * 4096 / 4.5, v * 2048 / 4.5)
    contour = line(((hc + 0.5) % 1) - 0.5, 0.55 * scale, fwidth(hc)) * smoothstep(800, 1300, hm) * smoothstep(1.6, 2.6, k)
    land = mix(land, P_PEAK, 0.7 * contour)
    land = mix(land, P_SHADE, 0.22 * line(((hc + 0.42) % 1) - 0.5, 0.55 * scale, fwidth(hc)) * smoothstep(800, 1300, hm) * smoothstep(1.6, 2.6, k))
    land = mix(land, P_RIVER, 0.75 * line(river, 0.65 * scale) * smoothstep(1.2, 2.2, k) * lined(river))
    # the chosen country gilded, like the opening's cartouche, and edged in its pink
    # (the gold catches a light from the upper left as the globe turns it past)
    sheen = np.maximum(x * -0.42 + y * 0.52 + z * 0.744, 0) ** 10
    gild = P_SEL * (1 + 0.20 * hill + 0.05 * (vnoise(u * 2400, v * 1200) - 0.5) + 0.22 * sheen)[..., None]
    land = mix(land, gild, SEL_ALPHA * m)
    bl = line(border, 0.7 * scale) * lined(border)
    land = mix(land, P_BORDER, 0.75 * bl)
    land = mix(land, hexc('#C0705E'), line(border, 1.5 * scale) * lined(border) * (m > 0.04))
    col = mix(sea, land, landa)
    col = mix(col, P_COAST, line(d, 0.75 * scale * min(k ** 0.3, 1.3)))
    col = mix(col, P_GOLD, gl * np.where(landa > 0.5, 0.14, 0.22))
    col = col * (1 - 0.06 * (1 - z) ** 2)[..., None]
    return col


def render(w, h, scale, lon0, lat0, zoom, selected, persian=False):
    geo = np.asarray(Image.open(G / 'geo.png'))[..., :3]
    ter = np.asarray(Image.open(G / 'terrain.png'))[..., :3]
    ids = np.asarray(Image.open(G / 'ids.png'))
    W, H = int(w * scale), int(h * scale)
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
    ring_on = 1 - smoothstep(1.25, 1.7, zoom)
    ringpx = (rr - 1) * R / scale                   # points outside the globe's edge
    gold = np.zeros_like(rr)
    for at, wd in ((6.5, 1.1), (10.0, 0.7)):
        gold = np.maximum(gold, line(ringpx - at, wd * scale))
    ang = np.degrees(np.arctan2(x, y)) % 360
    tick = line(((ang + 5) % 10) - 5, 0.9 * scale, np.degrees(1 / np.maximum(rr * R, 1))) * ((ringpx > 7) & (ringpx < 9.6))
    dots = np.zeros_like(rr)
    for a in range(0, 360, 90):                     # the four quarters: three little dots, Kells-fashion
        for da, dr in ((0, 8.25), (-1.5, 8.25), (1.5, 8.25)):
            t = np.radians(a + da)
            dx = cx * scale + np.sin(t) * (R + dr * scale) - px
            dy = cy * scale - np.cos(t) * (R + dr * scale) - py
            dots = np.maximum(dots, 1 - smoothstep(1.1 * scale, 1.9 * scale, np.hypot(dx, dy)))
    page = mix(page, GOLD, np.clip(np.maximum(np.maximum(gold, tick * 0.8), dots), 0, 1) * ring_on)
    if persian:
        page = persian_page(px, py, x, y, rr, R, W, H, grain)

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
    gl = np.maximum(line(((glat + 7.5) % 15) - 7.5, 0.6 * scale, fwlat),
                    line(((glon + 7.5) % 15) - 7.5, 0.6 * scale, fwlon) * (np.abs(glat) < 75))
    # the selected country (the shader: gather, then bilinear)
    m = sample((ids == selected).astype(np.float32), u, v)[..., 0] if selected else np.zeros_like(u)
    if persian:
        col = persian_globe(scale, k, u, v, d, fine, border, hill, hm, river, depth, mott, gl, m, landa, x, y, z, rr, R, lined)
    else:
        # the sea: deeper is a little bluer, with a faint watercolour mottle and lines along the coasts
        sea = mix(OCEAN_SHALLOW, OCEAN_DEEP, depth)
        sea = sea * (0.985 + 0.03 * mott)[..., None]
        for at, a in ((-6, 0.15), (-13, 0.08)):
            sea = mix(sea, COAST, a * line(fine - at, 0.7 * scale) * ((fine < -2) & (fine > -15.5)))
        # the land: cream, warmer up high, the relief shaded from the north-west
        land = mix(LAND, HIGHLAND, 0.85 * smoothstep(250, 3200, hm))
        land = land * (1 + 0.30 * hill)[..., None]
        land = mix(land, RIVER, 0.75 * line(river, 0.65 * scale) * smoothstep(1.2, 2.2, k) * lined(river))
        # the selected country over its rivers, as a locator map has it; its borders drawn on top
        land = mix(land, LOCATOR * (1 + 0.12 * hill)[..., None], 0.94 * m)
        land = mix(land, BORDER, 0.8 * line(border, 0.7 * scale) * lined(border))
        col = mix(sea, land, landa)
        col = mix(col, COAST, line(d, 0.9 * scale * min(k ** 0.25, 1.4)))
        # the graticule, every 15 degrees
        col = mix(col, COAST, gl * np.where(landa > 0.5, 0.10, 0.16))
        # roundness: the limb a little shaded; the rim drawn
        col = col * (1 - 0.07 * (1 - z) ** 2)[..., None]
        col = mix(col, COAST, line((rr - 1) * R, 1.2 * scale))
    edge = smoothstep(1 + 0.5 / R, 1 - 0.5 / R, rr)
    out = np.where(inside[..., None], mix(page, col, edge), page)
    return Image.fromarray((np.clip(out, 0, 1) * 255).astype(np.uint8), 'RGB'), (cx, cy, r0 * zoom, M)


def short(name):
    return {'United States of America': 'United States'}.get(name, name)


def labels(img, w, h, scale, geom, zoom, selected, persian=False):
    """Roughly the app's label layer: the selected country, oceans and seas, then countries (and in the
    Persian style capitals), in that order of claim, none overlapping."""
    cx, cy, R, M = geom
    data = json.loads((G / 'globe.json').read_text())
    f = G / 'fonts'
    cap = ImageFont.truetype(str(f / 'LibertinusSerif-Regular.otf'), int(9.5 * scale))
    sel = ImageFont.truetype(str(f / 'LibertinusSerif-Semibold.otf'), int(11 * scale))
    sea = ImageFont.truetype(str(f / 'UncialAntiqua-Regular.ttf'), int(8.5 * scale))
    if persian:
        ocean = ImageFont.truetype(str(f / 'LibertinusSerif-Italic.otf'), int(11 * scale))
        sea = ImageFont.truetype(str(f / 'LibertinusSerif-Italic.otf'), int(9 * scale))
        town = ImageFont.truetype(str(f / 'LibertinusSerif-Italic.otf'), int(9.5 * scale))
    draw = ImageDraw.Draw(img)
    placed = []
    paper = P_PAPER if persian else PAPER

    def put(text, lon, lat, font, colour, tracking, panel=False, mark=False):
        la, lo = np.radians(lat), np.radians(lon)
        wv = np.array([np.cos(la) * np.sin(lo), np.sin(la), np.cos(la) * np.cos(lo)])
        vx, vy, vz = M.T @ wv
        if vz < 0.25:
            return
        sx, sy = (cx + vx * R) * scale, (cy - vy * R) * scale
        text = '\u2009'.join(text) if tracking else text
        bb = draw.textbbox((0, 0), text, font=font)
        tw, th = bb[2] - bb[0], bb[3] - bb[1]
        mk = 10 * scale if mark else 0                  # the capital's rosette, left of its name
        box = (sx - tw / 2 - 3 - mk, sy - th / 2 - 3, sx + tw / 2 + 3, sy + th / 2 + 3)
        if panel:
            box = (box[0] - 5 * scale, box[1] - 3 * scale, box[2] + 5 * scale, box[3] + 3 * scale)
        if any(not (box[2] < b[0] or box[0] > b[2] or box[3] < b[1] or box[1] > b[3]) for b in placed):
            return
        placed.append(box)
        a = min(1, (vz - 0.25) / 0.25)
        c = tuple(int(255 * (paper[i] * (1 - a) + colour[i] * a)) for i in range(3))
        if panel:                                       # a quiet text panel, ruled in gold and lapis
            col = lambda k: tuple(int(255 * v) for v in k)
            draw.rectangle(box, fill=col(hexc('#F7F0DF')), outline=col(P_GOLD), width=max(1, int(1.2 * scale)))
            i = 2.2 * scale
            draw.rectangle((box[0] + i, box[1] + i, box[2] - i, box[3] - i), outline=col(P_LAPIS), width=max(1, int(0.5 * scale)))
        if mark:
            rs = picture('rosette.png', (9 * scale, 9 * scale), stand_in_rosette)
            rs = np.concatenate([rs[..., :3] / np.maximum(rs[..., 3:], 1e-4), rs[..., 3:]], -1)   # straight again for PIL
            ri = Image.fromarray((np.clip(rs, 0, 1) * 255).astype(np.uint8), 'RGBA')
            img.paste(ri, (int(sx - tw / 2 - mk), int(sy - ri.size[1] / 2)), ri)
        draw.text((sx - tw / 2, sy - th / 2 - bb[1]), text, font=font, fill=c)

    level = np.log2(2 * np.pi * R / 256) + 1.2      # the web-map zoom the globe is drawn at
    for i, c in enumerate(data['countries']):
        if i + 1 == selected:
            if persian:
                put(short(c['n']).upper(), *c['c'], sel, P_INK, False, panel=True)
            else:
                put(short(c['n']).upper(), *c['c'], sel, hexc('#FFF7EA'), False)
    for s in sorted(data['seas'], key=lambda s: (not s['ocean'], s['r'])):
        if s['ocean'] or s['r'] <= level - 2.5:
            if persian:
                put(s['n'], *s['c'], ocean if s['ocean'] else sea, P_LAPIS, s['ocean'])
            else:
                put(s['n'], *s['c'], sea, hexc('#4A8DBB'), False)
    for i, c in sorted(enumerate(data['countries']), key=lambda ic: (ic[1]['min'], ic[1]['r'])):
        if c['min'] <= level - 0.5 and i + 1 != selected:
            put(short(c['n']).upper(), *c['c'], cap, hexc('#4A3D2E') if persian else hexc('#5F5A51'), False)
    if persian:
        for c in sorted(data['capitals'], key=lambda c: c.get('r') or 9):
            if level >= 5.6 + (c.get('r') or 5) * 0.25:
                put(c['n'], *c['c'], town, hexc('#4A3D2E'), False, mark=True)


def persian_caption(img, w, h, scale, selected):
    """Roughly the app's Persian caption (GlobeScreen's PersianCaption): the headpiece with the name in
    its cartouche, over a ruled text panel, at the foot of the screen."""
    data = json.loads((G / 'globe.json').read_text())
    if selected:
        c = data['countries'][selected - 1]
        title, eyebrow = c.get('long') or c['n'], (c.get('sub') or c.get('cont') or '')
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
    ef = ImageFont.truetype(str(f / 'LibertinusSerif-Regular.otf'), S(12.5))
    xf = ImageFont.truetype(str(f / 'LibertinusSerif-Italic.otf'), S(17))
    col = lambda k: tuple(int(255 * v) for v in k)
    inset = 5
    x0, x1 = 14, w - 14
    inner = x1 - x0 - 2 * inset
    head = picture('headpiece.png', (inner * scale, inner / 8 * scale), lambda: Image.new('RGBA', (2048, 256), col(P_LAPIS) + (255,)))
    head_h = inner / 8
    body_h = 12 + 12.5 * 1.3 + 6 + 17 * 1.3 + 12
    y1 = h - 34 - 10
    y0 = y1 - (2 * inset + head_h + body_h)
    d = ImageDraw.Draw(img)
    d.rectangle((S(x0), S(y0), S(x1), S(y1)), fill=col(hexc('#F7F0DF')))
    d.rectangle((S(x0 + 1), S(y0 + 1), S(x1 - 1), S(y1 - 1)), outline=col(P_GOLD), width=S(2))
    d.rectangle((S(x0 + 3.2), S(y0 + 3.2), S(x1 - 3.2), S(y1 - 3.2)), outline=col(P_PINK), width=max(1, S(0.8)))
    d.rectangle((S(x0 + inset), S(y0 + inset), S(x1 - inset), S(y1 - inset)), outline=col(P_LAPIS), width=max(1, S(0.5)))
    hp = np.concatenate([head[..., :3] / np.maximum(head[..., 3:], 1e-4), head[..., 3:]], -1)
    hi = Image.fromarray((np.clip(hp, 0, 1) * 255).astype(np.uint8), 'RGBA')
    img.paste(hi, (S(x0 + inset), S(y0 + inset)), hi)
    # the name in the cartouche (its box: PersianCaption.cartouche), shrunk to fit
    bx, by, bw, bh = 0.29, 0.2, 0.42, 0.6
    size = 22.0
    while True:
        tf = ImageFont.truetype(str(f / 'LibertinusSerif-Semibold.otf'), S(size))
        bb = d.textbbox((0, 0), title, font=tf)
        if bb[2] - bb[0] <= bw * inner * scale or size <= 9:
            break
        size -= 0.5
    cxp, cyp = S(x0 + inset + inner * (bx + bw / 2)), S(y0 + inset + head_h * (by + bh / 2))
    d.text((cxp - (bb[2] - bb[0]) / 2, cyp - (bb[3] + bb[1]) / 2), title, font=tf, fill=col(P_INK))
    ty = y0 + inset + head_h + 12
    d.text((S(x0 + inset + 16), S(ty)), '\u2009'.join(eyebrow.upper()), font=ef, fill=col(P_LAPIS))
    d.text((S(x0 + inset + 16), S(ty + 12.5 * 1.3 + 6)), text, font=xf, fill=col(hexc('#5A4E40')))


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
    a.add_argument('--scale', type=float, default=2)
    a.add_argument('--size', default='402x874')
    a.add_argument('--no-labels', action='store_true')
    a.add_argument('--style', default='wikipedia', choices=('wikipedia', 'persian'))
    o = a.parse_args()
    w, h = map(int, o.size.split('x'))
    names = [c['n'] for c in json.loads((G / 'globe.json').read_text())['countries']]
    selected = names.index(o.select) + 1 if o.select else 0
    img, geom = render(w, h, o.scale, o.lon, o.lat, o.zoom, selected, o.style == 'persian')
    if not o.no_labels:
        labels(img, w, h, o.scale, geom, o.zoom, selected, o.style == 'persian')
        if o.style == 'persian':
            persian_caption(img, w, h, o.scale, selected)
    img.save(o.out)


if __name__ == '__main__':
    main()
