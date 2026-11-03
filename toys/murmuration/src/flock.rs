//! The flock: starlings on the rules measured in real murmurations, and one of them is you.
//!
//! Measured: each bird heeds its seven nearest neighbours, by count, not by distance (Ballerini et al.
//! 2008, PNAS 105:1232, "topological interaction"), so a flock thins and stretches without falling
//! apart; birds cruise at about 10 m/s and turn by banking (Hildenbrandt, Carere & Hemelrijk 2010,
//! Behav. Ecol. 21:1349, the StarDisplay model, whose rules these are, simplified); birds at the edge
//! of the flock are drawn in harder than birds inside it (StarDisplay's centrality); the flock wheels
//! above its roost. Drawn: the weights, a little randomness in every bird, and the size of the flock.

pub const V0: f32 = 10.0;
const NEIGHBOURS: usize = 7;
const SEP_R: f32 = 1.0;
const W_SEP: f32 = 22.0;
const W_ALIGN: f32 = 3.5;
const W_COH: f32 = 1.7;
const W_SPEED: f32 = 1.6;
const ROOST_R: f32 = 30.0;
const W_ROOST: f32 = 0.06;
const ALT: f32 = 32.0;
const W_ALT: f32 = 0.05;
const NOISE: f32 = 1.0;
const MAX_ACC: f32 = 30.0;
const CELL: f32 = 1.8;
/// How much more the neighbours of the bird that is you heed it than any other (a lean is felt).
const YOU_WEIGHT: f32 = 4.0;
/// How far round you a lean is felt, metres.
const LEAN_R: f32 = 45.0;
/// How far round a startled bird the startle takes, metres.
const STARTLE_R: f32 = 9.0;

pub type V3 = [f32; 3];

#[inline]
pub fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
#[inline]
pub fn addv(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
#[inline]
pub fn scale(a: V3, s: f32) -> V3 {
    [a[0] * s, a[1] * s, a[2] * s]
}
#[inline]
pub fn dot(a: V3, b: V3) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
#[inline]
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
#[inline]
pub fn length(a: V3) -> f32 {
    dot(a, a).sqrt()
}
#[inline]
pub fn norm(a: V3) -> V3 {
    let l = length(a);
    if l > 1e-6 {
        scale(a, 1.0 / l)
    } else {
        [0.0, 0.0, 1.0]
    }
}

/// A small, fast random source (xorshift), seeded so a check sees the same flock.
pub struct Rng(u64);
impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.max(1))
    }
    pub fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn signed(&mut self) -> f32 {
        self.next() * 2.0 - 1.0
    }
    pub fn gauss3(&mut self) -> V3 {
        let mut v = [0.0; 3];
        for c in &mut v {
            *c = (self.signed() + self.signed() + self.signed()) * 0.577;
        }
        v
    }
}

pub struct Bird {
    pub pos: V3,
    pub vel: V3,
    /// The lift's direction (the wings' normal): up, leaning into the turn (smoothed).
    pub lift: V3,
    /// The wingbeat's phase (radians), rate (Hz), and how much it beats (0 gliding, 1 flapping).
    pub phase: f32,
    pub rate: f32,
    pub flap: f32,
    flap_to: f32,
    flap_left: f32,
    wander: V3,
    pub size: f32,
}

pub struct Flock {
    pub birds: Vec<Bird>,
    /// The bird that is you.
    pub you: usize,
    /// Your lean: an acceleration asked of your bird (m/s², world), and how strongly (0..1) it
    /// overrides the rules.
    pub lean: V3,
    pub lean_k: f32,
    /// The direction your lean asks for (unit), which the birds round you feel too.
    pub lean_dir: V3,
    pub roost: V3,
    centre_now: V3,
    startle: Option<(V3, V3, f32)>,
    startle_in: f32,
    rng: Rng,
    acc: Vec<V3>,
    cells: Vec<u32>,
    starts: Vec<u32>,
    fill: Vec<u32>,
    keys: Vec<u32>,
    sorted: Vec<[f32; 4]>,
    nbs: Vec<([(f32, u32); NEIGHBOURS], usize)>,
    lo: V3,
    dims: [usize; 3],
    pub time: f64,
}

const GRID_MAX: usize = 96;

impl Flock {
    pub fn new(n: usize, seed: u64) -> Flock {
        let mut rng = Rng::new(seed);
        let roost = [0.0, ALT, 0.0];
        let mut birds = Vec::with_capacity(n);
        // A wheeling sheet, as a flock arrives: flattened, moving round the roost.
        let r = (n as f32).cbrt() * 1.2;
        for _ in 0..n {
            let p = [rng.signed() * r * 2.2, rng.signed() * r * 0.5, rng.signed() * r * 1.2];
            let pos = addv([ROOST_R * 0.7, ALT, 0.0], p);
            let vel = addv([0.0, 0.0, V0], scale(rng.gauss3(), 0.8));
            birds.push(Bird {
                pos,
                vel,
                lift: [0.0, 1.0, 0.0],
                phase: rng.next() * std::f32::consts::TAU,
                rate: 10.0 + rng.signed() * 1.5,
                flap: 1.0,
                flap_to: 1.0,
                flap_left: rng.next() * 1.5,
                wander: [0.0; 3],
                size: 1.3 + rng.signed() * 0.1,
            });
        }
        let you = n / 2;
        Flock {
            birds,
            you,
            lean: [0.0; 3],
            lean_k: 0.0,
            lean_dir: [0.0, 0.0, 1.0],
            roost,
            centre_now: roost,
            startle: None,
            startle_in: 3.0,
            rng,
            acc: vec![[0.0; 3]; n],
            cells: Vec::new(),
            starts: Vec::new(),
            fill: Vec::new(),
            keys: Vec::new(),
            sorted: Vec::new(),
            nbs: Vec::new(),
            lo: [0.0; 3],
            dims: [1; 3],
            time: 0.0,
        }
    }

    /// Sorts the birds into a dense grid of cells over the flock's box (birds far outside it are
    /// kept in its edge cells: still found, just checked against their real distances).
    fn build_grid(&mut self) {
        let n = self.birds.len();
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for b in &self.birds {
            for k in 0..3 {
                lo[k] = lo[k].min(b.pos[k]);
                hi[k] = hi[k].max(b.pos[k]);
            }
        }
        let mut dims = [1usize; 3];
        for k in 0..3 {
            dims[k] = (((hi[k] - lo[k]) / CELL).floor() as usize + 1).clamp(1, GRID_MAX);
        }
        self.lo = lo;
        self.dims = dims;
        let cells = dims[0] * dims[1] * dims[2];
        self.starts.clear();
        self.starts.resize(cells + 1, 0);
        self.keys.resize(n, 0);
        for (i, b) in self.birds.iter().enumerate() {
            let k = self.cell_index(self.cell(b.pos));
            self.keys[i] = k as u32;
            self.starts[k + 1] += 1;
        }
        for k in 0..cells {
            self.starts[k + 1] += self.starts[k];
        }
        self.cells.resize(n, 0);
        self.sorted.resize(n, [0.0; 4]);
        self.fill.clear();
        self.fill.extend_from_slice(&self.starts[..cells]);
        for i in 0..n {
            let k = self.keys[i] as usize;
            let at = self.fill[k] as usize;
            self.cells[at] = i as u32;
            let p = self.birds[i].pos;
            self.sorted[at] = [p[0], p[1], p[2], 0.0];
            self.fill[k] += 1;
        }
    }

    fn cell(&self, p: V3) -> [usize; 3] {
        let mut c = [0usize; 3];
        for k in 0..3 {
            c[k] = (((p[k] - self.lo[k]) / CELL).max(0.0) as usize).min(self.dims[k] - 1);
        }
        c
    }

    fn cell_index(&self, c: [usize; 3]) -> usize {
        (c[2] * self.dims[1] + c[1]) * self.dims[0] + c[0]
    }

    /// The seven nearest to bird `i` (their squared distances and indices), nearest first.
    fn nearest(&self, i: usize, out: &mut [(f32, u32); NEIGHBOURS]) -> usize {
        let p = self.birds[i].pos;
        let c = self.cell(p);
        for e in out.iter_mut() {
            *e = (f32::MAX, u32::MAX);
        }
        let mut found = 0usize;
        for ring in 1..=2usize {
            let r = ring as isize;
            for dz in -r..=r {
                let z = c[2] as isize + dz;
                if z < 0 || z >= self.dims[2] as isize {
                    continue;
                }
                for dy in -r..=r {
                    let y = c[1] as isize + dy;
                    if y < 0 || y >= self.dims[1] as isize {
                        continue;
                    }
                    for dx in -r..=r {
                        if ring == 2 && dx.abs().max(dy.abs()).max(dz.abs()) < 2 {
                            continue;
                        }
                        let x = c[0] as isize + dx;
                        if x < 0 || x >= self.dims[0] as isize {
                            continue;
                        }
                        let k = self.cell_index([x as usize, y as usize, z as usize]);
                        let (s0, s1) = (self.starts[k] as usize, self.starts[k + 1] as usize);
                        for s in s0..s1 {
                            let q = self.sorted[s];
                            let d = [q[0] - p[0], q[1] - p[1], q[2] - p[2]];
                            let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
                            if d2 < out[NEIGHBOURS - 1].0 {
                                let j = self.cells[s];
                                if j as usize == i {
                                    continue;
                                }
                                let mut at = NEIGHBOURS - 1;
                                while at > 0 && out[at - 1].0 > d2 {
                                    out[at] = out[at - 1];
                                    at -= 1;
                                }
                                out[at] = (d2, j);
                                found = (found + 1).min(NEIGHBOURS);
                            }
                        }
                    }
                }
            }
            // Done when seven are found nearer than any unsearched cell could be.
            let reach = ring as f32 * CELL;
            if found >= NEIGHBOURS && out[NEIGHBOURS - 1].0 <= reach * reach {
                return found;
            }
        }
        found
    }

    /// The flock's centre and its mean velocity.
    pub fn centre(&self) -> (V3, V3) {
        let mut c = [0.0; 3];
        let mut v = [0.0; 3];
        for b in &self.birds {
            c = addv(c, b.pos);
            v = addv(v, b.vel);
        }
        let k = 1.0 / self.birds.len().max(1) as f32;
        (scale(c, k), scale(v, k))
    }

    /// One step of `dt` seconds (keep it at a sixtieth or less).
    pub fn step(&mut self, dt: f32) {
        self.time += dt as f64;
        // The flock's pull wanders over the reed bed: a slow figure (StarDisplay's roost is fixed; a
        // real flock ranges over it, sweeping out and back).
        let tt = self.time as f32;
        self.roost = [45.0 * (tt * 0.047).sin(), ALT + 7.0 * (tt * 0.083).sin(), 35.0 * (tt * 0.031 + 1.0).sin()];
        self.build_grid();
        self.centre_now = self.centre().0;
        let n = self.birds.len();
        let you = self.you;
        // The neighbours, found on every core (the search only reads), then the rules in turn.
        let mut nbs = std::mem::take(&mut self.nbs);
        nbs.resize(n, ([(0.0, 0); NEIGHBOURS], 0));
        let threads = std::thread::available_parallelism().map(|t| t.get()).unwrap_or(1).clamp(1, 6);
        let chunk = n.div_ceil(threads);
        {
            let me = &*self;
            std::thread::scope(|sc| {
                for (c, part) in nbs.chunks_mut(chunk).enumerate() {
                    sc.spawn(move || {
                        for (k, e) in part.iter_mut().enumerate() {
                            e.1 = me.nearest(c * chunk + k, &mut e.0);
                        }
                    });
                }
            });
        }
        let mut lost: Vec<usize> = Vec::new();
        for i in 0..n {
            let (nb, found) = nbs[i];
            if found < NEIGHBOURS {
                lost.push(i);
            }
            self.acc[i] = self.rules(i, &nb[..found]);
        }
        self.nbs = nbs;
        // A bird with too few near it (one that slipped away, you) still heeds the seven nearest,
        // however far: found the slow way, for those few.
        for &i in lost.iter().take(24) {
            let p = self.birds[i].pos;
            let mut nb = [(f32::MAX, u32::MAX); NEIGHBOURS];
            for (j, b) in self.birds.iter().enumerate() {
                if j == i {
                    continue;
                }
                let d = sub(b.pos, p);
                let d2 = dot(d, d);
                if d2 < nb[NEIGHBOURS - 1].0 {
                    let mut at = NEIGHBOURS - 1;
                    while at > 0 && nb[at - 1].0 > d2 {
                        nb[at] = nb[at - 1];
                        at -= 1;
                    }
                    nb[at] = (d2, j as u32);
                }
            }
            self.acc[i] = self.rules(i, &nb);
        }
        // Agitation: now and then a patch of birds startles into a sharp banked turn (as at a
        // falcon's pass, or for no reason a watcher can see); alignment carries it through the
        // flock as a wave (Procaccini et al. 2011), and the flock folds.
        self.startle_in -= dt;
        if self.startle_in <= 0.0 {
            let i = (self.rng.next() * n as f32) as usize % n;
            let v = norm(self.birds[i].vel);
            let side = norm(cross([0.0, 1.0, 0.0], v));
            let sgn = if self.rng.next() < 0.5 { -1.0 } else { 1.0 };
            self.startle = Some((self.birds[i].pos, scale(addv(side, [0.0, self.rng.signed() * 0.4, 0.0]), sgn), 0.45));
            self.startle_in = 3.0 + self.rng.next() * 5.0;
        }
        if let Some((at, dir, left)) = self.startle {
            for (k, b) in self.birds.iter().enumerate() {
                // Your bird is yours: a startle near it moves the birds round you, not you.
                if k == self.you {
                    continue;
                }
                let d = length(sub(b.pos, at));
                if d < STARTLE_R {
                    let w = 1.0 - d / STARTLE_R;
                    self.acc[k] = addv(self.acc[k], scale(dir, 22.0 * w));
                }
            }
            let left = left - dt;
            self.startle = if left > 0.0 { Some((at, dir, left)) } else { None };
        }
        // Your lean: asked of your bird, over the rules as strongly as the finger leans; and felt by
        // the birds round you, most by the nearest, so the flock bends with you rather than letting
        // you go (they would follow you anyway, a few wingbeats later; this is that, sooner).
        let a = self.acc[you];
        self.acc[you] = addv(scale(a, 1.0 - 0.75 * self.lean_k), self.lean);
        if self.lean_k > 0.0 {
            let pyou = self.birds[you].pos;
            for i in 0..n {
                if i == you {
                    continue;
                }
                let b = &self.birds[i];
                let d = length(sub(b.pos, pyou));
                if d < LEAN_R {
                    let w = (1.0 - d / LEAN_R) * 0.5 * self.lean_k;
                    let want = sub(scale(self.lean_dir, length(b.vel).max(V0 * 0.8)), b.vel);
                    self.acc[i] = addv(scale(self.acc[i], 1.0 - 0.6 * w), scale(want, 2.6 * w));
                }
            }
        }
        for i in 0..n {
            let mut a = self.acc[i];
            let al = length(a);
            let cap = if i == you { MAX_ACC * 1.6 } else { MAX_ACC };
            if al > cap {
                a = scale(a, cap / al);
            }
            let b = &mut self.birds[i];
            b.vel = addv(b.vel, scale(a, dt));
            b.pos = addv(b.pos, scale(b.vel, dt));
            if b.pos[1] < 4.0 {
                b.pos[1] = 4.0;
                b.vel[1] = b.vel[1].abs();
            }
            // The lift leans into the turn: gravity's reaction plus the sideways pull.
            let f = norm(b.vel);
            let side = sub(a, scale(f, dot(a, f)));
            let want = norm(addv([0.0, 9.81, 0.0], side));
            let k = (dt * 8.0).min(1.0);
            b.lift = norm(addv(scale(b.lift, 1.0 - k), scale(want, k)));
            // Wingbeats: bouts of flapping and gliding; flapping harder when slow or climbing.
            b.flap_left -= dt;
            if b.flap_left <= 0.0 {
                let r = self.rng.next();
                b.flap_to = if r < 0.7 { 1.0 } else { 0.0 };
                b.flap_left = 0.3 + self.rng.next() * 1.4;
            }
            let speed = length(b.vel);
            let need = ((V0 - speed) * 0.5 + f[1] * 2.0).clamp(0.0, 1.0);
            let to = b.flap_to.max(need);
            b.flap += (to - b.flap) * (dt * 6.0).min(1.0);
            b.phase = (b.phase + dt * b.rate * std::f32::consts::TAU * (0.4 + 0.6 * b.flap)) % std::f32::consts::TAU;
        }
    }

    fn rules(&mut self, i: usize, nb: &[(f32, u32)]) -> V3 {
        let b = &self.birds[i];
        let p = b.pos;
        let v = b.vel;
        let mut sep = [0.0; 3];
        let mut avg_v = [0.0; 3];
        let mut centre = [0.0; 3];
        let mut dirs = [0.0; 3];
        let mut wsum = 0.0;
        for &(d2, j) in nb {
            let o = &self.birds[j as usize];
            let w = if j as usize == self.you { YOU_WEIGHT } else { 1.0 };
            let d = sub(o.pos, p);
            let dist = d2.sqrt().max(1e-3);
            if dist < SEP_R {
                sep = addv(sep, scale(d, -(SEP_R - dist) / (dist * SEP_R)));
            }
            avg_v = addv(avg_v, scale(o.vel, w));
            centre = addv(centre, scale(o.pos, w));
            dirs = addv(dirs, scale(d, 1.0 / dist));
            wsum += w;
        }
        let mut a = [0.0; 3];
        if wsum > 0.0 {
            avg_v = scale(avg_v, 1.0 / wsum);
            centre = scale(centre, 1.0 / wsum);
            // Centrality: neighbours all to one side means the flock's edge; pull in harder there.
            let edge = length(dirs) / nb.len() as f32;
            a = addv(a, scale(sep, W_SEP));
            a = addv(a, scale(sub(avg_v, v), W_ALIGN));
            a = addv(a, scale(sub(centre, p), W_COH * (0.2 + 2.6 * edge * edge)));
        }
        // Cruise speed.
        let speed = length(v).max(0.1);
        let f = scale(v, 1.0 / speed);
        a = addv(a, scale(f, (V0 - speed) * W_SPEED));
        // The roost: turn back toward it beyond its circle (horizontally), and keep the height.
        let to = addv(scale(sub(self.roost, self.centre_now), 0.6), scale(sub(self.roost, p), 0.4));
        let hd = (to[0] * to[0] + to[2] * to[2]).sqrt();
        if hd > ROOST_R {
            let k = W_ROOST * (hd - ROOST_R);
            a = addv(a, [to[0] / hd * k * V0 * 0.2, 0.0, to[2] / hd * k * V0 * 0.2]);
        }
        a[1] += W_ALT * (self.roost[1] - p[1]) * 3.0 - 0.5 * v[1];
        // A little randomness: a slowly wandering urge in every bird.
        let g = self.rng.gauss3();
        let b = &mut self.birds[i];
        b.wander = addv(scale(b.wander, 0.97), scale(g, 0.03 * 4.0));
        addv(a, scale(b.wander, NOISE))
    }
}

#[cfg(test)]
mod bench {
    #[test]
    fn step_cost() {
        let mut f = super::Flock::new(6000, 7);
        for _ in 0..300 {
            f.step(1.0 / 60.0);
        }
        let t = std::time::Instant::now();
        for _ in 0..60 {
            f.step(1.0 / 60.0);
        }
        let mut found = [0usize; 8];
        for i in 0..f.birds.len() {
            let mut nb = [(0.0f32, 0u32); super::NEIGHBOURS];
            found[f.nearest(i, &mut nb)] += 1;
        }
        let (c, _) = f.centre();
        let mut ext = [0.0f32; 3];
        for b in &f.birds { for k in 0..3 { ext[k] = ext[k].max((b.pos[k] - c[k]).abs()); } }
        println!("step {:.2} ms, found {:?}, extent {:?}", t.elapsed().as_secs_f64() * 1000.0 / 60.0, found, ext);
    }
}
