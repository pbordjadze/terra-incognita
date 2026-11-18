//! The delta's land and its water. The Okavango fan falls about 1 m in 3,400 (some 70 m over its 250 km), so
//! relief of a metre or two decides everything: the old channels a little lower, the islands (most begun as
//! termite mounds) a little higher. The flood comes down the Panhandle at the top and spreads as water
//! does, from higher water to lower, over the land's own fall plus the phone's tilt; as it goes it sinks into
//! the Kalahari sand and is drunk and breathed off by the plants, so it reaches only so far (about 98 % of
//! what enters never leaves the delta). Where it lies, the grass greens, and stays green a while after.
//!
//! A grid of cells, each `CELL_M` metres across; heights and depths in metres.

/// One cell, metres (the screen spans about 14 by 30 km).
pub const CELL_M: f32 = 190.0;
/// The fan's own fall a cell, down the screen (1 in 3,400).
pub const FALL: f32 = CELL_M / 3400.0;
/// The phone's tilt, as the land's: the sine of the phone's lean times this is the extra fall a cell. A
/// phone leaning 15° tips the land about three times its own fall: the toy's tilt is scaled down some 300
/// times, since the real land's whole lean is a hundredth of a degree.
pub const TILT: f32 = FALL * 11.0;
/// The steepest lean that counts (its sine): past it, more tilt changes nothing.
pub const TILT_MAX: f32 = 0.5;
/// How freely water runs between cells: the share of a head difference that moves a substep.
pub const FLOW: f32 = 0.2;
/// The depth at which water runs freely (metres); shallower, it runs as depth^5/3.
pub const RUNS: f32 = 0.5;
/// Water lost a second where it lies (metres): into the sand and to the air with the rule on, to the air
/// alone with it off.
pub const SINKS: f32 = 0.005;
pub const DRIES: f32 = 0.0008;
/// The Panhandle's flow in, metres a second over its mouth's cells.
pub const INFLOW: f32 = 30.0;
/// The mouth: its centre (a share of the width) and half-width in cells.
pub const MOUTH: (f32, usize) = (0.42, 3);
/// Water shallower than this is the land merely damp (metres).
pub const WET: f32 = 0.03;

/// The rules as the sheet's switches name them, in `Land::on`'s order.
pub const RULES: [&str; 2] = ["Water sinks into the sand", "Islands stand a little higher"];

pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn n(&mut self) -> f32 {
        let u = self.f().max(1e-7);
        (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * self.f()).cos()
    }
}

pub struct Land {
    pub w: usize,
    pub h: usize,
    /// The ground's height without the islands, and the islands' own rise (metres).
    pub ground: Vec<f32>,
    pub mound: Vec<f32>,
    /// Standing water (metres).
    pub water: Vec<f32>,
    /// How green the grass is: rises where water lies, fades slowly after (0..1).
    pub green: Vec<f32>,
    /// The phone's tilt as the land's extra fall a cell, across and down the screen.
    pub tilt: [f32; 2],
    pub on: [bool; 2],
    flux: Vec<f32>,
    level: Vec<f32>,
    /// The surface without its water, worked out once a `run` (ground, islands, tilt, finger).
    base: Vec<f32>,
}

/// Smooth value noise at `p` (cells), lattice `cell` cells apart.
fn noise(seed: u64, p: [f32; 2], cell: f32) -> f32 {
    let hash = |i: i64, j: i64| {
        let mut x = (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ seed;
        x ^= x >> 31;
        x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x ^= x >> 29;
        (x >> 40) as f32 / (1u64 << 24) as f32
    };
    let (x, y) = (p[0] / cell, p[1] / cell);
    let (i, j) = (x.floor() as i64, y.floor() as i64);
    let (fx, fy) = (x - x.floor(), y - y.floor());
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = hash(i, j) + (hash(i + 1, j) - hash(i, j)) * sx;
    let b = hash(i, j + 1) + (hash(i + 1, j + 1) - hash(i, j + 1)) * sx;
    a + (b - a) * sy
}

impl Land {
    pub fn new(w: usize, h: usize, seed: u64) -> Land {
        let mut rng = Rng(seed | 1);
        let mut ground = vec![0.0; w * h];
        for y in 0..h {
            for x in 0..w {
                let p = [x as f32, y as f32];
                let relief = (noise(seed, p, 22.0) - 0.5) * 1.6
                    + (noise(seed ^ 1, p, 7.0) - 0.5) * 1.3
                    + (noise(seed ^ 2, p, 2.5) - 0.5) * 0.35;
                // the fan: falling away from the Panhandle's mouth every way, a little faster down it
                let (dx, dy) = (x as f32 - MOUTH.0 * w as f32, y as f32 + 6.0);
                let r = (dx * dx * 1.3 + dy * dy).sqrt();
                ground[y * w + x] = -FALL * r + relief;
            }
        }
        let mut land = Land {
            w,
            h,
            ground,
            mound: vec![0.0; w * h],
            water: vec![0.0; w * h],
            green: vec![0.0; w * h],
            tilt: [0.0, 0.0],
            on: [true, true],
            flux: vec![0.0; w * h],
            level: vec![0.0; w * h],
            base: vec![0.0; w * h],
        };
        land.carve(&mut rng);
        land.raise_islands(&mut rng);
        land
    }

    /// The old channels: a river from the mouth that wanders down the fan and splits, each branch thinner
    /// and shallower, cut a metre or two into the ground.
    fn carve(&mut self, rng: &mut Rng) {
        let mut cut = vec![0.0; self.w * self.h];
        let mut stack = vec![(
            MOUTH.0 * self.w as f32,
            0.0f32,
            0.0f32,
            2.0f32,
            1.0f32,
            0u32,
        )];
        while let Some((mut x, mut y, mut a, mut wd, mut depth, gen)) = stack.pop() {
            while wd > 0.5 && y < self.h as f32 && x > 0.0 && x < self.w as f32 {
                a += rng.n() * 0.12 - a * 0.05;
                x += a.sin() * 0.7;
                y += a.cos() * 0.7;
                self.cut(&mut cut, x, y, wd, depth);
                wd *= 0.993;
                depth *= 0.997;
                if gen < 5 && rng.f() < 0.018 {
                    let k = 0.45 + 0.2 * rng.f();
                    stack.push((
                        x,
                        y,
                        a + 0.4 + 0.4 * rng.f(),
                        wd * (1.0 - k) * 1.3,
                        depth * 0.85,
                        gen + 1,
                    ));
                    a -= 0.2 + 0.3 * rng.f();
                    wd *= k * 1.2;
                }
            }
        }
        for (g, c) in self.ground.iter_mut().zip(&cut) {
            *g -= c;
        }
        // the flood already in the channels' upper reaches, and the grass green beside them, so the
        // toy needs only a few seconds' run before it opens
        for (i, c) in cut.iter().enumerate() {
            if i / self.w < self.h * 2 / 3 {
                self.water[i] = 0.8 * c;
                self.green[i] = (c * 2.0).min(1.0);
            }
        }
    }

    /// One stretch of channel: the ground at most `depth` lower there, falling off over its width (a
    /// channel crossing another keeps the deeper of the two).
    fn cut(&mut self, cut: &mut [f32], x: f32, y: f32, wd: f32, depth: f32) {
        let r = (wd * 2.5).ceil() as i64;
        for j in -r..=r {
            for i in -r..=r {
                let (cx, cy) = (x as i64 + i, y as i64 + j);
                if cx < 0 || cy < 0 || cx >= self.w as i64 || cy >= self.h as i64 {
                    continue;
                }
                let d2 = (cx as f32 + 0.5 - x).powi(2) + (cy as f32 + 0.5 - y).powi(2);
                let c = &mut cut[cy as usize * self.w + cx as usize];
                *c = c.max(depth * (-d2 / (2.0 * wd * wd)).exp());
            }
        }
    }

    /// The islands: most began as a termite mound on a rise, then trees took root and salts gathered; a
    /// metre or two above the floodplain, a few hundred metres to a few kilometres across.
    fn raise_islands(&mut self, rng: &mut Rng) {
        for _ in 0..(self.w * self.h / 260) {
            let (x, y) = (rng.f() * self.w as f32, rng.f() * self.h as f32);
            let r = 0.8 + 2.6 * rng.f().powi(2);
            let hgt = 1.2 + 1.2 * rng.f();
            let n = (r * 2.5).ceil() as i64;
            for j in -n..=n {
                for i in -n..=n {
                    let (cx, cy) = (x as i64 + i, y as i64 + j);
                    if cx < 0 || cy < 0 || cx >= self.w as i64 || cy >= self.h as i64 {
                        continue;
                    }
                    let d2 = (cx as f32 + 0.5 - x).powi(2) + (cy as f32 + 0.5 - y).powi(2);
                    let m = &mut self.mound[cy as usize * self.w + cx as usize];
                    *m = m.max(hgt * (-(d2 / (r * r)).powi(2)).exp());
                }
            }
        }
    }

    /// The surface at cell `i` (x, y) without its water, as the flow sees it: ground, islands, the
    /// tilt.
    fn bed(&self, i: usize, x: usize, y: usize) -> f32 {
        let mut s = self.ground[i];
        if self.on[1] {
            s += self.mound[i];
        }
        s -= self.tilt[0] * (x as f32 - self.w as f32 * 0.5)
            + self.tilt[1] * (y as f32 - self.h as f32 * 0.5);
        s
    }

    /// The ground's height under the water at `i` as the picture sees it (islands as switched).
    pub fn height(&self, i: usize) -> f32 {
        self.ground[i] + if self.on[1] { self.mound[i] } else { 0.0 }
    }

    /// `n` substeps of the flow, the whole of `dt` seconds of loss and greening.
    pub fn run(&mut self, dt: f32, n: usize) {
        let (w, h) = (self.w, self.h);
        let c = MOUTH.0 * w as f32;
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                self.base[i] = self.bed(i, x, y);
            }
        }
        for _ in 0..n {
            // the flood coming down the Panhandle
            for k in 0..=2 * MOUTH.1 {
                let x = (c as usize + k).saturating_sub(MOUTH.1).min(w - 1);
                self.water[x] += INFLOW * dt / n as f32 / (2 * MOUTH.1 + 1) as f32;
            }
            for i in 0..w * h {
                self.level[i] = self.base[i] + self.water[i];
            }
            self.flux.iter_mut().for_each(|f| *f = 0.0);
            for y in 0..h {
                for x in 0..w {
                    let i = y * w + x;
                    if x + 1 < w {
                        self.exchange(i, i + 1);
                    }
                    if y + 1 < h {
                        self.exchange(i, i + w);
                    }
                }
            }
            for i in 0..w * h {
                self.water[i] = (self.water[i] + self.flux[i]).max(0.0);
            }
            // off the screen's bottom and sides where the surface falls that way, the flood runs on
            for y in 0..h {
                for x in [0, w - 1] {
                    self.spill(y * w + x, if x == 0 { 1 } else { -1 }, 0);
                }
            }
            // and off the top, beside the river's mouth, when the phone is tipped that way
            let mouth = (MOUTH.0 * w as f32) as usize;
            for x in 0..w {
                self.spill((h - 1) * w + x, 0, -1);
                if x + MOUTH.1 + 2 < mouth || x > mouth + MOUTH.1 + 2 {
                    self.spill(x, 0, 1);
                }
            }
        }
        let lose = if self.on[0] { SINKS } else { DRIES } * dt;
        for i in 0..w * h {
            let wet = self.water[i] > WET;
            self.water[i] = (self.water[i] - lose).max(0.0);
            let g = &mut self.green[i];
            *g = if wet {
                (*g + dt * 0.5).min(1.0)
            } else {
                (*g - dt * 0.02).max(0.0)
            };
        }
    }

    /// Water moving between neighbours `a` and `b` this substep, from the higher surface to the lower.
    fn exchange(&mut self, a: usize, b: usize) {
        let d = self.level[a] - self.level[b];
        // a thin sheet barely moves and a deep channel runs freely (Manning: flow goes as depth^5/3)
        let up = if d > 0.0 {
            self.water[a]
        } else {
            self.water[b]
        };
        if up < 0.004 {
            // a film this thin only soaks in where it lies
            return;
        }
        let k = FLOW * (up / RUNS).min(1.0).powf(5.0 / 3.0);
        let f = if d > 0.0 {
            (k * d).min(self.water[a] * 0.24)
        } else {
            (k * d).max(-self.water[b] * 0.24)
        };
        self.flux[a] -= f;
        self.flux[b] += f;
    }

    /// An edge cell `i` losing water off the screen when its surface falls outward (the cell inward is
    /// `dx`, `dy` away; beyond the edge the land goes on falling as it does here).
    fn spill(&mut self, i: usize, dx: i64, dy: i64) {
        let j = (i as i64 + dx + dy * self.w as i64) as usize;
        let fall = self.level[j] - self.level[i] + self.water[i] - self.water[j];
        if fall > 0.0 {
            // falling outward from j to i: it goes on falling from i outward
            self.water[i] = (self.water[i] - (FLOW * fall).min(self.water[i] * 0.24)).max(0.0);
        }
    }
}
