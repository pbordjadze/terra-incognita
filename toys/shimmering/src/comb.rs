//! The comb's curtain of giant honeybees (Apis dorsata), seen face on: a grid of cells (`CELL` cm), one
//! bee a cell where the comb is, each either resting or flipping its abdomen up. A flip is catching: a
//! resting bee beside a flipping one flips too, after a short, random delay, and then rests a while before
//! it can flip again, so a few bees started by a hornet send a wave across the curtain (an excitable
//! medium; Kastberger et al. 2008, SOURCES.md). The hornet (`Hornet`) hovers in front of the comb, starts
//! flips where it hovers and is pushed away by the waves.

/// One bee a cell, this many centimetres across.
pub const CELL: f32 = 1.0;

/// The rules as switches (the sheet's), in order.
pub const RULES: [&str; 3] = [
    "Neighbours follow",
    "Rest after a flip",
    "Waves drive it off",
];

/// The simulation's step (seconds): shorter than any delay between neighbours.
const STEP: f32 = 1.0 / 240.0;
/// A flip lasts this long (seconds): the abdomen up, held, and down again.
const FLIP_S: f32 = 0.34;
/// The abdomen is up for this long of it.
const UP_S: f32 = 0.18;
/// A flipping bee sets its neighbours off during this first part of its flip.
const EXCITE_S: f32 = 0.11;
/// After a flip a bee rests this long before it can flip again.
const REST_S: f32 = 1.0;
/// Without the rest, a bee can flip again this soon after it began.
const EAGER_S: f32 = 0.2;
/// How quickly a flipping neighbour sets a bee off (a second): fronts run about half a metre a second.
const CATCH_PER_S: f32 = 7.0;
/// A hornet starts this many flips a second where it hovers, from this far (cm).
const START_PER_S: f32 = 3.0;
const START_CM: f32 = 3.0;
/// The waves push the hornet this hard (cm/s² a flipping bee near it), from this near (cm).
const PUSH: f32 = 30.0;
const PUSH_CM: f32 = 7.0;

pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// 0..1
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// -1..1
    pub fn s(&mut self) -> f32 {
        self.f() * 2.0 - 1.0
    }
}

/// The hornet (Vespa), hovering in front of the comb: the finger's when one is down, its own otherwise.
pub struct Hornet {
    pub p: [f32; 2],
    pub v: [f32; 2],
    /// where it is making for, and how long before it picks another place
    target: [f32; 2],
    retarget: f32,
    /// seconds left of being driven off (it keeps away)
    pub fleeing: f32,
    /// the finger, when it holds the hornet, and the waves' shove away from it
    pub finger: Option<[f32; 2]>,
    shove: [f32; 2],
}

pub struct Comb {
    pub w: f32,
    pub h: f32,
    pub gw: usize,
    pub gh: usize,
    pub on: [bool; 3],
    pub hornet: Hornet,
    bee: Vec<bool>,
    /// when each bee began its last flip (seconds)
    fired: Vec<f32>,
    /// how far up each abdomen is, 0..1, and the flash of a flip just begun
    pose: Vec<f32>,
    flash: Vec<f32>,
    t: f32,
    acc: f32,
    rng: Rng,
}

impl Comb {
    pub fn new(w: f32, h: f32, seed: u64) -> Comb {
        let (gw, gh) = (
            (w / CELL).ceil() as usize + 1,
            (h / CELL).ceil() as usize + 1,
        );
        let mut bee = vec![false; gw * gh];
        let (cx, top, a, b) = shape(w, h);
        for j in 0..gh {
            for i in 0..gw {
                let x = (i as f32 + 0.5) * CELL;
                let y = (j as f32 + 0.5) * CELL;
                let q = ((x - cx) / a).powi(2) + ((y - top) / b).powi(2);
                bee[j * gw + i] = y > top && q < 1.0;
            }
        }
        let start = [w * 0.8, h * 0.95];
        Comb {
            w,
            h,
            gw,
            gh,
            on: [true; 3],
            hornet: Hornet {
                p: start,
                v: [0.0, -10.0],
                target: [w * 0.55, h * 0.45],
                retarget: 2.0,
                fleeing: 0.0,
                finger: None,
                shove: [0.0; 2],
            },
            bee,
            fired: vec![-100.0; gw * gh],
            pose: vec![0.0; gw * gh],
            flash: vec![0.0; gw * gh],
            t: 0.0,
            acc: 0.0,
            rng: Rng(seed),
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.acc += dt.clamp(0.0, 0.1);
        while self.acc >= STEP {
            self.acc -= STEP;
            self.t += STEP;
            self.spread();
            self.start_flips();
            self.fly(STEP);
        }
        let k = 1.0 - (-dt / 0.035).exp();
        let fade = (-dt / 0.25).exp();
        for i in 0..self.pose.len() {
            let age = self.t - self.fired[i];
            let up = if age < UP_S { 1.0 } else { 0.0 };
            self.pose[i] += (up - self.pose[i]) * k;
            self.flash[i] *= fade;
        }
    }

    /// A bee can flip now: it is not mid-flip, and (with the rest) has rested since its last.
    fn ready(&self, i: usize) -> bool {
        let age = self.t - self.fired[i];
        if self.on[1] {
            age > FLIP_S + REST_S
        } else {
            age > EAGER_S
        }
    }

    fn fire(&mut self, i: usize) {
        self.fired[i] = self.t;
        self.flash[i] = 1.0;
    }

    /// Each resting bee beside flipping ones flips too, sooner the more of them there are.
    fn spread(&mut self) {
        if !self.on[0] {
            return;
        }
        let (gw, gh) = (self.gw as i32, self.gh as i32);
        let mut catch = Vec::new();
        for j in 0..gh {
            for i in 0..gw {
                let c = (j * gw + i) as usize;
                if !self.bee[c] || !self.ready(c) {
                    continue;
                }
                let mut n = 0.0;
                for (dx, dy, wgt) in NEIGHBOURS {
                    let (x, y) = (i + dx, j + dy);
                    if x < 0 || y < 0 || x >= gw || y >= gh {
                        continue;
                    }
                    let d = (y * gw + x) as usize;
                    let age = self.t - self.fired[d];
                    if self.bee[d] && (0.0..EXCITE_S).contains(&age) {
                        n += wgt;
                    }
                }
                if n > 0.0 && self.rng.f() < 1.0 - (-CATCH_PER_S * n * STEP).exp() {
                    catch.push(c);
                }
            }
        }
        for c in catch {
            self.fire(c);
        }
    }

    /// The hornet near the comb sets off a few bees where it hovers.
    fn start_flips(&mut self) {
        let p = self.hornet.p;
        if self.rng.f() >= START_PER_S * STEP {
            return;
        }
        let a = self.rng.f() * std::f32::consts::TAU;
        let r = self.rng.f().sqrt() * START_CM;
        let q = [p[0] + a.cos() * r, p[1] + a.sin() * r];
        if let Some(c) = self.cell_at(q) {
            // a little knot of bees, as a wave starts with a few
            for (dx, dy, _) in NEIGHBOURS.iter().take(4).chain([(0, 0, 1.0)].iter()) {
                let (x, y) = (c.0 as i32 + dx, c.1 as i32 + dy);
                if x < 0 || y < 0 || x >= self.gw as i32 || y >= self.gh as i32 {
                    continue;
                }
                let d = y as usize * self.gw + x as usize;
                if self.bee[d] && self.ready(d) {
                    self.fire(d);
                }
            }
        }
    }

    fn cell_at(&self, q: [f32; 2]) -> Option<(usize, usize)> {
        let (i, j) = ((q[0] / CELL).floor(), (q[1] / CELL).floor());
        if i < 0.0 || j < 0.0 || i as usize >= self.gw || j as usize >= self.gh {
            return None;
        }
        let (i, j) = (i as usize, j as usize);
        self.bee[j * self.gw + i].then_some((i, j))
    }

    /// The waves' push on the hornet: away from the bees flipping near it.
    fn push(&self) -> [f32; 2] {
        if !self.on[2] {
            return [0.0; 2];
        }
        let p = self.hornet.p;
        let r = (PUSH_CM / CELL).ceil() as i32;
        let (ci, cj) = ((p[0] / CELL) as i32, (p[1] / CELL) as i32);
        let mut f = [0.0f32; 2];
        for j in (cj - r).max(0)..(cj + r + 1).min(self.gh as i32) {
            for i in (ci - r).max(0)..(ci + r + 1).min(self.gw as i32) {
                let c = j as usize * self.gw + i as usize;
                let age = self.t - self.fired[c];
                if !self.bee[c] || !(0.0..FLIP_S).contains(&age) {
                    continue;
                }
                let d = [
                    p[0] - (i as f32 + 0.5) * CELL,
                    p[1] - (j as f32 + 0.5) * CELL,
                ];
                let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(0.5);
                if l < PUSH_CM {
                    let k = PUSH * (1.0 - l / PUSH_CM) / l;
                    f[0] += d[0] * k;
                    f[1] += d[1] * k;
                }
            }
        }
        f
    }

    fn fly(&mut self, dt: f32) {
        let push = self.push();
        let wob = [self.rng.s() * 400.0, self.rng.s() * 400.0];
        let (w, h) = (self.w, self.h);
        let pushed = (push[0] * push[0] + push[1] * push[1]).sqrt();
        let hn = &mut self.hornet;
        if let Some(f) = hn.finger {
            // the finger holds it; a wave shoves it off the finger for a moment
            hn.shove[0] = (hn.shove[0] + push[0] * dt * 0.06) * (1.0 - dt * 3.0);
            hn.shove[1] = (hn.shove[1] + push[1] * dt * 0.06) * (1.0 - dt * 3.0);
            let to = [f[0] + hn.shove[0], f[1] + hn.shove[1]];
            hn.v = [(to[0] - hn.p[0]) * 18.0, (to[1] - hn.p[1]) * 18.0];
            hn.p = [hn.p[0] + hn.v[0] * dt, hn.p[1] + hn.v[1] * dt];
            return;
        }
        hn.retarget -= dt;
        hn.fleeing = (hn.fleeing - dt).max(0.0);
        if pushed > 90.0 && hn.fleeing <= 0.0 {
            // driven off: it backs away out of the curtain's reach for a while
            hn.fleeing = 3.5;
            let side = if hn.p[0] < w * 0.5 { -0.15 } else { 1.15 };
            hn.target = [w * side, h * (0.75 + 0.2 * (hn.p[1] / h))];
        } else if hn.retarget <= 0.0 && hn.fleeing <= 0.0 {
            hn.retarget = 2.5;
            let u = (hn.p[0] * 7.31 + hn.p[1] * 3.17).sin() * 0.5 + 0.5;
            let v = (hn.p[0] * 2.13 - hn.p[1] * 5.71).cos() * 0.5 + 0.5;
            hn.target = [w * (0.2 + 0.6 * u), h * (0.25 + 0.4 * v)];
        }
        let to = [hn.target[0] - hn.p[0], hn.target[1] - hn.p[1]];
        let a = [
            to[0] * 2.5 - hn.v[0] * 2.2 + wob[0] * 0.1 + push[0],
            to[1] * 2.5 - hn.v[1] * 2.2 + wob[1] * 0.1 + push[1],
        ];
        hn.v = [hn.v[0] + a[0] * dt, hn.v[1] + a[1] * dt];
        hn.p = [hn.p[0] + hn.v[0] * dt, hn.p[1] + hn.v[1] * dt];
    }

    /// Four numbers a cell for the shader: (abdomen up 0..1, a bee here 0/1, the flip's flash, -).
    pub fn texels(&self, out: &mut Vec<[f32; 4]>) {
        out.clear();
        out.extend((0..self.bee.len()).map(|i| {
            [
                self.pose[i],
                if self.bee[i] { 1.0 } else { 0.0 },
                self.flash[i],
                0.0,
            ]
        }));
    }
}

/// The comb's shape on a screen `w` by `h` cm: a half ellipse hanging from the overhang (its centre's x, its
/// top, its half width, its depth).
pub fn shape(w: f32, h: f32) -> (f32, f32, f32, f32) {
    (w * 0.48, h * 0.11, w * 0.78, h * 0.72)
}

/// The eight neighbours, the nearer four first, with how strongly each sets a bee off.
const NEIGHBOURS: [(i32, i32, f32); 8] = [
    (1, 0, 1.0),
    (-1, 0, 1.0),
    (0, 1, 1.0),
    (0, -1, 1.0),
    (1, 1, 0.7),
    (-1, 1, 0.7),
    (1, -1, 0.7),
    (-1, -1, 0.7),
];

#[cfg(test)]
mod tests {
    use super::*;

    fn flipping(c: &Comb) -> usize {
        c.fired.iter().filter(|&&f| c.t - f < FLIP_S).count()
    }

    #[test]
    fn a_wave_spreads_and_passes() {
        let mut c = Comb::new(46.0, 100.0, 7);
        c.hornet.p = [-50.0, -50.0];
        let mid = 40 * c.gw + 23;
        c.fire(mid);
        let mut most = 0;
        for _ in 0..60 {
            c.step(1.0 / 60.0);
            most = most.max(flipping(&c));
        }
        assert!(most > 200, "the wave never spread ({most})");
        for _ in 0..120 {
            c.step(1.0 / 60.0);
        }
        assert_eq!(flipping(&c), 0, "the wave never died out");
    }

    #[test]
    fn without_neighbours_it_stays_put() {
        let mut c = Comb::new(46.0, 100.0, 7);
        c.on[0] = false;
        c.hornet.p = [-50.0, -50.0];
        c.fire(40 * c.gw + 23);
        c.step(0.2);
        assert_eq!(flipping(&c), 1);
    }
}
