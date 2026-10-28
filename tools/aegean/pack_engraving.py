"""The leaf's day engraving for the globe pack: Prototypes/sea-between/engraving.png.

python3 tools/aegean/pack_engraving.py LEFT TOP RIGHT BOTTOM   (run by tools/aegean/pack.cjs)

The box, in whole pixels of Leaf/data/day.png (3142 x 2772, painted by tools/aegean/paint.cjs), is
the sheet pack.cjs chose; it keeps the painting's pixels (one is about a pixel on the 13-inch iPad at
the globe's closest zoom), scaled whole only if a side would pass 2048, so its edges and its exact
longitude and latitude bounds are unchanged. A neat line is ruled just inside its edge, as round the
leaf's own page, so the chart reads as a sheet lying on the globe. Then 256 colours, no dithering:
the engraving has few inks. Nothing is fetched.
"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
SRC = ROOT / 'Leaf' / 'data' / 'day.png'
OUT = ROOT / 'Prototypes' / 'sea-between' / 'engraving.png'
LIMIT = 2048

src = Image.open(SRC).convert('RGB').crop(tuple(int(v) for v in sys.argv[1:5]))
if max(src.size) > LIMIT:
    k = LIMIT / max(src.size)
    src = src.resize((round(src.width * k), round(src.height * k)), Image.Resampling.LANCZOS)
img = src.convert('RGBA')
WIDTH, height = img.size

# leaf.css: the chart's 1 px rule (#9eada7) at its edge, and a fine ink line inset (ink at 18%)
rule = Image.new('RGBA', img.size, (0, 0, 0, 0))
d = ImageDraw.Draw(rule)
d.rectangle((0, 0, WIDTH - 1, height - 1), outline=(158, 173, 167, 255), width=4)
d.rectangle((12, 12, WIDTH - 13, height - 13), outline=(41, 70, 82, 70), width=2)
img = Image.alpha_composite(img, rule).convert('RGB')

img.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(OUT, optimize=True)
print(f'engraving.png: {WIDTH} x {height}, {OUT.stat().st_size} bytes')
