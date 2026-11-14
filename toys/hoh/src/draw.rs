//! The stand as the eye sees it, standing on the valley floor: the trunks rising out of the top of
//! the picture into the canopy, the young trees' cones, the logs with their moss and root wads, the
//! stilts, sword ferns, the light let down through a gap; far off, all of it goes into the mist.
//! Painted far to near (toy-kit paint.rs); the floor and the mist are the field's (scene.wgsl).

use crate::forest::{Forest, Log, Tree, FALL_S, Z0};
use toy_kit::paint::Painter;

/// The eye's height above the floor (m): standing on a low log.
pub const EYE: f32 = 6.5;

pub struct View {
    pub f: f32,
    pub cx: f32,
    pub horizon: f32,
}

impl View {
    pub fn of(w: f32, h: f32) -> View {
        View {
            f: 0.75 * w.max(0.42 * h),
            cx: 0.5 * w,
            horizon: 0.57 * h,
        }
    }
    pub fn project(&self, p: [f32; 3]) -> Option<[f32; 2]> {
        if p[2] < 0.3 {
            return None;
        }
        Some([
            self.cx + self.f * p[0] / p[2],
            self.horizon - self.f * (p[1] - EYE) / p[2],
        ])
    }
    /// Where on the floor a screen point (px) looks.
    pub fn ground(&self, q: [f32; 2]) -> Option<[f32; 2]> {
        let dy = q[1] - self.horizon;
        if dy <= 1.0 {
            return None;
        }
        let z = self.f * EYE / dy;
        Some([(q[0] - self.cx) * z / self.f, z])
    }
}

/// How far into the mist a thing at `z` is.
pub fn fog(z: f32) -> f32 {
    1.0 - (-z / 45.0).exp()
}
pub const MIST: [f32; 3] = [0.56, 0.64, 0.55];

fn mix3(a: [f32; 3], b: [f32; 3], k: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * k,
        a[1] + (b[1] - a[1]) * k,
        a[2] + (b[2] - a[2]) * k,
    ]
}
fn rgba(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0], c[1], c[2], a]
}
fn scale3(c: [f32; 3], k: f32) -> [f32; 3] {
    [c[0] * k, c[1] * k, c[2] * k]
}

pub fn hash(seed: u32, i: u32) -> f32 {
    let mut x = seed ^ i.wrapping_mul(0x9e37_79b9);
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// A sword fern on the floor (m), its size and turn.
pub struct Fern {
    pub x: f32,
    pub z: f32,
    pub s: f32,
    pub seed: u32,
}

pub fn ferns(seed: u32) -> Vec<Fern> {
    // in clumps, as sword ferns grow
    let mut out = Vec::new();
    for c in 0..40u32 {
        let cz = 3.0 + 45.0 * hash(seed, c * 7).powf(1.4);
        let cx = (hash(seed, c * 7 + 1) * 2.0 - 1.0) * (cz * 0.7 + 1.0);
        let n = 1 + (hash(seed, c * 7 + 2) * 5.0) as u32;
        for i in 0..n {
            let j = c * 16 + i;
            let z = cz + (hash(seed, j * 5 + 3) - 0.5) * 3.0;
            let x = cx + (hash(seed, j * 5 + 4) - 0.5) * 3.5;
            if z < 2.0 {
                continue;
            }
            out.push(Fern {
                x,
                z,
                s: 0.4 + 0.8 * hash(seed, j * 5 + 5).powf(1.5),
                seed: seed ^ (j * 977),
            });
        }
    }
    out
}

enum Item {
    Tree(usize),
    Fall(usize),
    Log(usize, f32, f32, bool),
    Fern(usize),
}

pub struct Scene<'a> {
    pub view: &'a View,
    pub forest: &'a Forest,
    pub ferns: &'a [Fern],
    pub ppp: f32,
    pub t: f32,
}

impl Scene<'_> {
    pub fn paint(&self, p: &mut Painter) {
        let f = self.forest;
        let mut items: Vec<(f32, Item)> = Vec::with_capacity(f.trees.len() + 400);
        for (i, t) in f.trees.iter().enumerate() {
            // a tree stands over the log it rooted on: its roots go over the log
            items.push((t.z - 1.5, Item::Tree(i)));
        }
        for (i, fl) in f.falls.iter().enumerate() {
            // going over toward the eye it comes in front of what stands nearer than its foot
            let mid = fl.tree.z + fl.dir[1] * fl.angle().sin() * fl.tree.h * 0.5;
            items.push((mid.min(fl.tree.z) - 1.5, Item::Fall(i)));
        }
        for (i, l) in f.logs.iter().enumerate() {
            let d = [l.b[0] - l.a[0], l.b[1] - l.a[1]];
            let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
            // finer pieces where it comes near, so each piece's size steps only a little from the next
            let near = l.a[1].min(l.b[1]).max(Z0);
            let n = (len / (0.25 + 0.05 * near).min(1.2))
                .ceil()
                .clamp(1.0, 160.0) as usize;
            for k in 0..n {
                let (u0, u1) = (k as f32 / n as f32, (k + 1) as f32 / n as f32);
                let zm = l.a[1] + d[1] * (u0 + u1) * 0.5;
                // its body, then (after any nearer piece's body) its moss and underside, so the
                // pieces' round ends never show
                items.push((zm, Item::Log(i, u0, u1, false)));
                items.push((zm - 1.3, Item::Log(i, u0, u1, true)));
            }
        }
        for (i, fe) in self.ferns.iter().enumerate() {
            items.push((fe.z, Item::Fern(i)));
        }
        items.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, it) in &items {
            match *it {
                Item::Tree(i) => self.tree(p, &f.trees[i], None),
                Item::Fall(i) => {
                    let fl = &f.falls[i];
                    self.tree(p, &fl.tree, Some((fl.dir, fl.angle())));
                }
                Item::Log(i, u0, u1, top) => self.log(p, &f.logs[i], u0, u1, top),
                Item::Fern(i) => self.fern(p, &self.ferns[i]),
            }
        }
        self.shafts(p);
    }

    fn k(&self, z: f32) -> f32 {
        self.view.f / z.max(0.3)
    }

    /// A point of a standing (or falling) tree: `up` metres up its axis from its foot.
    fn axis(&self, t: &Tree, lean: Option<([f32; 2], f32)>, up: f32, out: f32) -> Option<[f32; 2]> {
        let (dir, a) = lean.unwrap_or(([1.0, 0.0], 0.0));
        let (s, c) = a.sin_cos();
        // across the axis (for a crown's width): horizontal, square to the fall
        let side = [-dir[1], dir[0]];
        let side = if lean.is_some() { side } else { [1.0, 0.0] };
        self.view.project([
            t.x + dir[0] * s * up + side[0] * out,
            t.base + c * up,
            t.z + dir[1] * s * up + side[1] * out,
        ])
    }

    fn tree(&self, p: &mut Painter, t: &Tree, lean: Option<([f32; 2], f32)>) {
        let k = self.k(t.z);
        // going over toward the eye it comes out of the mist as it comes nearer
        let zf = lean.map_or(t.z, |(dir, a)| {
            (t.z + dir[1] * a.sin() * t.h * 0.5).clamp(Z0, t.z)
        });
        let fz = fog(zf);
        let light = self.forest.light_at([t.x, t.z]);
        let alpha = t.fade.clamp(0.0, 1.0);
        let brown = 1.0 - alpha;
        let soft = self.ppp;
        let d = t.d();
        let hw = (0.5 * d * k).max(0.35 * self.ppp);
        let var = hash(t.seed, 1);
        let bark = mix3([0.25, 0.22, 0.20], [0.30, 0.21, 0.17], var);
        let bark_f = mix3(bark, MIST, fz);
        // the feet: stilts where it rooted on a log, a flare of roots where it rooted on the floor
        if t.base > 0.05 && lean.is_none() && t.h > 1.5 {
            let legs = mix3(scale3(bark, 0.8), MIST, fz);
            self.stilts(p, t, k, legs, alpha);
        } else if lean.is_none() && t.h > 1.5 {
            // rooted on the floor: a flare of roots into the moss
            for j in 0..5 {
                let s = j as f32 / 2.0 - 1.0;
                let (Some(top), Some(foot)) = (
                    self.axis(t, None, d * 0.9, 0.0),
                    self.view
                        .project([t.x + s * d * 0.95, 0.0, t.z - 0.2 * (1.0 - s.abs())]),
                ) else {
                    continue;
                };
                p.line(
                    top,
                    foot,
                    hw * (0.5 - 0.2 * s.abs()),
                    soft,
                    rgba(bark_f, alpha),
                );
            }
        }
        // the trunk, tapering: short pieces, each a little narrower, so no step shows where they meet
        let n = if hw > 3.0 * self.ppp { 12 } else { 4 };
        for i in 0..n {
            let (a, b) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
            // the first piece starts its own half-width up, so its round end sits on the foot, not below it
            let a = if i == 0 {
                (hw / k).min(0.5 * b * t.h) / t.h
            } else {
                a
            };
            let w = 1.0 - 0.88 * b.powf(1.6);
            let (Some(pa), Some(pb)) = (
                self.axis(t, lean, a * t.h, 0.0),
                self.axis(t, lean, b * t.h, 0.0),
            ) else {
                continue;
            };
            if pa[1] < -50.0 && pb[1] < -50.0 {
                continue;
            }
            p.line(pa, pb, hw * w, soft, rgba(bark_f, alpha));
        }
        // the lit side, one stroke, so its translucent ends don't stack where pieces meet
        if hw > 1.5 * self.ppp {
            if let (Some(pa), Some(pb)) = (
                self.axis(t, lean, 0.0, 0.0),
                self.axis(t, lean, 0.7 * t.h, 0.0),
            ) {
                let lit = mix3(scale3(bark, 1.55), MIST, fz);
                let o = -hw * 0.3;
                p.line(
                    [pa[0] + o, pa[1]],
                    [pb[0] + o * 0.5, pb[1]],
                    hw * 0.26,
                    hw * 0.5,
                    rgba(lit, alpha * 0.55),
                );
            }
        }
        if t.h > 20.0 && lean.is_none() {
            self.beards(p, t, k, fz, alpha);
        }
        self.crown(p, t, lean, k, fz, light, alpha, brown);
    }

    /// Roots that grew down round a log to the floor: legs, splayed, as tall as the log was.
    fn stilts(&self, p: &mut Painter, t: &Tree, k: f32, bark: [f32; 3], alpha: f32) {
        let d = t.d();
        // a young tree's roots are few and hug its log; a grown one's stand as legs
        let grown = ((t.h - 2.0) / 25.0).clamp(0.0, 1.0);
        let legs = 3 + ((hash(t.seed, 3) + 4.0 * grown) as u32).min(4);
        let spread = d * 0.4 + t.base * (0.5 + 0.2 * grown);
        let r = ((d * 0.12).max(0.06 * t.base) * k).max(0.4 * self.ppp);
        // the legs straddle the line the log lay along, so where it was is a tunnel under them
        let across = [-t.lay[1], t.lay[0]];
        let at = |side: f32, out: f32, along: f32, y: f32| {
            [
                t.x + across[0] * side * out + t.lay[0] * along,
                y,
                t.z + across[1] * side * out + t.lay[1] * along,
            ]
        };
        for i in 0..legs {
            let side = if i % 2 == 0 { 1.0 } else { -1.0 };
            let along =
                (i as f32 / (legs - 1) as f32 - 0.5) * d * 1.4 + (hash(t.seed, 20 + i) - 0.5) * 0.3;
            let out = spread * (0.85 + 0.3 * hash(t.seed, 40 + i));
            let top = at(side, d * 0.3, along * 0.3, t.base + d * 0.15);
            let knee = at(side, out * 0.8, along * 0.7, t.base * 0.55);
            let foot = at(side, out, along, 0.0);
            let pts = [top, knee, foot].map(|q| self.view.project(q));
            if let [Some(a), Some(b), Some(c)] = pts {
                p.line(a, b, r * 1.15, self.ppp, rgba(bark, alpha));
                p.line(b, c, r, self.ppp, rgba(bark, alpha));
            }
        }
    }

    /// Clubmoss hanging from the branch stubs of an old tree, in beards.
    fn beards(&self, p: &mut Painter, t: &Tree, k: f32, fz: f32, alpha: f32) {
        let moss = mix3([0.44, 0.52, 0.22], MIST, fz * 0.9);
        let stub = mix3([0.22, 0.19, 0.16], MIST, fz);
        let n = 6;
        for i in 0..n {
            let y = t.base + 4.0 + (t.h * 0.55) * (i as f32 + hash(t.seed, 60 + i)) / n as f32;
            let s = if hash(t.seed, 80 + i) < 0.5 {
                -1.0
            } else {
                1.0
            };
            let len = 1.0 + 3.0 * hash(t.seed, 100 + i);
            let x0 = t.x + s * t.d() * 0.45;
            let x1 = x0 + s * len;
            let (Some(a), Some(b)) = (
                self.view.project([x0, y, t.z]),
                self.view.project([x1, y + len * 0.12, t.z]),
            ) else {
                continue;
            };
            if b[1] < -80.0 || a[1] > self.view.horizon * 2.6 {
                continue;
            }
            p.line(
                a,
                b,
                (0.07 * k).max(0.3 * self.ppp),
                self.ppp,
                rgba(stub, alpha),
            );
            // a clump where it holds, and strands of uneven length hanging from it
            for j in 0..3u32 {
                let u = 0.3 + 0.6 * hash(t.seed, 900 + i * 4 + j);
                let q = [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
                p.disc(
                    q,
                    (0.22 * k).max(self.ppp),
                    (0.2 * k).max(self.ppp),
                    rgba(moss, alpha * 0.6),
                );
            }
            let strands = 7;
            for j in 0..strands {
                let h = hash(t.seed, 200 + i * 8 + j);
                let u = 0.15 + 0.85 * hash(t.seed, 600 + i * 8 + j);
                let q = [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
                let hang = (0.2 + 1.6 * h * h) * (1.0 - 0.3 * u) * k;
                let sway = (self.t * 0.6 + hash(t.seed, 300 + i) * 6.0).sin() * 0.04 * hang;
                p.line(
                    q,
                    [q[0] + sway, q[1] + hang],
                    (0.05 * k).max(0.4 * self.ppp),
                    (0.05 * k).max(self.ppp),
                    rgba(moss, alpha * 0.7),
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)] // the crown's look takes the tree's place, light and life
    fn crown(
        &self,
        p: &mut Painter,
        t: &Tree,
        lean: Option<([f32; 2], f32)>,
        k: f32,
        fz: f32,
        light: f32,
        alpha: f32,
        brown: f32,
    ) {
        // the crown: a cone of drooping tiers, down to near the foot on a young tree, the top half on
        // an old one
        let start = if t.h < 14.0 {
            0.08
        } else {
            0.08 + 0.4 * ((t.h - 14.0) / 30.0).min(1.0)
        };
        let len = t.h * (1.0 - start);
        let reach = (0.2 + 0.16 * t.h).min(6.0);
        let green = mix3([0.07, 0.17, 0.10], [0.16, 0.30, 0.13], light * 0.8);
        let green = mix3(green, [0.42, 0.28, 0.13], brown);
        let green = mix3(green, MIST, fz * 0.92);
        // tiers about 3.5 px apart, but on a near tree no branch thicker than a few px: more of them
        let cap = 3.5 * self.ppp;
        let tiers = ((len * k / (3.5 * self.ppp)).max(len * k * 0.45 / cap) as usize).clamp(2, 90);
        let thick = (len / tiers as f32 * k * 0.45).clamp(0.5 * self.ppp, cap);
        for i in 0..tiers {
            let u = i as f32 / tiers as f32;
            if u < 0.85 && hash(t.seed, 500 + i as u32) < 0.22 {
                continue;
            }
            let up = t.h * start + len * u;
            let Some(c) = self.axis(t, lean, up, 0.0) else {
                continue;
            };
            if c[1] < -60.0 {
                continue;
            }
            for s in [-1.0f32, 1.0] {
                let j = 400 + i as u32 * 2 + (s > 0.0) as u32;
                let w = reach * (1.0 - u).powf(0.85) * (0.45 + 0.65 * hash(t.seed, j));
                // a branch droops, and its foliage hangs heavier toward the tip
                let mut prev = c;
                for q in 1..=3 {
                    let v = q as f32 / 3.0;
                    let droop = w * (0.08 * v + 0.28 * v * v);
                    let Some(pt) = self.axis(t, lean, up - droop, s * w * v) else {
                        break;
                    };
                    let th = thick * (1.15 - 0.35 * v);
                    p.line(prev, pt, th, self.ppp, rgba(green, alpha));
                    prev = pt;
                }
            }
        }
        if let (Some(a), Some(b)) = (
            self.axis(t, lean, t.h * 0.97, 0.0),
            self.axis(t, lean, t.h * 1.03 + 0.1, 0.0),
        ) {
            p.line(a, b, thick * 0.6, self.ppp, rgba(green, alpha));
        }
    }

    fn log(&self, p: &mut Painter, l: &Log, u0: f32, u1: f32, top: bool) {
        let r = l.r();
        if r < 0.03 {
            return;
        }
        let d = [l.b[0] - l.a[0], l.b[1] - l.a[1]];
        let at = |u: f32, up: f32| {
            self.view
                .project([l.a[0] + d[0] * u, up, l.a[1] + d[1] * u])
        };
        let zm = l.a[1] + d[1] * (u0 + u1) * 0.5;
        let k = self.k(zm);
        let fz = fog(zm);
        let rot = l.rot();
        let moss = l.moss();
        let light = self.forest.light_at([l.a[0] + d[0] * u0, zm]);
        let bark = mix3([0.29, 0.21, 0.16], [0.30, 0.17, 0.11], rot);
        let green = mix3([0.26, 0.40, 0.12], [0.42, 0.56, 0.18], light);
        // the root wad at its foot, standing up, earth and roots, sinking as it rots
        if u0 == 0.0 && !top {
            // (its roots torn up with it: a plate of earth, roots sticking out of its rim)
            let wad = (l.r0 * 1.5 + 0.4) * (1.0 - (l.age / 160.0).min(1.0));
            let back = [-d[0] * 0.015, -d[1] * 0.015];
            if wad > 0.1 {
                if let Some(c) = self
                    .view
                    .project([l.a[0] + back[0], wad * 0.8, l.a[1] + back[1]])
                {
                    let earth = mix3([0.22, 0.17, 0.12], MIST, fz);
                    let root = mix3([0.30, 0.24, 0.19], MIST, fz);
                    let rk = wad * k;
                    for i in 0..9u32 {
                        let a = std::f32::consts::TAU * (i as f32 + hash(l.seed, 700 + i)) / 9.0;
                        let len = rk * (0.8 + 0.5 * hash(l.seed, 720 + i));
                        let tip = [c[0] + a.cos() * len, c[1] + a.sin() * len * 0.9];
                        p.line(
                            c,
                            tip,
                            (rk * 0.06).max(0.5 * self.ppp),
                            self.ppp,
                            rgba(root, 1.0),
                        );
                    }
                    for i in 0..5u32 {
                        let o = [
                            (hash(l.seed, 740 + i) - 0.5) * rk * 0.7,
                            (hash(l.seed, 760 + i) - 0.5) * rk * 0.6,
                        ];
                        let rr = rk * (0.45 + 0.3 * hash(l.seed, 780 + i));
                        p.disc(
                            [c[0] + o[0], c[1] + o[1]],
                            rr,
                            self.ppp * 1.5,
                            rgba(earth, 1.0),
                        );
                    }
                }
            }
        }
        let (Some(a), Some(b)) = (at(u0, r), at(u1, r)) else {
            return;
        };
        let rp = r * k;
        let body = mix3(bark, MIST, fz);
        if !top {
            p.line(a, b, rp, self.ppp, rgba(body, 1.0));
            return;
        }
        // moss over its back, thicker with the years, placed in the world rather than by the piece's
        // size, so where the log comes toward the eye the pieces still meet end to end
        // (opaque, its colour from the bark to the moss: overlapping pieces would show if translucent)
        let cap = mix3(mix3(bark, green, 0.25 + 0.75 * moss), MIST, fz);
        let lift = r * (1.5 - 0.3 * moss);
        let (Some(ca), Some(cb)) = (at(u0, lift), at(u1, lift)) else {
            return;
        };
        p.line(ca, cb, rp * (0.3 + 0.5 * moss), self.ppp, rgba(cap, 1.0));
    }

    fn fern(&self, p: &mut Painter, fe: &Fern) {
        let k = self.k(fe.z);
        let fz = fog(fe.z);
        let green = mix3([0.12, 0.30, 0.11], MIST, fz);
        let dark = mix3([0.06, 0.16, 0.07], MIST, fz);
        let Some(c) = self.view.project([fe.x, 0.0, fe.z]) else {
            return;
        };
        let fronds = 11;
        for i in 0..fronds {
            let a = std::f32::consts::PI
                * (0.08 + 0.84 * (i as f32 + hash(fe.seed, i)) / fronds as f32);
            let len = fe.s * (0.75 + 0.4 * hash(fe.seed, 30 + i));
            let sway = (self.t * 0.8 + hash(fe.seed, 60 + i) * 6.0).sin() * 0.03;
            let mut prev = c;
            for s in 1..=4 {
                let u = s as f32 / 4.0;
                // up and out, the tip arching over
                let out = len * u;
                let lift = len * (0.9 * u - 0.75 * u * u);
                let q = [
                    c[0] - (a + sway).cos() * out * k,
                    c[1] - lift * k - (a.sin() - 0.5) * out * k * 0.15,
                ];
                let w = (0.07 * len * (1.0 - u * 0.7) * k).max(0.35 * self.ppp);
                p.line(
                    prev,
                    q,
                    w,
                    self.ppp,
                    rgba(if s < 2 { dark } else { green }, 1.0),
                );
                prev = q;
            }
        }
    }

    /// The light let down through a gap: a pale shaft from the canopy to the floor, as strong as the
    /// gap is open.
    fn shafts(&self, p: &mut Painter) {
        let f = self.forest;
        for (c, light) in f.cells() {
            if light < 0.45
                || c[1] > 55.0
                || hash(((c[0] + 99.0) * 7.0) as u32, (c[1] * 13.0) as u32) > 0.18
            {
                continue;
            }
            let Some(g) = self.view.project([c[0], 0.0, c[1]]) else {
                continue;
            };
            let k = self.k(c[1]);
            let top = [g[0] - 0.35 * self.view.horizon, -0.1 * self.view.horizon];
            let a = (light - 0.45) * 0.07 * (1.0 - fog(c[1]) * 0.6);
            let w = Forest::CELL * 0.35 * k;
            p.glow(top, g, w, w * 2.0, [0.95, 0.97, 0.85, a]);
        }
        // a fall's dust and needles drifting down where it landed (the first seconds after)
        for fl in &f.falls {
            if fl.t > FALL_S * 0.9 {
                if let Some(g) = self.view.project([fl.tree.x, 1.0, fl.tree.z]) {
                    let k = self.k(fl.tree.z);
                    p.glow(g, g, 6.0 * k, 6.0 * k, [0.9, 0.85, 0.7, 0.05]);
                }
            }
        }
    }
}
