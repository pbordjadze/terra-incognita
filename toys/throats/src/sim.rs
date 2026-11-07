//! The hillside: a torus of rocks, each held by one lizard with an orange, blue or yellow throat, or
//! a boulder no lizard holds. The rule (a stochastic cyclic cellular automaton, Griffeath's kind):
//! every lizard keeps an eye on the rocks within `RANGE`; when at least `THRESHOLD` of them are held
//! by the colour that beats its own (orange beats blue, blue beats yellow, yellow beats orange), it
//! loses its rock to one of them with probability `TAKE` a step. That's all: the spirals come out of
//! it, their spacing set by the range and the threshold.
//!
//! The drawing's side: each flip of a step is shown at its own moment inside the step (`due`), the
//! winner running in from a neighbour that wears its colour (`from`), the loser running off.

pub const BOULDER: u8 = 0;
/// What a rock no lizard holds is (`Hill::ground`).
pub const OUTCROP: u8 = 1;
pub const SHRUB: u8 = 2;
/// open dry grass between the rocky patches: no rocks to hold
pub const GRASS: u8 = 3;
/// The share of the hill left to grass (where it is set by a smooth noise, thinner near outcrops).
pub const BARE: f32 = 0.24;
pub const ORANGE: u8 = 1;
pub const BLUE: u8 = 2;
pub const YELLOW: u8 = 3;

pub const RANGE: i32 = 2;
pub const THRESHOLD: u32 = 5;
pub const TAKE: f32 = 0.5;
/// Steps the hill has turned before it is first seen.
pub const WARM: usize = 220;

/// `a` beats `b`: orange (1) beats blue (2), blue beats yellow (3), yellow beats orange.
#[inline]
pub fn beats(a: u8, b: u8) -> bool {
    a != BOULDER && b != BOULDER && b == a % 3 + 1
}

/// The colour that beats `k`.
#[inline]
pub fn beater(k: u8) -> u8 {
    (k + 1) % 3 + 1
}

pub struct Rng(pub u64);
#[allow(clippy::should_implement_trait)] // next() is a number generator's step, not an Iterator's
impl Rng {
    #[inline]
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    #[inline]
    pub fn unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
    #[inline]
    pub fn below(&mut self, n: u32) -> u32 {
        (((self.next() >> 32) * n as u64) >> 32) as u32
    }
}

/// A flip waiting for its moment in the step.
#[derive(Clone, Copy)]
pub struct Flip {
    pub i: u32,
    pub to: u8,
    /// the neighbour the winner comes from, as a cell offset
    pub from: [i8; 2],
    /// its moment, as a fraction of the step
    pub at: f32,
}

pub struct Hill {
    pub w: usize,
    pub h: usize,
    /// the colours as the model has them (the state the next step reads)
    pub kind: Vec<u8>,
    /// the colours as they are shown (catching up with `kind` through the step)
    pub shown: Vec<u8>,
    /// the colour a rock's last lizard wore, while it runs off
    pub was: Vec<u8>,
    /// a lizard's own look, 0–63, new with each lizard
    pub look: Vec<u8>,
    /// facing, radians
    pub heading: Vec<f32>,
    /// where the lizard on the rock came from, cells (drawn sliding home)
    pub from: Vec<[f32; 2]>,
    /// seconds since the rock changed hands
    pub since: Vec<f32>,
    /// what the ground is where no lizard can be (OUTCROP, SHRUB; 0 where lizards live)
    pub ground: Vec<u8>,
    pending: Vec<Flip>,
    pub rng: Rng,
    counts: [Vec<u16>; 3],
}

impl Hill {
    pub fn new(w: usize, h: usize, seed: u64) -> Hill {
        let n = w * h;
        let mut hill = Hill {
            w,
            h,
            kind: vec![ORANGE; n],
            shown: vec![ORANGE; n],
            was: vec![ORANGE; n],
            look: vec![0; n],
            heading: vec![0.0; n],
            from: vec![[0.0; 2]; n],
            since: vec![9.0; n],
            ground: vec![0; n],
            pending: Vec::new(),
            rng: Rng(seed | 1),
            counts: [vec![0; n], vec![0; n], vec![0; n]],
        };
        hill.seed_pinwheels(seed);
        hill.warm(WARM);
        hill
    }

    /// Steps taken before the first frame, so the hill opens on spirals already turning.
    pub fn warm(&mut self, steps: usize) {
        for _ in 0..steps {
            self.step();
        }
        let rest: Vec<Flip> = std::mem::take(&mut self.pending);
        for f in rest {
            self.show(f);
        }
        for i in 0..self.w * self.h {
            self.since[i] = 9.0;
            self.was[i] = self.shown[i];
        }
    }

    /// The opening: a few pinwheels (three colours round a centre), each of which winds into a spiral
    /// at once, with a little noise; and the outcrops and shrubs no lizard holds.
    pub fn seed_pinwheels(&mut self, seed: u64) {
        let mut rng = Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1);
        let (w, h) = (self.w as f32, self.h as f32);
        let mut centres = Vec::new();
        // a jittered grid of centres, about 46 rocks apart
        let step = 46.0;
        let nx = (w / step).round().max(1.0) as usize;
        let ny = (h / step).round().max(1.0) as usize;
        for gy in 0..ny {
            for gx in 0..nx {
                let cx = (gx as f32 + 0.5 + (rng.unit() - 0.5) * 0.6) * w / nx as f32;
                let cy = (gy as f32 + 0.5 + (rng.unit() - 0.5) * 0.6) * h / ny as f32;
                let turn = rng.unit() * std::f32::consts::TAU;
                let hand = if rng.unit() < 0.5 { 1.0 } else { -1.0 };
                centres.push((cx, cy, turn, hand));
            }
        }
        for y in 0..self.h {
            for x in 0..self.w {
                let (mut best, mut bd) = (0, f32::MAX);
                for (c, &(cx, cy, _, _)) in centres.iter().enumerate() {
                    let dx = wrap(x as f32 - cx, w);
                    let dy = wrap(y as f32 - cy, h);
                    let d = dx * dx + dy * dy;
                    if d < bd {
                        bd = d;
                        best = c;
                    }
                }
                let (cx, cy, turn, hand) = centres[best];
                let a = hand * wrap(y as f32 - cy, h).atan2(wrap(x as f32 - cx, w)) + turn;
                let k = 1 + ((a / std::f32::consts::TAU).rem_euclid(1.0) * 3.0) as u8 % 3;
                let k = if rng.unit() < 0.06 {
                    1 + rng.below(3) as u8
                } else {
                    k
                };
                let i = y * self.w + x;
                self.kind[i] = k;
            }
        }
        // the ground no lizard holds: granite outcrops (big, few) and shrubs (small, more)
        self.ground = vec![0; self.w * self.h];
        let area = w * h;
        let blobs = [
            (OUTCROP, (area / 3800.0) as usize, 2.5, 8.0),
            (SHRUB, (area / 2500.0) as usize, 1.0, 2.4),
        ];
        for &(kind, count, r0, r1) in &blobs {
            for _ in 0..count {
                let cx = rng.unit() * w;
                let cy = rng.unit() * h;
                let r = r0 + rng.unit() * (r1 - r0);
                // a few lobes, so an outcrop is no circle
                let lobes: Vec<(f32, f32, f32)> = (0..3)
                    .map(|_| {
                        (
                            (rng.unit() - 0.5) * r,
                            (rng.unit() - 0.5) * r,
                            r * (0.5 + 0.5 * rng.unit()),
                        )
                    })
                    .collect();
                let ri = (r * 2.0).ceil() as i32 + 1;
                for dy in -ri..=ri {
                    for dx in -ri..=ri {
                        let fx = dx as f32 + 0.5 - cx.fract();
                        let fy = dy as f32 + 0.5 - cy.fract();
                        let inside = lobes
                            .iter()
                            .any(|&(lx, ly, lr)| (fx - lx).powi(2) + (fy - ly).powi(2) < lr * lr);
                        if inside {
                            let x = (cx as i32 + dx).rem_euclid(self.w as i32) as usize;
                            let y = (cy as i32 + dy).rem_euclid(self.h as i32) as usize;
                            let i = y * self.w + x;
                            self.kind[i] = BOULDER;
                            self.ground[i] = kind;
                        }
                    }
                }
            }
        }
        // the grass: where a smooth noise is low, less near the outcrops (lizards gather on rock)
        let near_rock = self.blur(
            &self
                .ground
                .iter()
                .map(|&g| (g == OUTCROP) as u8 as f32)
                .collect::<Vec<f32>>(),
            6,
        );
        let salt = rng.next();
        let mut v: Vec<(f32, usize)> = (0..self.w * self.h)
            .filter(|&i| self.ground[i] == 0)
            .map(|i| {
                let (x, y) = ((i % self.w) as f32, (i / self.w) as f32);
                let n =
                    0.8 * vnoise(
                        x / 24.0,
                        y / 24.0,
                        salt,
                        self.w as f32 / 24.0,
                        self.h as f32 / 24.0,
                    ) + 0.2
                        * vnoise(
                            x / 8.0,
                            y / 8.0,
                            salt ^ 0xABCD,
                            self.w as f32 / 8.0,
                            self.h as f32 / 8.0,
                        );
                (n + 2.5 * near_rock[i], i)
            })
            .collect();
        v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let bare = (BARE * (self.w * self.h) as f32) as usize;
        for &(_, i) in v.iter().take(bare) {
            self.ground[i] = GRASS;
            self.kind[i] = BOULDER;
        }
        // a patch of rocks too small for the game to go round in would hold one colour forever: grass
        let mut seen = vec![false; self.w * self.h];
        for start in 0..self.w * self.h {
            if seen[start] || self.kind[start] == BOULDER {
                continue;
            }
            let mut stack = vec![start];
            let mut patch = Vec::new();
            seen[start] = true;
            while let Some(i) = stack.pop() {
                patch.push(i);
                let (x, y) = ((i % self.w) as i32, (i / self.w) as i32);
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let j = ((y + dy).rem_euclid(self.h as i32) as usize) * self.w
                        + (x + dx).rem_euclid(self.w as i32) as usize;
                    if !seen[j] && self.kind[j] != BOULDER {
                        seen[j] = true;
                        stack.push(j);
                    }
                }
            }
            if patch.len() < 400 {
                for i in patch {
                    self.ground[i] = GRASS;
                    self.kind[i] = BOULDER;
                }
            }
        }
        for i in 0..self.w * self.h {
            self.shown[i] = self.kind[i];
            self.was[i] = self.kind[i];
            self.look[i] = rng.below(64) as u8;
            self.heading[i] = rng.unit() * std::f32::consts::TAU;
            self.since[i] = 9.0;
            self.from[i] = [0.0; 2];
        }
        self.pending.clear();
    }

    /// One step of the rule, read from `kind` as it is; the flips wait in `pending` to be shown.
    pub fn step(&mut self) {
        // apply what is still waiting from the last step at once
        let rest: Vec<Flip> = std::mem::take(&mut self.pending);
        for f in rest {
            self.show(f);
        }
        let (w, h) = (self.w, self.h);
        // box counts of each colour within RANGE (separable sums on the torus)
        let r = RANGE as usize;
        let mut row = vec![0u16; w * h];
        for c in 0..3u8 {
            let col = c + 1;
            for y in 0..h {
                let base = y * w;
                let mut s: u16 = 0;
                for dx in 0..=2 * r {
                    let x = (dx + w - r) % w;
                    s += (self.kind[base + x] == col) as u16;
                }
                for x in 0..w {
                    row[base + x] = s;
                    let out = (x + w - r) % w;
                    let inn = (x + r + 1) % w;
                    s -= (self.kind[base + out] == col) as u16;
                    s += (self.kind[base + inn] == col) as u16;
                }
            }
            let cnt = &mut self.counts[c as usize];
            for x in 0..w {
                let mut s: u16 = 0;
                for dy in 0..=2 * r {
                    let y = (dy + h - r) % h;
                    s += row[y * w + x];
                }
                for y in 0..h {
                    cnt[y * w + x] = s;
                    let out = (y + h - r) % h;
                    let inn = (y + r + 1) % h;
                    s -= row[out * w + x];
                    s += row[inn * w + x];
                }
            }
        }
        let mut next = self.kind.clone();
        for i in 0..w * h {
            let k = self.kind[i];
            if k == BOULDER {
                continue;
            }
            let p = beater(k);
            if self.counts[(p - 1) as usize][i] as u32 >= THRESHOLD && self.rng.unit() < TAKE {
                next[i] = p;
                let from = self.nearest(i, p);
                let at = self.rng.unit();
                self.pending.push(Flip {
                    i: i as u32,
                    to: p,
                    from,
                    at,
                });
            }
        }
        self.kind = next;
    }

    /// A neighbour (nearest first, a little random) that wears `k`.
    fn nearest(&mut self, i: usize, k: u8) -> [i8; 2] {
        let (x, y) = ((i % self.w) as i32, (i / self.w) as i32);
        let start = self.rng.below(8) as usize;
        const RING1: [(i32, i32); 8] = [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];
        for n in 0..8 {
            let (dx, dy) = RING1[(start + n) % 8];
            if self.at(x + dx, y + dy) == k {
                return [dx as i8, dy as i8];
            }
        }
        for n in 0..8 {
            let (dx, dy) = RING1[(start + n) % 8];
            if self.at(x + 2 * dx, y + 2 * dy) == k {
                return [2 * dx as i8, 2 * dy as i8];
            }
        }
        let a = self.rng.unit() * std::f32::consts::TAU;
        [(a.cos() * 1.5).round() as i8, (a.sin() * 1.5).round() as i8]
    }

    #[inline]
    fn at(&self, x: i32, y: i32) -> u8 {
        let x = x.rem_euclid(self.w as i32) as usize;
        let y = y.rem_euclid(self.h as i32) as usize;
        self.kind[y * self.w + x]
    }

    fn show(&mut self, f: Flip) {
        let i = f.i as usize;
        self.was[i] = self.shown[i];
        self.shown[i] = f.to;
        self.look[i] = self.rng.below(64) as u8;
        self.from[i] = [f.from[0] as f32, f.from[1] as f32];
        self.heading[i] =
            (-(f.from[1] as f32)).atan2(-(f.from[0] as f32)) + (self.rng.unit() - 0.5) * 0.8;
        self.since[i] = 0.0;
    }

    /// The drawing's time: the step's flips come due as `phase` (0–1 through the step) passes them;
    /// the clocks of the recently changed run on.
    pub fn reveal(&mut self, phase: f32, dt: f32) {
        for s in self.since.iter_mut() {
            if *s < 9.0 {
                *s += dt;
            }
        }
        let mut k = 0;
        while k < self.pending.len() {
            if self.pending[k].at <= phase {
                let f = self.pending.swap_remove(k);
                self.show(f);
            } else {
                k += 1;
            }
        }
    }

    /// Lay lizards of `k` on the rocks within `r` cells of (`cx`, `cy`), running in from `dir`.
    /// Returns how many rocks changed hands.
    pub fn pour(&mut self, cx: f32, cy: f32, r: f32, k: u8, dir: [f32; 2]) -> usize {
        let mut laid = 0;
        let ri = r.ceil() as i32 + 1;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let x = (cx.floor() as i32 + dx).rem_euclid(self.w as i32) as usize;
                let y = (cy.floor() as i32 + dy).rem_euclid(self.h as i32) as usize;
                let fx = cx.floor() + dx as f32 + 0.5 - cx;
                let fy = cy.floor() + dy as f32 + 0.5 - cy;
                if fx * fx + fy * fy > r * r {
                    continue;
                }
                let i = y * self.w + x;
                if self.kind[i] == BOULDER || self.kind[i] == k {
                    continue;
                }
                self.pending.retain(|f| f.i as usize != i);
                self.kind[i] = k;
                let jx = (self.rng.unit() - 0.5) * 1.2;
                let jy = (self.rng.unit() - 0.5) * 1.2;
                let from = [
                    (-fx * 0.6 - dir[0] * 1.2 + jx).round() as i8,
                    (-fy * 0.6 - dir[1] * 1.2 + jy).round() as i8,
                ];
                self.show(Flip {
                    i: i as u32,
                    to: k,
                    from,
                    at: 0.0,
                });
                laid += 1;
            }
        }
        laid
    }

    fn blur(&self, m: &[f32], passes: usize) -> Vec<f32> {
        let (w, h) = (self.w, self.h);
        let mut a = m.to_vec();
        for _ in 0..passes {
            let mut b = vec![0.0; w * h];
            for y in 0..h {
                for x in 0..w {
                    let mut s = 0.0;
                    for (dx, dy, k) in [
                        (0i32, 0i32, 4.0f32),
                        (1, 0, 2.0),
                        (-1, 0, 2.0),
                        (0, 1, 2.0),
                        (0, -1, 2.0),
                        (1, 1, 1.0),
                        (-1, 1, 1.0),
                        (1, -1, 1.0),
                        (-1, -1, 1.0),
                    ] {
                        let xx = (x as i32 + dx).rem_euclid(w as i32) as usize;
                        let yy = (y as i32 + dy).rem_euclid(h as i32) as usize;
                        s += a[yy * w + xx] * k;
                    }
                    b[y * w + x] = s / 16.0;
                }
            }
            a = b;
        }
        a
    }

    /// The ground's relief for the drawing, one texel a rock (RGBA8): the outcrops' height and the
    /// shrubs' and the grass's, each the mask blurred so the shapes come out round.
    pub fn relief(&self) -> Vec<[u8; 4]> {
        let out: Vec<f32> = self
            .ground
            .iter()
            .map(|&g| (g == OUTCROP) as u8 as f32)
            .collect();
        let shrub: Vec<f32> = self
            .ground
            .iter()
            .map(|&g| (g == SHRUB) as u8 as f32)
            .collect();
        let grass: Vec<f32> = self
            .ground
            .iter()
            .map(|&g| (g == GRASS) as u8 as f32)
            .collect();
        let out = self.blur(&out, 3);
        let shrub = self.blur(&shrub, 3);
        let grass = self.blur(&grass, 2);
        (0..self.w * self.h)
            .map(|i| {
                [
                    (out[i] * 255.0) as u8,
                    (shrub[i] * 255.0) as u8,
                    (grass[i] * 255.0) as u8,
                    255,
                ]
            })
            .collect()
    }

    pub fn counts(&self) -> [usize; 4] {
        let mut c = [0; 4];
        for &k in &self.kind {
            c[k as usize] += 1;
        }
        c
    }
}

#[inline]
fn wrap(d: f32, n: f32) -> f32 {
    d - (d / n).round() * n
}

/// Smooth value noise on a torus of `pw` x `ph` periods.
fn vnoise(x: f32, y: f32, salt: u64, pw: f32, ph: f32) -> f32 {
    let (pw, ph) = (pw.round().max(1.0) as i64, ph.round().max(1.0) as i64);
    let h = |ix: i64, iy: i64| -> f32 {
        let mut z = (ix.rem_euclid(pw) as u64).wrapping_mul(0x9E3779B97F4A7C15)
            ^ (iy.rem_euclid(ph) as u64).wrapping_mul(0xC2B2AE3D27D4EB4F)
            ^ salt;
        z ^= z >> 31;
        z = z.wrapping_mul(0xBF58476D1CE4E5B9);
        z ^= z >> 29;
        (z >> 40) as f32 / (1u64 << 24) as f32
    };
    let (ix, iy) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = h(ix, iy) + (h(ix + 1, iy) - h(ix, iy)) * sx;
    let b = h(ix, iy + 1) + (h(ix + 1, iy + 1) - h(ix, iy + 1)) * sx;
    a + (b - a) * sy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_cycle() {
        assert!(beats(ORANGE, BLUE) && beats(BLUE, YELLOW) && beats(YELLOW, ORANGE));
        assert!(!beats(BLUE, ORANGE) && !beats(ORANGE, ORANGE) && !beats(ORANGE, BOULDER));
        for k in [ORANGE, BLUE, YELLOW] {
            assert!(beats(beater(k), k));
        }
    }

    #[test]
    fn nothing_wins() {
        // a long run keeps all three colours, none of them taking the hill
        let mut hill = Hill::new(256, 256, 3);
        for _ in 0..600 {
            hill.step();
        }
        let c = hill.counts();
        let held = (c[1] + c[2] + c[3]) as f64;
        for k in 1..4 {
            let share = c[k] as f64 / held;
            assert!(
                share > 0.15 && share < 0.55,
                "colour {k} holds {share:.2} of the hill"
            );
        }
    }
}
