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


DRAW = {'whales': whales, 'murmuration': murmuration, 'mold': mold, 'fireflies': fireflies, 'bell': bell, 'swell': swell}

if __name__ == '__main__':
    for toy in sys.argv[1:] or DRAW:
        out = ROOT / 'toys' / toy / 'icon.png'
        DRAW[toy]().convert('RGB').save(out)
        print(out)
