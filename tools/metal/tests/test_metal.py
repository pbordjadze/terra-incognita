"""The Metal stand-in's own tests: it refuses what Metal refuses, accepts what Metal accepts, and
computes what the GPU computes where that is known.

    python3 tools/metal/tests/test_metal.py
"""
import struct, sys, tempfile, unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent))
import metal  # noqa: E402

CLOUDS = metal.ROOT / 'App/TerraIncognita/Clouds/Clouds.metal'
PROBE = HERE / 'probe.metal'


def compiles(src):
    with tempfile.NamedTemporaryFile('w', suffix='.metal', delete=False) as f:
        f.write(src)
    try:
        return metal.compile_check(Path(f.name))
    finally:
        Path(f.name).unlink()


class Refuses(unittest.TestCase):
    """Mistakes a session makes, often carried over from a GLSL twin: each must fail here as on the Mac."""

    MUTATIONS = [
        ('a GLSL vector type', 'float3 g = normalize(', 'vec3 g = normalize('),
        ("GLSL's mod", 'fmod(time.y, 4.0)', 'mod(time.y, 4.0)'),
        ("GLSL's two-argument atan", 'atan2(g.x, g.z)', 'atan(g.x, g.z)'),
        ("GLSL's texture()", 'landMask.sample(eastWest, uv)', 'texture(landMask, uv)'),
        ('a wider vector into a narrower', 'float3 white = float3(1.0, 0.995, 0.982)', 'float3 white = float4(1.0, 0.995, 0.982, 1.0)'),
        ('too many elements', 'float3 white = float3(1.0, 0.995, 0.982)', 'float3 white = float3(1.0, 0.995, 0.982, 1.0)'),
        ('too few elements', 'float3 white = float3(1.0, 0.995, 0.982)', 'float3 white = float3(1.0, 0.995)'),
        ('a missing semicolon', 'float r2 = dot(v, v);', 'float r2 = dot(v, v)'),
        ('a uniform that does not exist', 'U.disc.z', 'U.disk.z'),
        ('a 2D texture sampled at a 3D point', 'sky.sample(held, t0 / size)', 'sky.sample(held, float3(t0 / size, 0.0))'),
        ('a function given too few arguments', 'smoothstep(22.0, 34.0, lat)', 'smoothstep(22.0, lat)'),
        ('an undeclared variable', 'float px = 1.0 / (U.frame.z * R);', 'float px = 1.0 / (U.frame.z * radius);'),
        ("GLSL's inout", 'static void over(thread float4 &acc,', 'static void over(inout float4 acc,'),
        ('a float vector where an int vector is wanted', 'int3 cell = (int3(i) + d) % 16;', 'int3 cell = (i + d) % 16;'),
    ]

    def test_each_mutation_fails(self):
        src = CLOUDS.read_text()
        for name, old, new in self.MUTATIONS:
            with self.subTest(name):
                self.assertIn(old, src, f'{name}: the shader changed; update the mutation')
                ok, msg = compiles(src.replace(old, new, 1))
                self.assertFalse(ok, f'{name} compiled: the stand-in is laxer than Metal here')

    def test_errors_name_the_metal_line(self):
        src = CLOUDS.read_text().replace('U.disc.z', 'U.disk.z', 1)
        line = src[:src.index('U.disk.z')].count('\n') + 1
        ok, msg = compiles(src)
        self.assertIn(f'.metal:{line}:', msg)


class Accepts(unittest.TestCase):
    def test_the_apps_shaders(self):
        for path in metal.shaders():
            with self.subTest(path.name):
                ok, msg = metal.compile_check(path)
                self.assertTrue(ok, msg)

    def test_metal_forms(self):
        ok, msg = compiles('''#include <metal_stdlib>
using namespace metal;
static float4 forms(float3 a, float3 b, float2 c) {
    float3 z = 0.5; float3 o = float3(1); float2 xy = float2(a.xy); int i = int(floor(1.5));
    float3x3 m = float3x3(a, b, a); float3 d = m * a + a * m; float e = max(a.x, 0.5);
    float3 f = mix(a, b, 0.5); float3 g = clamp(a, 0.0, 1.0); float3 h = select(a, b, a < b);
    bool k = all(a < b) || any(a > b); float4 q = float4(a, 1); uint u = uint(e); int2 n = int2(c);
    half hh = 0.5h; float3 s = smoothstep(0.0, 1.0, a); float3 t = float3(a < b);
    float4 w = float4(c, c); float3 r = a.zyx * 2.0 + b.rgb; r.xy = c; r.x += 1;
    float l = length(c) + distance(a, b) + dot(a, b); float3 cr = normalize(cross(a, b));
    return float4(z + o + d + f + g + h + s + t + r + cr, e + l + float(i) + float(u) + float(n.x) + hh + w.x + (k ? 1 : 0));
}
fragment float4 f(float4 p [[position]]) { return forms(p.xyz, p.yzx, p.xy); }
''')
        self.assertTrue(ok, msg)


class Computes(unittest.TestCase):
    """What the GPU gives, where it is known exactly."""

    def frag(self, which, texture=None):
        tex = texture or metal.Texture.blank(4, 4)
        img, report = metal.run(PROBE, 'probeFragment', (4, 4), buffers={0: struct.pack('4f', which, 0, 0, 0)},
                                textures={0: tex})
        return img, report

    def filled(self):
        t, _ = metal.run(PROBE, 'probeFill', None, textures={0: metal.Texture.blank(4, 4)}, grid=(4, 4, 1), output=0)
        return t

    def test_kernel_writes_as_unorm8(self):
        t = self.filled()
        self.assertEqual(list(t.levels[0][0, 1, 3]), [255, 85, 128, 255])

    def test_derivatives_across_the_quad(self):
        img, report = self.frag(0)
        for y in range(4):
            for x in range(4):
                dx, dy, fw, dyy = img[y, x]
                self.assertEqual((dx, dy, fw), (1, 1, 3))
                y0 = (y // 2) * 2 + 0.5
                self.assertAlmostEqual(dyy, (y0 + 1) ** 2 - y0 ** 2)   # fine: within the quad
        self.assertEqual(report.strip(), '')

    def test_bilinear_and_gather(self):
        t = self.filled()
        img, _ = self.frag(1, t)
        self.assertAlmostEqual(float(img[0, 0, 0]), 0.5, places=5)
        img, _ = self.frag(2, t)
        # gather: (0,1), (1,1), (1,0), (0,0) of the footprint at (1,1)
        self.assertEqual([round(float(v) * 3) for v in img[0, 0]], [1, 2, 2, 1])

    def test_comparison_made_a_vector_is_one(self):
        img, _ = self.frag(3)
        self.assertEqual(list(img[0, 0]), [1, 0, 0, 1])

    def test_matrices_by_columns_and_fract(self):
        img, _ = self.frag(4)
        self.assertEqual(list(img[0, 0]), [1, 2, 3, 0.75])

    def test_divergent_derivatives_are_reported(self):
        src = '''#include <metal_stdlib>
using namespace metal;
struct P { float4 position [[position]]; };
fragment float4 f(P in [[stage_in]]) {
    if (in.position.x < 1.0) return float4(0.0);
    return float4(dfdx(in.position.y), 0.0, 0.0, 1.0);
}
'''
        with tempfile.NamedTemporaryFile('w', suffix='.metal', delete=False) as fh:
            fh.write(src)
        try:
            img, report = metal.run(Path(fh.name), 'f', (4, 2))
        finally:
            Path(fh.name).unlink()
        self.assertIn('line 6', report)


class Rewrites(unittest.TestCase):
    def test_literals(self):
        self.assertEqual(metal.rewrite('x = 0.5 + 1e-5 + 3. + .25 + 0.5h + 2 + 0x1F + 1u;'),
                         'x = 0.5f + 1e-5f + 3.f + .25f + 0.5f + 2 + 0x1F + 1u;')

    def test_comments_and_lines_kept(self):
        src = '// float3(1.0, 2.0) in a comment\nfloat3 v = float3(1.0); /* 0.5 */\n'
        out = metal.rewrite(src)
        self.assertEqual(out.count('\n'), src.count('\n'))
        self.assertIn('// float3(1.0, 2.0) in a comment', out)
        self.assertIn('/* 0.5 */', out)

    def test_constructors(self):
        self.assertEqual(metal.rewrite('float3x3 m = float3x3(a, b, c); static float3 f(int3 c)'),
                         'float3x3 m = __mtl_make_matrix<float3x3>(a, b, c); static float3 f(int3 c)')


if __name__ == '__main__':
    unittest.main(verbosity=1)
