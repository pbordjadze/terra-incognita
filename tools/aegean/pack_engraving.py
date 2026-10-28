"""The leaf's day engraving for the globe pack: Prototypes/sea-between/engraving.png.

python3 tools/aegean/pack_engraving.py SHEET.png   (run by tools/aegean/pack.cjs)

SHEET.png is the sheet pack.cjs had the leaf's own drawing code paint (the Aegean of the day
engraving, at the resolution the globe's close views need). It is kept pixel for pixel, scaled whole
only if a side would pass 4096 (the runtime's limit for a picture), so its edges and its exact
longitude and latitude bounds are unchanged. A neat line is ruled just inside its edge, as round the
leaf's own page, so the chart reads as a sheet lying on the globe. Then 256 colours, no dithering:
the engraving has few inks. Nothing is fetched.
"""
import sys
from pathlib import Path
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Prototypes' / 'sea-between' / 'engraving.png'
LIMIT = 4096

src = Image.open(sys.argv[1]).convert('RGB')
if max(src.size) > LIMIT:
    k = LIMIT / max(src.size)
    src = src.resize((round(src.width * k), round(src.height * k)), Image.Resampling.LANCZOS)
img = src.convert('RGBA')
WIDTH, height = img.size

# leaf.css: the chart's 1 px rule (#9eada7) at its edge, and a fine ink line inset (ink at 18%)
rule = Image.new('RGBA', img.size, (0, 0, 0, 0))
d = ImageDraw.Draw(rule)
line, inset = max(3, round(WIDTH / 420)), max(10, round(WIDTH / 140))
d.rectangle((0, 0, WIDTH - 1, height - 1), outline=(158, 173, 167, 255), width=line)
d.rectangle((inset, inset, WIDTH - 1 - inset, height - 1 - inset), outline=(41, 70, 82, 70), width=max(2, line // 2))
img = Image.alpha_composite(img, rule).convert('RGB')

img.quantize(colors=256, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE).save(OUT, optimize=True)
print(f'engraving.png: {WIDTH} x {height}, {OUT.stat().st_size} bytes')
