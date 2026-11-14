"""Lays a headless check's shots side by side, small, to look at a run at a glance.

    python3 tools/toys/pando_sheet.py <out-dir> <sheet.png> [name ...]

With no names, every PNG in the directory, in time order. (Written for Pando; any toy's run works.)
"""
import sys
from pathlib import Path

from PIL import Image


def main() -> None:
    out, sheet = Path(sys.argv[1]), Path(sys.argv[2])
    names = sys.argv[3:]
    files = [out / f"{n}.png" for n in names] if names else sorted(out.glob("*.png"), key=lambda p: p.stat().st_mtime)
    ims = [Image.open(f).convert("RGB") for f in files]
    h = 640
    ims = [im.resize((round(im.width * h / im.height), h)) for im in ims]
    s = Image.new("RGB", (sum(i.width for i in ims) + 8 * (len(ims) - 1), h), (240, 238, 232))
    x = 0
    for im in ims:
        s.paste(im, (x, 0))
        x += im.width + 8
    s.save(sheet)


if __name__ == "__main__":
    main()
