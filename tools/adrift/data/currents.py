"""Builds the current a float rides: Terra's own ocean sheets, rebuilt into adrift/data.

    python3 tools/adrift/data/currents.py <cache>

Source and layout: tools/ocean/build.py (the NOAA Global Drifter Program's climatology of
near-surface currents, Laurindo, Mariano and Lumpkin 2017, version 3.10, CC BY 4.0; its
`drifter_monthlymeans.nc` is downloaded into <cache> if missing, about 350 MB). This runs that
builder as it is and writes its two sheets here as well:

  currents.png  2888 x 1086 RGB, twelve 722 x 362 tiles of half-degree cells: R the eastward
                current, G the northward, 128 + 127 s sqrt(|v| / 2 m/s); B how measured, 255 where
                the climatology has the month's value, falling to 0 within about 3 degrees of it
  stream.png    the same grid: the month's stream function, (psi + 0.2) / 0.4 as sixteen bits
                (R high, G low), psi in m/s radians on the unit sphere

Adrift reads the same bytes Terra does, so the two seas agree to the texel; change the one
builder, and both.
"""
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'tools' / 'ocean'))
import build                                                      # noqa: E402  (tools/ocean/build.py)
from sheets import MANIFEST, OUT                                  # noqa: E402
import json                                                       # noqa: E402


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    build.OUT = [OUT / 'currents.png']
    build.STREAM = [OUT / 'stream.png']
    build.main()
    manifest = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else {}
    credit = ('This study used data collected and made freely available by the NOAA Global Drifter '
              'Program (https://www.aoml.noaa.gov/phod/gdp/) accessed from '
              'https://erddap.aoml.noaa.gov/gdp/erddap/index.html')
    manifest['currents'] = {
        'file': 'currents.png', 'width': 2888, 'height': 1086,
        'what': 'the month\'s mean near-surface current, eastward in R, northward in G, how measured in B',
        'cells': [720, 360], 'tiles': [4, 3], 'border': 1,
        'decode': 'sign(b - 128) * ((b - 128) / 127)^2 * max', 'max': build.VMAX, 'unit': 'm/s',
        'source': 'NOAA GDP drifter climatology v3.10 monthly means (Laurindo, Mariano and Lumpkin 2017), CC BY 4.0',
        'credit': credit}
    manifest['stream'] = {
        'file': 'stream.png', 'width': 2888, 'height': 1086,
        'what': 'the month\'s stream function, sixteen bits (R high, G low)',
        'cells': [720, 360], 'tiles': [4, 3], 'border': 1,
        'decode': '(R * 256 + G) / 65535 * 0.4 - 0.2', 'unit': 'm/s radians on the unit sphere',
        'source': 'solved from the currents sheet by tools/ocean/build.py', 'credit': credit}
    MANIFEST.write_text(json.dumps(manifest, indent=2, sort_keys=True) + '\n')


if __name__ == '__main__':
    main()
