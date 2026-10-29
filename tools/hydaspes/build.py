"""Builds the night crossing's ground and ways (Prototypes/hydaspes/) from the Jhelum's course today.

    python3 tools/hydaspes/build.py <cache dir>

Source: OpenStreetMap (© OpenStreetMap contributors, ODbL), read through the plain map call of its API
in half-degree tiles over the river between Jhelum city and Jalalpur Sharif (lon 73.2–74.0, lat
32.4–33.0), saved in <cache dir>/*.osm (downloaded there if missing; the tiles are not committed). From
it: the river's water as mapped (multipolygons and ways tagged natural=water, water=river), its
centreline (the ways named Jhelum River), the seasonal torrents off the Salt Range (waterway=stream),
the riverine woods and scrub (natural=wood, scrub, wetland). Canals and roads are left out: they came
later.

What is authored here, not read, is marked AUTHORED: the camps (Stein's reading of Arrian: Alexander's
great camp at Haranpur on the right bank, Porus opposite), the wooded headland at the bend by Jalalpur
and the island before it (Arrian 5.11: real landforms of the kind, placed on today's bend), the second
channel, the march route inland behind the hills' skirt (5.12: "keeping far away from the bank"), the
level sandy field Porus chose (5.15), and the hours of the night. The river has moved since 326 BCE and
the site is argued (SOURCES.md): this is a drawing after today's river, not a survey.

Writes:
  tools/hydaspes/terrain.js      window.TERRAIN: what the art sheet draws from (lon/lat), with the boxes
  Prototypes/hydaspes/data.js    window.HYDASPES: the dated ways, the chain the drag follows, the pace,
                                 the ticks, the pictures' bounds, the river's line for the far view
Rebuild, never hand-edit either.
"""
import json, math, os, sys, time, urllib.request, xml.etree.ElementTree as ET
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT_TOOL = ROOT / 'tools' / 'hydaspes'
OUT_PACK = ROOT / 'Prototypes' / 'hydaspes'
TILES = ['73.20,32.40,73.40,32.70', '73.40,32.40,73.60,32.55', '73.40,32.55,73.60,32.70', '73.60,32.40,74.00,32.70',
         '73.20,32.70,73.60,33.00', '73.60,32.70,73.80,33.00', '73.80,32.70,74.00,33.00']

# The pictures' bounds [west, south, east, north]: the wide valley (both camps and the headland), and
# the crossing close up; each about 4096 pixels wide, the rows a little stretched so ground pixels are
# near square (the pack's pictures are equirectangular).
WIDE = [73.22, 32.46, 73.86, 33.04]
CLOSE = [73.375, 32.595, 73.525, 32.715]

# AUTHORED: the places of the night (lon, lat). Haranpur is the right bank's village at the rail
# bridge; the camps lie a kilometre back from the water on either side. The headland is the wood at the
# bend north-east of Jalalpur Sharif; the island lies in the channel before it; the landing is on the
# left bank beyond the second channel; the field a few kilometres east, on the plain.
HARANPUR = (73.640, 32.862)
JALALPUR = (73.4055, 32.658)
BEND = (73.4395, 32.6515)            # the bend by Jalalpur where the headland meets the water
FIELD = (73.500, 32.627)             # the level sandy ground where Porus drew up
MONG = (73.5091, 32.6474)

# AUTHORED: the hours of the night, from dusk (18:00 of the day before, hour 0) to noon (18). The
# whole way to years: hour / 8766 after Y0 (the battle's year, 326 BCE; Arrian's season, the rivers in
# flood at midsummer).
Y0 = -325.37
H = {
    'dusk': 0.0, 'leave': 2.0, 'storm': 4.5, 'headland': 8.0, 'boats': 9.5, 'island': 10.5, 'calm': 10.75,
    'formed': 11.0, 'bank': 11.5, 'ford': 12.5, 'dawn': 11.2, 'chariots': 12.0, 'routed': 13.4,
    'porus': 13.5, 'field': 15.0, 'battle': 15.5, 'sweep': 16.0, 'turn': 16.6, 'craterus': 16.8,
    'rout': 17.4, 'end': 18.0,
}


# ---- reading the map ---------------------------------------------------------------------------------

def fetch(cache: Path):
    cache.mkdir(parents=True, exist_ok=True)
    for i, bbox in enumerate(TILES, 1):
        f = cache / f'tile{i}.osm'
        if f.exists() and f.stat().st_size > 1000:
            continue
        url = 'https://api.openstreetmap.org/api/0.6/map?bbox=' + bbox
        print('fetching', url)
        with urllib.request.urlopen(url, timeout=180) as r:
            f.write_bytes(r.read())
        time.sleep(5)


def read(cache: Path):
    nodes, ways, rels = {}, {}, []
    for f in sorted(cache.glob('*.osm')):
        if f.stat().st_size < 1000:
            continue
        for _, el in ET.iterparse(str(f)):
            if el.tag == 'node':
                nodes[int(el.get('id'))] = (float(el.get('lon')), float(el.get('lat')))
                el.clear()
            elif el.tag == 'way':
                tags = {t.get('k'): t.get('v') for t in el.findall('tag')}
                ways[int(el.get('id'))] = ([int(n.get('ref')) for n in el.findall('nd')], tags)
                el.clear()
            elif el.tag == 'relation':
                tags = {t.get('k'): t.get('v') for t in el.findall('tag')}
                rels.append(([(m.get('type'), int(m.get('ref')), m.get('role')) for m in el.findall('member')], tags))
                el.clear()
    return nodes, ways, rels


def inside(c, box, margin=0.03):
    return box[0] - margin <= c[0] <= box[2] + margin and box[1] - margin <= c[1] <= box[3] + margin


def rounded(coords, places=5):
    return [[round(c[0], places), round(c[1], places)] for c in coords]


def terrain(nodes, ways, rels):
    coords = lambda refs: [nodes[r] for r in refs if r in nodes]
    water, torrents, streams, woods, centre = [], [], [], [], []
    used = set()
    for members, t in rels:
        if t.get('natural') == 'water' and t.get('water') == 'river':
            outer, inner = [], []
            for ty, ref, role in members:
                if ty != 'way' or ref not in ways:
                    continue
                c = coords(ways[ref][0])
                if len(c) < 3 or not any(inside(p, WIDE) for p in c):
                    continue
                used.add(ref)
                (inner if role == 'inner' else outer).append(rounded(c))
            if outer:
                water.append({'outer': outer, 'inner': inner})
    for wid, (refs, t) in ways.items():
        c = coords(refs)
        if len(c) < 2 or not any(inside(p, WIDE) for p in c):
            continue
        # (the torrents off the hills are mapped as water too, the Kahan among them: drawn narrower)
        if t.get('natural') == 'water' and t.get('water') == 'river' and wid not in used and len(c) > 2:
            torrents.append(rounded(c))
        if t.get('waterway') == 'river' and 'Jhelum' in (t.get('name:en') or ''):
            centre.append(rounded(c))
        elif t.get('waterway') == 'stream':
            streams.append(rounded(c))
        if t.get('natural') in ('wood', 'scrub', 'wetland') and len(c) > 2:
            woods.append({'kind': t['natural'], 'c': rounded(c)})
    centre.sort(key=lambda c: -len(c))
    return {'water': water, 'torrents': torrents, 'centre': centre[0] if centre else [], 'streams': streams, 'woods': woods}


# ---- geometry (a local flat frame in kilometres about the valley) -------------------------------------

LAT0 = 32.75
KX = 111.32 * math.cos(math.radians(LAT0))
KY = 110.57


def km(p):
    return ((p[0] - 73.5) * KX, (p[1] - LAT0) * KY)


def deg(q):
    return (round(q[0] / KX + 73.5, 5), round(q[1] / KY + LAT0, 5))


def dist(a, b):
    a, b = km(a), km(b)
    return math.hypot(a[0] - b[0], a[1] - b[1])


def nearest_index(line, p):
    return min(range(len(line)), key=lambda i: dist(line[i], p))


def resample(line, step_km):
    """Points every step along a polyline (lon/lat), keeping the ends."""
    out = [line[0]]
    carry = 0.0
    for a, b in zip(line, line[1:]):
        d = dist(a, b)
        if d == 0:
            continue
        while carry + step_km <= d:
            carry += step_km
            f = carry / d
            out.append((a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f))
        carry -= d
    if out[-1] != line[-1]:
        out.append(line[-1])
    return out


def smooth(line, passes=3):
    for _ in range(passes):
        line = [line[0]] + [((a[0] + 2 * b[0] + c[0]) / 4, (a[1] + 2 * b[1] + c[1]) / 4)
                            for a, b, c in zip(line, line[1:], line[2:])] + [line[-1]]
    return line


def offset(line, left_km):
    """The polyline moved `left_km` to its left (the right bank's side, going downstream south-west)."""
    out = []
    pts = [km(p) for p in line]
    for i, p in enumerate(pts):
        a = pts[max(0, i - 1)]
        b = pts[min(len(pts) - 1, i + 1)]
        dx, dy = b[0] - a[0], b[1] - a[1]
        n = math.hypot(dx, dy) or 1
        out.append(deg((p[0] - dy / n * left_km, p[1] + dx / n * left_km)))
    return out


def timed(line, h0, h1):
    """A dated way: the line's points, each with its hour, by distance along it."""
    pts = list(line)
    along = [0.0]
    for a, b in zip(pts, pts[1:]):
        along.append(along[-1] + dist(a, b))
    total = along[-1] or 1
    return [[round(p[0], 5), round(p[1], 5), round(h0 + (h1 - h0) * s / total, 4)] for p, s in zip(pts, along)]


def arc(a, b, bulge_km, n=12):
    """A way from a to b curving out `bulge_km` to the left (a sweep)."""
    A, B = km(a), km(b)
    dx, dy = B[0] - A[0], B[1] - A[1]
    L = math.hypot(dx, dy) or 1
    nx, ny = -dy / L, dx / L
    out = []
    for i in range(n + 1):
        f = i / n
        s = math.sin(math.pi * f) * bulge_km
        out.append(deg((A[0] + dx * f + nx * s, A[1] + dy * f + ny * s)))
    return out


# ---- the night's ways ----------------------------------------------------------------------------------

def ways(T):
    centre = [tuple(c) for c in T['centre']]
    # downstream: the Jhelum runs south-west here; order the line from Jhelum city towards Jalalpur
    if centre[0][0] < centre[-1][0]:
        centre.reverse()
    i0 = nearest_index(centre, HARANPUR)
    i1 = nearest_index(centre, BEND)
    reach = centre[i0:i1 + 1]          # the river between the great camp and the headland (downstream)
    bank_a = centre[i0]                # the water at the great camp
    # (offset's left is the left bank going downstream, the south-east: the right bank is negative)
    near = [centre[i0 - 1], bank_a, centre[i0 + 1]]
    # AUTHORED: the camps a kilometre back from the water on each side, Alexander's on the right bank
    camp_a = offset(near, -2.3)[1]
    camp_p = offset(near, 2.4)[1]
    water_p = offset(near, 0.95)[1]    # the elephants at the water's edge
    glow_a = offset(near, -0.55)[1]    # the fires' light on the water, each side
    glow_p = offset(near, 0.55)[1]
    # AUTHORED: the march inland, behind the hills' skirt: the reach moved 3.5 km onto the right bank's
    # side, smoothed and cut straighter than the river (a column marches, it doesn't meander), from the
    # camp to the grove on the headland
    inland = smooth(resample(offset(resample(reach, 0.5), -3.5), 1.5), 6)
    # AUTHORED: the crossing, laid across the river at the bend: the headland's tip on the right bank, the
    # island in midstream (its near shore a little past the middle, its far shore short of the left bank),
    # the second channel, the landing on the left bank. Offsets in kilometres from the river's middle;
    # the river in flood is drawn about 1.7 km wide here (art.js widens it at the bend).
    fine = resample(reach + centre[i1 + 1:i1 + 40], 0.1)
    k = nearest_index(fine, BEND)
    def across(j, off): return offset(fine[max(0, j - 1):j + 2], off)[1]
    head_tip = across(k, -0.55)
    island_near = across(k + 3, -0.1)
    island_far = across(k + 8, 0.45)
    landing = across(k + 12, 0.85)
    headland = across(k - 3, -1.75)             # the grove, back from the tip (the column waits on land, not at the water)
    # the island's and the headland's outlines, for the pictures
    island = [across(k - 3, 0.1)] + [across(k + j, -0.16) for j in range(0, 20, 3)] + [across(k + 20, 0.15)] \
             + [across(k + j, 0.48) for j in range(18, 0, -3)]
    head_poly = [across(k, -0.52), across(k - 3, -0.6), across(k - 8, -0.75), across(k - 10, -1.3), across(k - 6, -1.7),
                 across(k + 1, -1.6), across(k + 5, -1.1), across(k + 4, -0.6)]
    # (the column's way begins at the camp's edge, a kilometre and a half out, not under its tents)
    march = resample([camp_a] + inland[2:-3] + [headland], 0.6)
    march = march[2:]
    crossing = [head_tip, island_near]
    island_way = [island_near, island_far]
    ford = [island_far, landing]
    advance = resample([landing, FIELD], 0.5)
    # Porus' side rides down the left bank: his son's chariots towards the landing, bogging in the clay
    # short of it; the army to the field
    bankside = smooth(resample(offset(resample(reach, 0.5), 1.4), 1.5), 4)
    j = nearest_index(bankside, FIELD)
    bog = deg(tuple(a * 0.72 + b * 0.28 for a, b in zip(km(landing), km(FIELD))))
    chariots = resample([camp_p] + bankside[2:j] + [bog], 0.8)
    porus = resample([camp_p] + bankside[2:j] + [FIELD], 0.8)
    # the field: Porus' line faces the landing (west); Alexander's horse sweeps round its left (south),
    # Coenus round its right (north) to the rear; the phalanx comes on straight
    fk = km(FIELD); lk = km(landing)
    fx = fk[0] - lk[0]; fy = fk[1] - lk[1]; L = math.hypot(fx, fy); ux, uy = fx / L, fy / L   # the line of advance
    nx, ny = -uy, ux                                                                          # across it (to the left)
    def at(along, across): return deg((fk[0] + ux * along + nx * across, fk[1] + uy * along + ny * across))
    front = at(-2.2, 0)                 # where Alexander halts to take up the foot
    line_l, line_r = at(0, -1.4), at(0, 1.4)   # the ends of Porus' line
    # (the sweeps end wide of the line, not in it: at the symbols' size they would tangle with it)
    alexander = [front, at(-1.7, -1.3), at(-0.9, -2.3), at(0.1, -2.3), at(0.5, -1.9)]
    coenus = [at(-2.0, 0.6), at(-1.6, 1.8), at(-0.6, 2.6), at(0.8, 2.5), at(1.8, 1.6)]
    phalanx = [at(-2.2, 0), at(-1.6, 0), at(-1.3, 0)]            # (it closes to a pike's length, not into the elephants)
    craterus = resample([camp_a, bank_a, water_p, camp_p], 0.4)
    return {
        'campA': camp_a, 'campP': camp_p, 'waterP': water_p, 'bankA': bank_a, 'glowA': glow_a, 'glowP': glow_p,
        'march': march, 'crossing': crossing, 'islandWay': island_way, 'ford': ford, 'advance': advance,
        'headTip': head_tip, 'islandNear': island_near, 'islandFar': island_far, 'landing': landing, 'headland': headland,
        'islandPoly': island, 'headPoly': head_poly, 'bend': fine[k], 'bendDir': [fine[k + 1][0] - fine[k - 1][0], fine[k + 1][1] - fine[k - 1][1]],
        'chariots': chariots, 'porus': porus, 'alexander': alexander, 'coenus': coenus, 'phalanx': phalanx,
        'craterus': craterus, 'field': FIELD, 'lineL': line_l, 'lineR': line_r, 'front': front, 'reach': reach,
        'axis': [ux, uy],
    }


def main(argv):
    if len(argv) != 2:
        print(__doc__); sys.exit(2)
    cache = Path(argv[1])
    fetch(cache)
    nodes, ways_, rels = read(cache)
    T = terrain(nodes, ways_, rels)
    W = ways(T)
    print('water', len(T['water']), 'woods', len(T['woods']), 'streams', len(T['streams']), 'centre', len(T['centre']))
    print('march %.1f km, chariots %.1f km, porus %.1f km' % (
        sum(dist(a, b) for a, b in zip(W['march'], W['march'][1:])),
        sum(dist(a, b) for a, b in zip(W['chariots'], W['chariots'][1:])),
        sum(dist(a, b) for a, b in zip(W['porus'], W['porus'][1:]))))

    # the art sheet's data: everything in lon/lat; the sheet projects into each picture's pixels
    authored = {'headland': list(W['headland']), 'headTip': list(W['headTip']), 'islandNear': list(W['islandNear']),
                'islandFar': list(W['islandFar']), 'landing': list(W['landing']), 'field': list(FIELD), 'jalalpur': list(JALALPUR),
                'island': [list(q) for q in W['islandPoly']], 'headPoly': [list(q) for q in W['headPoly']], 'bend': list(W['bend']),
                'haranpur': list(HARANPUR), 'campA': list(W['campA']), 'campP': list(W['campP']),
                'march': [list(p) for p in W['march']], 'lineL': list(W['lineL']), 'lineR': list(W['lineR']),
                'axis': W['axis']}
    terrain_js = {'wide': WIDE, 'close': CLOSE, 'water': T['water'], 'torrents': T['torrents'], 'centre': T['centre'], 'streams': T['streams'],
                  'woods': T['woods'], 'authored': authored}
    (OUT_TOOL / 'terrain.js').write_text('// Written by tools/hydaspes/build.py from OpenStreetMap (ODbL): never hand-edit.\n'
                                         'window.TERRAIN = ' + json.dumps(terrain_js, separators=(',', ':')) + ';\n')

    # the pack's data: dated ways (hours of the night; the script turns hours into the date's unit)
    chain = (timed(W['march'], H['leave'], H['headland']) + timed(W['crossing'], H['boats'], H['island'])
             + timed(W['islandWay'], H['formed'], H['bank']) + timed(W['ford'], H['bank'], H['ford'])
             + timed(W['advance'], H['ford'], H['field']) + timed(W['phalanx'], H['battle'], H['rout']))
    data = {
        'y0': Y0, 'hours': H, 'wide': WIDE, 'close': CLOSE,
        'places': {'haranpur': list(HARANPUR), 'jalalpur': list(JALALPUR), 'mong': list(MONG), 'headland': list(W['headland']),
                   'island': [round((W['islandNear'][0] + W['islandFar'][0]) / 2, 5), round((W['islandNear'][1] + W['islandFar'][1]) / 2, 5)],
                   'landing': list(W['landing']), 'field': list(FIELD), 'campA': list(W['campA']), 'campP': list(W['campP']),
                   'waterP': list(W['waterP']), 'bankA': list(W['bankA']), 'front': list(W['front']),
                   'glowA': list(W['glowA']), 'glowP': list(W['glowP'])},
        'chain': chain,
        'ways': {
            'column': timed(W['march'], H['leave'], H['headland']),
            'boats': timed(W['crossing'], H['boats'], H['island']),
            'onIsland': timed(W['islandWay'], H['formed'], H['bank']),
            'waders': timed(W['ford'], H['bank'], H['ford']),
            'advance': timed(W['advance'], H['ford'], H['field']),
            'chariots': timed(W['chariots'], H['chariots'], H['routed']),
            'porus': timed(W['porus'], H['porus'], H['field']),
            'alexander': timed(W['alexander'], H['sweep'], H['rout']),
            'coenus': timed(W['coenus'], H['sweep'], H['rout']),
            'phalanx': timed(W['phalanx'], H['battle'], H['rout']),
            'craterus': timed(W['craterus'], H['craterus'], H['end']),
        },
        'river': [[round(p[0], 4), round(p[1], 4)] for p in resample([tuple(c) for c in T['centre']], 0.4)],
    }
    (OUT_PACK).mkdir(parents=True, exist_ok=True)
    (OUT_PACK / 'data.js').write_text('// Written by tools/hydaspes/build.py (its docstring says from what): never hand-edit.\n'
                                      'window.HYDASPES = ' + json.dumps(data, separators=(',', ':')) + ';\n')
    print('wrote', OUT_TOOL / 'terrain.js', (OUT_TOOL / 'terrain.js').stat().st_size, 'bytes;',
          OUT_PACK / 'data.js', (OUT_PACK / 'data.js').stat().st_size, 'bytes')


if __name__ == '__main__':
    main(sys.argv)
