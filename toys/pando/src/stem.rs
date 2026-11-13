//! One stem, close: its chalky bark with the black scars where lower branches fell, a few branches,
//! and a few hundred round leaves. Each leaf hangs on a stalk flattened side to side, as long as the
//! blade, so the lightest air twists it over and back: gold, edge, pale underside, edge, gold. That
//! twist is a spring with little damping, kicked by the wind's turbulence, so a gust sets the whole
//! tree flickering, and a calm lets it settle to a tremble.

use crate::grove::{tone, with, PALE};
use crate::wind::hash;
use crate::Cam;
use std::f32::consts::{PI, TAU};
use toy_kit::paint::Painter;

pub struct Leaf {
    /// Where its stalk is attached (in screen widths) and the stalk's and blade's lengths.
    pub anchor: [f32; 2],
    pub stalk: f32,
    pub blade: f32,
    /// The angle it hangs at, still (radians; 0 points right, PI/2 down).
    pub rest: f32,
    /// Its twist about the stalk (0 face on, PI showing its underside) and its rate.
    pub twist: f32,
    pub twist_v: f32,
    /// Its swing in the picture's plane, and its rate.
    pub swing: f32,
    pub swing_v: f32,
    /// The twist's own frequency (Hz) and the seeds of its forcing.
    pub freq: f32,
    pub seed: u32,
}

/// The spring of a leaf's twist about its stalk: little damping, so it rings when kicked.
pub const TWIST_DAMP: f32 = 0.10;
/// How a leaf hangs and swings like a pendulum on its stalk (Hz).
const SWING_HZ: f32 = 1.1;

impl Leaf {
    /// One step of the leaf in a wind of `w` m/s blowing toward +x.
    pub fn step(&mut self, w: f32, t: f32, dt: f32) {
        let om = TAU * self.freq;
        let s1 = hash(self.seed ^ 0xa1) * TAU;
        let s2 = hash(self.seed ^ 0xb2) * TAU;
        // the turbulence's kicks, strongest near the leaf's own frequency; they grow with the wind's
        // square, as the force of moving air does
        let kick = (om * 0.93 * t + s1 + 1.5 * (0.37 * t + s2).sin()).sin()
            + 0.7 * (om * 1.11 * t + s2 + 1.1 * (0.23 * t + s1).sin()).sin();
        let force = 0.55 * w * w * om * om * 0.05 * kick;
        let acc = force - om * om * self.twist - 2.0 * TWIST_DAMP * om * self.twist_v;
        self.twist_v += acc * dt;
        self.twist += self.twist_v * dt;
        // the swing: a pendulum leaning downwind
        let ws = TAU * SWING_HZ;
        let lean = -0.06 * w * w / (1.0 + 0.05 * w * w);
        let jig = 0.04 * w * (1.3 * t + s1).sin();
        let acc = -ws * ws * (self.swing - lean - jig) - 2.0 * 0.15 * ws * self.swing_v;
        self.swing_v += acc * dt;
        self.swing += self.swing_v * dt;
    }

    /// The hanging angle now.
    pub fn angle(&self) -> f32 {
        self.rest + self.swing
    }
}

/// A finger's breath across the tree: where (px), which way (px/s), how strong and old.
struct Breath {
    at: [f32; 2],
    vel: [f32; 2],
    strength: f32,
    age: f32,
}

pub struct Stem {
    pub leaves: Vec<Leaf>,
    trunk: Vec<[f32; 2]>,
    scars: Vec<([f32; 2], f32)>,
    branches: Vec<Vec<[f32; 2]>>,
    twigs: Vec<([f32; 2], [f32; 2])>,
    breaths: Vec<Breath>,
    /// The tree's own gentle sway (screen widths) and its rate.
    sway: f32,
    sway_v: f32,
}

fn lerp2(a: [f32; 2], b: [f32; 2], k: f32) -> [f32; 2] {
    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k]
}

impl Stem {
    /// The stem of the grove's tree `seed`, laid out for a screen `aspect` (height / width) tall.
    pub fn new(seed: u32, aspect: f32) -> Stem {
        let mut k = seed;
        let mut rnd = || {
            k = k.wrapping_add(0x9e37_79b9);
            hash(k)
        };
        let h = aspect;
        // the trunk: up the left of the picture, leaning a little
        let mut trunk = Vec::new();
        for i in 0..=24 {
            let u = i as f32 / 24.0;
            let y = h * (1.05 - 1.12 * u);
            trunk.push([0.23 + 0.05 * u + 0.012 * (u * 7.0 + seed as f32).sin(), y]);
        }
        let mut scars = Vec::new();
        for _ in 0..7 + (rnd() * 5.0) as usize {
            let i = (rnd() * 22.0) as usize + 1;
            // within the trunk's width there
            let half = 0.07;
            scars.push((
                [
                    trunk[0][0] + (trunk[24][0] - trunk[0][0]) * i as f32 / 24.0 + 0.004,
                    trunk[i][1],
                ],
                half * (0.2 + 0.55 * rnd() * rnd()),
            ));
        }
        let mut branches = Vec::new();
        let mut twigs = Vec::new();
        let mut leaves = Vec::new();
        let nb = 6;
        for b in 0..nb {
            let u = 0.12 + 0.8 * (b as f32 + 0.5 * rnd()) / nb as f32;
            let i = ((1.0 - u) * 24.0) as usize;
            let start = trunk[i.min(24)];
            let mut p = [start[0] + 0.03, start[1]];
            let mut ang = -0.45 - 0.35 * rnd();
            let len = 0.75 + 0.35 * rnd();
            let mut pts = vec![p];
            for _ in 0..12 {
                ang += (rnd() - 0.45) * 0.14;
                p = [p[0] + ang.cos() * len / 12.0, p[1] + ang.sin() * len / 12.0];
                pts.push(p);
            }
            for j in (2..pts.len()).step_by(2) {
                for side in [-1.0f32, 1.0] {
                    let a = -PI / 2.0 + side * (0.4 + 0.7 * rnd()) + 0.3;
                    let l = 0.05 + 0.1 * rnd();
                    let e = [pts[j][0] + a.cos() * l, pts[j][1] + a.sin() * l];
                    twigs.push((pts[j], e));
                    for at in [e, lerp2(pts[j], e, 0.55)] {
                        for _ in 0..3 + (rnd() * 3.0) as usize {
                            let seed = (rnd() * 1.0e9) as u32;
                            leaves.push(Leaf {
                                anchor: [
                                    at[0] + (rnd() - 0.5) * 0.02,
                                    at[1] + (rnd() - 0.5) * 0.02,
                                ],
                                stalk: 0.035 + 0.025 * rnd(),
                                blade: 0.05 + 0.025 * rnd(),
                                rest: PI / 2.0 + (rnd() - 0.5) * 1.6,
                                twist: (rnd() - 0.5) * 0.6,
                                twist_v: 0.0,
                                swing: 0.0,
                                swing_v: 0.0,
                                freq: 3.0 + 2.0 * rnd(),
                                seed,
                            });
                        }
                    }
                }
            }
            branches.push(pts);
        }
        Stem {
            leaves,
            trunk,
            scars,
            branches,
            twigs,
            breaths: Vec::new(),
            sway: 0.0,
            sway_v: 0.0,
        }
    }

    /// A finger's drag across the tree: at `at` (screen widths), moving `vel` (widths a second).
    pub fn breathe(&mut self, at: [f32; 2], vel: [f32; 2]) {
        let s = (vel[0] * vel[0] + vel[1] * vel[1]).sqrt();
        if s < 0.02 {
            return;
        }
        self.breaths.push(Breath {
            at,
            vel,
            strength: 7.0 * (1.0 - (-s / 1.2).exp()),
            age: 0.0,
        });
        if self.breaths.len() > 40 {
            self.breaths.remove(0);
        }
    }

    fn breath_at(&self, p: [f32; 2]) -> f32 {
        let mut w = 0.0;
        for b in &self.breaths {
            let (dx, dy) = (p[0] - b.at[0], p[1] - b.at[1]);
            let env = (1.0 - b.age / 1.8).max(0.0);
            w += b.strength * env * (-(dx * dx + dy * dy) / 0.06).exp();
        }
        w
    }

    /// One step, in the grove's wind at this tree (`w`, m/s).
    pub fn step(&mut self, w: f32, t: f32, dt: f32) {
        for b in &mut self.breaths {
            b.at[0] += b.vel[0] * dt * 0.8;
            b.at[1] += b.vel[1] * dt * 0.8;
            b.age += dt;
        }
        self.breaths.retain(|b| b.age < 1.8);
        let om = TAU * 0.3;
        let acc = 0.0006 * w * w - om * om * self.sway - 2.0 * 0.1 * om * self.sway_v;
        self.sway_v += acc * dt;
        self.sway += self.sway_v * dt;
        // fine steps: a leaf's twist rings at a few hertz
        let n = (dt / (1.0 / 240.0)).ceil().max(1.0) as usize;
        let h = dt / n as f32;
        for i in 0..self.leaves.len() {
            let a = self.leaves[i].anchor;
            let local = w + self.breath_at(a);
            for s in 0..n {
                self.leaves[i].step(local, t + s as f32 * h, h);
            }
        }
    }

    /// How far a point at height `y` (screen widths) has swayed.
    fn moved(&self, p: [f32; 2], aspect: f32) -> [f32; 2] {
        let up = (1.0 - p[1] / aspect).clamp(0.0, 1.0);
        [p[0] + self.sway * up * up, p[1]]
    }

    /// Where a leaf's blade starts, and its middle (screen widths).
    pub fn blade(&self, l: &Leaf, aspect: f32) -> ([f32; 2], [f32; 2]) {
        let a = self.moved(l.anchor, aspect);
        let ang = l.angle();
        let d = [ang.cos(), ang.sin()];
        let base = [a[0] + d[0] * l.stalk, a[1] + d[1] * l.stalk];
        let mid = [
            base[0] + d[0] * l.blade * 0.5,
            base[1] + d[1] * l.blade * 0.5,
        ];
        (base, mid)
    }

    /// The leaf under a screen point (screen widths), if any is near.
    pub fn hit(&self, q: [f32; 2], aspect: f32) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, l) in self.leaves.iter().enumerate() {
            let (_, m) = self.blade(l, aspect);
            let d = ((q[0] - m[0]).powi(2) + (q[1] - m[1]).powi(2)).sqrt() / l.blade;
            if d < 1.6 && best.is_none_or(|b| d < b.1) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    pub fn draw(&self, p: &mut Painter, cam: &Cam, w: f32, aspect: f32, ppp: f32, fade: f32) {
        let px = |q: [f32; 2]| cam.pt([q[0] * w, q[1] * w]);
        let len = |r: f32| cam.len(r * w);
        // the trunk: chalk white with a greenish cast, shaded on its right
        // (one long stroke a layer, so no joins show)
        let (a, b) = (
            self.moved(self.trunk[0], aspect),
            self.moved(self.trunk[self.trunk.len() - 1], aspect),
        );
        let r = 0.07;
        for (off, wid, col) in [
            (0.0, 1.0, [0.80, 0.82, 0.74]),
            (0.45, 0.5, [0.68, 0.71, 0.63]),
            (0.62, 0.3, [0.58, 0.61, 0.54]),
            (-0.42, 0.35, [0.90, 0.91, 0.84]),
        ] {
            p.line(
                px([a[0] + r * off, a[1]]),
                px([b[0] + r * off, b[1]]),
                len(r * wid),
                len(if off == 0.0 { 0.003 } else { 0.02 }),
                with(col, fade),
            );
        }
        // the black eyes left where lower branches were shed
        for (c, s) in &self.scars {
            let c = self.moved(*c, aspect);
            // a dark eye: a lens across the bark, thickest in its middle, its ends drooping a little
            let mut last = None;
            for i in 0..=8 {
                let u = -1.0 + i as f32 / 4.0;
                let q = [c[0] + u * s * 0.85, c[1] + s * 0.12 * u.abs()];
                if let Some(l) = last {
                    let th = s * 0.15 * (1.0 - u * u).max(0.0) + 0.002;
                    p.line(
                        px(l),
                        px(q),
                        len(th),
                        len(0.002),
                        [0.10, 0.09, 0.08, 0.92 * fade],
                    );
                }
                last = Some(q);
            }
        }
        for pts in &self.branches {
            for j in 0..pts.len() - 1 {
                let r = 0.016 * (1.0 - j as f32 / pts.len() as f32) + 0.003;
                p.line(
                    px(self.moved(pts[j], aspect)),
                    px(self.moved(pts[j + 1], aspect)),
                    len(r),
                    len(0.002),
                    [0.42, 0.38, 0.33, fade],
                );
            }
        }
        for (a, b) in &self.twigs {
            p.line(
                px(self.moved(*a, aspect)),
                px(self.moved(*b, aspect)),
                len(0.0035),
                len(0.0015),
                [0.46, 0.41, 0.34, fade],
            );
        }
        for l in &self.leaves {
            let a = self.moved(l.anchor, aspect);
            let (base, mid) = self.blade(l, aspect);
            p.line(
                px(a),
                px(base),
                len(0.002),
                len(0.001).max(0.5 * ppp),
                [0.60, 0.48, 0.24, fade],
            );
            draw_blade(p, l, &|q| px(q), &|r| len(r), base, mid, fade);
        }
    }
}

/// A blade seen with its twist: a round leaf foreshortened across its stalk's line, gold face or pale
/// back, its brightness by how squarely it faces the low sun.
fn draw_blade(
    p: &mut Painter,
    l: &Leaf,
    px: &dyn Fn([f32; 2]) -> [f32; 2],
    len: &dyn Fn(f32) -> f32,
    base: [f32; 2],
    mid: [f32; 2],
    fade: f32,
) {
    let c = l.twist.cos();
    let face = c.abs().max(0.08);
    let half = l.blade * 0.5;
    let r = half * face;
    let d = [(mid[0] - base[0]) / half, (mid[1] - base[1]) / half];
    let reach = (half - r).max(0.0) * 0.92;
    let a = [mid[0] - d[0] * reach, mid[1] - d[1] * reach];
    let b = [mid[0] + d[0] * reach, mid[1] + d[1] * reach];
    let col = if c >= 0.0 {
        let g = tone(l.seed);
        let lit = 0.55 + 0.5 * face;
        [g[0] * lit, g[1] * lit, g[2] * lit]
    } else {
        let lit = 0.75 + 0.25 * face;
        [PALE[0] * lit, PALE[1] * lit, PALE[2] * lit]
    };
    p.line(px(a), px(b), len(r), len(0.002), with(col, fade));
    // its midrib, seen while it faces us
    let rib = [
        col[0] * 0.78,
        col[1] * 0.74,
        col[2] * 0.7,
        0.7 * face * fade,
    ];
    let end = [mid[0] + d[0] * half * 0.9, mid[1] + d[1] * half * 0.9];
    p.line(px(base), px(end), len(0.0012), len(0.001), rib);
}
