#!/usr/bin/env python3
"""Reiter's hexagonal snow-crystal automaton (C. A. Reiter, "A local cellular model for snow crystal
growth", Chaos, Solitons & Fractals 23, 2005) in numpy: the reference the toy's frost.wgsl follows,
for tuning its numbers off the device.

    python3 tools/toys/frost/reiter.py --beta 0.4 --gamma 0.001 --steps 2000 --n 201 out.png

Axial hex coordinates (q, r) on an n x n array; a cell's neighbours are (q±1, r), (q, r±1),
(q+1, r-1), (q-1, r+1). Each step: receptive cells (frozen, s >= 1, or next to one) gain gamma;
the rest's water diffuses (u += alpha/2 (mean of the six - u), receptive cells counting 0); the edge
is held at beta. Prints the steps to reach each radius.
"""
import argparse

import numpy as np
from PIL import Image


def nbrs(a):
    return [np.roll(a, 1, 0), np.roll(a, -1, 0), np.roll(a, 1, 1), np.roll(a, -1, 1),
            np.roll(np.roll(a, 1, 0), -1, 1), np.roll(np.roll(a, -1, 0), 1, 1)]

def run(n, alpha, beta, gamma, steps, every=None, edge=None):
    s = np.full((n, n), beta, np.float64)
    c = n // 2
    s[c, c] = 1.0
    q, r = np.meshgrid(np.arange(n) - c, np.arange(n) - c, indexing="ij")
    dist = (np.abs(q) + np.abs(r) + np.abs(q + r)) / 2
    border = dist >= c - 1
    reached = {}
    for k in range(steps):
        fr = s >= 1
        rec = fr.copy()
        for m in nbrs(fr):
            rec |= m
        u = np.where(rec, 0.0, s)
        v = np.where(rec, s + gamma, 0.0)
        mean = sum(nbrs(u)) / 6
        u = u + alpha / 2 * (mean - u)
        s = u + v
        s[border] = beta
        rmax = dist[s >= 1].max()
        for R in (10, 25, 50, 75, 95):
            if rmax >= R and R not in reached:
                reached[R] = k
        if rmax >= c - 3:
            break
    return s, dist, reached, k

def picture(s, n, scale=3):
    # the hex array drawn in the plane: x = q + r/2, y = r sqrt(3)/2
    W = int(n * scale * 1.0)
    ys, xs = np.mgrid[0:W, 0:W]
    X = (xs - W / 2) / scale
    Y = (ys - W / 2) / scale
    r = Y / (np.sqrt(3) / 2)
    q = X - r / 2
    qi = np.clip(np.round(q).astype(int) + n // 2, 0, n - 1)
    ri = np.clip(np.round(r).astype(int) + n // 2, 0, n - 1)
    v = s[qi, ri]
    out = np.where(v >= 1, 0.35 + 0.65 * np.clip((v - 1) / 0.6, 0, 1), v * 0.25)
    return Image.fromarray((np.clip(out, 0, 1) * 255).astype(np.uint8))

if __name__ == "__main__":
    ap = argparse.ArgumentParser()
    ap.add_argument("--alpha", type=float, default=1.0)
    ap.add_argument("--beta", type=float, default=0.4)
    ap.add_argument("--gamma", type=float, default=0.001)
    ap.add_argument("--steps", type=int, default=3000)
    ap.add_argument("--n", type=int, default=201)
    ap.add_argument("out")
    a = ap.parse_args()
    s, dist, reached, k = run(a.n, a.alpha, a.beta, a.gamma, a.steps)
    print(f"beta {a.beta} gamma {a.gamma}: steps {k}, reached {reached}")
    picture(s, a.n).save(a.out)
