#!/usr/bin/env python3
"""Draws a toy's placeholder icon from code (1024 px, toys/<toy>/icon.png): the SideStore source and
the IPA both use it (toys/ios/build.sh resizes it with sips). A toy that draws its own icon replaces
its entry here, or writes icon.png by its own tool; the name and the final icon are the owner's.

    python3 tools/toys/icon.py [<toy> ...]      # every toy's when none is named
"""
import math, random, sys
from pathlib import Path
from PIL import Image, ImageDraw, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
N = 1024


def glow(img, pts, r, col, blur):
    layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for x, y in pts:
        d.ellipse([x - r, y - r, x + r, y + r], fill=col)
    img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(blur)))
    img.alpha_composite(layer)


def whales():
    img = Image.new('RGBA', (N, N))
    d = ImageDraw.Draw(img)
    for y in range(N):
        k = y / N
        d.line([(0, y), (N, y)], fill=(int(18 + 10 * (1 - k)), int(52 + 40 * (1 - k)), int(92 + 50 * (1 - k)), 255))
    # two songs: two arcs of rings
    for cx, cy, col in [(380, 560, (210, 235, 245, 140)), (650, 470, (245, 220, 190, 140))]:
        for i in range(1, 6):
            r = 60 * i
            d.arc([cx - r, cy - r, cx + r, cy + r], 200, 340, fill=col, width=10)
    return img


def murmuration():
    img = Image.new('RGBA', (N, N))
    d = ImageDraw.Draw(img)
    for y in range(N):
        k = y / N
        d.line([(0, y), (N, y)], fill=(int(236 - 60 * k), int(190 - 70 * k), int(160 - 30 * k), 255))
    rnd = random.Random(3)
    pts = []
    for _ in range(900):
        t = rnd.random() * math.tau
        r = (rnd.random() ** 0.6)
        x = 512 + math.cos(t) * r * 330 * (1 + 0.5 * math.sin(2 * t))
        y = 500 + math.sin(t) * r * 150 + 80 * math.sin(x / 140)
        pts.append((x, y))
    for x, y in pts:
        d.ellipse([x - 5, y - 4, x + 5, y + 4], fill=(30, 26, 40, 230))
    return img


def mold():
    img = Image.new('RGBA', (N, N), (8, 10, 16, 255))
    rnd = random.Random(5)
    layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    nodes = [(rnd.uniform(150, 870), rnd.uniform(150, 870)) for _ in range(9)]
    for i, a in enumerate(nodes):
        for b in sorted(nodes, key=lambda p: (p[0] - a[0]) ** 2 + (p[1] - a[1]) ** 2)[1:3]:
            d.line([a, b], fill=(255, 214, 90, 255), width=14)
    img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(18)))
    img.alpha_composite(layer)
    glow(img, nodes, 22, (255, 240, 200, 255), 20)
    return img


def fireflies():
    img = Image.new('RGBA', (N, N), (10, 14, 24, 255))
    rnd = random.Random(7)
    pts = [(rnd.uniform(120, 900), rnd.uniform(200, 900)) for _ in range(28)]
    glow(img, pts, 16, (255, 226, 120, 255), 26)
    return img


def bell():
    # a pale Earth, a strike's rings running out from it, lit from the upper left
    img = Image.new('RGBA', (N, N), (243, 239, 230, 255))
    d = ImageDraw.Draw(img)
    c, r = N / 2, 400
    d.ellipse([c - r, c - r, c + r, c + r], fill=(170, 205, 230, 255))
    ring = Image.new('RGBA', img.size, (0, 0, 0, 0))
    rd = ImageDraw.Draw(ring)
    sx, sy = c - 120, c + 90
    for k, rad in enumerate([90, 190, 300]):
        a = 200 - 50 * k
        rd.ellipse([sx - rad, sy - rad, sx + rad, sy + rad], outline=(255, 252, 240, a), width=22 - 5 * k)
        rd.ellipse([sx - rad + 14, sy - rad + 14, sx + rad + 14, sy + rad + 14], outline=(40, 70, 110, a // 3), width=14 - 3 * k)
    mask = Image.new('L', img.size, 0)
    ImageDraw.Draw(mask).ellipse([c - r, c - r, c + r, c + r], fill=255)
    ring = ring.filter(ImageFilter.GaussianBlur(5))
    ring.putalpha(Image.composite(ring.getchannel('A'), Image.new('L', img.size, 0), mask))
    img.alpha_composite(ring)
    return img


def swell():
    # the sea, a wave's light bent as it crosses the floor
    img = Image.new('RGBA', (N, N), (150, 196, 226, 255))
    layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for k in range(5):
        rad = 150 + 150 * k
        box = [300 - rad, 330 - rad * 0.86, 300 + rad, 330 + rad * 0.86]
        d.arc(box, -10, 110, fill=(255, 253, 245, 230 - 35 * k), width=26 - 3 * k)
        d.arc([box[0] + 18, box[1] + 18, box[2] + 18, box[3] + 18], -10, 110, fill=(25, 60, 110, 90 - 12 * k), width=16 - 2 * k)
    img.alpha_composite(layer.filter(ImageFilter.GaussianBlur(6)))
    return img

def battle():
    """Battle Ink: on warm paper, the Roman mass in iron-gall cuts held in a ring of Tyrian purple."""
    img = Image.new('RGBA', (N, N), (239, 231, 214, 255))
    d = ImageDraw.Draw(img)
    rnd = random.Random(216)
    for _ in range(3000):  # paper fibres
        x, y = rnd.uniform(0, N), rnd.uniform(0, N)
        d.line([(x, y), (x + rnd.uniform(-2, 2), y + rnd.uniform(4, 14))], fill=(200, 186, 160, 60), width=1)

    def cut(x, y, a, L, w, col):
        pts_l, pts_r = [], []
        for k in range(9):
            t = k / 8
            s = (t - 0.5) * L
            hw = w * math.sin(math.pi * t) ** 0.7
            cx, cy = x + math.cos(a) * s, y + math.sin(a) * s
            pts_l.append((cx - math.sin(a) * hw, cy + math.cos(a) * hw))
            pts_r.append((cx + math.sin(a) * hw, cy - math.cos(a) * hw))
        d.polygon(pts_l + pts_r[::-1], fill=col)
    cx, cy = 512, 540
    for _ in range(520):  # Rome, pressed into the pocket
        r = 230 * math.sqrt(rnd.random())
        t = rnd.uniform(0, math.tau)
        x, y = cx + r * math.cos(t), cy + r * math.sin(t) * 0.8
        cut(x, y, math.pi / 2 + rnd.uniform(-0.25, 0.25) + (x - cx) / 900, 34, 4.5, (38, 46, 70, 255))
    for _ in range(700):  # Carthage, the ring closing round it, combed along it
        t = rnd.uniform(0, math.tau)
        r = rnd.uniform(270, 360)
        x, y = cx + r * math.cos(t), cy + r * math.sin(t) * 0.8
        cut(x, y, t + math.pi / 2 + rnd.uniform(-0.2, 0.2), 40, 5, (128, 34, 92, 255))
    return img


def carol():
    """One cream knight, large and centred, on a warm wood ground (the toy's own pieces)."""
    img = Image.new('RGBA', (N, N))
    d = ImageDraw.Draw(img)
    rnd = random.Random(7)
    for y in range(N):
        k = y / N
        g = 1 + 0.03 * math.sin(y / 9 + rnd.random() * 0.3)
        d.line([(0, y), (N, y)], fill=(int((196 - 40 * k) * g), int((140 - 32 * k) * g), int((86 - 22 * k) * g), 255))
    atlas = Image.open(ROOT / 'toys/carol/data/pieces.png')
    cell = atlas.crop((4 * 160, 0, 5 * 160, 160)).resize((900, 900), Image.LANCZOS)
    sil, rim = cell.split()[0], cell.split()[1]
    at = (62, 92)
    pad = Image.new('L', (N, N), 0)
    pad.paste(sil, (at[0] + 14, at[1] + 34))
    sh = Image.new('RGBA', (N, N), (40, 22, 10, 255))
    sh.putalpha(pad.filter(ImageFilter.GaussianBlur(24)).point(lambda v: int(v * 0.55)))
    img.alpha_composite(sh)
    body = Image.new('RGBA', (900, 900), (253, 244, 226, 255))
    body.putalpha(sil)
    img.alpha_composite(body, at)
    ink = Image.new('RGBA', (900, 900), (66, 44, 30, 255))
    ink.putalpha(rim.point(lambda v: int(v * 0.85)))
    img.alpha_composite(ink, at)
    return img

def meander():
    """A café-au-lait river swinging down a dark forest, a black-water oxbow beside its bend (the toy's own colours)."""
    img = Image.new('RGBA', (N, N), (34, 64, 44, 255))
    d = ImageDraw.Draw(img)
    rnd = random.Random(3)
    for _ in range(2600):  # the canopy's grain
        x, y, r = rnd.uniform(0, N), rnd.uniform(0, N), rnd.uniform(4, 14)
        g = rnd.uniform(-12, 12)
        d.ellipse([x - r, y - r, x + r, y + r], fill=(int(34 + g), int(66 + g), int(46 + g * 0.6), 255))
    river = [(N * 0.5 + 300 * math.sin(t * 2.2 + 0.4) * (0.6 + 0.4 * math.sin(t * 1.3)), -40 + t / math.tau * (N + 80)) for t in [i / 400 * math.tau for i in range(401)]]
    ox = [(N * 0.17 + 110 * math.cos(a), N * 0.45 + 140 * math.sin(a)) for a in [math.pi * (0.25 + 1.5 * i / 80) for i in range(81)]]
    def tube(pts, r, col):
        for i in range(len(pts) - 1):
            (x0, y0), (x1, y1) = pts[i], pts[i + 1]
            for k in range(6):
                x, y = x0 + (x1 - x0) * k / 6, y0 + (y1 - y0) * k / 6
                d.ellipse([x - r, y - r, x + r, y + r], fill=col)
    tube(ox, 22, (38, 38, 58, 255))
    tube(river, 59, (224, 207, 168, 255))
    tube(river, 46, (194, 161, 120, 255))
    return img


def jam():
    # the ring from above at dusk: cars round it, and one knot of red brake lights
    img = Image.new('RGBA', (N, N), (30, 34, 44, 255))
    d = ImageDraw.Draw(img)
    c, r, w = N / 2, 360, 70
    d.ellipse([c - r - w / 2, c - r - w / 2, c + r + w / 2, c + r + w / 2], fill=(18, 20, 26, 255))
    d.ellipse([c - r + w / 2, c - r + w / 2, c + r - w / 2, c + r - w / 2], fill=(30, 34, 44, 255))
    reds = []
    cars = [k * 360 / 16 for k in range(16)]
    # the knot: five cars bunched at the upper right
    cars = [a for a in cars if not 20 < a < 80] + [24 + 11 * k for k in range(5)]
    for a in cars:
        t = math.radians(a)
        x, y = c + r * math.cos(t), c - r * math.sin(t)
        hx, hy = -math.sin(t), -math.cos(t)
        L, W = 34, 15
        pts = [(x + hx * L * sx - hy * W * sy, y + hy * L * sx + hx * W * sy) for sx, sy in [(1, 1), (1, -1), (-1, -1), (-1, 1)]]
        d.polygon(pts, fill=(222, 224, 228, 255))
        if 20 < a < 80:
            reds.append((x - hx * L, y - hy * L))
    glow(img, reds, 16, (255, 40, 25, 230), 22)
    return img



def frost():
    """One ice flower grown by the toy's own rule (tools/toys/frost/reiter.py), white on the night glass."""
    import numpy as np
    sys.path.insert(0, str(ROOT / 'tools/toys/frost'))
    import reiter
    n = 241
    s = np.full((n, n), 0.55)
    c = n // 2
    s[c, c] = 1.0
    q, r = np.meshgrid(np.arange(n) - c, np.arange(n) - c, indexing='ij')
    dist = (abs(q) + abs(r) + abs(q + r)) / 2
    g = 0.005 * (1 + 0.4 * np.random.default_rng(5).uniform(-1, 1, (n, n)))
    while dist[s >= 1].max() < 105:
        fr = s >= 1
        rec = fr.copy()
        for m in reiter.nbrs(fr):
            rec |= m
        u = np.where(rec, 0, s)
        v = np.where(rec, s + g, 0)
        u = u + 0.5 * (sum(reiter.nbrs(u)) / 6 - u)
        s = u + v
        s[dist >= c - 1] = 0.55
    img = Image.new('RGBA', (N, N))
    d = ImageDraw.Draw(img)
    for y in range(N):
        k = y / N
        d.line([(0, y), (N, y)], fill=(int(4 + 14 * k), int(8 + 20 * k), int(26 + 40 * k), 255))
    ys, xs = np.mgrid[0:N, 0:N]
    sc = N / 236
    X, Y = (xs - N / 2) / sc, (ys - N / 2) / sc
    a = 0.26
    X, Y = X * math.cos(a) + Y * math.sin(a), -X * math.sin(a) + Y * math.cos(a)
    rr = Y / (math.sqrt(3) / 2)
    qq = X - rr / 2
    v = s[np.clip(np.round(qq).astype(int) + c, 0, n - 1), np.clip(np.round(rr).astype(int) + c, 0, n - 1)]
    age = np.clip(np.log1p(np.maximum(v - 1, 0) * 30) / 3, 0, 1)
    cover = (v >= 1).astype(float)
    col = np.stack([0.66 + 0.27 * age, 0.78 + 0.17 * age, 0.93 + 0.05 * age], -1) * 255
    alpha = cover * (0.6 + 0.37 * age)
    ice = Image.fromarray(np.dstack([col, alpha[..., None] * 255]).astype(np.uint8), 'RGBA').filter(ImageFilter.GaussianBlur(1.2))
    img.alpha_composite(ice)
    return img


def taste():
    # a soft five-armed creature on the dark, and behind it the faint path it walked, bright where it lingered
    img = Image.new('RGBA', (N, N), (16, 17, 24, 255))
    d = ImageDraw.Draw(img)
    path = [(250, 820), (330, 700), (300, 610), (420, 560), (520, 520)]
    for a, b in zip(path, path[1:]):
        d.line([a, b], fill=(150, 120, 200, 255), width=10)
    glow(img, path[2:3], 30, (230, 120, 255, 160), 40)
    star = []
    c, R = (560, 470), 300
    for i in range(400):
        t = i / 400 * 2 * math.pi
        r = R * (0.68 + 0.32 * ((0.5 + 0.5 * math.cos(5 * (t + 0.5 * 0.6))) ** 2.5))
        star.append((c[0] + r * math.cos(t), c[1] - r * math.sin(t)))
    halo = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    ImageDraw.Draw(halo).polygon(star, fill=(190, 90, 255, 200))
    img.alpha_composite(halo.filter(ImageFilter.GaussianBlur(36)))
    d = ImageDraw.Draw(img)
    d.polygon(star, fill=(160, 84, 245, 255))
    inner = [(c[0] + (x - c[0]) * 0.55, c[1] + (y - c[1]) * 0.55) for x, y in star]
    core = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    ImageDraw.Draw(core).polygon(inner, fill=(255, 140, 190, 255))
    img.alpha_composite(core.filter(ImageFilter.GaussianBlur(40)))
    glow(img, [c], 40, (255, 230, 200, 230), 50)
    return img



def walk():
    # night: a footprint map's contours rising round one hill, and the walker's lantern on it
    img = Image.new('RGBA', (N, N), (10, 13, 24, 255))
    d = ImageDraw.Draw(img)
    rnd = random.Random(224)
    cx, cy = 470, 560
    for k in range(7, 0, -1):
        pts = []
        for j in range(90):
            a = j / 90 * math.tau
            r = k * 52 * (1 + 0.10 * math.sin(3 * a + k) + 0.06 * math.sin(5 * a - k))
            pts.append((cx + r * math.cos(a), cy + r * math.sin(a) * 0.85))
        shade = int(30 + (7 - k) * 22)
        d.polygon(pts, fill=(shade, shade + 6, shade + 30, 255), outline=(200, 210, 235, 255), width=5)
    prints = [(cx + rnd.gauss(0, 90), cy + rnd.gauss(0, 80)) for _ in range(70)]
    glow(img, prints, 6, (255, 220, 160, 150), 3)
    glow(img, [(cx + 60, cy - 40)], 26, (255, 225, 170, 255), 30)
    return img


DRAW = {'meander': meander, 'whales': whales, 'murmuration': murmuration, 'mold': mold, 'fireflies': fireflies, 'bell': bell, 'swell': swell, 'battle': battle, 'carol': carol, 'jam': jam, 'frost': frost, 'taste': taste, 'walk': walk}

if __name__ == '__main__':
    for toy in sys.argv[1:] or DRAW:
        out = ROOT / 'toys' / toy / 'icon.png'
        DRAW[toy]().convert('RGB').save(out)
        print(out)
