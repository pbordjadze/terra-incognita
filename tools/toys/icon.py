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
    """A cream knight lifted off a wooden board's corner, on the slate table (the toy's own pieces)."""
    img = Image.new('RGBA', (N, N))
    d = ImageDraw.Draw(img)
    for y in range(N):
        k = y / N
        d.line([(0, y), (N, y)], fill=(int(51 - 23 * k), int(69 - 31 * k), int(87 - 36 * k), 255))
    q = 256
    for r in range(3):
        for c in range(3):
            light = (r + c) % 2 == 0
            x0, y0 = 512 - 1.5 * q + c * q + 40, 512 - 1.5 * q + r * q + 130
            d.rectangle([x0, y0, x0 + q, y0 + q], fill=(242, 219, 176, 255) if light else (181, 130, 79, 255))
    atlas = Image.open(ROOT / 'toys/carol/data/pieces.png')
    cell = atlas.crop((4 * 160, 0, 5 * 160, 160)).resize((720, 720), Image.LANCZOS)
    sil, rim, shadow, _ = cell.split()
    pad = Image.new('L', (N, N), 0)
    pad.paste(sil, (180, 200))
    sh = Image.new('RGBA', (N, N), (10, 12, 18, 255))
    sh.putalpha(pad.filter(ImageFilter.GaussianBlur(26)).point(lambda v: int(v * 0.5)))
    img.alpha_composite(sh)
    body = Image.new('RGBA', (720, 720), (252, 242, 222, 255))
    body.putalpha(sil)
    img.alpha_composite(body, (150, 110))
    ink = Image.new('RGBA', (720, 720), (60, 43, 33, 255))
    ink.putalpha(rim.point(lambda v: int(v * 0.9)))
    img.alpha_composite(ink, (150, 110))
    return img


DRAW = {'whales': whales, 'murmuration': murmuration, 'mold': mold, 'fireflies': fireflies, 'bell': bell, 'swell': swell, 'battle': battle, 'carol': carol}

if __name__ == '__main__':
    for toy in sys.argv[1:] or DRAW:
        out = ROOT / 'toys' / toy / 'icon.png'
        DRAW[toy]().convert('RGB').save(out)
        print(out)
