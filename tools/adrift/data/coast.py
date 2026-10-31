"""Builds the coast that holds a float: the globe's own signed distance to the shore.

    python3 tools/adrift/data/coast.py

Source: Terra's `Globe/geo.png` (4096 x 2048), built by tools/globe/build.py from Natural Earth's
1:50m land and lakes (public domain), its coast rasterised at 8192 and halved. Rebuild that first if
the source changes; this sheet only takes two of its channels.

Out, adrift/data/coast.png, 4096 x 2048 grey + alpha (two channels, no border: the app wraps the
longitude itself):
      L  the fine distance: 128 + 8 d for |d| <= 16 px, land positive
      A  the coarse one: 128 + d / 2 for |d| <= 254 px
in pixels of this sheet (one is 360 / 4096 degrees of longitude, about 9.8 km at the equator,
less toward the poles east-west). The rules read L within its range and A beyond; the sea is
where d < 0, and the gradient points to the land, so a float that steps onto it is put back at
the water's edge.
"""
import numpy as np
from PIL import Image
from sheets import ROOT, save


def main():
    geo = np.asarray(Image.open(ROOT / 'Globe' / 'geo.png').convert('RGBA'))
    la = np.stack([geo[..., 0], geo[..., 1]], -1)
    print(f'sea {np.mean(geo[..., 1] < 128):.3f} of the sheet')
    save('coast.png', la, 'LA', {
        'what': 'signed distance to the coast, fine in L (128 + 8 d, |d| <= 16 px) and coarse in A (128 + d / 2), land positive',
        'cells': [4096, 2048], 'tiles': [1, 1], 'border': 0, 'unit': 'px of 360/4096 degrees',
        'source': 'Natural Earth 1:50m land and lakes, via Terra\'s Globe/geo.png',
        'credit': 'Made with Natural Earth (public domain)'})


if __name__ == '__main__':
    main()
