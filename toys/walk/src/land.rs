//! The land, the walker and its footprints: everything the toy shows is these three.
//!
//! The land is a height `h` over the screen (`x` across in screen widths, 0..1; `y` down, 0..`aspect`):
//! a few hills of different heights far apart, small bumps and hollows on them, a gentle plain between.
//! The walker follows one rule, Metropolis's (N. Metropolis, A. W. Rosenbluth, M. N. Rosenbluth,
//! A. H. Teller and E. Teller, 1953): from where it stands it picks a spot at random anywhere within its
//! stride (uniformly in a disc, so a step and its way back are equally likely), and goes there if the
//! ground there is higher; if lower, it goes only with the chance `exp(K · (h(new) - h(here)))`, and
//! otherwise stays. Every try, gone or not, leaves a footprint where the walker then stands. Off the
//! screen there is no ground: a step there is never taken.
//!
//! So the footprints pile up in proportion to `exp(K · h)`, and the log of their pile, drawn as height,
//! is the land's own height (up to a constant the map takes out): the map is a measurement, not a
//! drawing of the land. `cargo test -p walk` checks it.

/// How strongly the walker prefers high ground: the chance of standing at a place goes as
/// `exp(K · h)`, so a summit (h ≈ 1) is `e^K` times as likely as the plain (h ≈ 0).
pub const K: f64 = 7.0;
/// The footprint map's cells across the screen (its height in cells follows the screen's shape).
pub const GW: usize = 144;
/// A footprint's softness, in cells (the map's finest detail).
const SOFT: f64 = 2.0;
const REACH: i32 = 5;

/// A small, fast, seeded random source (xorshift64*), so a plan plays the same walk every time.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    pub fn u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in [0, 1).
    pub fn f(&mut self) -> f64 {
        (self.u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f()
    }
}

/// One rounded rise (or, with a negative height, a hollow): `amp · exp(-d² / 2σ²)`.
#[derive(Clone, Copy, Debug)]
pub struct Bump {
    pub c: [f64; 2],
    pub sigma: f64,
    pub amp: f64,
}

pub struct Land {
    pub aspect: f64,
    pub bumps: Vec<Bump>,
    /// The hills' summits (their centres), for the tests and the check's plans.
    pub hills: Vec<[f64; 2]>,
    /// The highest the land reaches, and log ∫ exp(K h) over the screen (the constant the map takes out).
    pub top: f64,
    pub log_z: f64,
}

impl Land {
    /// A new land for a screen `aspect` (height / width) times as tall as it is wide.
    pub fn new(seed: u64, aspect: f64) -> Land {
        let mut r = Rng::new(seed ^ 0x1a2d_5eed);
        let aspect = aspect.clamp(0.5, 3.0);
        let mut bumps = Vec::new();
        // the hills: three (sometimes two, sometimes four), far apart, each its own height
        let n = match r.f() {
            f if f < 0.2 => 2,
            f if f < 0.85 => 3,
            _ => 4,
        };
        let gap = if n == 4 { 0.42 } else { 0.50 };
        let mut hills: Vec<[f64; 2]> = Vec::new();
        let mut tries = 0;
        while hills.len() < n && tries < 4000 {
            tries += 1;
            let c = [r.range(0.2, 0.8), r.range(0.16, 0.84) * aspect];
            let need = gap * (1.0 - tries as f64 / 8000.0);
            if hills
                .iter()
                .all(|h| (h[0] - c[0]).hypot(h[1] - c[1]) > need)
            {
                hills.push(c);
            }
        }
        let heights = [
            1.0,
            r.range(0.80, 0.94),
            r.range(0.66, 0.84),
            r.range(0.62, 0.76),
        ];
        for (i, &c) in hills.iter().enumerate() {
            let sigma = r.range(0.10, 0.13);
            bumps.push(Bump {
                c,
                sigma,
                amp: heights[i],
            });
            // its shoulders, spurs and hollows: the fine detail small steps find
            for _ in 0..6 {
                let a = r.range(0.0, std::f64::consts::TAU);
                let d = r.range(0.3, 1.5) * sigma;
                let s = r.range(0.022, 0.04);
                let amp = if r.f() < 0.65 {
                    r.range(0.08, 0.15)
                } else {
                    -r.range(0.06, 0.11)
                };
                bumps.push(Bump {
                    c: [c[0] + d * a.cos(), c[1] + d * a.sin()],
                    sigma: s,
                    amp,
                });
            }
        }
        // the plain between: gentle swells, low
        for _ in 0..4 {
            bumps.push(Bump {
                c: [r.range(0.0, 1.0), r.range(0.0, aspect)],
                sigma: r.range(0.18, 0.28),
                amp: r.range(0.04, 0.09),
            });
        }
        let mut land = Land {
            aspect,
            bumps,
            hills,
            top: 1.0,
            log_z: 0.0,
        };
        // the summit and the normalising constant, on a fine grid
        let (nx, ny) = (360, (360.0 * aspect) as usize);
        let da = (1.0 / nx as f64) * (aspect / ny as f64);
        let mut top = f64::MIN;
        let mut hs = Vec::with_capacity(nx * ny);
        for j in 0..ny {
            for i in 0..nx {
                let h = land.h([
                    (i as f64 + 0.5) / nx as f64,
                    (j as f64 + 0.5) / ny as f64 * aspect,
                ]);
                top = top.max(h);
                hs.push(h);
            }
        }
        let z: f64 = hs.iter().map(|&h| (K * (h - top)).exp()).sum::<f64>() * da;
        land.top = top;
        land.log_z = K * top + z.ln();
        land
    }

    pub fn inside(&self, p: [f64; 2]) -> bool {
        (0.0..=1.0).contains(&p[0]) && (0.0..=self.aspect).contains(&p[1])
    }

    /// The land's height at `p`.
    pub fn h(&self, p: [f64; 2]) -> f64 {
        self.bumps
            .iter()
            .map(|b| {
                let d2 = (p[0] - b.c[0]).powi(2) + (p[1] - b.c[1]).powi(2);
                b.amp * (-d2 / (2.0 * b.sigma * b.sigma)).exp()
            })
            .sum()
    }

    /// log of the chance of standing at `p`, per unit of area (screen widths squared): the land's
    /// height times K, less the constant.
    pub fn log_p(&self, p: [f64; 2]) -> f64 {
        K * self.h(p) - self.log_z
    }

    /// A place on a hill's shoulder, to begin a night from: the walker is seen to climb.
    pub fn start(&self, r: &mut Rng) -> [f64; 2] {
        let k = (r.f() * self.hills.len() as f64) as usize % self.hills.len().max(1);
        self.start_on(k, r)
    }

    /// The hill nearest the screen's middle (the first night's, so the others lie above and below it).
    pub fn middle_hill(&self) -> usize {
        let m = [0.5, 0.5 * self.aspect];
        (0..self.hills.len())
            .min_by(|&a, &b| {
                let d = |i: usize| (self.hills[i][0] - m[0]).hypot(self.hills[i][1] - m[1]);
                d(a).total_cmp(&d(b))
            })
            .unwrap_or(0)
    }

    /// A place on hill `k`'s shoulder.
    pub fn start_on(&self, k: usize, r: &mut Rng) -> [f64; 2] {
        let c = self
            .hills
            .get(k)
            .copied()
            .unwrap_or([0.5, 0.5 * self.aspect]);
        for _ in 0..200 {
            let a = r.range(0.0, std::f64::consts::TAU);
            let d = r.range(0.07, 0.12);
            let p = [c[0] + d * a.cos(), c[1] + d * a.sin()];
            if p[0] > 0.06 && p[0] < 0.94 && p[1] > 0.06 && p[1] < self.aspect - 0.06 {
                return p;
            }
        }
        c
    }
}

/// What one try did.
#[derive(Clone, Copy, Debug)]
pub struct Try {
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub went: bool,
}

/// The walker: where it stands, how far it may step, and its footprints.
pub struct Walker {
    pub at: [f64; 2],
    pub h: f64,
    /// The stride: the farthest one step may go, in screen widths.
    pub stride: f64,
    pub rng: Rng,
    pub tries: u64,
    pub went: u64,
    /// The footprint map: GW cells across, `gh` down; each try adds one footprint (a soft spot whose
    /// weights sum to 1) where the walker then stands.
    pub gh: usize,
    pub prints: Vec<f32>,
    pub count: u64,
}

impl Walker {
    pub fn new(land: &Land, seed: u64, stride: f64) -> Walker {
        let mut rng = Rng::new(seed);
        let at = land.start(&mut rng);
        let gh = ((GW as f64) * land.aspect).ceil() as usize;
        Walker {
            at,
            h: land.h(at),
            stride,
            rng,
            tries: 0,
            went: 0,
            gh,
            prints: vec![0.0; GW * gh],
            count: 0,
        }
    }

    /// One try: Metropolis's rule, exactly.
    pub fn step(&mut self, land: &Land) -> Try {
        // a spot uniformly within the stride's disc
        let a = self.rng.range(0.0, std::f64::consts::TAU);
        let d = self.stride * self.rng.f().sqrt();
        let to = [self.at[0] + d * a.cos(), self.at[1] + d * a.sin()];
        let from = self.at;
        let mut went = false;
        if land.inside(to) {
            let h = land.h(to);
            let u = self.rng.f();
            if h >= self.h || u < (K * (h - self.h)).exp() {
                self.at = to;
                self.h = h;
                went = true;
            }
        }
        self.tries += 1;
        if went {
            self.went += 1;
        }
        self.print(self.at);
        Try { from, to, went }
    }

    fn print(&mut self, p: [f64; 2]) {
        let cell = 1.0 / GW as f64;
        let (fx, fy) = (p[0] / cell - 0.5, p[1] / cell - 0.5);
        let (ix, iy) = (fx.round() as i32, fy.round() as i32);
        let mut w = [[0.0f64; (2 * REACH + 1) as usize]; (2 * REACH + 1) as usize];
        let mut sum = 0.0;
        for dy in -REACH..=REACH {
            for dx in -REACH..=REACH {
                let (cx, cy) = (ix + dx, iy + dy);
                let d2 = (cx as f64 - fx).powi(2) + (cy as f64 - fy).powi(2);
                let v = (-d2 / (2.0 * SOFT * SOFT)).exp();
                w[(dy + REACH) as usize][(dx + REACH) as usize] = v;
                sum += v;
            }
        }
        for dy in -REACH..=REACH {
            for dx in -REACH..=REACH {
                let (cx, cy) = (ix + dx, iy + dy);
                if cx >= 0 && cy >= 0 && (cx as usize) < GW && (cy as usize) < self.gh {
                    self.prints[cy as usize * GW + cx as usize] +=
                        (w[(dy + REACH) as usize][(dx + REACH) as usize] / sum) as f32;
                }
            }
        }
        self.count += 1;
    }

    /// The share of tries that went, over the walk so far.
    pub fn went_share(&self) -> f64 {
        self.went as f64 / self.tries.max(1) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASPECT: f64 = 2556.0 / 1179.0;

    /// Which hill a place belongs to (the nearest summit), or none on the plain.
    fn hill_of(land: &Land, p: [f64; 2]) -> Option<usize> {
        land.hills
            .iter()
            .enumerate()
            .map(|(i, c)| (i, (c[0] - p[0]).hypot(c[1] - p[1])))
            .filter(|(_, d)| *d < 0.2)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(i, _)| i)
    }

    /// One night (40 s at the toy's pace): the share of tries that went, the hills visited, the times
    /// it crossed from one hill to another.
    fn night(seed: u64, stride: f64) -> (f64, usize, usize) {
        let land = Land::new(seed, ASPECT);
        let mut w = Walker::new(&land, seed + 7, stride);
        let mut seen = vec![false; land.hills.len()];
        let mut last = None;
        let mut crossings = 0;
        for _ in 0..(40.0 * crate::PACE) as usize {
            w.step(&land);
            if let Some(k) = hill_of(&land, w.at) {
                seen[k] = true;
                if last.is_some() && last != Some(k) {
                    crossings += 1;
                }
                last = Some(k);
            }
        }
        (
            w.went_share(),
            seen.iter().filter(|s| **s).count(),
            crossings,
        )
    }

    #[test]
    fn strides_feel_different() {
        for (name, stride) in [
            ("tiny", 0.008),
            ("small", 0.035),
            ("good", 0.3),
            ("huge", 2.0),
        ] {
            let mut sum = (0.0, 0.0, 0.0);
            let n = 24;
            for s in 0..n {
                let (a, h, c) = night(100 + s, stride);
                sum.0 += a / n as f64;
                sum.1 += h as f64 / n as f64;
                sum.2 += c as f64 / n as f64;
            }
            eprintln!(
                "{name:6} stride {stride:5}: went {:.2}, hills seen {:.2}, crossings {:.1}",
                sum.0, sum.1, sum.2
            );
            match name {
                // tiny steps: nearly every one goes, and the walker stays on its first hill
                "tiny" => assert!(sum.0 > 0.9 && sum.1 < 1.3, "tiny"),
                "small" => assert!(sum.0 > 0.6 && sum.1 < 1.8, "small"),
                // a good stride: about half go, and the walker crosses between hills many times
                "good" => assert!(
                    sum.0 > 0.12 && sum.0 < 0.6 && sum.1 > 2.5 && sum.2 > 10.0,
                    "good"
                ),
                // huge strides: almost every one is refused
                _ => assert!(sum.0 < 0.06, "huge"),
            }
        }
    }

    /// The footprints, taken long enough, are the land: their log, against the land's own height,
    /// where they are thick.
    #[test]
    fn the_map_is_the_land() {
        let land = Land::new(3, ASPECT);
        let mut w = Walker::new(&land, 11, 0.3);
        for _ in 0..1_500_000 {
            w.step(&land);
        }
        let cell = 1.0 / GW as f64;
        let mut err = 0.0;
        let mut n = 0.0;
        for j in 0..w.gh {
            for i in 0..GW {
                let c = w.prints[j * GW + i] as f64;
                let p = [(i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell];
                if c > 200.0 && land.inside(p) {
                    let est = (c / (w.count as f64 * cell * cell)).ln();
                    let real = land.log_p(p);
                    err += (est - real).abs();
                    n += 1.0;
                }
            }
        }
        let mean = err / n;
        eprintln!("cells {n}, mean |log error| {mean:.3}");
        // the footprints' softness blurs the land's finest bumps a little
        assert!(n > 500.0 && mean < 0.16, "{mean}");
    }

    #[test]
    fn lands_are_whole() {
        for s in 0..200 {
            for aspect in [ASPECT, 2752.0 / 2064.0] {
                let land = Land::new(s, aspect);
                assert!(land.hills.len() >= 2);
                assert!(land.top > 0.9 && land.top < 1.4, "top {}", land.top);
                assert!(land.log_z.is_finite());
            }
        }
    }
}
