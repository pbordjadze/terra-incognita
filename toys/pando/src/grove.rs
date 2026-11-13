//! The grove: Pando's stems on the slope below, seen from a little above the crowns, late in
//! September when the whole clone has turned gold at once. Each stem sways on its own period; each
//! crown's leaves tremble, and where a gust is passing more of them turn their pale undersides up, so
//! a gust shows as a pale wave running over the gold.

use crate::wind::{hash, Wind};
use crate::Cam;
use toy_kit::paint::Painter;

/// The camera's height above the stems' feet (m), and how far the nearest and farthest stems are.
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
    /// Sway: displacement at the top (m, along the wind) and its rate; its own frequency (Hz).
    pub d: f32,
    pub v: f32,
    pub freq: f32,
    pub seed: u32,
}

pub struct Grove {
    pub trees: Vec<Tree>,
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
    1.0 - (-z / 260.0).exp()
}
pub const HAZE: [f32; 3] = [0.86, 0.70, 0.52];

fn mix3(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * k,
        a[1] + (b[1] - a[1]) * k,
        a[2] + (b[2] - a[2]) * k,
    ]
}

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
        let mut z = NEAR;
        while z < FAR {
            let half = 0.62 * z + 12.0;
            let mut x = -half;
            while x < half {
                let h = 13.0 + 7.0 * rnd();
                trees.push(Tree {
                    x: x + (rnd() - 0.5) * SPACING * 0.8,
                    z: z + (rnd() - 0.5) * SPACING * 0.8,
                    h,
                    r: h * (0.075 + 0.03 * rnd()),
                    d: 0.0,
                    v: 0.0,
                    // a stem this size sways about once every two to four seconds
                    freq: 0.25 + 0.2 * rnd(),
                    seed: (rnd() * 1.0e9) as u32,
                });
                x += SPACING;
            }
            // farther off, only every few rows show between the crowns: thin them as they go
            z += SPACING * (1.0 + z / 70.0);
        }
        // far to near, so nearer crowns cover farther ones
        trees.sort_by(|a, b| b.z.total_cmp(&a.z));
        Grove { trees }
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
            let Some(c) = v.project([t.x, t.h * 0.72, t.z]) else {
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

    pub fn draw(&self, p: &mut Painter, v: &View, cam: &Cam, wind: &Wind, ppp: f32, fade: f32) {
        let t = wind.t;
        for tr in &self.trees {
            let k = fog(tr.z);
            let foot = v.project([tr.x, 0.0, tr.z]);
            let top = v.project([tr.x + tr.d, tr.h, tr.z]);
            let (Some(foot), Some(top)) = (foot, top) else {
                continue;
            };
            let s = v.f / tr.z;
            // the stem: chalk white, greener in the shade
            let bark = mix3([0.86, 0.87, 0.80], HAZE, k);
            let wid = (s * 0.14).max(0.5 * ppp);
            let a = cam.pt(foot);
            let b = cam.pt([
                foot[0] + (top[0] - foot[0]) * 0.6,
                foot[1] + (top[1] - foot[1]) * 0.6,
            ]);
            p.line(a, b, cam.len(wid), cam.len(0.6 * ppp), with(bark, fade));
            // its crown: a cloud of small leaf clusters, swaying with the stem
            let rx = s * tr.r;
            let ry = s * tr.h * 0.27;
            let c = v.project([tr.x, tr.h * 0.70, tr.z]).unwrap_or(top);
            let blob = (rx * 0.24).max(1.1 * ppp);
            let n = ((rx * ry) / (blob * blob) * 1.5).clamp(5.0, 240.0) as u32;
            let w = wind.at([tr.x, tr.z]);
            let share = turned(w);
            for j in 0..n {
                let hseed = tr.seed.wrapping_add(j.wrapping_mul(7919));
                let ang = hash(hseed) * std::f32::consts::TAU;
                let rr = hash(hseed ^ 0x5555).sqrt();
                let (ox, oy) = (ang.cos() * rr, ang.sin() * rr);
                // the higher a cluster, the farther the sway carries it
                let up = 0.5 - 0.5 * oy;
                let sway = s * tr.d * (0.4 + 0.6 * up);
                let freq = 2.0 + 4.0 * hash(hseed ^ 0x77);
                let ph = hash(hseed ^ 0x99) * std::f32::consts::TAU;
                let flick = (ph + std::f32::consts::TAU * freq * t).sin();
                let pale = flick > 1.0 - 2.0 * share;
                // lit from the low sun on the left and from above
                let lit = 0.42 + 0.48 * up + 0.16 * (-ox);
                let base = tone(hseed);
                let col = if pale {
                    let l = 0.7 + 0.35 * up;
                    mix3(
                        [PALE[0] * l, PALE[1] * l, PALE[2] * l],
                        [1.0, 0.97, 0.85],
                        (w / 10.0).min(0.3),
                    )
                } else {
                    [base[0] * lit, base[1] * lit, base[2] * lit]
                };
                let col = mix3(col, HAZE, k);
                let q = [c[0] + ox * rx + sway, c[1] + oy * ry];
                let br = blob * if pale { 0.8 } else { 1.0 };
                if pale {
                    // an underside is a flat leaf turned edge-on and over: a short pale streak
                    p.line(
                        cam.pt([q[0] - br * 0.6, q[1]]),
                        cam.pt([q[0] + br * 0.6, q[1]]),
                        cam.len(br * 0.55),
                        cam.len(0.5 * ppp),
                        with(col, fade),
                    );
                } else {
                    p.disc(cam.pt(q), cam.len(br), cam.len(0.6 * ppp), with(col, fade));
                }
            }
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
