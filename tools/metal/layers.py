"""The app's Metal layers for metal.py: which map goes to which binding and how the app fills each
layer's uniforms (GlobeView, CloudsLayer, OceanLayer, mirrored here: change them together), so a
shader can be drawn at a camera without a phone, and set beside its desktop twin.

    python3 tools/metal/metal.py shot globe+ocean+clouds out.png [--device ipad] [--at 12,-20,3.4]
        [--style ground] [--month 10] [--days 0.4] [--time 12] [--scale 1]
    python3 tools/metal/metal.py twin clouds [--device phone] [--at ...] [--month 10] [--days 0.4]

A shot stacks the layers named, bottom first, each drawn at its own scale (pixels a point) and laid
over the one before as the stage composites them (no names: GlobeView draws those apart). `twin`
draws a layer's web twin in headless Chromium, takes the uniforms its WebGL program was given and
the pixels it wrote, draws the Metal with those uniforms, and reports how far apart the two are
(build/metal/twin/: the twin, the Metal and their difference, side by side).
"""
import json, math, re, struct, subprocess, sys, tempfile, time
from pathlib import Path

import numpy as np

import metal

ROOT = metal.ROOT
APP = ROOT / 'App' / 'TerraIncognita'
GLOBE = ROOT / 'Globe'
sys.path.insert(0, str(ROOT / 'tools' / 'globe'))
import preview  # noqa: E402

SIZES = {'phone': (402, 874), 'ipad': (1032, 1376)}
VIEWS = {'earth': (15, 8, 1), 'atlantic': (-35, 35, 1), 'namib': (12, -18, 2.2), 'descent': (12, -20, 3.4),
         'ground': (14, -22, 5.5), 'close': (15, -23, 8), 'aegean': (25, 38, 4)}


def camera(w, h, lon, lat, zoom, persian=False):
    """The globe's camera in points, as GlobeView places it (tools/clouds/compose.py's, for the twins)."""
    cx, cy, r0 = preview.layout(w, h, persian)
    return {'lon': lon, 'lat': lat, 'zoom': zoom, 'cx': cx, 'cy': cy, 'radius': r0 * zoom, 'width': w, 'height': h}


def columns(lon, lat):
    """World from view, Ry(lon) . Rx(-lat), by columns (as CloudsLayer and OceanLayer pass it)."""
    a, b = math.radians(lon), math.radians(-lat)
    ca, sa, cb, sb = math.cos(a), math.sin(a), math.cos(b), math.sin(b)
    return (ca, 0, -sa, 0), (sa * sb, cb, ca * sb, 0), (sa * cb, -sb, ca * cb, 0)


def pack(*vectors):
    return b''.join(struct.pack('4f', *v) for v in vectors)


_cache = {}


def cached(key, make):
    if key not in _cache:
        _cache[key] = make()
    return _cache[key]


def blank():
    return cached('blank', lambda: metal.Texture.blank(1, 1))


def noise_tile():
    """The clouds' (and the sea's) tile of noise, as `cloudsNoiseTile` makes it on the GPU."""
    def make():
        tile, _ = metal.run(APP / 'Clouds/Clouds.metal', 'cloudsNoiseTile', None,
                            textures={0: metal.Texture.blank(96, 96, 96)}, grid=(96, 96, 96), output=0)
        return tile
    return cached('tile', make)


def ground_maps(month):
    """The ground style's maps for a month (GlobeView.loadGround), mipmapped as it uploads them."""
    name = f'{month:02d}'
    return cached(f'ground{name}', lambda: (
        metal.Texture.from_png(GLOBE / 'ground' / f'{name}.jpg', 4, mipmapped=True),
        metal.Texture.from_png(GLOBE / 'ground' / f'ice-{name}.png', 4, mipmapped=True),
        metal.Texture.from_png(GLOBE / 'ground' / 'floor.png', 4, mipmapped=True)))


def geo():
    return cached('geo', lambda: metal.Texture.from_png(GLOBE / 'geo.png', 4, mipmapped=True))


class Globe:
    """GlobeView's drawing: Globe.metal over the whole screen, at the screen's scale."""
    shader, fragment, scale = APP / 'Globe/Globe.metal', 'globe_fragment', 1.0
    twin = 'tools/globe/preview.py'
    # (its twin samples the maps without mipmaps, so it is compared with the Metal's first level, and
    # takes derivatives over the whole picture where a GPU takes them in its quad: lines differ a little)
    lean, apart = 1.0, 2.0

    def bind(self, o, cam, size, scale):
        persian, ground, night = o.style == 'persian', o.style in ('ground', 'night'), o.style == 'night'
        w, h = cam['width'], cam['height']
        R = cam['radius']
        reading = preview.reading(w, h)
        smooth = lambda a, b, x: (lambda t: t * t * (3 - 2 * t))(min(max((x - a) / (b - a), 0), 1))
        ring = (1 - smooth(1.25, 1.7, cam['zoom'])) * (0 if ground else 1)
        m = preview.rotation(cam['lon'], cam['lat'])                  # world from view, by rows
        top, bottom, left, right = preview.safe_insets(w, h)
        u = pack((size[0], size[1], cam['cx'] * scale, cam['cy'] * scale),
                 (R * scale, scale * reading, cam['zoom'], ring),
                 (0, 0, 1 if persian else 0, preview.page_alpha(cam['zoom']) if persian else 0),
                 (*m[0], 0), (*m[1], 0), (*m[2], 0),
                 (top * scale, bottom * scale, left * scale, right * scale),
                 (0, 1 if ground else 0, 1 if night else 0, o.night_time if night else 0))
        tex = {0: geo(),
               1: cached('terrain', lambda: metal.Texture.from_png(GLOBE / 'terrain.png', 4, mipmapped=True)),
               2: cached('ids', lambda: metal.Texture.from_png(GLOBE / 'ids.png', 1)),
               3: cached('band', lambda: metal.Texture.from_png(GLOBE / 'band.png', 4, True, premultiply=True)),
               4: cached('fleuron', lambda: metal.Texture.from_png(GLOBE / 'fleuron.png', 4, True, premultiply=True)),
               5: cached('braid', lambda: metal.Texture.from_png(GLOBE / 'braid.png', 4, mipmapped=True))}
        land, ice, floor = ground_maps(o.month) if ground else (blank(), blank(), blank())
        tex.update({6: land, 7: ice, 8: floor,
                    9: cached('night', lambda: metal.Texture.from_png(GLOBE / 'ground' / 'night.jpg', 4, mipmapped=True)) if night else blank()})
        return {0: u}, tex


class Clouds:
    """CloudsLayer: Clouds.metal over the globe, at its own scale (1.25 to start: it follows the GPU)."""
    shader, fragment, scale = APP / 'Clouds/Clouds.metal', 'cloudsFragment', 1.25
    page, ready, draw = 'tools/clouds/web/preview.html', 'html[data-clouds="ready"]', '__clouds'
    # (thresholded noise: a twentieth of a pixel moves an eighth of their edges, and the GPUs' fast
    # exp and pow more: far apart pixel by pixel; what a changed constant does is lean one way)
    lean, apart = 2.2, 6.0

    @staticmethod
    def cyclones(days):
        """CloudsLayer.cyclones: two float4s each of ten."""
        def pcg3d(x, y, z):
            M = 0xFFFFFFFF
            x, y, z = (x * 1664525 + 1013904223) & M, (y * 1664525 + 1013904223) & M, (z * 1664525 + 1013904223) & M
            x = (x + y * z) & M; y = (y + z * x) & M; z = (z + x * y) & M
            x ^= x >> 16; y ^= y >> 16; z ^= z >> 16
            x = (x + y * z) & M; y = (y + z * x) & M; z = (z + x * y) & M
            return x / 4294967295, y / 4294967295, z / 4294967295
        smooth = lambda a, b, x: (lambda t: t * t * (3 - 2 * t))(min(max((x - a) / (b - a), 0), 1))
        out = []
        for k in range(10):
            hemi = 1 if k < 5 else -1
            t = days / 6 + k * 0.2 + (0.1 if hemi < 0 else 0)
            n = math.floor(t)
            age = t - n
            r = pcg3d(k, n, 977)
            lat, lon = hemi * (38 + 22 * r[1]) * math.pi / 180, 2 * math.pi * r[0]
            size = 0.09 + 0.07 * r[2]
            wind = 2.6 * smooth(0, 0.55, age) * (1 - smooth(0.78, 1, age))
            out += [(math.cos(lat) * math.sin(lon), math.sin(lat), math.cos(lat) * math.cos(lon), size),
                    (-hemi * wind, math.cos(3 * size), 0, 0)]
        return out

    def textures(self):
        return {0: cached('sky', lambda: metal.Texture.from_png(APP / 'Clouds/CloudsSky.png', 4)),
                1: cached('land', lambda: metal.Texture.from_png(APP / 'Clouds/CloudsLand.png', 1)),
                2: noise_tile()}

    def bind(self, o, cam, size, scale):
        reading = preview.reading(cam['width'], cam['height'])
        m0 = o.month - 1
        # (CloudsLook.on: by night a thin grey veil without shadows)
        night = o.style == 'night'
        u = pack((size[0], size[1], scale, reading), (cam['cx'], cam['cy'], cam['radius'], cam['zoom']),
                 *columns(cam['lon'], cam['lat']), (o.days, m0, (m0 + 1) % 12, 0), (0, 0, 0, 0),
                 (0.05, 0.09, 0.18, 0 if night else 0.78), (0.06, 0.05, 0.04, 0 if night else 0.47),
                 (0.5 if night else 1, 0.75, o.quiet, 0.36 if night else o.bright))
        return {0: u, 1: pack(*self.cyclones(o.days))}, self.textures()

    def from_twin(self, u, size):
        r = u['uRot']
        u = (pack((size[0], size[1], u['uPx'], u['uReading']), (*u['uDisc'], u['uZoom']),
                  (*r[0:3], 0), (*r[3:6], 0), (*r[6:9], 0), (u['uDays'], *u['uMonth']), u['uSun'],
                  u['uSeaShadow'], u['uLandShadow'], u['uTone']))
        return u

    def twin_buffers(self, u, size):
        return {0: self.from_twin(u, size), 1: pack(*u['uCyclone'])}

    def twin_query(self, o):
        return f'month={o.month}&days={o.days}&quiet={o.quiet}&bright={o.bright}', None


class Ocean:
    """OceanLayer: Ocean.metal under the clouds, at its own scale (1.5 to start: it follows the GPU)."""
    shader, fragment, scale = APP / 'Ocean/Ocean.metal', 'oceanFragment', 1.5
    page, ready, draw = 'tools/ocean/web/preview.html', 'html[data-ocean="ready"]', '__ocean'
    lean, apart = 0.3, 0.5

    def textures(self, month):
        return {0: cached('currents', lambda: metal.Texture.from_png(APP / 'Ocean/OceanCurrents.png', 4)),
                1: cached('stream', lambda: metal.Texture.from_png(APP / 'Ocean/OceanStream.png', 4)),
                2: geo(), 3: ground_maps(month)[1], 4: noise_tile()}

    def bind(self, o, cam, size, scale):
        reading = preview.reading(cam['width'], cam['height'])
        m0 = o.month - 1
        u = pack((size[0], size[1], scale, reading), (cam['cx'], cam['cy'], cam['radius'], cam['zoom']),
                 *columns(cam['lon'], cam['lat']), (o.time, m0, (m0 + 1) % 12, 0), (1, 1, o.time, 0),
                 *[(0, 0, 0, 0)] * 8)
        return {0: u}, self.textures(o.month)

    def twin_buffers(self, u, size):
        r = u['uRot']
        return {0: pack((size[0], size[1], u['uPx'], u['uReading']), (*u['uDisc'], u['uZoom']),
                        (*r[0:3], 0), (*r[3:6], 0), (*r[6:9], 0), (u['uTime'], *u['uMonth']), u['uTone'],
                        *u['uRipple'])}

    def twin_query(self, o):
        return f'month={o.month}', {'fn': '__ocean', 'value': o.time}


LAYERS = {'globe': Globe(), 'ocean': Ocean(), 'clouds': Clouds()}


def fields(shader):
    """The uniforms' floats by name (`time.z`), from the shader's own struct of float4s."""
    src = Path(shader).read_text()
    struct = re.search(r'struct\s+\w*Uniforms\s*\{(.*?)\};', src, re.S).group(1)
    names = []
    for m in re.finditer(r'float4\s+([^;]+);', re.sub(r'//[^\n]*', '', struct)):
        for part in m.group(1).split(','):
            pm = re.match(r'\s*(\w+)(?:\[(\d+)\])?', part)
            for k in range(int(pm.group(2) or 1)):
                base = f'{pm.group(1)}[{k}]' if pm.group(2) else pm.group(1)
                names += [f'{base}.{c}' for c in 'xyzw']
    return names


def view(o):
    w, h = SIZES[o.device]
    lon, lat, zoom = VIEWS[o.at] if o.at in VIEWS else map(float, o.at.split(','))
    return camera(w, h, lon, lat, zoom, persian=o.style == 'persian')


def draw(layer, o, cam, scale=None):
    """A layer's drawable at its scale: premultiplied RGBA floats, the runtime's report, and which
    pixels took a derivative the GPU leaves undefined."""
    scale = scale or o.scale or layer.scale
    size = (max(1, round(cam['width'] * scale)), max(1, round(cam['height'] * scale)))
    buffers, textures = layer.bind(o, cam, size, scale)
    return metal.run(layer.shader, layer.fragment, size, buffers, textures, parted=True)


def resized(rgba, size):
    """A premultiplied drawable scaled to another size, as its layer is shown (bilinear)."""
    from PIL import Image
    if rgba.shape[1] == size[0] and rgba.shape[0] == size[1]:
        return rgba
    return np.stack([np.asarray(Image.fromarray(np.ascontiguousarray(rgba[..., c]), 'F').resize(size, Image.BILINEAR))
                     for c in range(4)], -1)


def cmd_shot(o):
    names = o.layers.split('+')
    cam = view(o)
    out_scale = o.scale or max(LAYERS[n].scale for n in names)
    size = (round(cam['width'] * out_scale), round(cam['height'] * out_scale))
    acc = np.zeros((size[1], size[0], 4), np.float32)
    for name in names:
        t0 = time.time()
        rgba, report, undefined = draw(LAYERS[name], o, cam)
        shown = rgba[..., 3] > 0
        print(f'{name}: {rgba.shape[1]}x{rgba.shape[0]} in {time.time() - t0:.1f} s')
        if undefined.any():
            # (README.md › Derivatives apart: on the device these come from the neighbours' values as
            # they stood; here from the neighbours on the same line, or none)
            print(f'  {100 * undefined[shown].mean():.1f}% of what it shows took a derivative with its quad apart '
                  f'(undefined on a GPU; may differ on the device):')
            print('\n'.join('    ' + line for line in report.strip().splitlines()))
        acc = resized(rgba, size) + acc * (1 - resized(rgba, size)[..., 3:4])
    metal.to_png(acc, o.out, background=None if names[0] != 'globe' else (0, 0, 0))
    print(o.out)
    return 0


def cmd_twin(o):
    layer = LAYERS[o.layer]
    if o.layer == 'globe':
        return twin_globe(o)
    cam = view(o)
    scale = o.scale or 1.0
    out = ROOT / 'build' / 'metal' / 'twin'
    out.mkdir(parents=True, exist_ok=True)
    stem = out / f'{o.layer}-{o.device}-{o.at.replace(",", "_")}'
    query, hold = layer.twin_query(o)
    job = {'page': str(ROOT / layer.page), 'ready': layer.ready, 'query': query, 'width': cam['width'],
           'height': cam['height'], 'scale': scale, 'camera': cam, 'hold': hold, 'draw': layer.draw, 'out': str(stem)}
    with tempfile.NamedTemporaryFile('w', suffix='.json', delete=False) as f:
        json.dump(job, f)
    r = subprocess.run(['node', str(metal.HERE / 'twin.cjs'), f.name], capture_output=True, text=True)
    Path(f.name).unlink()
    if r.returncode:
        print(r.stderr, file=sys.stderr)
        return 2
    frame = json.loads(Path(f'{stem}.json').read_text())
    size = (frame['width'], frame['height'])
    twin = np.fromfile(f'{stem}.rgba', np.uint8).reshape(size[1], size[0], 4).astype(np.float32) / 255
    # the twin's own uniforms, so the comparison is of the shaders alone
    _, textures = layer.bind(o, cam, size, frame['uniforms']['uPx'])
    rgba, report, undefined = metal.run(layer.shader, layer.fragment, size, layer.twin_buffers(frame['uniforms'], size),
                                        textures, parted=True)
    if report.strip():
        print(report.rstrip())
    # and the app's own fill beside the twin's: where the hosts differ
    app_u = np.frombuffer(layer.bind(o, cam, size, frame['uniforms']['uPx'])[0][0], np.float32)
    twin_u = np.frombuffer(layer.twin_buffers(frame['uniforms'], size)[0], np.float32)
    names = fields(layer.shader)
    hosts = [names[i] if i < len(names) else str(i) for i in range(len(app_u))
             if abs(app_u[i] - twin_u[i]) > 1e-3 * max(1, abs(app_u[i]))]
    return compare(o, stem, twin, rgba, undefined, hosts)


def compare(o, stem, twin, rgba, undefined, hosts=(), truncate=False, block=32):
    """How far apart, in 8-bit steps of the premultiplied drawable, where the GPU defines the result
    (not in a quad that took a derivative with its pixels apart: there the GPU, the twin and the
    stand-in each give their own). Over each 32-pixel block, how far the Metal leans from the twin
    (the mean signed difference: a colour, an opacity, a weight changed in one of the two leans its
    blocks one way, where rounding at a threshold leans both ways and cancels) and how far apart in
    all (the mean absolute difference: a width or a contrast changed); the 99th percentile of each
    over the blocks, against the layer's tolerances. A picture of where. `truncate`: the twin's 8 bits
    are truncated (preview.py's), where the GPU's are rounded."""
    from PIL import Image
    metal8 = (np.floor if truncate else np.round)(np.clip(rgba, 0, 1) * 255)
    twin8 = np.round(np.clip(twin, 0, 1) * 255)
    shown = (metal8[..., 3] > 0) | (twin8[..., 3] > 0)
    ok = ~undefined & shown
    diff = (metal8 - twin8) * ok[..., None]
    d = np.abs(diff).max(-1)[ok]
    H, W = ok.shape
    h, w = H // block * block, W // block * block
    cells = lambda a: a[:h, :w].reshape(H // block, block, W // block, block, *a.shape[2:]).sum((1, 3))
    n = cells(ok.astype(np.float32))
    full = n >= block * block / 4                       # (blocks at least a quarter shown and defined)
    lean = np.abs(cells(diff)[full] / n[full][:, None]).max(-1) if full.any() else np.zeros(1)
    apart = (cells(np.abs(diff))[full] / n[full][:, None]).max(-1) if full.any() else np.zeros(1)
    lean99, apart99 = float(np.percentile(lean, 99)), float(np.percentile(apart, 99))
    print(f'{stem.name}: {twin.shape[1]}x{twin.shape[0]}; {100 * undefined[shown].mean() if shown.any() else 0:.1f}% of '
          f'what shows has derivatives the GPU leaves undefined, set aside')
    print(f'  pixels: {100 * (d <= 2).mean() if d.size else 100:.2f}% within 2 steps, '
          f'{100 * (d <= 8).mean() if d.size else 100:.2f}% within 8; blocks of {block}: lean {lean99:.2f}, '
          f'apart {apart99:.2f} steps (99th percentile)')
    if hosts:
        print(f'  the app\'s uniforms and the twin\'s differ at {", ".join(hosts)} (layers.py mirrors the app)')
    heat = np.clip(np.abs(metal8 - twin8).max(-1) * 16, 0, 255)
    heat = np.stack([heat, heat * ~undefined, heat * ~undefined], -1)    # (red where undefined)
    over = lambda a: (a[..., :3] + 0.5 * (1 - a[..., 3:4])) * 255          # on grey, to see alpha
    side = np.concatenate([over(twin8 / 255), over(metal8 / 255), heat], 1)
    Image.fromarray(np.clip(side, 0, 255).astype(np.uint8)).save(f'{stem}-compare.png')
    print(f'  {Path(f"{stem}-compare.png").relative_to(ROOT)}: the twin, the Metal, their difference x16 (red: undefined)')
    layer = LAYERS[o.layer]
    most_lean, most_apart = o.lean or layer.lean, o.apart or layer.apart
    agree = lean99 <= most_lean and apart99 <= most_apart
    print(f'  agree (within {most_lean} lean, {most_apart} apart)' if agree
          else f'  APART: past {most_lean} lean or {most_apart} apart (a block leaning one way, or further apart than '
               f'rounding takes it: one of the two has changed)')
    return 0 if agree else 1


def twin_globe(o):
    """The globe's twin is preview.py, numpy line for line: drawn at the same camera, no names."""
    cam = view(o)
    scale = o.scale or 1.0
    out = ROOT / 'build' / 'metal' / 'twin'
    out.mkdir(parents=True, exist_ok=True)
    stem = out / f'globe-{o.style}-{o.device}-{o.at.replace(",", "_")}'
    kw = {'persian': o.style == 'persian'}
    if o.style == 'ground':
        kw.update(ground=True, month=o.month)
    img, _ = preview.render(cam['width'], cam['height'], scale, cam['lon'], cam['lat'], cam['zoom'], 0, **kw)
    twin = np.asarray(img.convert('RGBA'), np.float32) / 255
    size = (round(cam['width'] * scale), round(cam['height'] * scale))
    buffers, textures = LAYERS['globe'].bind(o, cam, size, scale)
    rgba, report, undefined = metal.run(LAYERS['globe'].shader, LAYERS['globe'].fragment, size, buffers, textures,
                                        parted=True, defines=['MTL_BASE_LEVEL'])
    if report.strip():
        print(report.rstrip())
    return compare(o, stem, twin, rgba, undefined, truncate=True)


def commands(sub):
    def common(p):
        p.add_argument('--device', default='ipad', choices=SIZES)
        p.add_argument('--at', default='earth', help='lon,lat,zoom or a view: ' + ', '.join(VIEWS))
        p.add_argument('--style', default='ground', choices=('ground', 'night', 'wikipedia', 'persian'))
        p.add_argument('--month', type=int, default=10)
        p.add_argument('--days', type=float, default=0.4, help="the clouds' simulated days")
        p.add_argument('--quiet', type=float, default=0, help="the clouds' quiet rule, 0 or 1 (clear where the reader looks)")
        p.add_argument('--bright', type=float, default=1, help="how bright the clouds are (1 by day)")
        p.add_argument('--night-time', type=float, default=0, help="the night's seconds (its lights shimmer by them)")
        p.add_argument('--time', type=float, default=12.0, help="the sea's seconds")
        p.add_argument('--scale', type=float, default=None, help="pixels a point (default: each layer's own)")
    s = sub.add_parser('shot', help='draw layers as their Metal would on the device')
    s.add_argument('layers', help='globe, ocean, clouds, bottom first, joined by +')
    s.add_argument('out')
    common(s)
    s.set_defaults(fn=cmd_shot)
    t = sub.add_parser('twin', help='a layer beside its desktop twin, from the same uniforms')
    t.add_argument('layer', choices=LAYERS)
    t.add_argument('--lean', type=float, default=None, help="the most a block may lean one way, in steps (each layer's own by default)")
    t.add_argument('--apart', type=float, default=None, help="the most a block may be apart, in steps (each layer's own)")
    common(t)
    t.set_defaults(fn=cmd_twin)
