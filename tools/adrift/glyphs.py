#!/usr/bin/env python3
"""Builds the look's type: a signed-distance atlas of Libertinus Serif, Regular and Italic, for the date,
the note and the credits (adrift/crates/adrift-core/src/look/words.wgsl draws from it at any size, with
its halo).

    python3 tools/adrift/glyphs.py            # writes look/type/libertinus.png and libertinus.json

Each glyph is rasterised by PIL from Globe/fonts/LibertinusSerif-Regular.otf at EM pixels an em and
turned into a signed distance (an exact Euclidean transform in numpy, Felzenszwalb and Huttenlocher's
two passes, in texels, positive inside), stored as (d / SPREAD) * 0.5 + 0.5 in an 8-bit atlas. The
JSON gives, per character, its advance and bearings in texels at EM, and its rectangle in the atlas
(texels), which the Rust scales to points. Libertinus's kerning is in GPOS, which PIL's basic layout
doesn't apply, so none is written: the date and the credits are short lines set with their tracking.
The em is 64 texels: the words are 10 and 17 points at two or three pixels a point (20 to 77 pixels
with the reader's text size), so the atlas is sampled near 1:1 and never minified past a half; the
Rust gives it mipmaps for the rest. Needs numpy and Pillow only. Never hand-edit the output.
"""
import json
from pathlib import Path
import numpy as np
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
FONTS = {'regular': ROOT / 'Globe' / 'fonts' / 'LibertinusSerif-Regular.otf', 'italic': ROOT / 'Globe' / 'fonts' / 'LibertinusSerif-Italic.otf'}
OUT = ROOT / 'adrift' / 'crates' / 'adrift-core' / 'src' / 'look' / 'type'
EM, SPREAD, PAD = 64, 8, 14
SS = 4                         # rasterised at SS times the em and thresholded there, so hairlines survive
CHARS = [chr(c) for c in range(32, 127)] + ['·', '–', '—', '’', '‘', '“', '”', 'é', 'ü', 'ö', 'ñ', '°', '█']   # (the block: a solid cell the look draws pages with)
INF = 1e12


def edt1(f):
    """The squared distance transform of a row of costs (INF where empty): the lower envelope of parabolas."""
    n = len(f)
    d, v, z = np.empty(n), np.zeros(n, int), np.empty(n + 1)
    k, v[0], z[0], z[1] = 0, 0, -INF, INF
    for q in range(1, n):
        s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k])
        while s <= z[k]:
            k -= 1
            s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k])
        k += 1
        v[k], z[k], z[k + 1] = q, s, INF
    k = 0
    for q in range(n):
        while z[k + 1] < q:
            k += 1
        d[q] = (q - v[k]) ** 2 + f[v[k]]
    return d


def edt(mask):
    """Euclidean distance from each False cell to the nearest True cell."""
    f = np.where(mask, 0.0, INF)
    g = np.stack([edt1(f[:, j]) for j in range(f.shape[1])], 1)
    return np.sqrt(np.stack([edt1(g[i]) for i in range(g.shape[0])], 0))


def sdf(mask):
    d = np.where(mask, edt(~mask) - 0.5, -(edt(mask) - 0.5))
    enc = np.clip(d / (SPREAD * SS) * 0.5 + 0.5, 0, 1)
    h, w = enc.shape
    return enc.reshape(h // SS, SS, w // SS, SS).mean((1, 3))      # (box-filtered down to the em)


def face(name, cells):
    """Rasterises one face's glyphs into cells (appended) and returns its metrics and glyph table."""
    font = ImageFont.truetype(str(FONTS[name]), EM)
    big = ImageFont.truetype(str(FONTS[name]), EM * SS)
    glyphs = {}
    for ch in CHARS:
        l, t, r, b = font.getbbox(ch, anchor='ls')
        w, h = r - l + 1, b - t + 1
        adv = font.getlength(ch)
        if r - l <= 0 or b - t <= 0:
            glyphs[ch] = {'advance': adv, 'left': 0, 'top': 0, 'w': 0, 'h': 0, 'x': 0, 'y': 0}
            continue
        im = Image.new('L', ((w + 2 * PAD) * SS, (h + 2 * PAD) * SS), 0)
        ImageDraw.Draw(im).text(((PAD - l) * SS, (PAD - t) * SS), ch, font=big, fill=255, anchor='ls')
        if ch == '█':                                     # solid to its cell's edge: inside everywhere
            cells.append(((name, ch), np.ones(((h + 2 * PAD), (w + 2 * PAD)), np.float32), l - PAD, t - PAD, adv))
            continue
        cells.append(((name, ch), sdf(np.asarray(im) > 127), l - PAD, t - PAD, adv))
    asc, desc = font.getmetrics()
    return {'font': FONTS[name].stem, 'ascent': asc, 'descent': desc, 'glyphs': glyphs}


def main():
    cells, faces = [], {}
    for name in FONTS:
        faces[name] = face(name, cells)
    W, x, y, rowh = 1024, 0, 0, 0                            # packed in rows
    for (name, ch), cell, left, top, adv in cells:
        h, w = cell.shape
        if x + w > W:
            x, y, rowh = 0, y + rowh, 0
        faces[name]['glyphs'][ch] = {'advance': adv, 'left': left, 'top': top, 'w': w, 'h': h, 'x': x, 'y': y}
        x, rowh = x + w, max(rowh, h)
    H = y + rowh
    atlas = np.zeros((H, W), np.float32)
    for (name, ch), cell, *_ in cells:
        g = faces[name]['glyphs'][ch]
        atlas[g['y']:g['y'] + g['h'], g['x']:g['x'] + g['w']] = cell
    OUT.mkdir(parents=True, exist_ok=True)
    Image.fromarray((atlas * 255 + 0.5).astype(np.uint8)).save(OUT / 'libertinus.png', optimize=True)
    (OUT / 'libertinus.json').write_text(json.dumps({
        'about': 'Signed-distance atlas of Libertinus Serif Regular and Italic (OFL, Globe/fonts) for words.wgsl: built by tools/adrift/glyphs.py, never hand-edited. Texels at em pixels an em; a glyph is drawn at (pen.x + left, baseline + top) with its w x h, scaled by size / em; the atlas holds (distance / spread) * 0.5 + 0.5, positive inside.',
        'em': EM, 'spread': SPREAD, 'width': W, 'height': H, 'faces': faces}, ensure_ascii=False))
    print(OUT / 'libertinus.png', W, H, len(cells), 'glyphs')


if __name__ == '__main__':
    main()
