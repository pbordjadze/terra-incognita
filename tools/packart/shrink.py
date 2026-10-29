"""Shrinks a drawn PNG for a pack without changing what it shows: a picture lying on the globe is drawn
at thousands of pixels a side, and Chromium's encoder writes it at 6–12 MB; with its colours held to
64 levels a channel (its alpha kept whole, so a soft edge stays soft: a palette would band it into
rings) and Pillow's optimizer, it writes at about a quarter of that, at an error the eye can't find in a
night's ground or a morning's. The one art drawer calls this for a canvas sheet's picture that says
`shrink: 'posterize'` (tools/packart/render.cjs); a sprite's frame keeps its full colour.

    python3 tools/packart/shrink.py <png> [<png> ...]        (each rewritten in place; prints the bytes)

Standard library and Pillow only (the globe's own builders use Pillow).
"""
import sys
from pathlib import Path
from PIL import Image


def shrink(path: Path) -> tuple[int, int]:
    before = path.stat().st_size
    im = Image.open(path).convert('RGBA')
    r, g, b, a = im.split()
    held = [c.point(lambda v: (v >> 2) << 2) for c in (r, g, b)]
    Image.merge('RGBA', held + [a]).save(path, 'PNG', optimize=True)
    return before, path.stat().st_size


if __name__ == '__main__':
    if len(sys.argv) < 2:
        print(__doc__)
        sys.exit(2)
    for p in sys.argv[1:]:
        b, a = shrink(Path(p))
        print(f'{p}: {b / 1e6:.1f} MB -> {a / 1e6:.2f} MB')
