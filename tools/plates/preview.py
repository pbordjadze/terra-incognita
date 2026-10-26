#!/usr/bin/env python3
"""What the app shows of a look's plates, without a phone: the same camera, film and layering as
App/TerraIncognita/Atlas (Camera.swift, AtlasCanvas.swift), rendered with Pillow.

    tools/plates/preview.py <look> --times 4 9.5 20      frames at those moments: preview-<look>.png
    tools/plates/preview.py <look> --video out.mp4       the whole film (needs ffmpeg)
      [--screen 402x874] [--scale 2] [--fps 30] [--out <dir>]

Keep the port in step with the Swift: thresholds, easing, drift and dive timings.
"""
import argparse
import json
import math
import os
import subprocess
import sys

from PIL import Image, ImageDraw

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', '..')


class Ladder:
    def __init__(self, plates, sw, sh):
        self.p, self.sw, self.sh, self.n = plates, sw, sh, len(plates)

    def cover(self, k): return max(self.sw / self.p[k]['size'][0], self.sh / self.p[k]['size'][1])
    def rest_zoom(self, k): return 1.4 * self.cover(k)
    def ratio(self, k): return self.p[k]['child'][2] / self.p[k + 1]['size'][0]

    def up(self, c):
        k = c[0] - 1; f = self.ratio(k); ox, oy = self.p[k]['child'][:2]
        return (k, ox + c[1] * f, oy + c[2] * f, c[3] / f)

    def down(self, c):
        k = c[0]; f = self.ratio(k); ox, oy = self.p[k]['child'][:2]
        return (k + 1, (c[1] - ox) / f, (c[2] - oy) / f, c[3] * f)

    def express(self, c, k):
        while c[0] > k: c = self.up(c)
        while c[0] < k: c = self.down(c)
        return c

    def normalized(self, c):
        while c[0] < self.n - 1 and self.down(c)[3] >= 0.6 * self.rest_zoom(c[0] + 1): c = self.down(c)
        while c[0] > 0 and c[3] < 0.55 * self.rest_zoom(c[0]): c = self.up(c)
        return c

    def opacity(self, k, z):
        if k == 0: return 1
        start = max(0.25 * self.rest_zoom(k), 1.3 * self.rest_zoom(k - 1) * self.ratio(k - 1))
        return smoothstep(start, max(0.6 * self.rest_zoom(k), start * 1.25), z)
    def rest(self, k): return (k, self.p[k]['size'][0] / 2, self.p[k]['size'][1] / 2, self.rest_zoom(k))
    def stop(self, k):
        while k > 0 and self.p[k].get('pass'): k -= 1
        return k


def smoothstep(a, b, x):
    t = min(max((x - a) / (b - a), 0), 1)
    return t * t * (3 - 2 * t)


def ease(t):
    t = min(max(t, 0), 1)
    return 4 * t * t * t if t < 0.5 else 1 - (-2 * t + 2) ** 3 / 2


def glide(a, b, e):
    z = math.exp(math.log(a[3]) + (math.log(b[3]) - math.log(a[3])) * e)
    w = e if abs(b[3] - a[3]) < 1e-9 else (1 / a[3] - 1 / z) / (1 / a[3] - 1 / b[3])
    return (a[0], a[1] + (b[1] - a[1]) * w, a[2] + (b[2] - a[2]) * w, z)


def film(L):
    shots, t = [], 0.0
    stops = [k for k in range(L.n) if not L.p[k].get('pass')]
    for i, k in enumerate(stops):
        last = i == len(stops) - 1
        s = L.rest(k)
        tg = s if last else L.express(L.rest(stops[i + 1]), k)
        d = list(s)
        if last:
            d[3] *= 1.18; d[2] -= L.p[k]['size'][1] * 0.04
        else:
            d[1] += (tg[1] - s[1]) * 0.3; d[2] += (tg[2] - s[2]) * 0.3; d[3] *= 1.12
        d = tuple(d)
        dwell = 8.0 if i == 0 else 11.0 if last else 7.0
        shots.append((t, t + dwell, s, d, True)); t += dwell
        if not last:
            dive = 2.4 + 0.8 * math.log(tg[3] / d[3])
            shots.append((t, t + dive, d, tg, False)); t += dive
    return shots, t


def camera_at(shots, t):
    sh = next((s for s in shots if t < s[1]), shots[-1])
    return glide(sh[2], sh[3], ease((t - sh[0]) / (sh[1] - sh[0]))), sh


class Images:
    """Each layer's picture, with halved copies for drawing it small without shimmer."""
    def __init__(self, folder):
        self.folder, self.cache = folder, {}

    def get(self, f, reduce=1):
        key = (f, reduce)
        if key not in self.cache:
            if reduce == 1:
                self.cache[key] = Image.open(os.path.join(self.folder, f)).convert('RGBA')
            else:
                self.cache[key] = self.get(f, reduce // 2).reduce(2)
        return self.cache[key]


def motion_at(m, t):
    """offset (x, y), rotation (degrees, about anchor), opacity factor"""
    if not m: return (0, 0), 0, 1
    ph = 2 * math.pi * (t / m.get('period', 6) + m.get('phase', 0))
    k = m['kind']
    if k == 'bob': return (m['amp'][0] * math.sin(ph), m['amp'][1] * math.sin(ph)), 0, 1
    if k == 'sway': return (0, 0), m['angle'] * math.sin(ph), 1
    if k == 'breathe': return (0, 0), 0, m['min'] + (m['max'] - m['min']) * (0.5 + 0.5 * math.sin(ph))
    return (0, 0), 0, 1


def feather_mask(size, rect, width):
    """A plate's footprint on screen, its edges fading over `width` pixels (the app's edge mask)."""
    from PIL import ImageFilter
    m = Image.new('L', size, 0)
    x0, y0, x1, y1 = rect
    inset = width / 2
    ImageDraw.Draw(m).rectangle([x0 + inset, y0 + inset, x1 - inset, y1 - inset], fill=255)
    return m.filter(ImageFilter.GaussianBlur(max(width / 3, 0.1))) if width > 1 else m


def draw_plate(canvas, L, imgs, k, c, opacity, t, S, feather=0, names=1):
    """Draws plate k at camera c (in plate k) onto canvas (screen x S pixels); feather is the edge
    fade, as a fraction of the plate's width, and names the opacity of its labels layer."""
    p = L.p[k]
    W, H = p['size']
    z = c[3] * S                                     # canvas pixels per plate pixel
    s = L.rest_zoom(k) / c[3]
    off = (-(c[1] - W / 2) * s, -(c[2] - H / 2) * s)
    cw, ch = canvas.size
    for spec in p['layers']:
        m = spec.get('motion')
        if m and m['kind'] == 'sheen':
            continue                                 # a moving highlight: left to the app
        d = spec.get('depth', 0)
        (mx, my), rot, fade = motion_at(m, t)
        fx, fy, fw, fh = spec.get('frame', [0, 0, W, H])
        ox, oy = fx + off[0] * d + mx, fy + off[1] * d + my
        # the layer's picture, reduced while it is drawn at less than half its pixels
        reduce = 1
        while z * reduce < 0.5 and reduce < 8: reduce *= 2
        img = imgs.get(spec['file'], reduce)
        if rot:
            ax, ay = m.get('anchor', [fx + fw / 2, fy + fh / 2])
            img = img.rotate(-rot, resample=Image.BICUBIC, center=((ax - fx) / reduce, (ay - fy) / reduce))
        # screen (x, y) -> layer picture: affine inverse
        a = 1 / (z * reduce)
        cx = c[1] - cw / 2 / z
        cy = c[2] - ch / 2 / z
        data = (a, 0, (cx - ox) / reduce, 0, a, (cy - oy) / reduce)
        part = img.transform((cw, ch), Image.AFFINE, data, resample=Image.BILINEAR, fillcolor=(0, 0, 0, 0))
        alpha = opacity * fade * (names if spec['file'].split('/')[-1] == 'labels.png' else 1)
        if alpha < 1:
            part.putalpha(part.getchannel('A').point(lambda v: int(v * alpha)))
        if feather > 0:
            x0, y0 = (0 - cx) * z, (0 - cy) * z
            mask = feather_mask((cw, ch), (x0, y0, x0 + W * z, y0 + H * z), feather * W * z)
            part.putalpha(Image.composite(part.getchannel('A'), Image.new('L', (cw, ch), 0), mask))
        canvas.alpha_composite(part)


def frame(L, imgs, c, t, S):
    canvas = Image.new('RGBA', (int(L.sw * S), int(L.sh * S)), (18, 20, 25, 255))
    b = c[0]
    current = L.opacity(b, c[3])
    covers = c[3] >= L.cover(b) * 0.999
    vis = []
    if b > 0 and (current < 1 or not covers): vis.append((b - 1, 1))
    vis.append((b, 1 if not vis else current))
    if b + 1 < L.n:
        o = L.opacity(b + 1, L.down(c)[3])
        if o > 0.001: vis.append((b + 1, o))
    for k, o in vis:
        ck = L.express(c, k)
        # as the app: soft edges while a plate fades up, hardening as it takes over, and narrow ones
        # while the plate behind shows past it
        fade = FEATHER * (1 - o) if k > b or (k == b and o < 1) else 0
        narrow = 0.04 if k > b else 0.04 * (1 - smoothstep(0.85, 1, ck[3] / L.cover(k))) if k == b and k > 0 else 0
        # names fade once a dive swells them past about twice their size
        s = L.stop(k)
        names = 1 - smoothstep(1.7, 2.8, L.express(c, s)[3] / L.rest_zoom(s))
        draw_plate(canvas, L, imgs, k, ck, o, t, S, feather=max(fade, narrow), names=names)
    return canvas


FEATHER = 0.2


def caption(L, sh, t):
    if not sh[4] or not (sh[0] + 0.6 < t < sh[1] - 0.4): return None
    return L.p[sh[2][0]]['caption']


def overlay_caption(img, cap, S):
    if not cap: return img
    d = ImageDraw.Draw(img)
    w, h = img.size
    grad = Image.new('L', (1, 256), 0)
    for y in range(256): grad.putpixel((0, y), int(140 * (y / 255) ** 1.5))
    shade = Image.new('RGBA', (w, int(h * 0.24)), (0, 0, 0, 255))
    shade.putalpha(grad.resize((w, int(h * 0.24))))
    img.alpha_composite(shade, (0, h - int(h * 0.24)))
    y = h - int(110 * S)
    d.text((24 * S, y), cap['eyebrow'].upper(), fill=(220, 115, 85), font_size=int(11 * S))
    d.text((24 * S, y + 18 * S), cap['title'], fill=(247, 235, 205), font_size=int(26 * S))
    d.text((24 * S, y + 52 * S), cap['text'], fill=(235, 222, 194), font_size=int(14 * S))
    return img


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('look')
    ap.add_argument('--times', type=float, nargs='*')
    ap.add_argument('--video')
    ap.add_argument('--screen', default='402x874')
    ap.add_argument('--scale', type=float, default=2)
    ap.add_argument('--fps', type=int, default=30)
    ap.add_argument('--out', default='.')
    a = ap.parse_args()
    sw, sh = map(int, a.screen.split('x'))
    folder = os.path.join(ROOT, 'Atlas', a.look)
    plates = json.load(open(os.path.join(folder, 'plates.json')))['plates']
    L = Ladder(plates, sw, sh)
    imgs = Images(folder)
    shots, duration = film(L)
    if a.times:
        frames = []
        for t in a.times:
            c, shot = camera_at(shots, t)
            frames.append(overlay_caption(frame(L, imgs, L.normalized(c), t, a.scale), caption(L, shot, t), a.scale))
        w, h = frames[0].size
        sheet = Image.new('RGB', (w * len(frames) + 8 * (len(frames) + 1), h + 16), 'white')
        for i, f in enumerate(frames): sheet.paste(f.convert('RGB'), (8 + i * (w + 8), 8))
        path = os.path.join(a.out, f'preview-{a.look}.png')
        sheet.save(path)
        print(path, f'film {duration:.1f} s')
    if a.video:
        S = a.scale
        w, h = int(sw * S) // 2 * 2, int(sh * S) // 2 * 2
        ff = subprocess.Popen(['ffmpeg', '-y', '-loglevel', 'error', '-f', 'rawvideo', '-pix_fmt', 'rgb24', '-s', f'{w}x{h}',
                               '-r', str(a.fps), '-i', '-', '-pix_fmt', 'yuv420p', '-crf', '20', a.video], stdin=subprocess.PIPE)
        for i in range(int(duration * a.fps) + a.fps):
            t = min(i / a.fps, duration)
            c, shot = camera_at(shots, t)
            img = overlay_caption(frame(L, imgs, L.normalized(c), t, S), caption(L, shot, t), S).convert('RGB').crop((0, 0, w, h))
            ff.stdin.write(img.tobytes())
        ff.stdin.close(); ff.wait()
        print(a.video, f'{duration:.1f} s')


if __name__ == '__main__':
    sys.exit(main())
