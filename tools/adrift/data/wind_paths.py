"""Draws things on the wind and their maybes over a month on a plain map, without the app: what
the wind's rules (adrift-core `sea::wind`, `AirRules`) actually do, to look at.

    python3 tools/adrift/data/wind_paths.py <out dir> [month] [maybes]

(from the repository root, after `python3 tools/adrift/assets.py adrift/build/assets`). Runs the
rules through `cargo test -p adrift-core wind_paths_for_the_picture -- --ignored` twice and writes,
each 2160 x 1080, equirectangular over the coast sheet's land and sea:
  wind-aloft-MM.png  every thing kept up for the month (no settling, no rain): the medium itself,
                     each thing's own path (world 0) bold in its colour, its maybes (worlds 1..n)
                     thin in the same colour, a white dot where it was lifted
  wind-fates-MM.png  the same things with the rules' settling and rain-out: where each path came
                     down, a dot on land, a ring on the sea (where the sea's rules could take it)
The paths are the rules' own, a quarter day a step; only the drawing is for looking.
"""
import colorsys
import itertools
import json
import os
import subprocess
import sys
import tempfile
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[3]
W, H = 2160, 1080
S = 2  # supersampling


def run(month, maybes, aloft):
    out = Path(tempfile.mkstemp(suffix='.json')[1])
    env = dict(os.environ, ADRIFT_WIND_OUT=str(out), ADRIFT_WIND_MONTH=str(month),
               ADRIFT_WIND_MAYBES=str(maybes), ADRIFT_WIND_ALOFT='1' if aloft else '0')
    subprocess.run(['cargo', 'test', '-q', '--release', '-p', 'adrift-core', 'wind_paths_for_the_picture', '--',
                    '--ignored'], cwd=ROOT / 'adrift', env=env, check=True, stdout=subprocess.DEVNULL)
    doc = json.loads(out.read_text())
    out.unlink()
    return doc


def base():
    m = json.loads((ROOT / 'adrift' / 'data' / 'manifest.json').read_text())
    geo = Image.open(ROOT / m['files']['geo.png']).convert('RGB').resize((W * S, H * S), Image.BILINEAR)
    land = np.asarray(geo)[..., 0] > 128
    img = np.empty((H * S, W * S, 3), np.uint8)
    img[:] = (226, 236, 242)
    img[land] = (238, 232, 220)
    return Image.fromarray(img)


def xy(p):
    return ((p[0] + 180) / 360 * W * S, (90 - p[1]) / 180 * H * S)


def line(d, pts, rgba, width):
    seg = []
    for a, b in itertools.pairwise(pts):
        if abs(b[0] - a[0]) > 180:          # across the date line: break
            if len(seg) > 1:
                d.line(seg, fill=rgba, width=width, joint='curve')
            seg = []
            continue
        if not seg:
            seg = [xy(a)]
        seg.append(xy(b))
    if len(seg) > 1:
        d.line(seg, fill=rgba, width=width, joint='curve')


def draw(doc, fates):
    img = base().convert('RGBA')
    layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for i, t in enumerate(doc['things']):
        h = (i * 0.618034) % 1.0
        r, g, b = (int(255 * c) for c in colorsys.hls_to_rgb(h, 0.42, 0.75))
        for p in t['paths'][1:]:
            line(d, p['points'], (r, g, b, 70), 1 * S)
        line(d, t['paths'][0]['points'], (r, g, b, 235), 3 * S)
        if fates:
            for p in t['paths']:
                x, y = xy(p['points'][-1])
                rad = (5 if p['k'] == 0 else 3) * S
                if p['down'] == 'land':
                    d.ellipse((x - rad, y - rad, x + rad, y + rad), fill=(r, g, b, 230))
                elif p['down'] == 'sea':
                    d.ellipse((x - rad, y - rad, x + rad, y + rad), outline=(r, g, b, 230), width=S)
        x, y = xy(t['from'])
        d.ellipse((x - 4 * S, y - 4 * S, x + 4 * S, y + 4 * S), fill=(255, 255, 255, 255), outline=(40, 40, 40, 255), width=S)
    img = Image.alpha_composite(img, layer).convert('RGB')
    return img.resize((W, H), Image.LANCZOS)


def main():
    out = Path(sys.argv[1])
    month = int(sys.argv[2]) if len(sys.argv) > 2 else 7
    maybes = int(sys.argv[3]) if len(sys.argv) > 3 else 12
    out.mkdir(parents=True, exist_ok=True)
    for aloft, name in ((True, 'aloft'), (False, 'fates')):
        doc = run(month, maybes, aloft)
        path = out / f'wind-{name}-{month:02d}.png'
        draw(doc, not aloft).save(path)
        downs = [p['down'] for t in doc['things'] for p in t['paths']]
        steps = [len(p['points']) - 1 for t in doc['things'] for p in t['paths']]
        print(f'{path}: {len(doc["things"])} things x {maybes + 1} paths; mean flight {np.mean(steps) / 4:.1f} days; '
              f'down on land {downs.count("land")}, on the sea {downs.count("sea")}, still up {downs.count("aloft")}')


if __name__ == '__main__':
    main()
