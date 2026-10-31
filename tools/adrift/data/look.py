"""Draws Adrift's data sheets so they can be judged by eye: what the floats will actually ride.

    python3 tools/adrift/data/look.py <out dir> [month]

Writes, each 1440 x 720 over the coast sheet's land and sea:
  wind-MM.png      the month's wind as streaks (longer and brighter where it blows harder)
  eddy.png         the eddy diffusivity (the root of K, zonal and meridional summed), warm where
                   the sea wanders most, and the unmeasured sea left grey
  ice-MM.png       the month's ice over the sea, with the 15% edge that holds a float
  drifters.png     every bundled drifter run, a thin line each, so the lonely seas show
  currents-MM.png  the month's current as streaks, for comparison with the wind
A sheet is read exactly as the app will (the manifest's `files` says where, `sheets` how), so
what is drawn is what the rules see; the drawing itself is only for looking.
"""
import json, struct, sys
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[3]
DATA = ROOT / 'adrift' / 'data'
W, H = 1440, 720


def manifest():
    return json.loads((DATA / 'manifest.json').read_text())


def open_sheet(m, name, mode):
    """A file the app loads, by the name the code uses, from where the manifest says it is."""
    return np.asarray(Image.open(ROOT / m['files'][name]).convert(mode))


def tile_of(sheet, entry, month):
    """A month's cells (no border) from a tiled sheet, as float arrays per channel."""
    w, h = entry['cells']
    tw, th = w + 2 * entry['border'], h + 2 * entry['border']
    x, y = (month % 4) * tw, (month // 4) * th
    b = entry['border']
    return sheet[y + b:y + b + h, x + b:x + b + w].astype(np.float64)


def resample(a, w=W, h=H):
    """Nearest-cell resample onto the drawing's grid."""
    ys = (np.arange(h) * a.shape[0] / h).astype(int)
    xs = (np.arange(w) * a.shape[1] / w).astype(int)
    return a[ys][:, xs]


def ground():
    coast = open_sheet(manifest(), 'geo.png', 'RGBA')
    sea = resample(coast[..., 1] < 128)
    img = np.where(sea[..., None], np.array([[[214, 228, 240]]]), np.array([[[236, 232, 220]]])).astype(np.uint8)
    return img, sea


def streaks(draw, u, v, scale, colour, step=12):
    """Short lines from every step-th cell along the vector, length by magnitude."""
    hh, ww = u.shape
    for y in range(step // 2, hh, step):
        for x in range(step // 2, ww, step):
            a, b = u[y, x], v[y, x]
            m = np.hypot(a, b)
            if m < 1e-3:
                continue
            L = min(scale * m, step * 1.6)
            dx, dy = a / m * L, -b / m * L
            k = min(1.0, m / (u.std() * 3 + 1e-9))
            c = tuple(int(colour[i] * k + 200 * (1 - k)) for i in range(3))
            draw.line([(x - dx / 2, y - dy / 2), (x + dx / 2, y + dy / 2)], fill=c, width=1)
            draw.ellipse([x + dx / 2 - 1, y + dy / 2 - 1, x + dx / 2 + 1, y + dy / 2 + 1], fill=c)


def main():
    out = Path(sys.argv[1])
    out.mkdir(parents=True, exist_ok=True)
    month = int(sys.argv[2]) - 1 if len(sys.argv) > 2 else 0
    m = manifest()
    img, sea = ground()

    # wind
    e = m['sheets']['wind']
    sheet = open_sheet(m, 'wind.png', 'RGB')
    t = tile_of(sheet, e, month)
    u, v = (t[..., 0] - 128) / 127 * e['max'], (t[..., 1] - 128) / 127 * e['max']
    pic = Image.fromarray(img.copy())
    streaks(ImageDraw.Draw(pic), resample(u), resample(v), 1.6, (40, 60, 90), 14)
    pic.save(out / f'wind-{month + 1:02d}.png')

    # currents
    e = m['sheets']['currents']
    sheet = open_sheet(m, 'currents.png', 'RGB')
    t = tile_of(sheet, e, month)
    s = (t[..., :2] - 128) / 127
    uv = np.sign(s) * s * s * e['max']
    uv[t[..., 2] < 64] = 0
    pic = Image.fromarray(img.copy())
    streaks(ImageDraw.Draw(pic), resample(uv[..., 0]), resample(uv[..., 1]), 14, (20, 70, 140), 10)
    pic.save(out / f'currents-{month + 1:02d}.png')

    # eddy
    e = m['sheets']['eddy']
    sheet = open_sheet(m, 'eddy.png', 'RGB')
    t = tile_of(sheet, e, 0)
    k = (t[..., :2] / 255) ** 2 * e['max']
    root = np.sqrt(k.sum(-1))                               # m/s^0.5: the step's size
    measured = t[..., 2] / 255
    x = np.clip(root / np.sqrt(60000), 0, 1)
    warm = np.stack([255 * x + 214 * (1 - x), 140 * x + 228 * (1 - x), 60 * x + 240 * (1 - x)], -1)
    grey = np.array([[[200, 200, 200]]])
    col = warm * measured[..., None] + grey * (1 - measured[..., None])
    layer = resample(col)
    pic = np.where(sea[..., None], layer, img).astype(np.uint8)
    Image.fromarray(pic).save(out / 'eddy.png')

    # ice
    e = m['sheets']['ice']
    t = open_sheet(m, f'ice-{month + 1:02d}.png', 'L').astype(np.float64) / 255
    ice = resample(t)
    pic = img.copy().astype(np.float64)
    white = np.array([250, 250, 252])
    a = np.clip(ice, 0, 1)[..., None]
    pic = np.where(sea[..., None], pic * (1 - a) + white * a, pic)
    edge = (ice >= e['hold']) & sea
    rim = edge & ~np.roll(edge, 1, 0) | edge & ~np.roll(edge, 1, 1) | edge & ~np.roll(edge, -1, 0) | edge & ~np.roll(edge, -1, 1)
    pic[rim] = [90, 120, 180]
    Image.fromarray(pic.astype(np.uint8)).save(out / f'ice-{month + 1:02d}.png')

    # drifters
    pic = Image.fromarray(img.copy())
    d = ImageDraw.Draw(pic)
    raw = (ROOT / m['files']['drifters.bin']).read_bytes()
    assert raw[:4] == b'ADRF'
    version, count = struct.unpack_from('<II', raw, 4)
    p, runs, points = 12, 0, 0
    for _ in range(count):
        id_, day0, n, flags = struct.unpack_from('<QfHH', raw, p)
        p += 16
        xy = np.frombuffer(raw, '<i2', 2 * n, p).reshape(n, 2) / 180.0
        p += 4 * n
        px = (xy[:, 0] + 180) / 360 * W
        py = (90 - xy[:, 1]) / 180 * H
        seg = [(px[0], py[0])]
        for i in range(1, n):
            if abs(px[i] - px[i - 1]) > W / 2:
                d.line(seg, fill=(190, 110, 40), width=1)
                seg = []
            seg.append((px[i], py[i]))
        if len(seg) > 1:
            d.line(seg, fill=(190, 110, 40), width=1)
        runs += 1
        points += n
    pic.save(out / 'drifters.png')
    print(f'{runs} runs, {points} points drawn')


if __name__ == '__main__':
    main()
