#!/usr/bin/env python3
"""The device shell's text on the Linux check's screenshots: a stand-in for toys/ios/Sources/main.swift.

The info mark's line, the rules and their origins, the switches panel (switches.rs), the toy's title line (toy-kit's info.rs) are set by the iOS shell in the system's text,
so a check's frame lacks them; the check writes what was showing beside each shot as
`<shot>.shell.json` (size, ppp, line, rules, origins, top: the card hung from the top). This draws them as the shell lays them out, into
`<shot>.shell.png`, so a screenshot shows what the reader sees:

    python3 tools/kit/shell_text.py <check out dir> [...]     # every shot with a .shell.json under it

Stand-ins, not the device: Inter for SF (close in width), the system's default text size (callout
16 pt, footnote 13 pt), an iPhone's 34-point bottom safe area, and a blur with a dark tint for
UIKit's ultra-thin dark material. The layout numbers are main.swift's: change both together.
"""
import json
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFilter, ImageFont

FONT = '/usr/share/fonts/opentype/inter/Inter-Regular.otf'
SAFE_BOTTOM, SAFE_TOP = 34.0, 47.0


def wrap(draw, text, font, width):
    out = []
    for para in text.split('\n'):
        line = ''
        for word in para.split(' '):
            t = (line + ' ' + word).strip()
            if draw.textlength(t, font=font) <= width or not line:
                line = t
            else:
                out.append(line)
                line = word
        out.append(line)
    return out


def rules(img, text, k, top=False):
    W, H = img.size[0] / k, img.size[1] / k
    w = min(W - 32, 480)
    pad = (14, 16)
    body = ImageFont.truetype(FONT, round(16 * k))
    note = ImageFont.truetype(FONT, round(13 * k))
    d = ImageDraw.Draw(img)
    parts = text.split('\n\n')
    rows = []  # (text, font, alpha, space before in points, line height)
    for i, p in enumerate(parts[0].split('\n')):
        for j, l in enumerate(wrap(d, p, body, (w - 2 * pad[1]) * k)):
            rows.append((l, body, 0.95, 7 if i and not j else 0, 21))
    if len(parts) > 1:
        for i, l in enumerate(wrap(d, '\n'.join(parts[1:]), note, (w - 2 * pad[1]) * k)):
            rows.append((l, note, 0.64, 13 if not i else 0, 18))
    th = sum(r[3] + r[4] for r in rows)
    foot = H - 60
    h = min(th + 2 * pad[0], foot - SAFE_TOP - 8)
    x0, y0 = (W - w) / 2, (SAFE_TOP + 8 if top else foot - h)
    box = tuple(round(v * k) for v in (x0, y0, x0 + w, y0 + h))
    glass(img, box, k, 16)
    d = ImageDraw.Draw(img, 'RGBA')
    y = y0 + pad[0]
    for t, f, a, before, lh in rows:
        y += before
        if y + lh > y0 + h:
            break
        d.text(((x0 + pad[1]) * k, (y + lh * 0.12) * k), t, font=f, fill=(255, 255, 255, round(255 * a)))
        y += lh


def glass(img, box, k, radius, tint=0.75):
    """UIKit's ultra-thin dark material with the shell's tint (0.62 alpha; the switches' panel 0.3), over
    `box` (pixels)."""
    g = img.crop(box).filter(ImageFilter.GaussianBlur(14 * k))
    g = Image.blend(g, Image.new("RGB", g.size, (15, 15, 17)), tint)
    mask = Image.new('L', g.size, 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, g.size[0] - 1, g.size[1] - 1), radius=round(radius * k), fill=255)
    img.paste(g, box[:2], mask)


def caption(img, text, k):
    """The demo's line: two lines at most, on a small dark glass as wide as it needs, its middle 74 points
    above the safe area's foot."""
    W, H = img.size[0] / k, img.size[1] / k
    pad = (8, 14)
    most = min(W - 48, 560) - 2 * pad[1]
    f = ImageFont.truetype(FONT, round(16 * k))
    d = ImageDraw.Draw(img, 'RGBA')
    if d.textlength(text, font=f) > most * k and '. ' in text:  # (too long for one: break after the first sentence)
        text = text.replace('. ', '.\n', 1)
    lines = wrap(d, text, f, most * k)[:2]
    w = max(d.textlength(t, font=f) for t in lines) / k
    lh = 21
    h = lh * len(lines)
    mid = H - SAFE_BOTTOM - 74
    x0, y0 = (W - w) / 2 - pad[1], mid - h / 2 - pad[0]
    glass(img, tuple(round(v * k) for v in (x0, y0, x0 + w + 2 * pad[1], y0 + h + 2 * pad[0])), k, 14)
    d = ImageDraw.Draw(img, 'RGBA')
    for i, t in enumerate(lines):
        tw = d.textlength(t, font=f)
        d.text(((W * k - tw) / 2, (y0 + pad[0] + i * lh + lh * 0.12) * k), t, font=f, fill=(255, 255, 255, 242))


def switches(img, text, k):
    """The switches panel (main.swift's layoutSwitches): small, in the bottom right corner, its foot 60 points
    above the screen's, on a lighter glass; a row a rule, its label (footnote, right-aligned, dimmer when off)
    beside an iOS switch at three quarters size (38 by 23 points; green when on)."""
    W, H = img.size[0] / k, img.size[1] / k
    rows = [r.split('\t') for r in text.split('\n') if r.count('\t') == 1]
    pad, gap, row, sw, sh = 10, 8, 30, 38.25, 23.25
    f = ImageFont.truetype(FONT, round(13 * k))
    d = ImageDraw.Draw(img, 'RGBA')
    most = min(W - 32, 320) - 2 * pad - gap - sw
    label_w = min(max((d.textlength(lb, font=f) / k for lb, _ in rows), default=0), most)
    w = pad + label_w + gap + sw + pad
    h = pad + row * len(rows) + pad - 6
    x0, y0 = W - 16 - w, H - 60 - h
    glass(img, tuple(round(v * k) for v in (x0, y0, x0 + w, y0 + h)), k, 14, 0.4)
    d = ImageDraw.Draw(img, 'RGBA')
    for i, (label, on) in enumerate(rows):
        on = on == '1'
        mid = y0 + pad - 3 + row * (i + 0.5)
        tw = d.textlength(label, font=f)
        d.text(((x0 + pad + label_w) * k - tw, (mid - 8) * k), label, font=f,
               fill=(255, 255, 255, round(255 * (0.95 if on else 0.55))))
        sx, sy = x0 + w - pad - sw, mid - sh / 2
        track = (52, 199, 89, 255) if on else (120, 120, 128, 110)
        d.rounded_rectangle([sx * k, sy * k, (sx + sw) * k, (sy + sh) * k], radius=sh / 2 * k, fill=track)
        kx, r = sx + (sw - sh / 2 if on else sh / 2), sh / 2 - 1.5
        d.ellipse([(kx - r) * k, (mid - r) * k, (kx + r) * k, (mid + r) * k], fill=(255, 255, 255, 255))


def title(img, text, k):
    """The toy's quiet title line (Toy::title): footnote, white at 0.55, centred at the top."""
    W = img.size[0] / k
    f = ImageFont.truetype(FONT, round(13 * k))
    d = ImageDraw.Draw(img, 'RGBA')
    w = min(W - 96, 420) * k
    while d.textlength(text, font=f) > w and len(text) > 2:
        text = text[:-2] + '…'
    tw = d.textlength(text, font=f)
    d.text(((W * k - tw) / 2, (SAFE_TOP + 14 + 4) * k), text, font=f, fill=(255, 255, 255, 140))


def main():
    for root in sys.argv[1:] or ['.']:
        for j in sorted(Path(root).rglob('*.shell.json')):
            s = json.loads(j.read_text())
            shot = j.with_name(j.name[: -len('.shell.json')] + '.png')
            img = Image.open(shot).convert('RGB')
            if s.get('rules') or s.get('origins'):
                rules(img, s.get('rules') or s['origins'], s['ppp'], s.get('top', False))
            if s.get('switches'):
                switches(img, s['switches'], s['ppp'])
            if s.get('title'):
                title(img, s['title'], s['ppp'])
            if s.get('line'):
                caption(img, s['line'], s['ppp'])
            out = j.with_name(j.name[: -len('.shell.json')] + '.shell.png')
            img.save(out)
            print(out)


if __name__ == '__main__':
    main()
