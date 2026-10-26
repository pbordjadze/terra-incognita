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


def layout(w, h):
    """The globe's centre and resting radius in points (GlobeView's `place`)."""
    return w / 2, h * 0.43, 0.455 * min(w, h * 0.55)


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


def render(w, h, scale, lon0, lat0, zoom, selected):
    geo = np.asarray(Image.open(G / 'geo.png'))[..., :3]
    ter = np.asarray(Image.open(G / 'terrain.png'))[..., :3]
    ids = np.asarray(Image.open(G / 'ids.png'))
    W, H = int(w * scale), int(h * scale)
    cx, cy, r0 = layout(w, h)
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
    # a line's distance changes about a texel a texel; where its sign flips (far from any line) it
    # jumps sixteen: no line there
    texels = fwidth(u * 4096) + fwidth(v * 2048)
    lined = lambda f: (fwidth(f) < 2.5 * texels) & (np.abs(f) < 7)
    landa = smoothstep(-0.5, 0.5, d / np.maximum(fwidth(d), 1e-5))
    k = np.clip(zoom, 1, 8)
    # the sea: deeper is a little bluer, with a faint watercolour mottle and lines along the coasts
    depth = np.clip(-np.minimum(height, 0) * 87 / 5500, 0, 1) ** 0.6
    sea = mix(OCEAN_SHALLOW, OCEAN_DEEP, depth)
    mott = vnoise(u * 90, v * 45) * 0.6 + vnoise(u * 260, v * 130) * 0.4
    sea = sea * (0.985 + 0.03 * mott)[..., None]
    for at, a in ((-6, 0.15), (-13, 0.08)):
        sea = mix(sea, COAST, a * line(fine - at, 0.7 * scale) * ((fine < -2) & (fine > -15.5)))
    # the land: cream, warmer up high, the relief shaded from the north-west
    hm = np.maximum(height, 0) * 50
    land = mix(LAND, HIGHLAND, 0.85 * smoothstep(250, 3200, hm))
    land = land * (1 + 0.30 * hill)[..., None]
    land = mix(land, RIVER, 0.75 * line(river, 0.65 * scale) * smoothstep(1.2, 2.2, k) * lined(river))
    # the selected country over its rivers, as a locator map has it; its borders drawn on top
    if selected:
        m = sample((ids == selected).astype(np.float32), u, v)[..., 0]   # the shader: gather, then bilinear
        land = mix(land, LOCATOR * (1 + 0.12 * hill)[..., None], 0.94 * m)
    land = mix(land, BORDER, 0.8 * line(border, 0.7 * scale) * lined(border))
    col = mix(sea, land, landa)
    col = mix(col, COAST, line(d, 0.9 * scale * min(k ** 0.25, 1.4)))
    # the graticule, every 15 degrees
    glat, glon = np.degrees(lat), np.degrees(lon)
    fwlat = fwidth(glat)
    fwlon = np.minimum(fwidth(glon), fwidth(glon % 360))     # the antimeridian's jump, or 0's
    gl = np.maximum(line(((glat + 7.5) % 15) - 7.5, 0.6 * scale, fwlat),
                    line(((glon + 7.5) % 15) - 7.5, 0.6 * scale, fwlon) * (np.abs(glat) < 75))
    col = mix(col, COAST, gl * np.where(landa > 0.5, 0.10, 0.16))
    # roundness: the limb a little shaded; the rim drawn
    col = col * (1 - 0.07 * (1 - z) ** 2)[..., None]
    col = mix(col, COAST, line((rr - 1) * R, 1.2 * scale))
    edge = smoothstep(1 + 0.5 / R, 1 - 0.5 / R, rr)
    out = np.where(inside[..., None], mix(page, col, edge), page)
    return Image.fromarray((np.clip(out, 0, 1) * 255).astype(np.uint8), 'RGB'), (cx, cy, r0 * zoom, M)


def short(name):
    return {'United States of America': 'United States'}.get(name, name)


def labels(img, w, h, scale, geom, zoom, selected):
    """Roughly the app's label layer: the selected country, oceans and seas, then countries, in that
    order of claim, none overlapping."""
    cx, cy, R, M = geom
    data = json.loads((G / 'globe.json').read_text())
    f = G / 'fonts'
    cap = ImageFont.truetype(str(f / 'LibertinusSerif-Regular.otf'), int(9.5 * scale))
    sel = ImageFont.truetype(str(f / 'LibertinusSerif-Semibold.otf'), int(11 * scale))
    sea = ImageFont.truetype(str(f / 'UncialAntiqua-Regular.ttf'), int(8.5 * scale))
    draw = ImageDraw.Draw(img)
    placed = []

    def put(text, lon, lat, font, colour, tracking):
        la, lo = np.radians(lat), np.radians(lon)
        wv = np.array([np.cos(la) * np.sin(lo), np.sin(la), np.cos(la) * np.cos(lo)])
        vx, vy, vz = M.T @ wv
        if vz < 0.25:
            return
        sx, sy = (cx + vx * R) * scale, (cy - vy * R) * scale
        text = ' '.join(text) if tracking else text
        bb = draw.textbbox((0, 0), text, font=font)
        tw, th = bb[2] - bb[0], bb[3] - bb[1]
        box = (sx - tw / 2 - 3, sy - th / 2 - 3, sx + tw / 2 + 3, sy + th / 2 + 3)
        if any(not (box[2] < b[0] or box[0] > b[2] or box[3] < b[1] or box[1] > b[3]) for b in placed):
            return
        placed.append(box)
        a = min(1, (vz - 0.25) / 0.25)
        c = tuple(int(255 * (PAPER[i] * (1 - a) + colour[i] * a)) for i in range(3))
        draw.text((sx - tw / 2, sy - th / 2 - bb[1]), text, font=font, fill=c)

    level = np.log2(2 * np.pi * R / 256) + 1.2      # the web-map zoom the globe is drawn at
    for i, c in enumerate(data['countries']):
        if i + 1 == selected:
            put(short(c['n']).upper(), *c['c'], sel, hexc('#FFF7EA'), False)
    for s in sorted(data['seas'], key=lambda s: (not s['ocean'], s['r'])):
        if s['ocean'] or s['r'] <= level - 2.5:
            put(s['n'], *s['c'], sea, hexc('#4A8DBB'), False)
    for i, c in sorted(enumerate(data['countries']), key=lambda ic: (ic[1]['min'], ic[1]['r'])):
        if c['min'] <= level - 0.5 and i + 1 != selected:
            put(short(c['n']).upper(), *c['c'], cap, hexc('#5F5A51'), False)


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
    o = a.parse_args()
    w, h = map(int, o.size.split('x'))
    names = [c['n'] for c in json.loads((G / 'globe.json').read_text())['countries']]
    selected = names.index(o.select) + 1 if o.select else 0
    img, geom = render(w, h, o.scale, o.lon, o.lat, o.zoom, selected)
    if not o.no_labels:
        labels(img, w, h, o.scale, geom, o.zoom, selected)
    img.save(o.out)


if __name__ == '__main__':
    main()
