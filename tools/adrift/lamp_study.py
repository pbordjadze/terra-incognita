#!/usr/bin/env python3
"""Before/after sheets of the lamps (R25: lamps with character), from two `adrift-check play` runs.

    python3 tools/adrift/lamp_study.py phone <before.png> <after.png> <out.png>
    python3 tools/adrift/lamp_study.py close <out.png> <day.png> <night.png> x,y [x,y ...]
    python3 tools/adrift/lamp_study.py frames <out.png> <label> <png> [<label> <png> ...]

`phone`: the two screenshots side by side at half size (what a phone shows), labelled nothing (no
text: the left is before, the right after). `close`: crops round each lamp (pixels in the 1179x2556
shot), x3 nearest-neighbour, day over night, so the glass, the flame and the light on the water can
be judged pixel by pixel. `frames`: stills (tiktok.sh's at-<s>s.png, master left, re-encode right)
stacked, before over after. Run the plans first:

    adrift-check play adrift/crates/adrift-core/src/look/plans/seven-lamps.json <dir>        (and -night)
"""
import sys

from PIL import Image


def phone(b, a, out):
    b, a = Image.open(b).convert("RGB"), Image.open(a).convert("RGB")
    w, h = b.size[0] // 2, b.size[1] // 2
    sheet = Image.new("RGB", (2 * w + 12, h), (255, 255, 255))
    sheet.paste(b.resize((w, h), Image.LANCZOS), (0, 0))
    sheet.paste(a.resize((w, h), Image.LANCZOS), (w + 12, 0))
    sheet.save(out)


def close(out, day, night, pts):
    rows = [Image.open(p).convert("RGB") for p in (day, night)]
    cw, ch, k = 90, 100, 3
    sheet = Image.new("RGB", (cw * k * len(pts), ch * k * len(rows)))
    for j, im in enumerate(rows):
        for i, (x, y) in enumerate(pts):
            c = im.crop((x - cw // 2, y - 40, x + cw // 2, y - 40 + ch)).resize((cw * k, ch * k), Image.NEAREST)
            sheet.paste(c, (i * cw * k, j * ch * k))
    sheet.save(out)


def frames(out, pairs):
    ims = [Image.open(p).convert("RGB") for _, p in pairs]
    w = max(i.size[0] for i in ims)
    sheet = Image.new("RGB", (w, sum(i.size[1] for i in ims) + 8 * (len(ims) - 1)), (255, 255, 255))
    y = 0
    for i in ims:
        sheet.paste(i, (0, y))
        y += i.size[1] + 8
    sheet.save(out)


if __name__ == "__main__":
    cmd = sys.argv[1]
    if cmd == "phone":
        phone(*sys.argv[2:5])
    elif cmd == "close":
        close(sys.argv[2], sys.argv[3], sys.argv[4], [tuple(int(v) for v in s.split(",")) for s in sys.argv[5:]])
    elif cmd == "frames":
        a = sys.argv[3:]
        frames(sys.argv[2], list(zip(a[0::2], a[1::2])))
    else:
        sys.exit(__doc__)
