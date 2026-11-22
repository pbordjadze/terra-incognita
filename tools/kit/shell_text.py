#!/usr/bin/env python3
"""The iPhone frame's chrome on the Linux check's screenshots: a stand-in for toys/ios/Sources/main.swift.

The demo's line, the sheet (the rules, their style's credit, switches and buttons, or the origins in their
place), the toy's title line and its label (Terra's postcard) are the frame's, set in the system's text, so a
check's frame lacks them; the check writes the chrome showing beside each shot as `<shot>.shell.json` (size,
ppp and `chrome`: toy-kit's chrome.rs as JSON). This draws them as the iPhone's frame lays them out, into
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
BOLD = '/usr/share/fonts/opentype/inter/Inter-SemiBold.otf'
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


# The sheet (main.swift's layoutRules): the rules, then the switches a row each, then the buttons.
PAD = (14, 16)
SWITCH_ROW, SW, SH = 38, 51, 31
MORE_ROW = 34
LINK = (125, 190, 255)
MORE = {'demo': 'Show me', 'origins': 'Where this comes from'}


def sheet_rows(d, text, w, k):
    """The sheet's text as rows: (text, font, alpha, space before in points, line height); the rules in callout,
    a dimmer footnote after a blank line."""
    body = ImageFont.truetype(FONT, round(16 * k))
    note = ImageFont.truetype(FONT, round(13 * k))
    parts = text.split('\n\n') if text else ['']
    rows = []
    for i, p in enumerate(parts[0].split('\n') if parts[0] else []):
        for j, l in enumerate(wrap(d, p, body, (w - 2 * PAD[1]) * k)):
            rows.append((l, body, 0.95, 7 if i and not j else 0, 21))
    if len(parts) > 1:
        for i, l in enumerate(wrap(d, '\n'.join(parts[1:]), note, (w - 2 * PAD[1]) * k)):
            rows.append((l, note, 0.64, 13 if not i else 0, 18))
    return rows


def sheet_switch(d, x0, w, y, label, on, k):
    """One switch's row from `y`: its label left (dimmer when off), an iOS switch right (green when on)."""
    body = ImageFont.truetype(FONT, round(16 * k))
    mid = y + SWITCH_ROW / 2
    d.text(((x0 + PAD[1]) * k, (mid - 10) * k), label, font=body, fill=(255, 255, 255, round(255 * (0.95 if on else 0.55))))
    sx, sy = x0 + w - PAD[1] - SW, mid - SH / 2
    track = (52, 199, 89, 255) if on else (120, 120, 128, 110)
    d.rounded_rectangle([sx * k, sy * k, (sx + SW) * k, (sy + SH) * k], radius=SH / 2 * k, fill=track)
    kx, r = sx + (SW - SH / 2 if on else SH / 2), SH / 2 - 2
    d.ellipse([(kx - r) * k, (mid - r) * k, (kx + r) * k, (mid + r) * k], fill=(255, 255, 255, 255))


def sheet_links(d, x0, w, y, links, k):
    """Under a hairline from `y`, the buttons in the link colour."""
    body = ImageFont.truetype(FONT, round(16 * k))
    d.line([((x0 + PAD[1]) * k, y * k), ((x0 + w - PAD[1]) * k, y * k)], fill=(255, 255, 255, 40), width=max(1, round(k / 2)))
    y += 6
    x = x0 + PAD[1]
    for t in links:
        d.text((x * k, (y + MORE_ROW / 2 - 10) * k), t, font=body, fill=LINK + (255,))
        x += d.textlength(t, font=body) / k + 24


def sheet(img, text, k, top=False, switch_text=None, more=None):
    """The sheet: at most 480 points wide, centred, its foot 60 points above the screen's (or hung from the top);
    the rules, a row a switch, and under a hairline the buttons."""
    W, H = img.size[0] / k, img.size[1] / k
    w = min(W - 32, 480)
    rows = sheet_rows(ImageDraw.Draw(img), text, w, k)
    sws = [r.split('\t') for r in (switch_text or '').split('\n') if r.count('\t') == 1]
    links = [MORE[m] for m in (more or '').split('\n') if m in MORE]
    th = sum(r[3] + r[4] for r in rows)
    sw_h = (6 if rows else 0) + SWITCH_ROW * len(sws)
    more_h = (12 + MORE_ROW) if links else 0
    foot = H - 60
    h = min(th + sw_h + more_h + 2 * PAD[0], foot - SAFE_TOP - 8)
    x0, y0 = (W - w) / 2, (SAFE_TOP + 8 if top else foot - h)
    glass(img, tuple(round(v * k) for v in (x0, y0, x0 + w, y0 + h)), k, 16)
    d = ImageDraw.Draw(img, 'RGBA')
    y = y0 + PAD[0]
    for t, f, a, before, lh in rows:
        y += before
        d.text(((x0 + PAD[1]) * k, (y + lh * 0.12) * k), t, font=f, fill=(255, 255, 255, round(255 * a)))
        y += lh
    y += 6 if rows and sws else 0
    for label, on in sws:
        sheet_switch(d, x0, w, y, label, on == '1', k)
        y += SWITCH_ROW
    if links:
        sheet_links(d, x0, w, y + 6, links, k)


def label(img, lab, k):
    """A toy's own words (Toy::label: its box, alpha, ink and lines): in its box, a heading (title 3, semibold),
    a quieter line (subheadline), the rest plain (subheadline); dark ink or light, at the label's alpha."""
    x, y, w, _h = lab['at']
    a, lines = lab['alpha'], lab['lines']
    ink = (28, 28, 30) if lab['dark'] else (255, 255, 255)
    fonts = [ImageFont.truetype(BOLD, round(20 * k)), ImageFont.truetype(FONT, round(15 * k)), ImageFont.truetype(FONT, round(15 * k))]
    alphas = [1.0, 0.6, 0.85]
    heights = [26, 20, 20]
    if a <= 0.0:
        return
    # (on a clear layer composited after: Pillow draws text whose fill has alpha 0 as if opaque)
    layer = Image.new('RGBA', img.size, (0, 0, 0, 0))
    d = ImageDraw.Draw(layer)
    for i, t in enumerate(lines):
        j = min(i, 2)
        for row in wrap(d, t, fonts[j], w * k)[:2]:
            d.text((x * k, (y + heights[j] * 0.1) * k), row, font=fonts[j], fill=ink + (round(255 * a * alphas[j]),))
            y += heights[j]
        y += 2 if i == 0 else 0
    img.paste(layer, (0, 0), layer)


def glass(img, box, k, radius, tint=0.75):
    """UIKit's ultra-thin dark material with the shell's tint (0.62 alpha; the switches' panel 0.3, drawn as 0.4), over
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


def sheet_text(sh):
    """The sheet's text as the frame sets it: the lines, then a blank line and the dimmer notes (the toy's own,
    then its style's credit)."""
    notes = ([sh['note']] if sh['note'] else []) + ([sh['style']['credit']] if sh.get('style') else [])
    body = '\n'.join(sh['lines'])
    return body + ('\n\n' + '\n'.join(notes) if notes else '')


def draw(img, c, k):
    """Everything the chrome `c` shows, over the shot."""
    if c.get('label'):
        label(img, c['label'], k)
    sh = c.get('sheet')
    if sh:
        rules = sh['page'] == 'rules'
        switches = ''.join(f"{s['label']}\t{1 if s['on'] else 0}\n" for s in sh['switches']) if rules else None
        more = ('demo\n' if sh['demo'] else '') + ('origins\n' if sh['origins'] else '') if rules else None
        sheet(img, sheet_text(sh), k, sh['top'], switches, more)
    if c.get('title'):
        title(img, c['title'], k)
    if c.get('caption'):
        caption(img, c['caption'], k)


def main():
    for root in sys.argv[1:] or ['.']:
        for j in sorted(Path(root).rglob('*.shell.json')):
            s = json.loads(j.read_text())
            shot = j.with_name(j.name[: -len('.shell.json')] + '.png')
            img = Image.open(shot).convert('RGB')
            draw(img, s['chrome'], s['ppp'])
            out = j.with_name(j.name[: -len('.shell.json')] + '.shell.png')
            img.save(out)
            print(out)


if __name__ == '__main__':
    main()
