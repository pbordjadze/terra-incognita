//! The stand: a patch of the Hoh's valley floor, some 70 m across and 80 m deep, and its trees, logs
//! and seedlings, run a year at a time (fractions of a year in a frame). The rules, each a line:
//!
//! - Seed rains on the whole floor every year (Sitka spruce and western hemlock), but a seedling on the
//!   moss carpet is smothered within a year or two; one on a fallen log, raised out of the moss, lives
//!   (`FLOOR`, the first switch).
//! - Young trees grow in light: under the closed canopy hardly at all, fast in the gap a fallen giant
//!   left. A tree overtopped by a taller neighbour close by is starved of light and, after some years
//!   of it, dies (`SHADE`, the second switch), so a log's crowded line of seedlings thins to a few.
//! - A log rots, slowly, over a couple of centuries. A tree that rooted on it keeps its feet where the
//!   log's top was: its roots grew down round the log to the ground, and when the log is gone they stand
//!   as stilts, in a row as straight as the log was.
//! - Now and then a giant falls (or a finger fells one) and becomes the next log.
//!
//! Places are metres: x across the view (right positive), z away from the eye.

use std::f32::consts::FRAC_PI_2;

/// The rules as switches (toy-kit switches.rs), in the order of `Forest::on`.
pub const RULES: [&str; 2] = ["Moss smothers", "Shade thins"];
pub const FLOOR: usize = 0;
pub const SHADE: usize = 1;

/// The stand: x in -X..X, z in Z0..Z1 (m).
pub const X: f32 = 38.0;
pub const Z0: f32 = 1.5;
pub const Z1: f32 = 90.0;
/// The tallest a spruce here grows (m).
const HMAX: f32 = 74.0;
/// A young tree's growth in full light (m a year).
const GROW: f32 = 0.55;
/// A log's life (years) until it is gone into the floor; it starts to sink after `FIRM`.
pub const LOG_LIFE: f32 = 260.0;
const FIRM: f32 = 50.0;
/// Seeds that land on the stand a year.
const SEEDS: f32 = 2600.0;
/// A giant's chance of falling by itself, a year (slowed from the real 1/150 or so: see SOURCES.md).
const FALLS: f32 = 1.0 / 600.0;
/// Seconds a falling tree takes to come down (real time, whatever the years run at).
pub const FALL_S: f32 = 2.6;
/// The light grid's cell and the neighbour grid's cell (m).
const CELL: f32 = 2.0;
const NEAR_CELL: f32 = 4.0;
const MAX_TREES: usize = 1600;
/// The trail the eye stands on: nothing takes root nearer than this (m).
pub const TRAIL: f32 = 8.0;

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed ^ 0x9e37_79b9_7f4a_7c15)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
    /// 0..1
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
}

#[derive(Clone)]
pub struct Tree {
    pub x: f32,
    pub z: f32,
    /// How high its feet stand (m): the top of the log it rooted on, 0 on the floor.
    pub base: f32,
    pub h: f32,
    pub seed: u32,
    /// The log it rooted on, while that log lasts.
    pub log: Option<u32>,
    /// Years spent overtopped, running.
    shaded: f32,
    /// A seedling on the moss: the years it has left (none when it will live).
    doomed: Option<f32>,
    /// 1 alive; falls to 0 as it dies, and it is gone.
    pub fade: f32,
    dying: bool,
}

impl Tree {
    /// The trunk's diameter at the foot (m).
    pub fn d(&self) -> f32 {
        0.02 + self.h * 0.03
    }
    pub fn dying(&self) -> bool {
        self.dying
    }
    /// Its own vigour (0.6..1.4) and the height it tops out at (m): no two trees grow alike.
    fn vigour(&self) -> f32 {
        0.6 + 0.8 * (self.seed & 0xffff) as f32 / 65535.0
    }
    fn top(&self) -> f32 {
        HMAX * (0.72 + 0.28 * (self.seed >> 16) as f32 / 65535.0)
    }
}

pub struct Log {
    pub id: u32,
    /// Its foot (the root wad) and its far end, on the floor (m).
    pub a: [f32; 2],
    pub b: [f32; 2],
    /// Its radius when it fell (m), its age (years), its height when it stood (m).
    pub r0: f32,
    pub age: f32,
    pub seed: u32,
}

impl Log {
    /// Its radius now: firm for decades, then sinking into the floor.
    pub fn r(&self) -> f32 {
        let k = ((self.age - FIRM) / (LOG_LIFE - FIRM)).clamp(0.0, 1.0);
        self.r0 * (1.0 - k * k * (3.0 - 2.0 * k)).max(0.0)
    }
    /// How far into its rotting it is (0 fresh, 1 gone).
    pub fn rot(&self) -> f32 {
        (self.age / LOG_LIFE).clamp(0.0, 1.0)
    }
    /// How much of it the moss has covered.
    pub fn moss(&self) -> f32 {
        smooth((self.age - 2.0) / 35.0)
    }
    pub fn dir(&self) -> [f32; 2] {
        let d = [self.b[0] - self.a[0], self.b[1] - self.a[1]];
        let l = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-4);
        [d[0] / l, d[1] / l]
    }
    /// The distance from `p` to its axis on the floor, and how far along it (0..1).
    pub fn dist(&self, p: [f32; 2]) -> (f32, f32) {
        let ab = [self.b[0] - self.a[0], self.b[1] - self.a[1]];
        let ap = [p[0] - self.a[0], p[1] - self.a[1]];
        let l2 = (ab[0] * ab[0] + ab[1] * ab[1]).max(1e-6);
        let t = ((ap[0] * ab[0] + ap[1] * ab[1]) / l2).clamp(0.0, 1.0);
        let q = [self.a[0] + ab[0] * t - p[0], self.a[1] + ab[1] * t - p[1]];
        ((q[0] * q[0] + q[1] * q[1]).sqrt(), t)
    }
    /// Whether a seedling can take on it: once the moss has come, until it is too far gone.
    fn seedbed(&self) -> f32 {
        smooth((self.age - 1.0) / 8.0) * (1.0 - smooth((self.age - 140.0) / 60.0))
    }
}

/// A tree coming down: real seconds since it went, and the way it falls.
pub struct Fall {
    pub tree: Tree,
    pub dir: [f32; 2],
    pub t: f32,
}

impl Fall {
    /// How far over it is (radians from upright): slow to start, then all at once.
    pub fn angle(&self) -> f32 {
        let u = (self.t / FALL_S).min(1.0);
        u * u * u * FRAC_PI_2
    }
}

pub fn smooth(x: f32) -> f32 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

pub struct Forest {
    pub trees: Vec<Tree>,
    pub logs: Vec<Log>,
    pub falls: Vec<Fall>,
    pub on: [bool; 2],
    pub year: f64,
    pub rng: Rng,
    next_id: u32,
    /// Light reaching the floor, a cell each (0 dark .. 1 open sky).
    light: Vec<f32>,
    gw: usize,
    gh: usize,
    near: Vec<Vec<u32>>,
    nw: usize,
    nh: usize,
    seeds_owed: f32,
    /// Falls that landed since the sound last asked (and how big the biggest was, m).
    pub landed: Vec<f32>,
    /// Whether giants fall by themselves (off in the history the opening is grown from, but for
    /// those the history asks for).
    pub natural: bool,
    /// Where giants may fall by themselves: only where the eye sees (a z-limit and the view's slope).
    pub seen: (f32, f32),
    /// Giants that have started down (a running count, for the sound).
    pub started: u32,
}

impl Forest {
    pub fn new(seed: u64) -> Forest {
        let gw = (2.0 * X / CELL).ceil() as usize;
        let gh = ((Z1 - Z0) / CELL).ceil() as usize;
        let nw = (2.0 * X / NEAR_CELL).ceil() as usize;
        let nh = ((Z1 - Z0) / NEAR_CELL).ceil() as usize;
        Forest {
            trees: Vec::new(),
            logs: Vec::new(),
            falls: Vec::new(),
            on: [true; 2],
            year: 0.0,
            rng: Rng::new(seed),
            next_id: 1,
            light: vec![1.0; gw * gh],
            gw,
            gh,
            near: vec![Vec::new(); nw * nh],
            nw,
            nh,
            seeds_owed: 0.0,
            landed: Vec::new(),
            natural: true,
            seen: (45.0, 0.5),
            started: 0,
        }
    }

    pub fn plant(&mut self, x: f32, z: f32, h: f32, base: f32) {
        let seed = self.rng.next_u64() as u32;
        self.trees.push(Tree {
            x,
            z,
            base,
            h,
            seed,
            log: None,
            shaded: 0.0,
            doomed: None,
            fade: 1.0,
            dying: false,
        });
    }

    /// The light on the floor at `p` (m).
    pub fn light_at(&self, p: [f32; 2]) -> f32 {
        let gx = ((p[0] + X) / CELL).floor();
        let gz = ((p[1] - Z0) / CELL).floor();
        if gx < 0.0 || gz < 0.0 || gx as usize >= self.gw || gz as usize >= self.gh {
            return 0.3;
        }
        self.light[gz as usize * self.gw + gx as usize]
    }

    /// The light grid's cells: their middle (m) and light, for the painter.
    pub fn cells(&self) -> impl Iterator<Item = ([f32; 2], f32)> + '_ {
        (0..self.gh).flat_map(move |j| {
            (0..self.gw).map(move |i| {
                (
                    [-X + (i as f32 + 0.5) * CELL, Z0 + (j as f32 + 0.5) * CELL],
                    self.light[j * self.gw + i],
                )
            })
        })
    }

    pub const CELL: f32 = CELL;

    /// The light grid's size (cells across, cells deep).
    pub fn grid(&self) -> [usize; 2] {
        [self.gw, self.gh]
    }

    /// The canopy: each tall crown shades the floor round it; what's left is the light.
    fn shine(&mut self) {
        let mut cover = vec![0.0f32; self.gw * self.gh];
        for t in &self.trees {
            if t.h < 18.0 || t.dying {
                continue;
            }
            let w = 0.85 * ((t.h - 18.0) / 30.0).min(1.0);
            let r = 1.5 + t.h * 0.05;
            let reach = (r * 1.6 / CELL).ceil() as i32;
            let ci = ((t.x + X) / CELL) as i32;
            let cj = ((t.z - Z0) / CELL) as i32;
            for j in (cj - reach).max(0)..=(cj + reach).min(self.gh as i32 - 1) {
                for i in (ci - reach).max(0)..=(ci + reach).min(self.gw as i32 - 1) {
                    let cx = -X + (i as f32 + 0.5) * CELL;
                    let cz = Z0 + (j as f32 + 0.5) * CELL;
                    let d2 = ((cx - t.x).powi(2) + (cz - t.z).powi(2)) / (r * r);
                    cover[j as usize * self.gw + i as usize] += w * (-d2).exp();
                }
            }
        }
        for (l, c) in self.light.iter_mut().zip(cover) {
            *l = (1.05 - 1.1 * c).clamp(0.06, 1.0);
        }
    }

    fn bucket(&self, x: f32, z: f32) -> Option<usize> {
        let i = ((x + X) / NEAR_CELL).floor();
        let j = ((z - Z0) / NEAR_CELL).floor();
        if i < 0.0 || j < 0.0 || i as usize >= self.nw || j as usize >= self.nh {
            return None;
        }
        Some(j as usize * self.nw + i as usize)
    }

    /// Whether a taller tree close by overtops tree `k`.
    fn overtopped(&self, k: usize) -> bool {
        let t = &self.trees[k];
        let i0 = ((t.x + X) / NEAR_CELL).floor() as i32;
        let j0 = ((t.z - Z0) / NEAR_CELL).floor() as i32;
        for j in j0 - 2..=j0 + 2 {
            for i in i0 - 2..=i0 + 2 {
                if i < 0 || j < 0 || i >= self.nw as i32 || j >= self.nh as i32 {
                    continue;
                }
                for &o in &self.near[j as usize * self.nw + i as usize] {
                    let o = &self.trees[o as usize];
                    if o.dying || o.h < t.h * 1.15 + 0.2 {
                        continue;
                    }
                    let reach = 0.3 + 0.075 * o.h.min(40.0);
                    let d2 = (o.x - t.x).powi(2) + (o.z - t.z).powi(2);
                    if d2 < reach * reach {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Runs `years` (any amount; split into steps of at most a year).
    pub fn run(&mut self, years: f32) {
        let mut left = years;
        while left > 1e-5 {
            let dy = left.min(1.0);
            self.step(dy);
            left -= dy;
        }
    }

    fn step(&mut self, dy: f32) {
        self.year += dy as f64;
        self.shine();
        for b in &mut self.near {
            b.clear();
        }
        for k in 0..self.trees.len() {
            let (x, z) = (self.trees[k].x, self.trees[k].z);
            if let Some(b) = self.bucket(x, z) {
                self.near[b].push(k as u32);
            }
        }
        self.grow(dy);
        self.seed_rain(dy);
        for l in &mut self.logs {
            l.age += dy;
        }
        let gone: Vec<u32> = self
            .logs
            .iter()
            .filter(|l| l.age >= LOG_LIFE)
            .map(|l| l.id)
            .collect();
        if !gone.is_empty() {
            self.logs.retain(|l| l.age < LOG_LIFE);
            for t in &mut self.trees {
                if t.log.is_some_and(|i| gone.contains(&i)) {
                    t.log = None;
                }
            }
        }
        self.trees.retain(|t| t.fade > 0.0);
        if self.natural {
            self.giants_fall(dy);
        }
    }

    fn grow(&mut self, dy: f32) {
        for k in 0..self.trees.len() {
            let over = self.trees[k].h < 50.0 && self.overtopped(k);
            let floor_light = self.light_at([self.trees[k].x, self.trees[k].z]);
            let shade_on = self.on[SHADE];
            let roll = self.rng.f();
            let t = &mut self.trees[k];
            if t.dying {
                t.fade -= dy / 2.0;
                continue;
            }
            // a tall tree's crown is up in the light whatever the floor gets
            let light = floor_light + (1.0 - floor_light) * smooth((t.h - 22.0) / 22.0);
            let young = 0.3 + 0.7 * smooth(t.h / 4.0);
            let mut g = GROW * t.vigour() * light * young * (1.0 - t.h / t.top()).max(0.0);
            if over && shade_on {
                g *= 0.15;
                t.shaded += dy;
                if t.shaded > 5.0 && roll < 0.3 * dy {
                    t.dying = true;
                }
            } else {
                t.shaded = (t.shaded - dy).max(0.0);
            }
            t.h += g * dy;
            if let Some(left) = t.doomed.as_mut() {
                *left -= dy;
                if *left <= 0.0 {
                    t.dying = true;
                }
            }
        }
    }

    fn seed_rain(&mut self, dy: f32) {
        self.seeds_owed += SEEDS * dy;
        while self.seeds_owed >= 1.0 {
            self.seeds_owed -= 1.0;
            let x = self.rng.range(-X, X);
            let z = self.rng.range(Z0 + 0.5, Z1 * 0.75);
            let roll = self.rng.f();
            // where the eye stands is the trail, trodden bare of seedlings
            if self.trees.len() >= MAX_TREES || z < TRAIL {
                continue;
            }
            let mut on_log = None;
            for l in &self.logs {
                let (d, _) = l.dist([x, z]);
                let r = l.r();
                if d < r * 0.7 && roll < 0.55 * l.seedbed() {
                    on_log = Some((l.id, r * 2.0 * (1.0 - 0.15 * (d / r).powi(2))));
                    break;
                }
            }
            if let Some((id, top)) = on_log {
                self.plant(x, z, 0.04, top);
                let t = self.trees.last_mut().expect("just planted");
                t.log = Some(id);
                continue;
            }
            // on the moss: a few show, and (while the moss smothers) are gone within a year or two
            if self.on[FLOOR] {
                if roll < 0.012 {
                    let left = self.rng.range(0.4, 1.8);
                    self.plant(x, z, 0.04, 0.0);
                    self.trees.last_mut().expect("just planted").doomed = Some(left);
                }
            } else if roll < 0.04 {
                self.plant(x, z, 0.04, 0.0);
            }
        }
    }

    fn giants_fall(&mut self, dy: f32) {
        let (zmax, slope) = self.seen;
        // one at a time, and not while the floor is already strewn
        if !self.falls.is_empty() || self.logs.len() >= 6 {
            return;
        }
        for k in 0..self.trees.len() {
            let t = &self.trees[k];
            if t.h < 50.0 || t.dying || t.z > zmax || t.x.abs() > t.z * slope {
                continue;
            }
            if self.rng.f() < FALLS * dy {
                let dir = self.way(k, None);
                self.fell(k, dir);
                return;
            }
        }
    }

    /// A way for tree `k` to fall: across the view, a little toward or away (always away when it is
    /// near the eye), to the left or right of `side` (a sign) if given.
    pub fn way(&mut self, k: usize, side: Option<f32>) -> [f32; 2] {
        let t = &self.trees[k];
        let s = side.unwrap_or(if self.rng.f() < 0.5 { -1.0 } else { 1.0 });
        let away = if t.z < 14.0 || self.rng.f() < 0.7 {
            1.0
        } else {
            -1.0
        };
        let a = self.rng.range(0.35, 1.0);
        [s * a.cos(), away * a.sin()]
    }

    /// Sends tree `k` down toward `dir`.
    pub fn fell(&mut self, k: usize, dir: [f32; 2]) {
        let tree = self.trees.swap_remove(k);
        self.falls.push(Fall { tree, dir, t: 0.0 });
        self.started += 1;
    }

    /// The falls in real time: each that reaches the floor becomes a log and crushes what's under it.
    pub fn tick(&mut self, dt: f32) {
        for f in &mut self.falls {
            f.t += dt;
        }
        let mut i = 0;
        while i < self.falls.len() {
            if self.falls[i].t < FALL_S {
                i += 1;
                continue;
            }
            let f = self.falls.swap_remove(i);
            self.land(&f.tree, f.dir);
        }
    }

    fn land(&mut self, t: &Tree, dir: [f32; 2]) {
        let len = t.h * 0.78;
        let a = [t.x + dir[0] * 0.4, t.z + dir[1] * 0.4];
        let b = [a[0] + dir[0] * len, a[1] + dir[1] * len];
        let id = self.next_id;
        self.next_id += 1;
        let log = Log {
            id,
            a,
            b,
            r0: (t.d() * 0.5).clamp(0.3, 1.25),
            age: 0.0,
            seed: t.seed,
        };
        let reach = log.r0 + 0.6;
        for o in &mut self.trees {
            if o.h < 22.0 && log.dist([o.x, o.z]).0 < reach {
                o.dying = true;
                o.fade = o.fade.min(0.6);
            }
        }
        self.logs.push(log);
        self.landed.push(t.h);
    }

    /// A tree falls at once, as if long ago (for the history the opening grows from).
    pub fn felled_long_ago(&mut self, k: usize, dir: [f32; 2]) {
        let t = self.trees.swap_remove(k);
        self.land(&t, dir);
        self.landed.clear();
    }

    /// The trees whose life the switches change: on the moss, every seedling's fate.
    pub fn set(&mut self, i: usize, on: bool) {
        self.on[i] = on;
        if i == FLOOR {
            for k in 0..self.trees.len() {
                let left = self.rng.range(0.3, 1.5);
                let t = &mut self.trees[k];
                if t.base > 0.0 || t.dying {
                    continue;
                }
                if on && t.h < 3.0 && t.doomed.is_none() {
                    t.doomed = Some(left);
                } else if !on {
                    t.doomed = None;
                }
            }
        }
    }
}
