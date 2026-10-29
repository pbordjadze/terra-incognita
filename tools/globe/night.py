"""Builds the night style's map (Globe/ground/night.jpg) from NASA's Black Marble.

    python3 tools/globe/night.py <cache-dir>

  ground/night.jpg  4096 x 2048 grey JPEG on the globe's own equirectangular grid (longitude across
                    from -180, latitude down from 90): the Earth's lights at night, 0 dark to 255 the
                    brightest city cores, as the night style draws them (Globe.metal's N_ palette:
                    the land and sea are drawn, the lights are measured). From NASA's Black Marble
                    2016, the grey composite of a year's cloud-free Suomi NPP VIIRS day-night band
                    views (13500 x 6750, 3 km), averaged down to the grid: its own tone kept, so a
                    village's glow stays faint and a capital's core white; the sea and the unlit land
                    are black in it already.

Source (fetched into the cache if missing): NASA Earth Observatory, Earth at Night 2016, the "3 km
gray" image (images by Joshua Stevens, using Suomi NPP VIIRS data from Miguel Román, NASA GSFC;
NASA imagery is in the public domain). Needs numpy and Pillow.
"""
import sys, urllib.request
from pathlib import Path
import numpy as np
from PIL import Image

Image.MAX_IMAGE_PIXELS = None
ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / 'Globe' / 'ground' / 'night.jpg'
SIZE = (4096, 2048)
URL = 'https://eoimages.gsfc.nasa.gov/images/imagerecords/144000/144897/BlackMarble_2016_3km_gray.jpg'


def fetch(url, path):
    if not path.exists():
        print('fetching', url, flush=True)
        path.parent.mkdir(parents=True, exist_ok=True)
        urllib.request.urlretrieve(url, path)
    return path


def main():
    cache = Path(sys.argv[1]) if len(sys.argv) > 1 else ROOT / 'build' / 'night'
    src = Image.open(fetch(URL, cache / 'BlackMarble_2016_3km_gray.jpg')).convert('L')
    # averaged down (the lights are points: a box filter keeps their sum, a Lanczos would ring round them)
    small = np.asarray(src.resize(SIZE, Image.BOX)).astype(np.float32)
    # the faintest glow (JPEG noise in the source's black) cut, the rest as it is
    small = np.where(small < 6, 0, small)
    Image.fromarray(small.astype(np.uint8), 'L').save(OUT, quality=88, optimize=True)
    lit = (small > 0).mean()
    print(f'{OUT.relative_to(ROOT)}: {SIZE[0]}x{SIZE[1]} grey, {OUT.stat().st_size / 1e6:.2f} MB, {100 * lit:.1f}% of cells lit')


if __name__ == '__main__':
    main()
