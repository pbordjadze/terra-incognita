"""Metal off the Mac: Terra's shaders compiled as C++ by clang on Linux, checked in seconds and drawn on
the CPU (README.md).

    python3 tools/metal/metal.py check [file.metal ...]     every shader in App/ by default
    python3 tools/metal/metal.py shot <layer> out.png [--lon --lat --zoom --size --scale --month ...]
    python3 tools/metal/metal.py twin <layer> [--lon ...]    the Metal and its desktop twin, compared

`check` compiles each file and every entry point's bindings, as the Mac would refuse them; `shot` draws
a layer (layers.py: globe, clouds, ocean) as its Metal would on the device; `twin` draws it beside
its web twin from the same uniforms and says how far apart they are (twin.py).

The source is rewritten a little on the way (`rewrite`): unsuffixed floating literals become floats,
as Metal takes them; a vector or matrix constructor becomes a call that converts values, never bits;
a comparison among a vector's elements gives Metal's 1 for true, not clang's -1. Line numbers are kept, so an
error names the .metal file's own line.
"""
import argparse, hashlib, json, os, re, subprocess, sys, tempfile
from pathlib import Path

try:
    import numpy as np                                  # (drawing needs it; `check` needs only clang)
except ImportError:
    np = None

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
INCLUDE = HERE / 'include'
RUNTIME = HERE / 'runtime.cpp'
BUILD = ROOT / 'build' / 'metal'
CXX = os.environ.get('METAL_CXX', 'clang++')
FLAGS = ['-std=c++17', '-x', 'c++', '-I', str(INCLUDE), '-Wno-unknown-attributes', '-flax-vector-conversions=none',
         '-Wno-unused-value']

SCALARS = ('float', 'half', 'int', 'uint', 'short', 'ushort', 'uchar', 'char', 'bool')
VECTORS = {f'{s}{n}' for s in SCALARS for n in (2, 3, 4)}
MATRICES = {f'{s}{c}x{r}' for s in ('float', 'half') for c in (2, 3, 4) for r in (2, 3, 4)}

TOKEN = re.compile(r'''
    (?P<comment>//[^\n]*|/\*.*?\*/)
  | (?P<pre>^[ \t]*\#[^\n]*)
  | (?P<string>"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*')
  | (?P<number>(?:0[xX][0-9a-fA-F]+[uUlL]*)|(?:(?:\d+\.\d*|\.\d+)(?:[eE][+-]?\d+)?|\d+[eE][+-]?\d+)[fFhH]?|\d+[uUlL]*)
  | (?P<ident>[A-Za-z_]\w*)
  | (?P<space>\s+)
  | (?P<op><<=|>>=|<<|>>|<=|>=|==|!=|&&|\|\||->|::|\+\+|--|[-+*/%&|^]=|\[\[|\]\]|.)
''', re.S | re.M | re.X)


def tokens(src):
    out, pos = [], 0
    while pos < len(src):
        m = TOKEN.match(src, pos)
        out.append((m.lastgroup, m.group()))
        pos = m.end()
    return out


def _floating(t):
    return not t.lower().startswith('0x') and ('.' in t or 'e' in t.lower()) and not t[-1] in 'uUlL'


def _close(toks, i):
    """The index of the parenthesis closing the one at toks[i]."""
    depth = 0
    for j in range(i, len(toks)):
        if toks[j][1] == '(':
            depth += 1
        elif toks[j][1] == ')':
            depth -= 1
            if depth == 0:
                return j
    raise SyntaxError('unbalanced parentheses')


def _compares(toks):
    """Whether an argument is a comparison at its own level (so its value is a bool vector); a ternary
    whose condition compares is not."""
    depth, found = 0, False
    for kind, t in toks:
        if t in '([{':
            depth += 1
        elif t in ')]}':
            depth -= 1
        elif depth == 0 and t == '?':
            return False
        elif depth == 0 and t in ('<', '>', '<=', '>=', '==', '!='):
            found = True
    return found


def _arguments(toks, templates=False):
    """Split at the commas of this level (in a declaration, `templates`: inside <> too)."""
    opens, closes = ('([{<', ')]}>') if templates else ('([{', ')]}')
    args, depth, cur = [], 0, []
    for tok in toks:
        if tok[1] in opens:
            depth += 1
        elif tok[1] in closes:
            depth -= 1
        if depth == 0 and tok[1] == ',':
            args.append(cur)
            cur = []
        else:
            cur.append(tok)
    if any(k != 'space' and k != 'comment' for k, _ in cur):
        args.append(cur)
    return args


def rewrite(src):
    toks = tokens(src)
    out, wrap = [], {}                                     # wrap: token index -> text to put before it
    for i, (kind, t) in enumerate(toks):
        if kind == 'number' and _floating(t):
            t = (t[:-1] if t[-1] in 'hHfF' else t) + 'f'
        elif kind == 'ident' and (t in VECTORS or t in MATRICES):
            j = i + 1
            while j < len(toks) and toks[j][0] in ('space', 'comment'):
                j += 1
            if j < len(toks) and toks[j][1] == '(':
                name = t.replace('half', 'float')
                t = f'__mtl_make_matrix<{name}>' if t in MATRICES else f'__mtl_make<{name}>'
                # a comparison among the elements: Metal's true is 1, clang's -1
                k = j + 1
                for arg in _arguments(toks[j + 1:_close(toks, j)]):
                    if _compares(arg):
                        wrap[k] = wrap.get(k, '') + '__mtl_truth('
                        wrap[k + len(arg)] = ')' + wrap.get(k + len(arg), '')
                    k += len(arg) + 1
        if i in wrap:
            t = wrap.pop(i) + t
        out.append(t)
    return ''.join(out) + ''.join(wrap.values())


# ---- entry points: the fragment and kernel functions, and their bindings

ENTRY = re.compile(r'\b(fragment|kernel)\s+([\w:<>, ]+?)\s+(\w+)\s*\(', re.S)


def entries(src):
    """Each entry point: (kind, name, [(type, name, attribute)])."""
    found = []
    clean = re.sub(r'//[^\n]*|/\*.*?\*/', lambda m: ' ' * len(m.group()), src, flags=re.S)
    for m in ENTRY.finditer(clean):
        toks = tokens(clean[m.end() - 1:])
        close = _close(toks, 0)
        params = []
        for arg in _arguments(toks[1:close], templates=True):
            text = ''.join(t for _, t in arg).strip()
            attr = re.search(r'\[\[\s*(.*?)\s*\]\]', text)
            decl = re.sub(r'\[\[.*?\]\]', '', text).strip()
            pm = re.match(r'(.*?)([&*]?)\s*(\w+)$', decl, re.S)
            ptype, ref, pname = pm.group(1).strip(), pm.group(2), pm.group(3)
            if ptype.endswith(('&', '*')):
                ptype, ref = ptype[:-1].strip(), ptype[-1]
            params.append((ptype, ref, pname, attr.group(1) if attr else ''))
        found.append((m.group(1), m.group(3), params))
    return found


def _stage_member(src, struct):
    m = re.search(r'struct\s+' + struct + r'\s*\{(.*?)\};', src, re.S)
    if m:
        pm = re.search(r'(\w+)\s*\[\[\s*position\s*\]\]', m.group(1))
        if pm:
            return pm.group(1)
    raise SyntaxError(f'{struct} has no [[position]] member')


def adapter(src, kind, name, params, symbol):
    """The C entry the runtime calls for one fragment or kernel."""
    args, setup = [], []
    for ptype, ref, pname, attr in params:
        base = ptype.replace('constant ', '').replace('device ', '').replace('const ', '').strip()
        slot = re.match(r'(buffer|texture)\((\d+)\)', attr)
        if attr == 'stage_in':
            member = _stage_member(src, base)
            setup.append(f'{base} in_; in_.{member} = float4{{x, y, 0.5f, 1.0f}};')
            args.append('in_')
        elif attr == 'position':
            args.append('float4{x, y, 0.5f, 1.0f}')
        elif attr == 'thread_position_in_grid':
            args.append({'uint': 'x', 'uint2': 'uint2{x, y}', 'uint3': 'uint3{x, y, z}'}[base])
        elif slot and slot.group(1) == 'buffer':
            k = slot.group(2)
            args.append(f'*(const {base} *)b->buffer[{k}]' if ref == '&' else f'(const {base} *)b->buffer[{k}]')
        elif slot:
            args.append(f'{base}{{b->texture[{slot.group(2)}]}}')
        else:
            raise SyntaxError(f'{name}: the stand-in does not bind [[{attr}]] ({pname}); add it to metal.py')
    if kind == 'fragment':
        return (f'extern "C" void {symbol}(const __mtl_bindings *b, float x, float y, float *out) {{\n'
                f'    {" ".join(setup)}\n    float4 r = {name}({", ".join(args)});\n'
                f'    out[0] = r.x; out[1] = r.y; out[2] = r.z; out[3] = r.w;\n}}\n')
    return (f'extern "C" void {symbol}(const __mtl_bindings *b, unsigned x, unsigned y, unsigned z) {{\n'
            f'    {name}({", ".join(args)});\n}}\n')


def translate(path, only=None):
    """The C++ for a .metal file: rewritten, with an adapter for every entry point (or only `only`,
    under the runtime's fixed symbol)."""
    path = Path(path)
    src = path.read_text()
    shown = os.path.relpath(path, ROOT) if path.is_relative_to(ROOT) else str(path)
    out = [f'#line 1 "{shown}"\n', rewrite(src), '\n#line 1 "(metal.py adapters)"\n', 'using namespace metal;\n']
    for kind, name, params in entries(src):
        if only is None:
            out.append(adapter(src, kind, name, params, f'__mtl_check_{name}'))
        elif name == only:
            out.append(adapter(src, kind, name, params, '__mtl_fragment' if kind == 'fragment' else '__mtl_kernel'))
    return ''.join(out)


def compile_check(path, warnings=False):
    """(ok, messages) for one file, compiled as the Mac would and with every binding checked."""
    with tempfile.NamedTemporaryFile('w', suffix='.cpp', delete=False) as f:
        f.write(translate(path))
    try:
        cmd = [CXX, *FLAGS, '-fsyntax-only', *([] if warnings else ['-w']), f.name]
        r = subprocess.run(cmd, capture_output=True, text=True)
        return r.returncode == 0, r.stderr
    finally:
        os.unlink(f.name)


def shaders():
    return sorted((ROOT / 'App').rglob('*.metal'))


# ---- building and running

def build(path, function, defines=()):
    """A binary that runs one fragment or kernel of a .metal file on the CPU (cached by its sources).
    `defines`: the stand-in's switches (MTL_BASE_LEVEL: every sample from the first level)."""
    cpp = translate(path, only=function)
    flags = [*FLAGS, *(f'-D{d}' for d in defines)]
    key = hashlib.sha256((cpp + RUNTIME.read_text() + (INCLUDE / 'metal_stdlib').read_text()
                          + (INCLUDE / 'metal_runtime.h').read_text() + ' '.join(flags) + CXX).encode()).hexdigest()[:16]
    BUILD.mkdir(parents=True, exist_ok=True)
    exe = BUILD / f'{Path(path).stem}-{function}-{key}'
    if not exe.exists():
        src = BUILD / f'{Path(path).stem}-{function}-{key}.cpp'
        src.write_text(cpp)
        cmd = [CXX, *flags, '-O2', '-w', str(src), '-x', 'c++', str(RUNTIME), '-o', str(exe), '-lpthread', '-lm']
        r = subprocess.run(cmd, capture_output=True, text=True)
        if r.returncode:
            raise RuntimeError(f'{path}: {function} did not build\n{r.stderr}')
    return exe


class Texture:
    """A texture as the device holds it: per level, RGBA bytes for an 8-bit unorm format (r8Unorm,
    rgba8Unorm: what a sample reads is the byte over 255), else RGBA floats."""

    def __init__(self, rgba, levels=1, unorm8=True):
        a = np.asarray(rgba)
        if a.ndim == 3:
            a = a[None]
        if unorm8 and a.dtype != np.uint8:
            a = np.round(np.clip(a, 0, 1) * 255).astype(np.uint8)
        self.unorm8 = unorm8
        self._levels = [a if unorm8 else a.astype(np.float32)]
        for _ in range(1, levels):
            self._levels.append(self._half(self._levels[-1]))
        d, h, w, _ = a.shape
        self.shape, self.count, self._file = (w, h, d), levels, None

    @classmethod
    def _kept(cls, path, w, h, d, levels, unorm8):
        t = cls.__new__(cls)
        t.unorm8, t._levels, t.shape, t.count, t._file = unorm8, None, (w, h, d), levels, Path(path)
        return t

    @property
    def levels(self):
        if self._levels is None:                            # (read back from its file only when asked)
            raw = np.fromfile(self._file, np.uint8 if self.unorm8 else np.float32)
            self._levels, at = [], 0
            w, h, d = self.shape
            for k in range(self.count):
                lw, lh, ld = max(w >> k, 1), max(h >> k, 1), max(d >> k, 1)
                self._levels.append(raw[at:at + lw * lh * ld * 4].reshape(ld, lh, lw, 4))
                at += lw * lh * ld * 4
        return self._levels

    def _half(self, a):
        # a 2x2 box, as a blit encoder's generateMipmaps; an odd edge keeps its last texel
        a = a.astype(np.float32)
        d, h, w, _ = a.shape
        a = a[:, :max(h // 2, 1) * 2 if h > 1 else 1, :max(w // 2, 1) * 2 if w > 1 else 1]
        if h > 1:
            a = (a[:, 0::2] + a[:, 1::2]) / 2
        if w > 1:
            a = (a[:, :, 0::2] + a[:, :, 1::2]) / 2
        return np.round(a).astype(np.uint8) if self.unorm8 else a

    @staticmethod
    def full_levels(w, h):
        n = 1
        while max(w, h) >> n:
            n += 1
        return n

    @classmethod
    def from_bytes(cls, data, channels, mipmapped=False):
        """8-bit data as an 8-bit texture of that many channels (r8, rg8, rgba8: the rest 0, alpha 1)."""
        data = np.asarray(data, np.uint8)
        h, w = data.shape[:2]
        out = np.zeros((h, w, 4), np.uint8)
        out[..., 3] = 255
        d = data.reshape(h, w, -1)
        k = min(channels, d.shape[2])
        out[..., :k] = d[..., :k]
        return cls(out, cls.full_levels(w, h) if mipmapped else 1, True)

    @classmethod
    def from_png(cls, path, channels, mipmapped=False, premultiply=False):
        """A picture's bytes as they are (no colour management, as the app uploads its maps), in a
        texture of `channels` (an RGB picture gets an opaque alpha); `premultiply` as GlobeView does
        for its ornaments."""
        st = Path(path).stat()
        key = hashlib.sha256(f'{Path(path).resolve()}|{st.st_mtime_ns}|{st.st_size}|{channels}|{mipmapped}|{premultiply}|1'
                             .encode()).hexdigest()[:20]
        meta = BUILD / 'textures' / f'{key}.json'
        if meta.exists():                                   # (decoded before: read as it was kept)
            m = json.loads(meta.read_text())
            if (BUILD / 'textures' / m['file']).exists():
                return cls._kept(BUILD / 'textures' / m['file'], m['w'], m['h'], 1, m['levels'], True)
        from PIL import Image
        img = Image.open(path)
        a = np.asarray(img)
        if a.ndim == 2:
            a = a[..., None]
        if a.shape[2] == 3:
            a = np.concatenate([a, np.full(a.shape[:2] + (1,), 255, np.uint8)], 2)
        if premultiply and a.shape[2] == 4:
            al = a[..., 3:4].astype(np.uint16)
            a = np.concatenate([((a[..., :3].astype(np.uint16) * al + 127) // 255).astype(np.uint8), a[..., 3:4]], 2)
        t = cls.from_bytes(a, channels, mipmapped)
        meta.write_text(json.dumps({'file': t.file().name, 'w': t.shape[0], 'h': t.shape[1], 'levels': t.count}))
        return t

    @classmethod
    def blank(cls, w, h, d=1, unorm8=True):
        return cls(np.zeros((d, h, w, 4), np.uint8 if unorm8 else np.float32), 1, unorm8)

    def file(self):
        """Its levels in one file, as the runtime reads them (kept, by content, in build/metal/)."""
        if self._file is None:
            digest = hashlib.sha256()
            for a in self.levels:
                digest.update(np.ascontiguousarray(a).data)
            path = BUILD / 'textures' / digest.hexdigest()[:20]
            if not path.exists():
                path.parent.mkdir(parents=True, exist_ok=True)
                tmp = path.with_suffix('.part')
                with open(tmp, 'wb') as f:
                    for a in self.levels:
                        f.write(np.ascontiguousarray(a).tobytes())
                tmp.replace(path)
            self._file = path
        return self._file

    def floats(self, level=0):
        a = self.levels[level]
        return a.astype(np.float32) / 255 if self.unorm8 else a


def run(path, function, size, buffers=(), textures=(), threads=None, grid=None, output=None, parted=False, defines=()):
    """Runs a fragment over a w x h drawable (RGBA floats, premultiplied as the shader returns them), or
    a kernel over `grid`, returning the texture bound at `output` as it wrote it. Buffers are bytes,
    textures Texture, each by its index. Also returns the runtime's report (derivatives that parted),
    and with `parted` a mask of the pixels whose quad took a derivative apart (undefined on a GPU)."""
    exe = build(path, function, defines)
    with tempfile.TemporaryDirectory() as tmp:
        tmp = Path(tmp)
        job = [f'threads {threads or os.cpu_count() or 4}']
        if grid:
            job.append(f'kernel {grid[0]} {grid[1]} {grid[2]}')
        else:
            job.append(f'fragment {size[0]} {size[1]}')
        for i, b in dict(buffers).items():
            p = tmp / f'buffer{i}'
            p.write_bytes(bytes(b))
            job.append(f'buffer {i} {p}')
        for i, t in dict(textures).items():
            w, h, d = t.shape
            job.append(f'texture {i} {t.file()} {w} {h} {d} {t.count} {1 if t.unorm8 else 0}')
        result = tmp / 'out'
        job.append(f'output {result} {output if output is not None else -1}')
        (tmp / 'job').write_text('\n'.join(job) + '\n')
        r = subprocess.run([str(exe), str(tmp / 'job')], capture_output=True, text=True)
        if r.returncode:
            raise RuntimeError(f'{path}: {function} failed\n{r.stderr}')
        if grid:
            t = dict(textures)[output]
            w, h, d = t.shape
            raw = np.fromfile(result, np.uint8 if t.unorm8 else np.float32)
            return Texture(raw[:w * h * d * 4].reshape(d, h, w, 4), 1, t.unorm8), r.stderr
        image = np.fromfile(result, np.float32).reshape(size[1], size[0], 4)
        if parted:
            return image, r.stderr, np.fromfile(f'{result}.parted', np.uint8).reshape(size[1], size[0]).astype(bool)
        return image, r.stderr


def to_png(rgba, path, background=None):
    """What the drawable would hold: premultiplied, over a background if given, in 8 bits."""
    from PIL import Image
    a = np.asarray(rgba, np.float32)
    if background is not None:
        bg = np.asarray(background, np.float32)
        a = a.copy()
        a[..., :3] = a[..., :3] + bg[..., :3] * (1 - a[..., 3:4])
        a[..., 3] = 1
    img = np.round(np.clip(a, 0, 1) * 255).astype(np.uint8)
    if background is None:                                 # PNG wants straight alpha
        al = img[..., 3:4].astype(np.float32) / 255
        rgb = np.where(al > 0, np.clip(a[..., :3] / np.maximum(a[..., 3:4], 1e-6), 0, 1), 0)
        img[..., :3] = np.round(rgb * 255).astype(np.uint8)
    Image.fromarray(img, 'RGBA').save(path)


# ---- the command line

def cmd_check(args):
    files = [Path(p) for p in args.files] or shaders()
    bad = 0
    for f in files:
        ok, msg = compile_check(f, args.warnings)
        shown = os.path.relpath(f.resolve(), ROOT)
        names = ', '.join(n for _, n, _ in entries(f.read_text()))
        print(f'{"ok  " if ok else "FAIL"} {shown} ({names})')
        if msg.strip():
            print(msg.rstrip())
        bad += not ok
    if bad:
        print(f'{bad} of {len(files)} shaders would not compile (tools/metal/README.md says what the stand-in checks)')
    return 1 if bad else 0


def main():
    a = argparse.ArgumentParser(description=__doc__.split('\n')[0])
    sub = a.add_subparsers(dest='cmd', required=True)
    c = sub.add_parser('check', help='compile every shader (or those named) as the Mac would')
    c.add_argument('files', nargs='*')
    c.add_argument('--warnings', action='store_true', help="show the compiler's warnings too")
    c.set_defaults(fn=cmd_check)
    try:
        import layers
        layers.commands(sub)
    except ImportError as e:                            # (`check` needs only clang: CI's Linux job)
        for name in ('shot', 'twin'):
            p = sub.add_parser(name, help=f'(needs {e.name}: pip install numpy pillow)')
            p.add_argument('rest', nargs='*')
            p.set_defaults(fn=lambda _, name=name, need=e.name: sys.exit(f'{name} needs {need}: pip install numpy pillow'))
    args = a.parse_args()
    sys.exit(args.fn(args))


if __name__ == '__main__':
    sys.path.insert(0, str(HERE))
    main()
