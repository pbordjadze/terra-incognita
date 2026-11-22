//! The drop plotted as the SC 4020 plotted Frank Sinden's *Force, Mass and Motion* (Bell Labs, 1965): a
//! beam on a tube face, drawing short straight vectors between addressable points ("the maximum vector
//! length is 1/16th of maximum deflection", so a curve is a chain of them) and plotted points, photographed
//! frame by frame (SOURCES.md). Every mark here is one of those: the drop's rim a polygon of short
//! vectors, a cell a point or a short rod, its swim a vector behind it, a spirillum a zig-zag chain, a
//! protist an outline, silt on the glass single points. No haze and no soft focus: the tube draws lines
//! or nothing. The photographic film is `sc4020.rs`.

use crate::drop::Drop;
use std::f32::consts::TAU;
use toy_kit::paint::Painter;

/// The beam's half-width and edge, in points.
const BEAM: f32 = 0.32;
const EDGE: f32 = 0.3;
/// One cell in this many is plotted: the drop's crowd thinned so each plotted body can be followed (the
/// rest swim on unplotted).
pub const PLOTTED: usize = 3;
/// How many places a plotted cell's path keeps, one laid every `EVERY` film frames: separate points.
pub const TRAIL: usize = 6;
pub const EVERY: i64 = 3;

/// One vector of the beam at brightness `i` (0..1).
fn vector(painter: &mut Painter, a: [f32; 2], b: [f32; 2], k: f32, i: f32) {
    painter.glow(a, b, BEAM * k, EDGE * k, [1.0, 1.0, 1.0, i]);
}

/// A closed outline of `n` vectors round `c`, half-axes `u` and `v` along heading `dir`.
fn outline(
    painter: &mut Painter,
    c: [f32; 2],
    (u, v): (f32, f32),
    dir: f32,
    n: usize,
    k: f32,
    i: f32,
) {
    let (dx, dy) = (dir.cos(), dir.sin());
    let at = |j: usize| {
        let t = j as f32 / n as f32 * TAU;
        let (a, b) = (t.cos() * u, t.sin() * v);
        [(c[0] + dx * a - dy * b) * k, (c[1] + dy * a + dx * b) * k]
    };
    for j in 0..n {
        vector(painter, at(j), at(j + 1), k, i);
    }
}

/// Silt on the glass: the same few points every frame, made from the drop's size (points).
pub fn silt(d: &Drop) -> Vec<[f32; 2]> {
    let mut h: u64 = 0x5117_D00D;
    let mut f = || {
        h ^= h << 13;
        h ^= h >> 7;
        h ^= h << 17;
        (h >> 40) as f32 / (1u64 << 24) as f32
    };
    (0..140)
        .filter_map(|_| {
            let (a, r) = (f() * TAU, f().sqrt() * 0.95);
            let p = [d.c[0] + a.cos() * r * d.r, d.c[1] + a.sin() * r * d.ry];
            (f() < 0.8).then_some(p)
        })
        .collect()
}

/// Adds this frame's place of each plotted cell to its path (`trails`, one per plotted cell, oldest
/// first), keeping the last `TRAIL`.
pub fn lay(trails: &mut Vec<Vec<[f32; 2]>>, d: &Drop) {
    let n = d.cells.len().div_ceil(PLOTTED);
    trails.resize(n, Vec::new());
    for (t, c) in trails.iter_mut().zip(d.cells.iter().step_by(PLOTTED)) {
        if t.len() == TRAIL {
            t.remove(0);
        }
        t.push(c.p);
    }
}

/// Plots the drop, its cells, their paths and the fingers' poles into `painter` (pixels a point `k`).
pub fn plot(
    painter: &mut Painter,
    d: &Drop,
    trails: &[Vec<[f32; 2]>],
    silt: &[[f32; 2]],
    fingers: &[[f32; 2]],
    k: f32,
) {
    // the rim: a polygon of short vectors, and a second just inside, the meniscus's two edges
    outline(painter, d.c, (d.r, d.ry), 0.0, 96, k, 0.95);
    outline(painter, d.c, (d.r - 4.0, d.ry - 4.0), 0.0, 96, k, 0.3);
    for s in silt {
        let p = [s[0] * k, s[1] * k];
        vector(painter, p, p, k, 0.35);
    }
    // the protists: outlines, as a cell wall catches the light
    for f in &d.drifters {
        outline(painter, f.p, (f.len * 0.4, f.len * 0.2), f.a, 20, k, 0.55);
    }
    for (j, c) in d.cells.iter().step_by(PLOTTED).enumerate() {
        // its path: the points it was plotted at, frame after frame, as a plot lays a body's orbit down
        if let Some(t) = trails.get(j) {
            for p in t {
                let q = [p[0] * k, p[1] * k];
                vector(painter, q, q, k, 0.6);
            }
        }
        let wob = 0.35 * c.phase.sin();
        let (dx, dy) = ((c.a + wob).cos(), (c.a + wob).sin());
        let head = [c.p[0] * k, c.p[1] * k];
        // a cell far from the plane of focus is plotted at less intensity, never blurred
        let i = c.glint * (1.0 - 0.6 * c.blur);
        if c.spiral {
            let (nx, ny) = (-dy, dx);
            let mut prev = head;
            for j in 1..=6 {
                let u = j as f32 / 6.0;
                let w = (u * 9.0 + c.phase * 2.0).sin() * 0.8 * k;
                let p = [
                    head[0] - dx * u * 7.0 * k + nx * w,
                    head[1] - dy * u * 7.0 * k + ny * w,
                ];
                vector(painter, prev, p, k, 0.8 * i);
                prev = p;
            }
        } else {
            let rod = 1.1 * k;
            vector(
                painter,
                [head[0] - dx * rod, head[1] - dy * rod],
                [head[0] + dx * rod, head[1] + dy * rod],
                k,
                i,
            );
        }
    }
    // the finger's pole: a circle and a cross, as a plot marks a point it names
    for f in fingers {
        outline(painter, *f, (16.0, 16.0), 0.0, 32, k, 0.6);
        let (p, a) = ([f[0] * k, f[1] * k], 4.0 * k);
        vector(painter, [p[0] - a, p[1]], [p[0] + a, p[1]], k, 0.8);
        vector(painter, [p[0], p[1] - a], [p[0], p[1] + a], k, 0.8);
    }
}
