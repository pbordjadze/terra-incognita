#!/usr/bin/env python3
"""Draws a toy's placeholder icon from code (1024 px, toys/<toy>/icon.png): the SideStore source and
the IPA both use it (toys/ios/build.sh resizes it with sips). A toy that draws its own icon replaces
its entry here, or writes icon.png by its own tool; the name and the final icon are the owner's.

    python3 tools/toys/icon.py [<toy> ...]      # every toy's when none is named
"""
import itertools
import json
import math
import random
import sys
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
    cell = atlas.crop((4 * 160, 0, 5 * 160, 160)).resize((900, 900), Image.Resampling.LANCZOS)
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
    # Shortcut: a town at night round a lake, a pale bridge drawn across it, the cars on the
    # streets either end of it gone red, the long roads round empty
    img = Image.new('RGBA', (N, N), (16, 20, 24, 255))
    d = ImageDraw.Draw(img)
    c = N / 2
    d.ellipse([c - 150, c - 330, c + 150, c + 330], fill=(8, 16, 30, 255))
    s0, e0, a, b = (c, N * 0.92), (c, N * 0.08), (c - 230, c + 40), (c + 230, c - 40)

    def bez(p, q1, q2, r, n=60):
        out = []
        for i in range(n + 1):
            t = i / n
            u = 1 - t
            out.append(tuple(u ** 3 * p[k] + 3 * u * u * t * q1[k] + 3 * u * t * t * q2[k] + t ** 3 * r[k] for k in (0, 1)))
        return out
    left_st = bez(s0, (c - 200, c + 330), (c - 200, c + 330), a)
    right_st = bez(b, (c + 200, c - 330), (c + 200, c - 330), e0)
    left_long = bez(a, (-200, c + 60), (-200, N * 0.1), e0)
    right_long = bez(s0, (N + 200, N * 0.9), (N + 200, c - 60), b)
    for road in (left_long, right_long):
        d.line(road, fill=(40, 44, 50, 255), width=26)
    for road in (left_st, right_st, [(c, N), s0], [e0, (c, 0)]):
        d.line(road, fill=(54, 56, 62, 255), width=22)
    d.line([a, b], fill=(150, 152, 160, 255), width=26)
    reds = [left_st[i] for i in range(4, 60, 7)] + [right_st[i] for i in range(4, 60, 7)] + [(a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f) for f in (0.3, 0.7)]
    glow(img, reds, 13, (255, 60, 30, 240), 16)
    glow(img, [left_long[30], right_long[30]], 11, (255, 236, 200, 220), 14)
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
    for a, b in itertools.pairwise(path):
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


def screw():
    # a record on the dark, its plum label, the lamp's two wedges of light on the grooves
    img = Image.new('RGBA', (N, N), (10, 9, 12, 255))
    d = ImageDraw.Draw(img)
    c, R = N / 2, 430
    halo = Image.new('RGBA', img.size, (0, 0, 0, 0))
    ImageDraw.Draw(halo).ellipse([c - R - 30, c - R - 30, c + R + 30, c + R + 30], fill=(90, 30, 130, 150))
    img.alpha_composite(halo.filter(ImageFilter.GaussianBlur(40)))
    d = ImageDraw.Draw(img)
    d.ellipse([c - R, c - R, c + R, c + R], fill=(14, 13, 16, 255))
    for i in range(70):
        r = 150 + i * 3.9
        g = 22 + (i * 37 % 11)
        d.ellipse([c - r, c - r, c + r, c + r], outline=(g, g, g + 3, 255), width=1)
    sheen = Image.new('RGBA', img.size, (0, 0, 0, 0))
    sd = ImageDraw.Draw(sheen)
    for a0 in (-128, 52):
        sd.pieslice([c - R + 20, c - R + 20, c + R - 20, c + R - 20], a0 - 9, a0 + 9, fill=(220, 210, 235, 70))
    sheen = sheen.filter(ImageFilter.GaussianBlur(14))
    mask = Image.new('L', img.size, 0)
    ImageDraw.Draw(mask).ellipse([c - R + 10, c - R + 10, c + R - 10, c + R - 10], fill=255)
    ImageDraw.Draw(mask).ellipse([c - 150, c - 150, c + 150, c + 150], fill=0)
    img.paste(Image.alpha_composite(img, sheen), (0, 0), mask)
    d = ImageDraw.Draw(img)
    d.ellipse([c - 146, c - 146, c + 146, c + 146], fill=(92, 34, 112, 255))
    d.arc([c - 108, c - 108, c + 108, c + 108], 200, 300, fill=(240, 205, 140, 255), width=9)
    d.ellipse([c - 12, c - 12, c + 12, c + 12], fill=(140, 140, 148, 255))
    d.ellipse([c - 6, c - 6, c + 6, c + 6], fill=(20, 20, 22, 255))
    return img


def throats():
    # a spiral of three throat colours turning on a sunny slope, one lizard in the middle of it
    img = Image.new('RGBA', (N, N), (176, 146, 104, 255))
    d = ImageDraw.Draw(img)
    rnd = random.Random(257)
    cols = [(250, 118, 26, 255), (40, 108, 250, 255), (255, 228, 56, 255)]
    c = N / 2
    for y in range(0, N, 26):
        for x in range(0, N, 26):
            px, py = x + rnd.uniform(-8, 8), y + rnd.uniform(-8, 8)
            a = math.atan2(py - c, px - c)
            r = math.hypot(px - c, py - c)
            k = int(((a / math.tau) * 3 + r / 170) % 3)
            d.ellipse([px - 7, py - 7, px + 7, py + 7], fill=cols[k])
    # the lizard: body, head, tail, legs, its orange throat
    cx, cy = c, c
    d.ellipse([cx - 150, cy - 46, cx + 80, cy + 46], fill=(112, 88, 60, 255))
    d.ellipse([cx + 40, cy - 52, cx + 180, cy + 52], fill=(112, 88, 60, 255))
    d.polygon([(cx - 130, cy - 26), (cx - 130, cy + 26), (cx - 420, cy + 60), (cx - 430, cy + 52)], fill=(112, 88, 60, 255))
    for (x0, y0, x1, y1) in [(40, 0, 90, -120), (40, 0, 10, 120), (-90, 0, -150, -120), (-90, 0, -40, 120)]:
        d.line([cx + x0, cy + y0, cx + x1, cy + y1], fill=(112, 88, 60, 255), width=26)
    d.ellipse([cx + 70, cy - 44, cx + 176, cy + 44], fill=cols[0])
    for i in range(5):
        d.ellipse([cx - 120 + i * 40, cy - 14, cx - 96 + i * 40, cy + 14], fill=(60, 44, 30, 255))
    d.ellipse([cx + 140, cy - 34, cx + 156, cy - 18], fill=(20, 16, 12, 255))
    d.ellipse([cx + 140, cy + 18, cx + 156, cy + 34], fill=(20, 16, 12, 255))
    return img


def elkmont():
    # a dark wood at night, its trunks against a green glow, fireflies flashing low among them
    img = Image.new('RGBA', (N, N), (8, 11, 14, 255))
    d = ImageDraw.Draw(img)
    for y in range(N):
        g = max(0.0, 1.0 - abs(y - N * 0.58) / (N * 0.32))
        d.line([(0, y), (N, y)], fill=(int(10 + 40 * g), int(14 + 80 * g), int(12 + 20 * g), 255))
    rnd = random.Random(1106)
    for _ in range(14):
        x = rnd.uniform(0, N)
        w = rnd.uniform(14, 60)
        d.rectangle([x - w / 2, 0, x + w / 2, N], fill=(6, 9, 10, 255))
    glow = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    dots = [(rnd.uniform(40, N - 40), N * 0.58 + rnd.gauss(0, N * 0.09), rnd.uniform(4, 13)) for _ in range(70)]
    for x, y, r in dots:
        gd.ellipse([x - r * 3, y - r * 3, x + r * 3, y + r * 3], fill=(150, 220, 60, 90))
    img = Image.alpha_composite(img, glow.filter(ImageFilter.GaussianBlur(18)))
    d = ImageDraw.Draw(img)
    for x, y, r in dots:
        d.ellipse([x - r * 0.6, y - r * 0.6, x + r * 0.6, y + r * 0.6], fill=(225, 255, 150, 255))
    return img


def nazare():
    # the dusk sea off Nazaré: its real coast (the floor's land, cropped about the headland), the swell's
    # crests closing in on Praia do Norte, white water where they meet
    import numpy as np
    meta = json.loads((ROOT / 'toys' / 'nazare' / 'data' / 'floor.json').read_text())
    w, h = meta['w'], meta['h']
    ground = np.frombuffer((ROOT / 'toys' / 'nazare' / 'data' / 'floor.bin').read_bytes(), '<i2').reshape(h, w)
    px, py = meta['places']['praia_do_norte']['cell']
    half = 150
    x0, y0 = int(px) - int(half * 1.55), int(py) - half
    crop = ground[max(y0, 0):y0 + 2 * half, max(x0, 0):x0 + 2 * half]
    land = Image.fromarray(np.where(crop >= 0, 255, 0).astype('uint8')).resize((N, N), Image.Resampling.BILINEAR)
    img = Image.new('RGBA', (N, N), (14, 34, 46, 255))
    k = N / (2 * half)
    fx, fy = (px - x0) * k, (py - y0) * k
    glow = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    for j in range(7):
        r = 70 + j * 92
        gd.arc([fx - r, fy - r, fx + r, fy + r], 120, 240, fill=(200, 225, 230, 110), width=26)
    img = Image.alpha_composite(img, glow.filter(ImageFilter.GaussianBlur(10)))
    d = ImageDraw.Draw(img)
    for j in range(7):
        r = 70 + j * 92
        d.arc([fx - r, fy - r, fx + r, fy + r], 120, 240, fill=(214, 232, 232, 255), width=9 if j > 1 else 14)
    foam = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    ImageDraw.Draw(foam).ellipse([fx - 120, fy - 110, fx - 10, fy + 110], fill=(240, 246, 246, 230))
    img = Image.alpha_composite(img, foam.filter(ImageFilter.GaussianBlur(14)))
    ground_img = Image.new('RGBA', (N, N), (30, 28, 22, 255))
    img.paste(ground_img, (0, 0), land.filter(ImageFilter.GaussianBlur(1.5)))
    d = ImageDraw.Draw(img)
    tx, ty = (meta['places']['town']['cell'][0] - x0) * k, (meta['places']['town']['cell'][1] - y0) * k
    for dx, dy in [(10, 0), (30, 20), (5, 40), (40, -10), (22, 55), (55, 30)]:
        d.ellipse([tx + dx - 7, ty + dy - 7, tx + dx + 7, ty + dy + 7], fill=(255, 190, 110, 255))
    return img


def gradient(top, bottom):
    img = Image.new('RGBA', (N, N))
    d = ImageDraw.Draw(img)
    for y in range(N):
        f = y / (N - 1)
        d.line([0, y, N, y], fill=tuple(int(top[i] + (bottom[i] - top[i]) * f) for i in range(3)) + (255,))
    return img


def bats():
    # a dusk sky, a column of bats rising from the cave, bending off as a stream; the hawk above
    img = gradient((70, 96, 170), (246, 160, 104))
    d = ImageDraw.Draw(img)
    rnd = random.Random(1)
    d.rectangle([0, 900, N, N], fill=(24, 22, 26, 255))
    d.ellipse([430, 880, 600, 920], fill=(6, 5, 7, 255))
    for i in range(2600):
        h = rnd.random()
        if h < 0.72:
            y = 900 - h / 0.72 * 620
            x = 515 + 40 * math.sin(y / 70) + (20 + 60 * (900 - y) / 620) * rnd.uniform(-1, 1)
        else:
            s = (h - 0.72) / 0.28
            x = 520 + s * 520 + 30 * rnd.uniform(-1, 1) * (1 - s * 0.5)
            y = 290 + 90 * s * s + 26 * rnd.uniform(-1, 1)
        w = rnd.uniform(4, 9)
        d.line([x - w, y - rnd.uniform(0, 4), x, y + 2, x + w, y - rnd.uniform(0, 4)], fill=(18, 14, 20, 255), width=3)
    cx, cy = 300, 200
    d.polygon([(cx - 150, cy - 20), (cx - 40, cy + 6), (cx, cy - 10), (cx + 40, cy + 6), (cx + 150, cy - 20),
               (cx + 40, cy + 30), (cx + 14, cy + 70), (cx - 14, cy + 70), (cx - 40, cy + 30)], fill=(16, 12, 10, 255))
    return img


def huddle():
    # emperor penguins packed in a huddle on the sea ice at night, snow streaming past, the aurora above
    img = gradient((10, 20, 44), (64, 82, 116))
    d = ImageDraw.Draw(img)
    rnd = random.Random(2)
    glow(img, [(x, 150 + 60 * math.sin(x / 140)) for x in range(-40, N + 40, 30)], 30, (60, 230, 150, 70), 40)
    c = (512, 600)
    for r in range(0, 330, 30):
        n = max(1, int(2 * math.pi * r / 30))
        for k in range(n):
            a = k / n * math.tau + rnd.uniform(-0.1, 0.1)
            x, y = c[0] + r * math.cos(a), c[1] + r * 0.62 * math.sin(a)
            d.ellipse([x - 15, y - 13, x + 15, y + 13], fill=(14, 14, 18, 255))
            d.ellipse([x - 6, y - 12, x + 6, y - 2], fill=(236, 200, 120, 255))
    for i in range(160):
        x, y = rnd.uniform(0, N), rnd.uniform(0, N)
        d.line([x, y, x + 60, y + 8], fill=(220, 230, 255, 150), width=2)
    return img


def wave():
    # a stadium bowl at night full of phone lights, the wave a bright arc running round it
    img = gradient((6, 6, 14), (14, 12, 22))
    d = ImageDraw.Draw(img)
    rnd = random.Random(3)
    d.ellipse([200, 360, 824, 700], fill=(30, 70, 40, 255))
    for row in range(26):
        rx, ry = 330 + row * 12, 190 + row * 8
        n = int(2 * math.pi * rx / 12)
        for k in range(n):
            a = k / n * math.tau
            x, y = 512 + rx * math.cos(a), 530 + ry * math.sin(a)
            on = abs(((a - 4.2 + math.pi) % math.tau) - math.pi) < 0.35
            col = (255, 250, 220, 255) if on else (130 + rnd.randint(0, 60), 130 + rnd.randint(0, 50), 150, 255)
            r = 4.5 if on else 2.4
            y -= 10 if on else 0
            d.ellipse([x - r, y - r, x + r, y + r], fill=col)
    return img


def catatumbo():
    # night over a lake: a violet stroke forking down to the water, the cloud lit behind it, its reflection
    img = Image.new('RGBA', (N, N), (10, 10, 26, 255))
    glow = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    gd = ImageDraw.Draw(glow)
    gd.ellipse([180, -260, 860, 420], fill=(150, 120, 230, 150))
    glow = glow.filter(ImageFilter.GaussianBlur(120))
    img = Image.alpha_composite(img, glow)
    d = ImageDraw.Draw(img)
    hy = 640
    d.rectangle([0, hy, N, N], fill=(14, 12, 30, 255))
    d.rectangle([0, hy - 10, N, hy], fill=(4, 4, 8, 255))
    rnd = random.Random(327)
    x, y = 470.0, 0.0
    pts = [(x, y)]
    while y < hy:
        y += rnd.uniform(30, 60)
        x += rnd.uniform(-45, 45)
        pts.append((x, min(y, hy)))
    bolt = Image.new('RGBA', (N, N), (0, 0, 0, 0))
    bd = ImageDraw.Draw(bolt)
    bd.line(pts, fill=(170, 140, 255, 255), width=46)
    for i in (3, 6):
        bx, by = pts[i]
        bd.line([(bx, by), (bx + rnd.choice([-1, 1]) * 140, by + 120)], fill=(170, 140, 255, 255), width=24)
    bolt = bolt.filter(ImageFilter.GaussianBlur(22))
    img = Image.alpha_composite(img, bolt)
    d = ImageDraw.Draw(img)
    d.line(pts, fill=(250, 246, 255, 255), width=12)
    refl = [(px + rnd.uniform(-8, 8), 2 * hy - py) for (px, py) in pts if 2 * hy - py < N]
    if len(refl) > 1:
        d.line(refl, fill=(120, 100, 200, 255), width=10)
    return img


def bubblers():
    # pale sand from above: one crab at its burrow, its balls laid out in spokes all round
    img = Image.new('RGBA', (N, N), (222, 206, 178, 255))
    d = ImageDraw.Draw(img)
    c = N / 2
    rnd = random.Random(327)
    for s in range(14):
        a = s * math.tau / 14 + rnd.uniform(-0.1, 0.1)
        for k in range(1, 9):
            r = 120 + k * 42
            x, y = c + math.cos(a) * r + rnd.uniform(-5, 5), c + math.sin(a) * r + rnd.uniform(-5, 5)
            d.ellipse([x - 15 + 4, y - 15 + 5, x + 15 + 4, y + 15 + 5], fill=(170, 150, 120, 255))
            d.ellipse([x - 15, y - 15, x + 15, y + 15], fill=(236, 224, 200, 255))
    d.ellipse([c - 34, c - 34, c + 34, c + 34], fill=(40, 32, 26, 255))
    cx, cy = c + 70, c - 60
    for side in (-1, 1):
        for leg in range(4):
            a = (leg - 1.5) * 0.4
            d.line([cx, cy, cx + math.cos(a) * 30 + 0, cy + side * 95 + math.sin(a) * 40], fill=(150, 124, 92, 255), width=12)
    d.ellipse([cx - 62, cy - 62, cx + 62, cy + 62], fill=(204, 180, 140, 255))
    d.ellipse([cx + 40, cy - 40, cx + 56, cy - 24], fill=(20, 16, 12, 255))
    d.ellipse([cx + 40, cy + 24, cx + 56, cy + 40], fill=(20, 16, 12, 255))
    return img


def barchans():
    # a crescent dune from above in a low sun, horns downwind, its slip face in shade, on the grey reg
    img = Image.new('RGBA', (N, N), (150, 128, 106, 255))
    d = ImageDraw.Draw(img)
    c = N / 2
    d.ellipse([c - 300, c - 330, c + 300, c + 230], fill=(236, 170, 100, 255))
    d.ellipse([c - 210, c - 120, c + 210, c + 330], fill=(150, 128, 106, 255))
    d.chord([c - 210, c - 120, c + 210, c + 330], 180, 360, fill=(120, 84, 60, 255))
    d.ellipse([c - 170, c - 60, c + 170, c + 330], fill=(150, 128, 106, 255))
    d.polygon([(c - 300, c - 50), (c - 230, c + 330), (c - 180, c + 120)], fill=(236, 170, 100, 255))
    d.polygon([(c + 300, c - 50), (c + 230, c + 330), (c + 180, c + 120)], fill=(222, 154, 88, 255))
    return img


def pando():
    # one aspen leaf, round with a short tip, gold against the evening blue, on its flat stalk
    img = Image.new('RGBA', (N, N), (40, 52, 82, 255))
    d = ImageDraw.Draw(img)
    for y in range(N):
        g = y / N
        d.line([(0, y), (N, y)], fill=(int(40 + 140 * g), int(52 + 70 * g), int(82 - 20 * g), 255))
    cx, cy, r = N * 0.5, N * 0.56, N * 0.30
    pts = []
    for i in range(240):
        t = 2 * math.pi * i / 240
        rr = r * (1 + 0.1 * math.cos(t)) * (1 - 0.06 * math.cos(2 * t))
        rr *= 1 + 0.3 * math.exp(-(t / 0.16) ** 2) + 0.3 * math.exp(-((t - 2 * math.pi) / 0.16) ** 2)
        pts.append((cx + rr * math.sin(t), cy + rr * math.cos(t)))
    d.line([(cx + 30, N * 0.04), (cx, cy - r * 0.86)], fill=(214, 160, 60, 255), width=14)
    d.polygon(pts, fill=(246, 178, 34, 255))
    d.line([(cx, cy - r * 0.86), (cx, cy + r * 1.2)], fill=(255, 226, 140, 255), width=8)
    for k in range(4):
        y0 = cy - r * 0.7 + k * r * 0.38
        for sx in (-1, 1):
            d.line([(cx, y0), (cx + sx * r * (0.55 - 0.08 * k), y0 + r * 0.38)], fill=(255, 214, 120, 255), width=5)
    return img


DRAW = {'meander': meander, 'whales': whales, 'murmuration': murmuration, 'fireflies': fireflies, 'bell': bell, 'swell': swell, 'battle': battle, 'carol': carol, 'jam': jam, 'frost': frost, 'taste': taste, 'walk': walk, 'screw': screw, 'throats': throats, 'bats': bats, 'huddle': huddle, 'wave': wave, 'elkmont': elkmont, 'nazare': nazare, 'catatumbo': catatumbo, 'bubblers': bubblers, 'barchans': barchans, 'pando': pando}

if __name__ == '__main__':
    for toy in sys.argv[1:] or DRAW:
        out = ROOT / 'toys' / toy / 'icon.png'
        DRAW[toy]().convert('RGB').save(out)
        print(out)
