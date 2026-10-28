#!/usr/bin/env python3
"""Puts the expedition's recorded depths (fathoms) and positions into Sounding the deep's data.js.

    python3 tools/playground/sounding-the-deep/recorded-depths.py <NGDC_MURRAY1891_V01.txt> [data.js]

PANGAEA 849072's event "elevation", which the pack first used, converts the recorded fathoms at
1.6421 m a fathom rather than 1.8288 (Station 9: recorded 3,150 fathoms, 5,761 m; PANGAEA 5,172 m).
The file AWI transcribed into PANGAEA (https://epic.awi.de/38636/1/NGDC_MURRAY1891_V01.txt) has
each station's depth as whole fathoms x 1.8288 and its position as printed in Murray & Renard (1891).
This reads that file and, for every station already in data.js:
- sets `fathoms` (as recorded) and `depth` (fathoms x 1.8288, in whole metres);
- takes the file's position where PANGAEA's differs by more than 0.01 degree, except CHA-176, whose
  printed longitude (173 52 E, Murray & Renard p. 90) agrees with PANGAEA's, not the file's;
- CHA-64 is the one station the file also scales: its printed depth is (2700) fathoms (p. 56).
It changes nothing else, and says what it changed.
"""
import json
import sys

FATHOM = 1.8288
PRINTED_FATHOMS = {'CHA-64': 2700}       # Murray & Renard (1891) p. 56: "(2700)"
KEEP_POSITION = {'CHA-176'}              # p. 90 agrees with PANGAEA's longitude
PREFIX = 'window.SHIP_DATA='
HEADER = ('/* Observations: Murray and Renard (1891), recorded depths and positions via AWI\'s source file, '
          'stations via PANGAEA 849072; CC BY 3.0, attribution bundled. Natural Earth physical land: public domain. */')


def read_source(path):
    rows = {}
    for line in open(path, encoding='utf-8', errors='replace'):
        f = line.rstrip('\n').split('\t')
        if len(f) > 4 and f[0].startswith('CHA-'):
            try:
                rows.setdefault(f[0], (float(f[1]), float(f[2]), float(f[3])))
            except ValueError:
                pass
    return rows


def main():
    if len(sys.argv) < 2:
        sys.exit(__doc__)
    source = read_source(sys.argv[1])
    target = sys.argv[2] if len(sys.argv) > 2 else 'Prototypes/sounding-the-deep/data.js'
    lines = open(target, encoding='utf-8').read().split('\n')
    body = lines[1]
    assert body.startswith(PREFIX) and body.endswith(';'), 'data.js: unexpected layout'
    data = json.loads(body[len(PREFIX):-1])
    changed = []
    for o in data['observations']:
        lat, lon, elevation = source[o['station']]
        fathoms = PRINTED_FATHOMS.get(o['station'], round(-elevation / FATHOM))
        depth = round(fathoms * FATHOM)
        if o.get('depth') != depth:
            changed.append(f"{o['station']}: {o.get('depth')} m -> {fathoms} fathoms, {depth} m")
        o['fathoms'] = fathoms
        o['depth'] = depth
        if o['station'] not in KEEP_POSITION and (abs(lat - o['lat']) > 0.01 or abs(lon - o['lon']) > 0.01):
            changed.append(f"{o['station']}: position {o['lat']}, {o['lon']} -> {round(lat, 6)}, {round(lon, 6)}")
            o['lat'], o['lon'] = round(lat, 6), round(lon, 6)
    data['sources']['murrayRenard1891'] = {
        'title': 'Report on the deep-sea deposits (Report on the Scientific Results of the Voyage of H.M.S. Challenger)',
        'authors': 'Murray, John; Renard, A F', 'year': 1891,
        'url': 'https://epic.awi.de/38636/1/NGDC_MURRAY1891_V01.txt',
        'note': 'Recorded depths in fathoms and printed positions, as transcribed in the file AWI submitted to PANGAEA.'}
    data['transformations'] = (
        'Station labels and date-only event timestamps from the PANGAEA TSV event metadata, deduplicated and sorted by '
        'date. Depths are the recorded fathoms (Murray & Renard 1891, via AWI\'s source file; CHA-64 from the printed '
        'table), converted at 1.8288 m a fathom: PANGAEA 849072\'s elevation converts them at 1.6421 m a fathom. '
        'Positions as printed where PANGAEA\'s differ by more than 0.01 degree (not CHA-176, whose printed longitude '
        'agrees with PANGAEA).')
    limitations = [s for s in data['limitations'] if not s.startswith('Depth is reported')]
    limitations.append('Depth is the length of line paid out, read to the nearest marked interval and not corrected for '
                       'drift, at one station on one day: not a live depth under a continuously tracked ship.')
    data['limitations'] = limitations
    lines[0] = HEADER
    lines[1] = PREFIX + json.dumps(data, ensure_ascii=False, separators=(',', ':')) + ';'
    open(target, 'w', encoding='utf-8').write('\n'.join(lines))
    print('\n'.join(changed))
    print(f'{len(data["observations"])} stations; deepest {max(o["depth"] for o in data["observations"])} m')


if __name__ == '__main__':
    main()
