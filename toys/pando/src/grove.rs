//! The grove: Pando's stems on the slope below, seen from a little above the crowns, late in
//! September when the whole clone has turned gold at once. Each stem sways on its own period; each
//! crown's leaves tremble, and where a gust is passing more of them turn their pale undersides up, so
//! a gust shows as a pale wave running over the gold.

use crate::pen::Pen;
use crate::wind::{hash, Wind};
use crate::Cam;
use toy_kit::paint::Painter;

/// The camera's height above the stems' feet (m: standing on a rise at the grove's edge, looking in
/// under the crowns), and how far the nearest and farthest stems are.
pub const EYE: f32 = 27.0;
const NEAR: f32 = 10.0;
const FAR: f32 = 190.0;
/// Pando's stems stand about three metres apart (some 47,000 of them on 43 hectares).
const SPACING: f32 = 3.0;

pub struct Tree {
    pub x: f32,
    pub z: f32,
    /// Height (m) and the crown's half-width (m).
    pub h: f32,
    pub r: f32,
    /// How far it leans from upright (radians, across the view).
    pub lean: f32,
    /// Sway: displacement at the top (m, along the wind) and its rate; its own frequency (Hz).
    pub d: f32,
    pub v: f32,
    pub freq: f32,
    pub seed: u32,
}

pub struct Grove {
    pub trees: Vec<Tree>,
    /// the stalks are flattened (`RULES`' first): off, a leaf hardly turns, and no gust pales the crowns
    pub flat: bool,
}

/// The view of the grove: focal length (px), the middle, and the horizon's height (px).
pub struct View {
    pub f: f32,
    pub cx: f32,
    pub horizon: f32,
}

impl View {
    pub fn of(w: f32, h: f32) -> View {
        View {
            f: 0.95 * w.max(0.42 * h),
            cx: 0.5 * w,
            horizon: 0.2 * h,
        }
    }
    pub fn project(&self, p: [f32; 3]) -> Option<[f32; 2]> {
        if p[2] < 0.5 {
            return None;
        }
        Some([
            self.cx + self.f * p[0] / p[2],
            self.horizon - self.f * (p[1] - EYE) / p[2],
        ])
    }
    /// Where on the ground (y = 0) a screen point (px) looks.
    pub fn ground(&self, q: [f32; 2]) -> Option<[f32; 2]> {
        let dy = q[1] - self.horizon;
        if dy <= 1.0 {
            return None;
        }
        let z = self.f * EYE / dy;
        Some([(q[0] - self.cx) * z / self.f, z])
    }
}

/// How far into the haze a thing at `z` is.
pub fn fog(z: f32) -> f32 {
    1.0 - (-z / 150.0).exp()
}
pub const HAZE: [f32; 3] = [0.86, 0.70, 0.52];

/// The gold of a leaf's top in the low sun, and its pale underside.
pub const GOLD: [f32; 3] = [0.95, 0.64, 0.12];
pub const PALE: [f32; 3] = [0.93, 0.90, 0.70];

/// How many of a crown's leaves show their undersides in a wind of `w` m/s.
pub fn turned(w: f32) -> f32 {
    (0.02 + (w - 2.0).max(0.0) * 0.2).min(0.85)
}

impl Grove {
    pub fn new(seed: u32) -> Grove {
        let mut trees = Vec::new();
        let mut k = seed;
        let mut rnd = || {
            k = k.wrapping_add(0x9e37_79b9);
            hash(k)
        };
        // a wild stand, not rows: stems scattered at random, about one to every nine square metres,
        // of mixed heights, leaning a little each their own way
        let mut z = NEAR;
        while z < FAR {
            let dz = 2.0;
            let half = 0.62 * z + 12.0;
            // farther off, only some show between the crowns: thin them as they go
            let n = (2.0 * half * dz / (SPACING * SPACING) / (1.0 + z / 70.0)).round() as usize;
            for _ in 0..n {
                let young = rnd() < 0.12;
                let h = if young {
                    6.0 + 5.0 * rnd()
                } else {
                    13.0 + 8.0 * rnd()
                };
                trees.push(Tree {
                    x: (rnd() * 2.0 - 1.0) * half,
                    z: z + rnd() * dz,
                    h,
                    r: h * (0.07 + 0.05 * rnd()),
                    lean: (rnd() - 0.5) * 0.12,
                    d: 0.0,
                    v: 0.0,
                    // a stem this size sways about once every two to four seconds
                    freq: 0.25 + 0.2 * rnd(),
                    seed: (rnd() * 1.0e9) as u32,
                });
            }
            z += dz;
        }
        // far to near, so nearer crowns cover farther ones
        trees.sort_by(|a, b| b.z.total_cmp(&a.z));
        Grove { trees, flat: true }
    }

    pub fn step(&mut self, wind: &Wind, dt: f32) {
        for t in &mut self.trees {
            let w = wind.at([t.x, t.z]);
            let om = std::f32::consts::TAU * t.freq;
            // drag goes as the wind's square; at a strong gust the crown leans a metre or so
            let push = 0.035 * w * w * om * om / 4.0;
            let acc = push - om * om * t.d - 2.0 * 0.12 * om * t.v;
            t.v += acc * dt;
            t.d += t.v * dt;
        }
    }

    /// The tree a screen point (px) is on, nearest first.
    pub fn hit(&self, v: &View, q: [f32; 2]) -> Option<usize> {
        let mut best: Option<(usize, f32)> = None;
        for (i, t) in self.trees.iter().enumerate().rev() {
            let Some(c) = v.project([t.x + t.lean * t.h * 0.74, t.h * 0.74, t.z]) else {
                continue;
            };
            let rx = v.f * t.r * 1.2 / t.z;
            let ry = v.f * t.h * 0.3 / t.z;
            let (dx, dy) = ((q[0] - c[0]) / rx, (q[1] - c[1]) / ry);
            let d = dx * dx + dy * dy;
            if d < 1.0 {
                return Some(i);
            }
            if d < 3.0 && best.is_none_or(|b| d < b.1) {
                best = Some((i, d));
            }
        }
        best.map(|b| b.0)
    }

    /// The grove drawn with the plotter's pen (`pen.rs`), far to near: each stem two lines and its
    /// bark's eyes, each crown a ragged outline laid over paper (so it hides what is behind it), and in
    /// it a few of its leaves, a short stroke each, or a small ring for one turned pale side up.
    pub fn draw(&self, p: &mut Painter, v: &View, cam: &Cam, wind: &Wind, ppp: f32, fade: f32) {
        let t = wind.t;
        let pen = Pen::new(ppp, fade);
        for tr in &self.trees {
            // far off, only some of the crowns are drawn, so the far band stays open
            if hash(tr.seed ^ 0xFA) > 1.25 - tr.z / FAR {
                continue;
            }
            let foot = v.project([tr.x, 0.0, tr.z]);
            let top = v.project([tr.x + tr.d + tr.lean * tr.h, tr.h, tr.z]);
            let (Some(foot), Some(top)) = (foot, top) else {
                continue;
            };
            let s = v.f / tr.z;
            let c = v
                .project([tr.x + tr.lean * tr.h * 0.8, tr.h * 0.8, tr.z])
                .unwrap_or(top);
            stem(p, &pen, cam, tr, foot, c, s, ppp);
            self.crown(p, &pen, cam, tr, c, s, wind.at([tr.x, tr.z]), t, ppp);
        }
    }

    /// A crown: narrow and tall, its outline scalloped where clusters of leaves stand out, the top swaying
    /// farther than the bottom.
    #[allow(clippy::too_many_arguments)] // the tree, where it is, and how it is drawn
    fn crown(
        &self,
        p: &mut Painter,
        pen: &Pen,
        cam: &Cam,
        tr: &Tree,
        c: [f32; 2],
        s: f32,
        w: f32,
        t: f32,
        ppp: f32,
    ) {
        let hw = s * tr.r * 0.62;
        let hh = s * tr.h * 0.17;
        let sway = |up: f32| s * tr.d * (0.4 + 0.6 * up);
        // the paper first, in rows, a little inside the outline
        let rows = 10;
        for i in 0..rows {
            let y = -1.0 + (i as f32 + 0.5) * 2.0 / rows as f32;
            let half = hw * 0.92 * (1.0 - y * y).max(0.0).sqrt();
            let r = hh / rows as f32 * 1.4;
            let q = [c[0] + sway(0.5 - 0.5 * y), c[1] + y * hh];
            let dx = (half - r).max(0.0);
            pen.paper(
                p,
                cam.pt([q[0] - dx, q[1]]),
                cam.pt([q[0] + dx, q[1]]),
                cam.len(r.min(half)),
            );
        }
        // the outline: round lobes where clusters of leaves stand out, cusps between them
        let lobes = ((hw + hh) / (5.0 * ppp)).clamp(7.0, 16.0).round();
        let n = (lobes * 6.0) as u32;
        let ph = hash(tr.seed ^ 0x10B) * std::f32::consts::PI;
        let pts: Vec<[f32; 2]> = (0..n)
            .map(|i| {
                let th = i as f32 / n as f32 * std::f32::consts::TAU;
                let m = 0.9 + 0.12 * (0.5 * lobes * th + ph).sin().abs();
                let (ox, oy) = (th.cos(), th.sin());
                cam.pt([
                    c[0] + ox * hw * m + sway(0.5 - 0.5 * oy),
                    c[1] + oy * hh * m,
                ])
            })
            .collect();
        pen.path(p, &pts, true);
        // the leaves inside: one mark to about every six points square of crown
        let area = cam.len(hw) * cam.len(hh) * std::f32::consts::PI;
        let marks = (area / (6.0 * ppp).powi(2)).min(160.0) as u32;
        if cam.len(hw) < 3.0 * ppp {
            return;
        }
        let share = if self.flat { turned(w) } else { 0.02 };
        let len = 1.6 * ppp;
        for j in 0..marks {
            let hs = tr.seed.wrapping_add(j.wrapping_mul(7919));
            let ang = hash(hs) * std::f32::consts::TAU;
            let rr = hash(hs ^ 0x5555).sqrt() * 0.8;
            let (ox, oy) = (ang.cos() * rr, ang.sin() * rr);
            let q = cam.pt([c[0] + ox * hw + sway(0.5 - 0.5 * oy), c[1] + oy * hh]);
            let freq = 3.0 + 2.0 * hash(hs ^ 0x77);
            let ph = hash(hs ^ 0x99) * std::f32::consts::TAU;
            let flick = (ph + std::f32::consts::TAU * freq * t).sin();
            if flick > 1.0 - 2.0 * share {
                // an underside turned up: a little ring
                pen.circle(p, q, 0.9 * ppp);
            } else {
                // a leaf seen edge-on or flat: a short stroke, turning as it trembles
                let a = ph + 0.5 * flick;
                let d = [a.cos() * len * 0.5, a.sin() * len * 0.5];
                pen.line(p, [q[0] - d[0], q[1] - d[1]], [q[0] + d[0], q[1] + d[1]]);
            }
        }
    }
}

/// A stem: its two edges up into the crown (the crown's paper hides their tops), or one line when it
/// is thin, and the bark's dark eyes, chevrons where branches fell.
#[allow(clippy::too_many_arguments)] // the tree, where it is, and how it is drawn
fn stem(
    p: &mut Painter,
    pen: &Pen,
    cam: &Cam,
    tr: &Tree,
    foot: [f32; 2],
    c: [f32; 2],
    s: f32,
    ppp: f32,
) {
    let a = cam.pt(foot);
    let b = cam.pt([c[0] + s * tr.d * 0.3, c[1]]);
    let half = cam.len(s * 0.07).max(0.0);
    let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2))
        .sqrt()
        .max(1e-3);
    let d = [(b[0] - a[0]) / len, (b[1] - a[1]) / len];
    let n = [-d[1], d[0]];
    if half < 1.5 * pen.w {
        pen.line(p, a, b);
        return;
    }
    pen.paper(p, a, b, half);
    for side in [-1.0f32, 1.0] {
        let o = [n[0] * half * side, n[1] * half * side];
        pen.line(p, [a[0] + o[0], a[1] + o[1]], [b[0] + o[0], b[1] + o[1]]);
    }
    if half < 2.5 * ppp {
        return;
    }
    let count = (len / (half * 9.0)).min(7.0) as u32;
    for i in 0..count {
        let f = (i as f32 + 0.3 + 0.4 * hash(tr.seed.wrapping_add(i * 13))) / count.max(1) as f32;
        let q = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
        let w = half * (0.5 + 0.4 * hash(tr.seed ^ (i * 977)));
        let apex = [q[0] + d[0] * w * 0.45, q[1] + d[1] * w * 0.45];
        for side in [-1.0f32, 1.0] {
            pen.line(p, apex, [q[0] + n[0] * w * side, q[1] + n[1] * w * side]);
        }
    }
}

/// A cluster's gold, varying a little (and now and then still a little green).
pub fn tone(seed: u32) -> [f32; 3] {
    let u = hash(seed ^ 0x1234);
    if u < 0.06 {
        [0.70, 0.66, 0.20]
    } else if u < 0.22 {
        [0.88, 0.48, 0.10]
    } else if u < 0.45 {
        [1.0, 0.76, 0.22]
    } else {
        GOLD
    }
}

pub fn with(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}
