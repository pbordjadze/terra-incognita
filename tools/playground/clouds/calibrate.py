"""Measures the clouds shader's noise, so a measured cloud cover can become a threshold.

    python3 tools/playground/clouds/calibrate.py

The noise here is the shader's, line for line: pcg3d-hashed gradient noise, quintic, which sky.js
draws once into a tile 16 cells a side (wrapping round, six samples a cell, a byte each) that the
shader samples trilinearly, octaves rotated and 2.03 apart. Change them together. It prints one
octave's standard deviation, as the tile is sampled, which the shader divides by to score its fbm,
and that score's quantiles, which should be a standard normal's (5%: -1.645, 25%: -0.674, 50%: 0,
75%: 0.674, 95%: 1.645). The shader's logistic `cdf` then turns a score into the share of the sphere
below it, and thresholding at 1 - cover covers `cover` of it.
"""
import numpy as np

M32 = np.uint64(0xFFFFFFFF)
OCT = np.array([[0.00, 0.80, 0.60], [-0.80, 0.36, -0.48], [-0.60, -0.48, 0.64]])


def pcg3d(x, y, z):
    x = (x * 1664525 + 1013904223) & M32
    y = (y * 1664525 + 1013904223) & M32
    z = (z * 1664525 + 1013904223) & M32
    x = (x + y * z) & M32; y = (y + z * x) & M32; z = (z + x * y) & M32
    x ^= x >> np.uint64(16); y ^= y >> np.uint64(16); z ^= z >> np.uint64(16)
    x = (x + y * z) & M32; y = (y + z * x) & M32; z = (z + x * y) & M32
    return x, y, z


def gradient(ix, iy, iz):
    u = lambda a: (a.astype(np.int64) & 0xFFFFFFFF).astype(np.uint64)
    x, y, z = pcg3d(u(ix), u(iy), u(iz))
    return np.stack([x, y, z], -1).astype(np.float64) / 4294967295.0 * 2 - 1


def gnoise(p):
    i = np.floor(p)
    f = p - i
    u = f * f * f * (f * (f * 6 - 15) + 10)
    i = i.astype(np.int64)

    def corner(dx, dy, dz):
        return (gradient(i[..., 0] + dx, i[..., 1] + dy, i[..., 2] + dz) * (f - np.array([dx, dy, dz]))).sum(-1)

    mix = lambda a, b, t: a + (b - a) * t
    x0 = mix(mix(corner(0, 0, 0), corner(1, 0, 0), u[..., 0]), mix(corner(0, 1, 0), corner(1, 1, 0), u[..., 0]), u[..., 1])
    x1 = mix(mix(corner(0, 0, 1), corner(1, 0, 1), u[..., 0]), mix(corner(0, 1, 1), corner(1, 1, 1), u[..., 0]), u[..., 1])
    return mix(x0, x1, u[..., 2])


CELLS, PER = 16, 6


def tile():
    """sky.js's noiseTile: the noise at six samples a cell, wrapping every 16 cells, as bytes."""
    n = CELLS * PER
    c = (np.arange(n) + 0.5) / PER
    z, y, x = np.meshgrid(c, c, c, indexing='ij')
    p = np.stack([x, y, z], -1)
    i = np.floor(p).astype(np.int64)
    f = p - i
    u = f * f * f * (f * (f * 6 - 15) + 10)
    v = 0.0
    for dz in (0, 1):
        for dy in (0, 1):
            for dx in (0, 1):
                g = gradient((i[..., 0] + dx) % CELLS, (i[..., 1] + dy) % CELLS, (i[..., 2] + dz) % CELLS)
                w = (u[..., 0] if dx else 1 - u[..., 0]) * (u[..., 1] if dy else 1 - u[..., 1]) * (u[..., 2] if dz else 1 - u[..., 2])
                v = v + w * (g * (f - np.array([dx, dy, dz]))).sum(-1)
    return np.clip(np.round((v * 0.5 + 0.5) * 255), 0, 255) / 255 * 2 - 1      # [z, y, x]


def sampled(t, p):
    """The tile, trilinearly, at points p in cells (as the shader's texture lookup)."""
    n = t.shape[0]
    x = p * PER - 0.5
    i = np.floor(x).astype(np.int64)
    f = x - i
    out = 0.0
    for dz in (0, 1):
        for dy in (0, 1):
            for dx in (0, 1):
                w = (f[..., 0] if dx else 1 - f[..., 0]) * (f[..., 1] if dy else 1 - f[..., 1]) * (f[..., 2] if dz else 1 - f[..., 2])
                out = out + w * t[(i[..., 2] + dz) % n, (i[..., 1] + dy) % n, (i[..., 0] + dx) % n]
    return out


def main():
    rng = np.random.default_rng(2)
    v = rng.normal(size=(300000, 3))
    v /= np.linalg.norm(v, axis=1, keepdims=True)           # points on the sphere, as the shader samples
    print(f'the noise itself: sd {gnoise(v * 7.3 + 0.37).std():.4f}')
    t = tile()
    noise = lambda p: sampled(t, p)
    one = noise(v * 7.3 + 0.37)
    print(f'one octave, as the tile is sampled: mean {one.mean():.4f}, sd {one.std():.4f}')
    for octaves in (6, 3):
        s, a, w2, p = 0.0, 1.0, 0.0, v * 7.3
        for k in range(octaves):
            s = s + a * noise(p + 17.13 * k)
            w2 += a * a
            a *= 0.5
            p = (p @ OCT.T) * 2.03
        z = s / (one.std() * np.sqrt(w2))
        q = np.quantile(z, [0.05, 0.25, 0.5, 0.75, 0.95])
        print(f'{octaves} octaves, scored: mean {z.mean():.3f}, sd {z.std():.3f}, quantiles {np.round(q, 3)}')


if __name__ == '__main__':
    main()
