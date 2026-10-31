"""Builds the sea ice that holds a float: each month's usual ice concentration.

    python3 tools/adrift/data/ice.py

Source: Terra's own `Globe/ground/ice-MM.png` (1440 x 720), built by tools/globe/ground.py from the
NSIDC Sea Ice Index, version 4 (Fetterer et al.; monthly concentration, the month's mean over
2015-2024, 25 km polar stereographic; the pole the satellites never see counted as ice). Rebuild
those first if the source changes; this sheet only arranges them.

Out, adrift/data/ice.png, 5768 x 2166 grey: twelve 1442 x 722 tiles (sheets.py) of quarter-degree
cells, 255 all ice, 0 open water. A float is held where the concentration is 15% or more (the
Sea Ice Index's own edge, the 15% contour) and let go when the month's ice retreats past it.
"""
import numpy as np
from PIL import Image
from sheets import ROOT, months, save, tile

HOLD = 0.15


def main():
    tiles = []
    for m in range(1, 13):
        a = np.asarray(Image.open(ROOT / 'Globe' / 'ground' / f'ice-{m:02d}.png').convert('L'))
        print(f'month {m:2d}: ice {np.mean(a >= HOLD * 255):.3f} of the globe')
        tiles.append(tile(a))
    save('ice.png', months(tiles), 'L', {
        'what': 'the month\'s usual sea-ice concentration, 255 all ice',
        'cells': [1440, 720], 'tiles': [4, 3], 'border': 1, 'hold': HOLD,
        'source': 'NSIDC Sea Ice Index v4, monthly means 2015-2024, via Terra\'s Globe/ground/ice-MM.png',
        'credit': 'Fetterer, F., K. Knowles, W. N. Meier, M. Savoie and A. K. Windnagel. Sea Ice Index, Version 4. NSIDC'})


if __name__ == '__main__':
    main()
