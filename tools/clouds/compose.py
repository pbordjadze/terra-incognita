"""The clouds over the app's globe, for judging them without a phone: the globe as the app's shader
draws it (tools/globe/preview.py), the pack's overlay over it, and the names over that, as the app
layers them (GlobeView: the Metal globe, then a prototype's overlay, then the label layer).

    python3 tools/clouds/compose.py camera <w>x<h> <lon> <lat> <zoom>   # the overlay's camera, JSON
    python3 tools/clouds/compose.py compose <w>x<h> <lon> <lat> <zoom> <scale> <overlay.png> <out.png>
    python3 tools/clouds/compose.py frames <listing.json>                  # a film's frames

look.cjs and film.cjs run these (they draw the overlay in Chromium between them). The globe is the
Wikipedia style, or the ground with CLOUDS_GROUND=<month 1-12> (where the globe's preview has it).
"""
import json, os, sys
from pathlib import Path
import numpy as np
from PIL import Image

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'globe'))
import preview  # noqa: E402


def camera(w, h, lon, lat, zoom):
    cx, cy, r0 = preview.layout(w, h)
    return {'lon': lon, 'lat': lat, 'zoom': zoom, 'cx': cx, 'cy': cy, 'radius': r0 * zoom, 'width': w, 'height': h}


def compose(w, h, lon, lat, zoom, scale, overlay, out):
    month = os.environ.get('CLOUDS_GROUND')
    ground = {'ground': True, 'month': int(month)} if month else {}
    img, geom = preview.render(w, h, scale, lon, lat, zoom, 0, **ground)
    over = Image.open(overlay).convert('RGBA').resize(img.size, Image.LANCZOS)
    img = Image.alpha_composite(img.convert('RGBA'), over).convert('RGB')
    preview.labels(img, w, h, scale, geom, zoom, 0, False, [preview.buttons_box(w, h)], **ground)
    img.save(out)


def frames(listing, workers=3):
    """Composes every frame of a film (film.cjs writes the listing), a few at a time."""
    from multiprocessing import Pool
    jobs = json.loads(Path(listing).read_text())
    with Pool(workers) as pool:
        pool.starmap(compose, [(j['w'], j['h'], j['lon'], j['lat'], j['zoom'], j['scale'], j['overlay'], j['out']) for j in jobs])


if __name__ == '__main__':
    a = sys.argv[1:]
    if a[0] == 'frames':
        frames(a[1])
        sys.exit()
    w, h = map(int, a[1].split('x'))
    lon, lat, zoom = map(float, a[2:5])
    if a[0] == 'camera':
        print(json.dumps(camera(w, h, lon, lat, zoom)))
    else:
        compose(w, h, lon, lat, zoom, float(a[5]), a[6], a[7])
