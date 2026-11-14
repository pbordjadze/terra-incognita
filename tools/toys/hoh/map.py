"""Hoh's stand from above, to tune its history: the logs, and the trees by height (stilted ones ringed),
with the eye's view drawn in.

    cargo run -p hoh --release --example census -- [years] | python3 tools/toys/hoh/map.py <out.png>
"""
import sys

from PIL import Image, ImageDraw

S = 10  # px a metre
X, Z1 = 40, 90
img = Image.new('RGB', (2 * X * S, Z1 * S), (28, 40, 24))
d = ImageDraw.Draw(img)


def p(x, z):
    return (X * S + x * S, (Z1 - z) * S)


# the eye's view: f = 0.72 w, the picture's half-width 0.5 w
d.line([p(0, 0), p(-0.69 * Z1, Z1)], fill=(90, 90, 70))
d.line([p(0, 0), p(0.69 * Z1, Z1)], fill=(90, 90, 70))
for line in sys.stdin:
    f = line.split()
    if f[0] == 'L':
        ax, az, bx, bz, r, age = map(float, f[1:])
        d.line([p(ax, az), p(bx, bz)], fill=(150, 100, 60), width=max(1, int(r * 2 * S)))
    elif f[0] == 'T':
        x, z, base, h, dying = map(float, f[1:])
        if h < 1.0:
            continue
        r = max(1, int((0.15 + h * 0.015) * S))
        c = (80, 200, 90) if h < 30 else (220, 230, 200)
        if dying:
            c = (160, 90, 50)
        d.ellipse([p(x - r / S, z + r / S), p(x + r / S, z - r / S)], fill=c)
        if base > 0.3:
            d.ellipse([p(x - 2 * r / S, z + 2 * r / S), p(x + 2 * r / S, z - 2 * r / S)], outline=(220, 160, 90))
img.save(sys.argv[1])
