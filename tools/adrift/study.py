#!/usr/bin/env python3
"""Studies of Adrift's look over a real render of Terra's sea, before the wgpu build exists: the glass
float and its lamp, the ring that thickens into it, the wake, the maybes, the nearest drifter in silver,
what stays lit, and the date and credits as drawn things, drawn from the palette
(adrift/crates/adrift-core/src/look/palette.json) with numpy and PIL over a picture from Terra's Metal
stand-in, at the device's pixels.

    python3 tools/adrift/study.py all                 # every study into build/adrift/studies/ (renders the
                                                      # seas first with tools/metal/metal.py if they are missing)
    python3 tools/adrift/study.py float phone --at=-68,36,2.6 [--style night] [--days 45] [--maybes 2000]

A study is judged at device pixel size, not as a thumbnail: each picture is also cropped round the
float at 1:1 (`-1x.png`). The wake and the maybes are walks on Terra's own current sheet
(App/TerraIncognita/Ocean/OceanCurrents.png, the NOAA Global Drifter Program's climatology) with a
random eddy wander, held at the coast by Globe/geo.png; a stand-in for Truth's rules, enough to see
how a real wake bunches and spreads. The silver drifter is such a walk from another seed, standing in
for a real track. Reads only this machine.
"""
import argparse, json, math, subprocess, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / 'tools' / 'globe'))
import preview                                          # layout, rotation, reading: GlobeView's own
SPEC = json.loads((ROOT / 'adrift' / 'crates' / 'adrift-core' / 'src' / 'look' / 'palette.json').read_text())
FONTS = ROOT / 'Globe' / 'fonts'
BASE = ROOT / 'build' / 'adrift' / 'base'
OUT = ROOT / 'build' / 'adrift' / 'studies'
SIZES = {'phone': ((402, 874), 3), 'ipad': ((1032, 1376), 2)}
EARTH = 6371000.0


def col(style, name):
    c = SPEC['night' if style == 'night' else 'day'][name]
    return np.array([int(c['hex'][i:i + 2], 16) / 255 for i in (1, 3, 5)], np.float32), c.get('alpha', 1.0)


class Camera:
    """Points to pixels on the globe, as GlobeView places it (preview.layout, preview.rotation)."""

    def __init__(self, device, lon, lat, zoom):
        (self.w, self.h), self.px = SIZES[device]
        self.reading = preview.reading(self.w, self.h)
        cx, cy, r0 = preview.layout(self.w, self.h)
        self.cx, self.cy, self.R, self.zoom = cx, cy, r0 * zoom, zoom
        self.rot = preview.rotation(lon, lat)            # world from view

    def project(self, lon, lat):
        """A position's pixel (x, y) and its facing (z of the view's unit vector; hidden below 0)."""
        lo, la = np.radians(lon), np.radians(lat)
        g = np.stack([np.cos(la) * np.sin(lo), np.sin(la), np.cos(la) * np.cos(lo)], -1)
        v = g @ self.rot                                   # rot^T g
        x = (self.cx + v[..., 0] * self.R) * self.px
        y = (self.cy - v[..., 1] * self.R) * self.px
        return x, y, v[..., 2]


class Sea:
    """Terra's current sheet and coast, for the stand-in walks."""

    def __init__(self):
        self.cur = np.asarray(Image.open(ROOT / 'App' / 'TerraIncognita' / 'Ocean' / 'OceanCurrents.png'), np.float32)
        self.geo = np.asarray(Image.open(ROOT / 'Globe' / 'geo.png').convert('RGBA'), np.float32)

    def current(self, m, lon, lat):
        """Eastward and northward m/s at lon, lat for month m (0 = January), bilinear as the shader's."""
        u = (lon / 360.0 + 0.5) % 1.0
        v = 0.5 - lat / 180.0
        ox, oy = (m % 4) * 722, (m // 4) * 362
        px, py = ox + 1 + u * 720 - 0.5, oy + 1 + v * 360 - 0.5
        x0, y0 = np.floor(px).astype(int), np.floor(py).astype(int)
        fx, fy = px - x0, py - y0
        s = lambda xx, yy: self.cur[np.clip(yy, 0, 1085), np.clip(xx, 0, 2887)]
        c = (s(x0, y0) * (1 - fx)[..., None] * (1 - fy)[..., None] + s(x0 + 1, y0) * fx[..., None] * (1 - fy)[..., None]
             + s(x0, y0 + 1) * (1 - fx)[..., None] * fy[..., None] + s(x0 + 1, y0 + 1) * fx[..., None] * fy[..., None])
        sg = (c[..., :2] - 128.0) / 127.0
        return np.sign(sg) * sg * sg * 2.0, c[..., 2] / 255.0

    def land(self, lon, lat):
        u = (lon / 360.0 + 0.5) % 1.0
        v = 0.5 - lat / 180.0
        x = np.clip((u * self.geo.shape[1]).astype(int), 0, self.geo.shape[1] - 1)
        y = np.clip((v * self.geo.shape[0]).astype(int), 0, self.geo.shape[0] - 1)
        return self.geo[y, x, 0] > 128.0

    def walk(self, lon, lat, month, days, seeds, eddy=2500.0, dt=0.25):
        """Positions (days+1, n, 2) of n walks from lon, lat: the current, a Gaussian wander of eddy
        m^2/s, held where the coast is."""
        rng = np.random.default_rng(seeds)
        n = len(seeds)
        p = np.repeat(np.array([[lon, lat]], np.float64), n, 0)
        out = [p.copy()]
        steps = int(round(days / dt))
        sd = math.sqrt(2 * eddy * dt * 86400)
        for i in range(steps):
            (uv, _) = self.current(month, p[:, 0], p[:, 1])
            d = uv * dt * 86400 + rng.normal(0, sd, (n, 2))
            q = p.copy()
            q[:, 0] += np.degrees(d[:, 0] / (EARTH * np.cos(np.radians(p[:, 1]))))
            q[:, 1] += np.degrees(d[:, 1] / EARTH)
            held = self.land(q[:, 0], q[:, 1])
            p = np.where(held[:, None], p, q)
            if (i + 1) % int(round(1 / dt)) == 0:
                out.append(p.copy())
        return np.stack(out)


# ---- drawing on a float RGBA canvas, premultiplied, at the device's pixels

class Canvas:
    def __init__(self, base):
        self.im = np.asarray(base.convert('RGBA'), np.float32) / 255.0
        self.h, self.w = self.im.shape[:2]

    def over(self, x, y, reach, cover, colour, alpha):
        """Lays colour over the pixels within reach of (x, y): cover(dx, dy) gives each pixel's coverage."""
        x0, x1 = int(max(math.floor(x - reach), 0)), int(min(math.ceil(x + reach) + 1, self.w))
        y0, y1 = int(max(math.floor(y - reach), 0)), int(min(math.ceil(y + reach) + 1, self.h))
        if x1 <= x0 or y1 <= y0:
            return
        yy, xx = np.mgrid[y0:y1, x0:x1]
        a = np.clip(cover(xx + 0.5 - x, yy + 0.5 - y), 0, 1) * alpha
        self.im[y0:y1, x0:x1, :3] = colour * a[..., None] + self.im[y0:y1, x0:x1, :3] * (1 - a[..., None])
        self.im[y0:y1, x0:x1, 3] = a + self.im[y0:y1, x0:x1, 3] * (1 - a)

    def disc(self, x, y, r, colour, alpha=1.0):
        self.over(x, y, r + 1, lambda dx, dy: np.clip(r + 0.5 - np.hypot(dx, dy), 0, 1), colour, alpha)

    def ring(self, x, y, r, w, colour, alpha=1.0):
        self.over(x, y, r + w + 1, lambda dx, dy: np.clip(w / 2 + 0.5 - np.abs(np.hypot(dx, dy) - r), 0, 1), colour, alpha)

    def glow(self, x, y, r, colour, alpha):
        self.over(x, y, 3 * r, lambda dx, dy: np.exp(-(dx * dx + dy * dy) / (r * r)), colour, alpha)

    def line(self, x0, y0, x1, y1, w, colour, alpha=1.0):
        dx, dy = x1 - x0, y1 - y0
        L2 = dx * dx + dy * dy
        def cover(px, py):
            t = np.clip(((px + x0 - x0) * dx + (py + y0 - y0) * dy) / max(L2, 1e-6), 0, 1)
            return np.clip(w / 2 + 0.5 - np.hypot(px - t * dx, py - t * dy), 0, 1)
        cx, cy = x0, y0
        reach = math.hypot(dx, dy) + w + 1
        self.over(cx, cy, reach, lambda ex, ey: cover(ex, ey), colour, alpha)

    def text(self, x, y, s, size_pt, px, ink, halo, halo_a, font='LibertinusSerif-Regular', anchor='ms', tracking=0.0):
        """Words with their halo drawn in, 1.5 reading points each side (Terra's rule), through PIL."""
        f = ImageFont.truetype(str(FONTS / f'{font}.otf'), int(round(size_pt * px)))
        layer = Image.new('L', (self.w, self.h), 0)
        d = ImageDraw.Draw(layer)
        d.text((x, y), s, font=f, fill=255, anchor=anchor)
        ink_a = np.asarray(layer, np.float32) / 255.0
        hw = int(round(1.5 * px))
        halo_a_map = np.asarray(layer.filter(__import__('PIL.ImageFilter', fromlist=['MaxFilter']).MaxFilter(2 * hw + 1)), np.float32) / 255.0
        halo_a_map = np.asarray(Image.fromarray((halo_a_map * 255).astype(np.uint8)).filter(__import__('PIL.ImageFilter', fromlist=['GaussianBlur']).GaussianBlur(0.6)), np.float32) / 255.0
        for a, c, k in ((halo_a_map, halo, halo_a), (ink_a, ink, 1.0)):
            a = a * k
            self.im[..., :3] = c * a[..., None] + self.im[..., :3] * (1 - a[..., None])
            self.im[..., 3] = a + self.im[..., 3] * (1 - a)

    def save(self, path):
        Image.fromarray((np.clip(self.im, 0, 1) * 255 + 0.5).astype(np.uint8)).save(path)


# ---- the things drawn

def float_radius(cam):
    s = SPEC['sizes']['float_radius']
    t = (cam.zoom - s['zoom'][0]) / (s['zoom'][1] - s['zoom'][0])
    return (s['far'] + (s['near'] - s['far']) * min(max(t, 0), 1)) * cam.reading * cam.px


def draw_float(cv, cam, style, x, y, lit=False, held=False):
    """The glass float: a wash of glass in a dark rim, the lamp a solid amber disc at its heart with a
    warm-white core, the sun's highlight upper left, and the lamp's light on the water round it."""
    r = float_radius(cam)
    px = cam.reading * cam.px
    rim = SPEC['sizes']['rim'] * px
    if lit:
        c, a = col(style, 'lit')
        cv.glow(x, y, r * 2.4, c, a)
    c, a = col(style, 'lamp_glow')
    cv.glow(x, y, r * 1.25, c, a)
    c, a = col(style, 'glass_body')
    cv.disc(x, y, r, c, a)
    lamp, _ = col(style, 'lamp')
    core, _ = col(style, 'lamp_core')
    rl = r * SPEC['sizes']['lamp']
    cv.disc(x, y, rl, lamp, 1.0)
    cv.over(x, y, rl, lambda dx, dy: np.clip(1.0 - np.hypot(dx, dy) / (rl * 0.72), 0, 1) ** 1.6, core, 1.0)
    c, a = col(style, 'glass_rim')
    cv.ring(x, y, r - rim / 2, rim, c, a)
    c, a = col(style, 'glass_light')
    cv.over(x - 0.36 * r, y - 0.42 * r, 0.55 * r,
            lambda dx, dy: np.exp(-((dx * 1.2) ** 2 + (dy * 2.4) ** 2) / (0.16 * r * r)), c, a)
    if held:
        c, a = col(style, 'glass_rim')
        cv.ring(x, y, r * 1.55, 0.6 * px, c, a * 0.45)


def draw_ring(cv, cam, style, x, y, t):
    """The touch's ring as the finger rests, t in [0, 1] of the beat: a thin ring drawing in and thickening
    until, at 1, it is the float's rim."""
    s = SPEC['sizes']['ring']
    r = (s['start'] + (s['rest'] - s['start']) * t) * cam.reading * cam.px
    rf = float_radius(cam)
    r = r * (1 - t * t) + rf * t * t
    w = (0.8 + (SPEC['sizes']['rim'] - 0.8) * t) * cam.reading * cam.px
    c, a = col(style, 'ring_shade')
    cv.ring(x, y, r + w * 1.2, w * 1.4, c, a * (1 - 0.5 * t))
    c, a = col(style, 'ring')
    cv.ring(x, y, r, w, c, a * (1 - 0.3 * t))
    if t > 0.4:
        k = min((t - 0.4) / 0.4, 1.0)                       # the glass forms over the middle of the beat
        c, a = col(style, 'glass_body')
        cv.disc(x, y, r - w / 2, c, a * k)
        c, a = col(style, 'glass_rim')
        cv.ring(x, y, r, w, c, a * k)
    if t > 0.7:
        k = (t - 0.7) / 0.3                                  # the lamp lights at its end
        c, a = col(style, 'lamp')
        cv.disc(x, y, rf * SPEC['sizes']['lamp'] * k, c, k)


def draw_wake(cv, cam, style, path):
    """Beads, one a day, newest last: amber on a dark edge, fading and shrinking with age."""
    s = SPEC['sizes']['bead']
    n = len(path) - 1
    bead, _ = col(style, 'bead')
    edge, edge_a = col(style, 'bead_edge')
    for i in range(n):
        age = (n - i) / n
        x, y, z = cam.project(path[i, 0], path[i, 1])
        if z < 0.3:
            continue
        r = (s['new'] + (s['old'] - s['new']) * age) * cam.reading * cam.px
        a = (1 - age) ** 1.4 * 0.95 + 0.05
        if edge_a > 0:
            cv.disc(x, y, r + 0.45 * cam.px, edge, edge_a * a)
        cv.disc(x, y, r, bead, a)


def draw_maybes(cv, cam, style, ends):
    """Thousands of pale specks where the float might be now; each with one dark grain."""
    r = SPEC['sizes']['maybe'] * cam.reading * cam.px
    c, a = col(style, 'maybe')
    k, ka = col(style, 'maybe_core')
    x, y, z = cam.project(ends[:, 0], ends[:, 1])
    for xi, yi, zi in zip(x, y, z):
        if zi < 0.3:
            continue
        cv.disc(xi, yi, r, c, a)
        if ka > 0:
            cv.disc(xi, yi, r * 0.35, k, ka)


def draw_silver(cv, cam, style, path):
    """The nearest real drifter's track: a silver wire on a dark line."""
    w = SPEC['sizes']['silver'] * cam.reading * cam.px
    we = SPEC['sizes']['silver_edge'] * cam.reading * cam.px
    pts = [cam.project(p[0], p[1]) for p in path]
    for colour, width, key in (('silver_edge', we, 'edge'), ('silver', w, 'wire')):
        c, a = col(style, colour)
        if a <= 0:
            continue
        for (x0, y0, z0), (x1, y1, z1) in zip(pts[:-1], pts[1:]):
            if min(z0, z1) < 0.3:
                continue
            cv.line(x0, y0, x1, y1, width, c, a)
    x, y, z = pts[-1]
    c, a = col(style, 'silver')
    cv.disc(x, y, w * 1.6, c, a)
    c, a = col(style, 'silver_edge')
    if a > 0:
        cv.ring(x, y, w * 1.6, 0.5 * cam.px, c, a)


def draw_words(cv, cam, style, date, credits):
    """The date and the credits at the foot, Libertinus on a halo of the sea."""
    px = cam.px
    ink, _ = col(style, 'date_ink')
    halo, ha = col(style, 'halo')
    y = (cam.h - (20 if min(cam.w, cam.h) >= 700 else 34) - 150 + 44) * px
    cv.text(cam.w / 2 * px, y, date, SPEC['sizes']['date'] * cam.reading * 1.0, px, ink, halo, ha)
    ink, _ = col(style, 'credit_ink')
    cv.text(cam.w / 2 * px, y + 22 * px, credits, SPEC['sizes']['credit'] * cam.reading, px, ink, halo, ha)


# ---- the studies

def base_for(device, at, style):
    name = f'{device}-{at.replace(",", "_")}-{style}.png'
    path = BASE / name
    if not path.exists():
        BASE.mkdir(parents=True, exist_ok=True)
        (w, h), px = SIZES[device]
        subprocess.run([sys.executable, str(ROOT / 'tools' / 'metal' / 'metal.py'), 'shot', 'globe+ocean', str(path),
                        '--device', device, f'--at={at}', '--scale', str(px), '--style', style, '--month', '10'], check=True)
    return Image.open(path)


def crop_1x(cv, x, y, half, path):
    x0, y0 = int(max(x - half, 0)), int(max(y - half, 0))
    im = Image.fromarray((np.clip(cv.im, 0, 1) * 255 + 0.5).astype(np.uint8))
    im.crop((x0, y0, min(x0 + 2 * half, cv.w), min(y0 + 2 * half, cv.h))).save(path)


def study(kind, device, at, style, days, maybes, put=None, month=9):
    OUT.mkdir(parents=True, exist_ok=True)
    lon, lat, zoom = map(float, at.split(','))
    cam = Camera(device, lon, lat, zoom)
    cv = Canvas(base_for(device, at, style))
    sea = Sea()
    plon, plat = put if put else (lon + 2.0, lat - 2.0)
    tag = f'{device}-{kind}-{style}-z{zoom:g}'
    sx, sy, _ = cam.project(plon, plat)
    if kind == 'ring':
        for i, t in enumerate((0.0, 0.3, 0.6, 0.85, 1.0)):
            c2 = Canvas(base_for(device, at, style))
            if t < 1.0:
                draw_ring(c2, cam, style, sx, sy, t)
            else:
                draw_float(c2, cam, style, sx, sy, lit=True)
            crop_1x(c2, sx, sy, int(60 * cam.px), OUT / f'{tag}-{i}-1x.png')
        return
    walks = sea.walk(plon, plat, month, days, list(range(maybes + 1)))
    real = walks[:, 0]
    others = walks[-1, 1:maybes + 1]
    silver = sea.walk(plon - 1.2, plat - 0.9, month, days + 20, [maybes + 7])[:, 0]
    # (a real track is reported every six hours and is smooth at this scale: the stand-in's daily
    # wander is smoothed over three days to look like one)
    silver = np.stack([np.convolve(silver[:, i], np.ones(3) / 3, mode='valid') for i in range(2)], -1)
    fx, fy, _ = cam.project(real[-1, 0], real[-1, 1])
    if kind in ('held', 'all'):
        draw_maybes(cv, cam, style, others)
    if kind in ('wake', 'held', 'all'):
        draw_wake(cv, cam, style, real)
    if kind in ('held', 'all'):
        draw_silver(cv, cam, style, silver)
    draw_float(cv, cam, style, fx, fy, lit=(kind != 'held'), held=(kind == 'held'))
    if kind == 'all':
        x2, y2, _ = cam.project(plon - 3, plat + 1.5)
        draw_float(cv, cam, style, x2, y2)
    draw_words(cv, cam, style, '14 October', 'Currents: NOAA Global Drifter Program  ·  Land: NASA Blue Marble  ·  Sea floor: GEBCO')
    cv.save(OUT / f'{tag}.png')
    crop_1x(cv, fx, fy, int((50 if kind == 'float' else 110) * cam.px), OUT / f'{tag}-1x.png')
    print(OUT / f'{tag}.png')


def sheets():
    """The crops the critics judge, joined into strips, into the look's studies/ (committed, small)."""
    dest = ROOT / 'adrift' / 'crates' / 'adrift-core' / 'src' / 'look' / 'studies'
    dest.mkdir(parents=True, exist_ok=True)
    def strip(names, out):
        ims = [Image.open(OUT / n) for n in names]
        sheet = Image.new('RGB', (sum(i.width for i in ims), max(i.height for i in ims)), (255, 255, 255))
        x = 0
        for i in ims:
            sheet.paste(i, (x, 0))
            x += i.width
        sheet.save(dest / out, optimize=True)
        print(dest / out)
    strip([f'phone-ring-ground-z2.6-{i}-1x.png' for i in range(5)], 'ring-beat-phone.png')
    strip(['phone-float-ground-z1-1x.png', 'phone-float-ground-z2.6-1x.png', 'phone-float-ground-z6-1x.png', 'ipad-float-ground-z2.6-1x.png'], 'float-by-zoom.png')
    strip(['phone-held-ground-z2.6-1x.png', 'phone-held-night-z2.6-1x.png'], 'held-phone-day-night.png')
    strip(['phone-held-ground-z1-1x.png', 'phone-held-ground-z6-1x.png'], 'held-phone-z1-z6.png')
    strip(['phone-wake-ground-z2.6-1x.png', 'ipad-all-ground-z2.6-1x.png'], 'wake-phone-and-ipad.png')
    im = Image.open(OUT / 'phone-all-ground-z2.6.png')
    im.crop((0, im.height - 900, im.width, im.height)).save(dest / 'foot-phone.png', optimize=True)
    print(dest / 'foot-phone.png')


def main():
    a = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    a.add_argument('kind', choices=('all', 'float', 'ring', 'wake', 'held', 'everything'))
    a.add_argument('device', nargs='?', default='phone', choices=SIZES)
    a.add_argument('--at', default='-68,36,2.6', help='lon,lat,zoom')
    a.add_argument('--style', default='ground', choices=('ground', 'night'))
    a.add_argument('--days', type=int, default=45)
    a.add_argument('--maybes', type=int, default=2000)
    a.add_argument('--put', default=None, help='lon,lat the float was put at (default: near the camera)')
    o = a.parse_args()
    put = tuple(map(float, o.put.split(','))) if o.put else None
    if o.kind == 'everything':
        for device, at in (('phone', '-68,36,2.6'), ('ipad', '-68,36,2.6'), ('phone', '-40,25,1'), ('phone', '-70,38,6')):
            for kind in ('float', 'wake', 'held', 'all'):
                study(kind, device, at, 'ground', o.days, o.maybes, put)
            study('ring', device, at, 'ground', o.days, o.maybes, put)
        for kind in ('all', 'held', 'ring'):
            study(kind, 'phone', '-68,36,2.6', 'night', o.days, o.maybes, put)
        sheets()
        return
    study(o.kind, o.device, o.at, o.style, o.days, o.maybes, put)


if __name__ == '__main__':
    main()
