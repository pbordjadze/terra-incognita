"""Builds the herds' data (Prototypes/herds/data.js) from the lineages below and the globe's own land.

    python3 tools/herds/build.py

The camel family's history told as herds: each lineage is a way across the map with a date at every
turn (tools/herds/lineages.py, with its sources), and each animal of a herd follows its lineage at a
place of its own beside the way, from the date it is born into the herd to the date its line ends or
today. Where lineages part, a herd's animals divide between them, and each new lineage is filled up
with animals born at the parting. Every animal's way is a sprite's dated path (docs/runtime3.md,
runtime 5): the globe's date moves it.

The ways are checked against the globe's land (Globe/geo.png's coast distance, the map the app
draws): an animal whose place would fall in the sea is drawn in towards its lineage's way until it is
on land; a lineage's own way must be on land (or on Beringia, below, while it was land), or the build
fails and says where. Between an author's turns, the way meanders a little, so a herd does not march
along a ruler.

Beringia: the sea floor shallower than about 90 m round the strait between Siberia and Alaska
(Globe/terrain.png's depth), drawn as a picture of land (beringia.png) that the pack lays on the globe
while the strait was land. It is a drawing of the shallow shelf, not a reconstruction of a coast at
any date (SOURCES.md).

Output (data.js, `window.HERDS`): `animals` (id, lineages by date, leader, path [[lon, lat, date], ...]),
`lineages` (id, kinds by date, name), `beringia` (its picture and bounds, and the dates it is shown),
`ticks` and `pace` for the date. Deterministic: the same lineages give the same file.
"""
import json, math, random, sys
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(Path(__file__).resolve().parent))
import lineages as L  # noqa: E402  (the authored history, beside this file)

OUT = ROOT / 'Prototypes' / 'herds' / 'data.js'
EARTH_KM = 6371.0
D = math.pi / 180

# ---- the globe's land ---------------------------------------------------------------------------------
_geo = np.array(Image.open(ROOT / 'Globe' / 'geo.png'))
_ter = np.array(Image.open(ROOT / 'Globe' / 'terrain.png'))
W, H = _geo.shape[1], _geo.shape[0]
# land: the coarse coast distance's sign (land positive), as the app's shader reads it
LAND = _geo[..., 1].astype(int) > 128
# the shelf: sea shallower than one step of the depth (87 m)
SHELF = (~LAND) & (_ter[..., 1].astype(int) >= 127)


def pixel(lon, lat):
    x = int(((lon + 180) % 360) / 360 * W) % W
    y = min(H - 1, max(0, int((90 - lat) / 180 * H)))
    return x, y


def on_land(lon, lat, bridge=False):
    x, y = pixel(lon, lat)
    return bool(LAND[y, x]) or (bridge and in_beringia(lon, lat))


# ---- Beringia -----------------------------------------------------------------------------------------
# the strait and its shallow shelf, within an ellipse round the strait (the whole Bering and Chukchi
# shelves are shallow: the ellipse keeps the drawing to the bridge between the two shores)
BERING_CENTRE, BERING_RADII = (-168.8, 65.8), (6.5, 2.4)    # degrees of longitude and latitude
BERING_BOX = (-176.0, 62.8, -161.5, 68.8)                   # west, south, east, north: the picture's bounds
BERING_STEP = 0.025                                         # degrees a pixel


def _bering_alpha():
    w, s, e, n = BERING_BOX
    cols, rows = round((e - w) / BERING_STEP), round((n - s) / BERING_STEP)
    lon = w + (np.arange(cols) + 0.5) * BERING_STEP
    lat = n - (np.arange(rows) + 0.5) * BERING_STEP
    LON, LAT = np.meshgrid(lon, lat)
    xs = (((LON + 180) % 360) / 360 * W).astype(int) % W
    ys = np.clip(((90 - LAT) / 180 * H).astype(int), 0, H - 1)
    inside = ((LON - BERING_CENTRE[0]) / BERING_RADII[0]) ** 2 + ((LAT - BERING_CENTRE[1]) / BERING_RADII[1]) ** 2
    # an edge that wanders like a coast: the ellipse pushed in and out by smooth noise at two scales
    rng = np.random.default_rng(1913)
    def smooth_noise(cell):
        g = rng.normal(0, 1, (rows // cell + 3, cols // cell + 3))
        y, x = np.arange(rows) / cell, np.arange(cols) / cell
        y0, x0 = y.astype(int), x.astype(int)
        fy, fx = (y - y0)[:, None], (x - x0)[None, :]
        fy, fx = fy * fy * (3 - 2 * fy), fx * fx * (3 - 2 * fx)
        a, b = g[y0][:, x0], g[y0][:, x0 + 1]
        c, d = g[y0 + 1][:, x0], g[y0 + 1][:, x0 + 1]
        return (a * (1 - fx) + b * fx) * (1 - fy) + (c * (1 - fx) + d * fx) * fy
    inside = inside + 0.16 * smooth_noise(40) + 0.06 * smooth_noise(12)
    # the sea that was land: shallow, and within that edge (softened over a few pixels)
    a = SHELF[ys, xs] * np.clip((1 - inside) / 0.04, 0, 1)
    return a, LAND[ys, xs]


_balpha, _bland = _bering_alpha()


def in_beringia(lon, lat):
    w, s, e, n = BERING_BOX
    if not (w <= lon <= e and s <= lat <= n):
        return False
    i, j = int((n - lat) / BERING_STEP), int((lon - w) / BERING_STEP)
    return 0 <= i < _balpha.shape[0] and 0 <= j < _balpha.shape[1] and _balpha[i, j] > 0.5


def beringia_picture(path):
    """The bridge as a picture lying on the globe: the tundra's colour where the sea was land, from the
    ground's July (Globe/ground/07.jpg) round the strait, a little mottled, its edge soft."""
    ground = np.array(Image.open(ROOT / 'Globe' / 'ground' / '07.jpg').convert('RGB')).astype(float)
    gh, gw = ground.shape[:2]
    w, s, e, n = BERING_BOX
    rows, cols = _balpha.shape
    lon = w + (np.arange(cols) + 0.5) * BERING_STEP
    lat = n - (np.arange(rows) + 0.5) * BERING_STEP
    LON, LAT = np.meshgrid(lon, lat)
    gx = (((LON + 180) % 360) / 360 * gw).astype(int) % gw
    gy = np.clip(((90 - LAT) / 180 * gh).astype(int), 0, gh - 1)
    land = _bland
    # the shores' own colour, spread out over the water from the nearest land in each row (a cheap fill)
    colour = ground[gy, gx]
    tone = colour[land].mean(axis=0) if land.any() else np.array([150, 150, 120.0])
    # mottled a little, as the tundra round it is (the colour of the shores' nearest land, blurred)
    near = colour.copy()
    near[~land] = tone
    k = 6
    pad = np.pad(near, ((k, k), (k, k), (0, 0)), mode='edge')
    rgb = sum(pad[k + dy:k + dy + rows, k + dx:k + dx + cols] for dy in range(-k, k + 1, 3) for dx in range(-k, k + 1, 3)) / 25
    rgb = np.clip(0.55 * rgb + 0.45 * tone[None, None, :], 0, 255)
    alpha = np.clip(_balpha * (~land) * 255, 0, 255)
    # a soft edge: average alpha over a 5-pixel neighbourhood
    k = 2
    pad = np.pad(alpha, k, mode='edge')
    alpha = sum(pad[k + dy:k + dy + rows, k + dx:k + dx + cols] for dy in range(-k, k + 1) for dx in range(-k, k + 1)) / (2 * k + 1) ** 2
    Image.fromarray(np.dstack([rgb, alpha]).astype(np.uint8), 'RGBA').save(path, optimize=True)
    return [w, s, e, n]


# ---- ways on the globe --------------------------------------------------------------------------------
def unit(lon, lat):
    return np.array([math.cos(lat * D) * math.sin(lon * D), math.sin(lat * D), math.cos(lat * D) * math.cos(lon * D)])


def place(v):
    v = v / np.linalg.norm(v)
    return math.atan2(v[0], v[2]) / D, math.asin(max(-1, min(1, v[1]))) / D


def slerp(a, b, f):
    om = math.acos(max(-1, min(1, float(a @ b))))
    if om < 1e-9:
        return a
    return (math.sin((1 - f) * om) * a + math.sin(f * om) * b) / math.sin(om)


def offset(lon, lat, east_km, north_km):
    """The place so many kilometres east and north of another (small offsets)."""
    dlat = north_km / EARTH_KM / D
    dlon = east_km / (EARTH_KM * max(0.05, math.cos(lat * D))) / D
    return lon + dlon, max(-89.9, min(89.9, lat + dlat))


def km_between(a, b):
    return math.acos(max(-1, min(1, float(unit(*a) @ unit(*b))))) * EARTH_KM


def bridge_open(date):
    """Beringia is land (the strait closed) at this date."""
    return any(a <= date <= b for a, b in L.BERINGIA_LAND)


def densify(way, rng, meander_km, sea_ok=False):
    """An author's turns [[lon, lat, date], ...] with points about every 250 km between them, meandering."""
    out = [list(way[0])]
    for (lo0, la0, d0), (lo1, la1, d1) in zip(way, way[1:]):
        a, b = unit(lo0, la0), unit(lo1, la1)
        dist = km_between((lo0, la0), (lo1, la1))
        n = max(1, int(dist / 250))
        side = rng.uniform(-1, 1)
        for i in range(1, n + 1):
            f = i / n
            lon, lat = place(slerp(a, b, f))
            if i < n:
                # a gentle bend, zero at both turns
                bend = meander_km * side * math.sin(math.pi * f) + meander_km * 0.3 * rng.uniform(-1, 1) * math.sin(math.pi * f)
                # perpendicular to the way: east-north of the heading turned a quarter
                h = math.atan2((lo1 - lo0) * math.cos(lat * D), la1 - la0)
                lon2, lat2 = offset(lon, lat, bend * math.cos(h), -bend * math.sin(h))
                if sea_ok or on_land(lon2, lat2, bridge_open(d0 + f * (d1 - d0))):
                    lon, lat = lon2, lat2
            out.append([lon, lat, d0 + f * (d1 - d0)])
    return out


def position_at(way, date):
    """Where a dense way is at a date (between its turns, on the great circle)."""
    if date <= way[0][2]:
        return way[0][0], way[0][1]
    for (lo0, la0, d0), (lo1, la1, d1) in zip(way, way[1:]):
        if d0 <= date <= d1:
            f = (date - d0) / (d1 - d0) if d1 > d0 else 1
            return place(slerp(unit(lo0, la0), unit(lo1, la1), f))
    return way[-1][0], way[-1][1]


# ---- the herds -----------------------------------------------------------------------------------------
def build():
    rng = random.Random(1857)
    lineages = {x['id']: x for x in L.LINEAGES}
    problems = []
    # each lineage's own dense way, starting where its parent is at the parting
    dense = {}
    for x in L.LINEAGES:
        way = [list(p) for p in x['way']]
        if x.get('parent'):
            parent = dense[x['parent']]
            start = way[0][2]
            lon, lat = position_at(parent, start)
            way = [[lon, lat, start]] + [p for p in way if p[2] > start]
        for lon, lat, date in way:
            if not on_land(lon, lat, bridge_open(date)) and not x.get('sea_ok'):
                problems.append(f"{x['id']}: [{lon:.2f}, {lat:.2f}] at {date:,.0f} is in the sea on the globe's map")
        dense[x['id']] = densify(way, rng, x.get('meander', 120), x.get('sea_ok', False))
        if not x.get('sea_ok'):
            for lon, lat, date in dense[x['id']]:
                if not on_land(lon, lat, bridge_open(date)):
                    problems.append(f"{x['id']}: its way between turns crosses the sea near [{lon:.2f}, {lat:.2f}] at {date:,.0f}")
                    break
    if problems:
        raise SystemExit('The lineages leave the land:\n  ' + '\n  '.join(problems))

    # the animals: born into a lineage (at its start, or at a date it grows), passed on to a child at a
    # parting, until their line ends
    animals = []
    members = {}       # lineage id -> its animals (dicts)
    leaders = {}       # lineage id -> [(since, animal)]: who leads it (the animal seen close), from when

    def lead(lin, date, animal):
        leaders.setdefault(lin, []).append((date, animal))

    def born(lin, date, count, leader=False):
        for _ in range(count):
            a = {'id': f"{lin}-{len([m for m in animals if m['first'] == lin]) + 1}", 'first': lin, 'segments': [(lin, date)],
                 'radius': rng.uniform(0.25, 1.0), 'angle': rng.uniform(0, 2 * math.pi), 'drift': rng.uniform(0, 2 * math.pi)}
            animals.append(a)
            members.setdefault(lin, []).append(a)
            if leader and _ == 0:
                lead(lin, date, a)

    for x in sorted(L.LINEAGES, key=lambda x: x['way'][0][2]):
        lid, start = x['id'], x['way'][0][2]
        if x.get('parent'):
            parent = x['parent']
            pool = members.get(parent, [])
            take = x.get('take', 0)
            # the parent's leader goes with the child that says `leader_from_parent`; otherwise it stays
            head = leaders[parent][-1][1] if leaders.get(parent) else None
            order = sorted([m for m in pool if m['segments'][-1][1] <= start], key=lambda m: m['id'])
            moved = [head] if x.get('leader_from_parent') and head in order else []
            moved += [m for m in order if m is not head][:max(0, take - len(moved))]
            for m in moved:
                pool.remove(m)
                m['segments'].append((lid, start))
                members.setdefault(lid, []).append(m)
            # each herd has one leader, the animal seen close: the parent's, the first to come over, or
            # one born now; a parent that goes on without its leader is led by the next of its own
            if moved:
                lead(lid, start, moved[0])
            if head in moved and pool:
                lead(parent, start, sorted(pool, key=lambda m: m['id'])[0])
            born(lid, start, max(0, x['members'] - len(moved)), leader=not moved)
        else:
            born(lid, start, x['members'], leader=True)
        for date, count in x.get('grow', []):
            born(lid, date, count)

    out = []
    for a in animals:
        pts = []
        segs = a['segments']
        for i, (lid, since) in enumerate(segs):
            until = segs[i + 1][1] if i + 1 < len(segs) else (lineages[lid].get('ends') if lineages[lid].get('ends') is not None else L.TODAY)
            way = dense[lid]
            x = lineages[lid]
            seg = [p for p in way if since < p[2] < until]
            lon0, lat0 = position_at(way, since)
            lon1, lat1 = position_at(way, until)
            for lon, lat, date in [[lon0, lat0, since]] + seg + [[lon1, lat1, until]]:
                # the animal's own place beside the way: a radius and a bearing that drift slowly with the date
                spread = x.get('spread', 170) * a['radius']
                k = (date - L.START) / (L.TODAY - L.START)
                ang = a['angle'] + 0.8 * math.sin(a['drift'] + k * 40)
                bridge = bridge_open(date)
                for shrink in (1, 0.7, 0.45, 0.25, 0.1, 0):
                    lon2, lat2 = offset(lon, lat, spread * shrink * math.cos(ang), spread * shrink * math.sin(ang))
                    if x.get('sea_ok') or on_land(lon2, lat2, bridge):
                        break
                pts.append([round(lon2, 3), round(lat2, 3), date])
        # drop repeats of a date (a parting's join), keeping the later lineage's place
        clean = []
        for p in pts:
            if clean and p[2] == clean[-1][2]:
                clean[-1] = p
            else:
                clean.append(p)
        lineage_by_date = [[since, lid] for lid, since in segs]
        # when it leads its herd: within each of its lineages, while that lineage's leader is it
        leads = []
        for i, (lid, since) in enumerate(segs):
            until = segs[i + 1][1] if i + 1 < len(segs) else (lineages[lid].get('ends') if lineages[lid].get('ends') is not None else L.TODAY)
            chain = leaders.get(lid, [])
            for j, (frm, who) in enumerate(chain):
                to = chain[j + 1][0] if j + 1 < len(chain) else math.inf
                lo, hi = max(frm, since), min(to, until)
                if who is a and hi > lo:
                    if leads and leads[-1][1] == lo:
                        leads[-1][1] = hi
                    else:
                        leads.append([lo, hi])
        out.append({'id': a['id'], 'leads': leads, 'lineages': lineage_by_date, 'path': clean})

    return {
        'start': L.START, 'today': L.TODAY,
        'pace': L.PACE, 'ticks': L.TICKS,
        'lineages': {x['id']: {'name': x['name'], 'kinds': x['kinds']} for x in L.LINEAGES},
        'animals': out,
        'beringia': {'image': 'beringia.png', 'bounds': beringia_picture(OUT.parent / 'beringia.png'), 'land': L.BERINGIA_LAND},
    }


def main():
    data = build()
    text = ('// Written by tools/herds/build.py from tools/herds/lineages.py and the globe\'s land. Never hand-edit.\n'
            'window.HERDS = ' + json.dumps(data, separators=(',', ':')) + ';\n')
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(text)
    n = len(data['animals'])
    pts = sum(len(a['path']) for a in data['animals'])
    print(f'{OUT.relative_to(ROOT)}: {n} animals, {pts} turns, {len(text) // 1024} KB; Beringia {data["beringia"]["bounds"]}')


if __name__ == '__main__':
    main()
