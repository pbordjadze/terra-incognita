//! The huddle: emperor penguins packed on the sea ice in a winter wind. Metres, seen from above; the
//! wind blows toward `wind` (a unit vector).
//!
//! A penguin with no one upwind of it is in the wind and gets cold; one inside is sheltered and warms.
//! A cold penguin on the windward edge leaves, walks round the outside, and presses in again on the
//! lee side. Packed penguins lean in toward the middle and cannot pass through each other. Now and
//! then one takes a small step forward, and its neighbours step after it: a wave runs through the
//! huddle. That is all; the huddle's turning over and its creeping downwind are those rules.

use std::collections::HashMap;

/// How close packed penguins stand, centre to centre (about 21 to a square metre).
pub const SPACING: f32 = 0.235;
/// A wave's step (5–10 cm) and how fast it passes from penguin to penguin (about 12 cm/s).
const STEP: f32 = 0.075;
const WAVE_SPEED: f32 = 0.12;
/// How long a step takes, and how long before a penguin will step again (seconds).
const STEP_S: f32 = 1.4;
const STEP_REST: f32 = 9.0;
/// Walking speed round the outside, metres a second.
const WALK: f32 = 0.42;

#[derive(Clone, Copy, PartialEq)]
pub enum State {
    Packed,
    /// walking round the outside: which way round (+1 anticlockwise), and the bearing it walks to
    /// (radians from the downwind direction)
    Walking(f32, f32),
    /// pressing back in on the lee side
    Joining,
}

#[derive(Clone, Copy)]
pub struct Penguin {
    pub p: [f32; 2],
    /// which way it faces (unit)
    pub face: [f32; 2],
    pub state: State,
    /// how cold (0 warm .. 1 must move), and the frost on its back (0..1)
    pub cold: f32,
    pub frost: f32,
    /// how much wind it takes (0 sheltered .. 1 in the full wind)
    pub exposed: f32,
    /// how squarely it faces the wind from the windward edge (0 not at all .. 1 first in line)
    pub facing: f32,
    /// a step: seconds since it began (`STEP_S` and over: standing), and seconds until it may step
    /// again; a step heard from a neighbour, not yet taken
    pub step: f32,
    pub rest: f32,
    pub pending: Option<f32>,
    /// the waddle's phase, and how much the penguin is moving (for the drawing)
    pub gait: f32,
    pub moving: f32,
}

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }
    pub fn unit(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.unit()
    }
}

pub struct Huddle {
    pub birds: Vec<Penguin>,
    /// the wind: where it blows to, and its strength (metres a second)
    pub wind: [f32; 2],
    pub gust: f32,
    pub t: f32,
    rng: Rng,
    next_wave: f32,
    cells: HashMap<(i32, i32), Vec<u32>>,
    /// penguins that stepped, joined, or set off this step (for the sound)
    pub stepped: u32,
    pub joined: Vec<[f32; 2]>,
}

fn sub(a: [f32; 2], b: [f32; 2]) -> [f32; 2] {
    [a[0] - b[0], a[1] - b[1]]
}
fn len(a: [f32; 2]) -> f32 {
    (a[0] * a[0] + a[1] * a[1]).sqrt()
}
fn dot(a: [f32; 2], b: [f32; 2]) -> f32 {
    a[0] * b[0] + a[1] * b[1]
}
fn norm(a: [f32; 2]) -> [f32; 2] {
    let l = len(a).max(1e-6);
    [a[0] / l, a[1] / l]
}
fn turn(a: [f32; 2], b: [f32; 2], k: f32) -> [f32; 2] {
    norm([a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k])
}

const CELL: f32 = 0.3;

fn cell(p: [f32; 2]) -> (i32, i32) {
    ((p[0] / CELL).floor() as i32, (p[1] / CELL).floor() as i32)
}

impl Huddle {
    /// `n` penguins packed in a disc round the origin, the wind blowing toward `wind`.
    pub fn new(n: usize, wind: [f32; 2], seed: u64) -> Huddle {
        let mut rng = Rng::new(seed);
        let mut birds = Vec::with_capacity(n);
        // a hexagonal packing, filled outward from the middle
        let mut spots = Vec::new();
        let k = 40;
        for j in -k..=k {
            for i in -k..=k {
                let x = (i as f32 + 0.5 * (j & 1) as f32) * SPACING;
                let y = j as f32 * SPACING * 0.866;
                // a ragged, slightly long disc, not a crystal
                let a = x.atan2(y);
                let k = 1.0
                    + 0.12 * (a * 2.0 + 0.7).sin()
                    + 0.08 * (a * 5.0 + 2.1).sin()
                    + 0.10 * rng.unit();
                spots.push([x * k * 1.15, y * k * 0.9]);
            }
        }
        // the nearest to the middle, each given a little luck, so the edge is ragged, not a hexagon
        let mut spots: Vec<(f32, [f32; 2])> = spots
            .into_iter()
            .map(|s| (len(s) + rng.range(0.0, 0.5), s))
            .collect();
        spots.sort_by(|a, b| a.0.total_cmp(&b.0));
        let spots = spots.into_iter().map(|(_, s)| s);
        let face = norm(wind);
        for s in spots.take(n) {
            birds.push(Penguin {
                p: [s[0] + rng.range(-0.09, 0.09), s[1] + rng.range(-0.09, 0.09)],
                face: turn(face, [rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)], 0.5),
                state: State::Packed,
                cold: rng.range(0.0, 0.6),
                frost: 0.0,
                exposed: 0.0,
                facing: 0.0,
                step: STEP_S,
                rest: 0.0,
                pending: None,
                gait: rng.range(0.0, 6.0),
                moving: 0.0,
            });
        }
        Huddle {
            birds,
            wind: norm(wind),
            gust: 6.0,
            t: 0.0,
            rng,
            next_wave: 4.0,
            cells: HashMap::new(),
            stepped: 0,
            joined: Vec::new(),
        }
    }

    /// The middle of the packed penguins, and the huddle's radius.
    pub fn middle(&self) -> ([f32; 2], f32) {
        let (mut c, mut n) = ([0.0f32; 2], 0.0f32);
        for b in &self.birds {
            if b.state == State::Packed {
                c[0] += b.p[0];
                c[1] += b.p[1];
                n += 1.0;
            }
        }
        let c = if n > 0.0 {
            [c[0] / n, c[1] / n]
        } else {
            [0.0; 2]
        };
        // the area the packed ones cover, as a disc
        let r = (n * SPACING * SPACING * 0.866 / std::f32::consts::PI).sqrt();
        (c, r)
    }

    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        self.stepped = 0;
        self.joined.clear();
        self.index();
        self.shelter();
        let (c, r) = self.middle();
        self.waves(dt);
        for i in 0..self.birds.len() {
            self.warm(i, dt);
            self.move_one(i, c, r, dt);
        }
        self.index();
        self.press();
    }

    fn index(&mut self) {
        for v in self.cells.values_mut() {
            v.clear();
        }
        for (i, b) in self.birds.iter().enumerate() {
            self.cells.entry(cell(b.p)).or_default().push(i as u32);
        }
    }

    fn near(&self, p: [f32; 2], r: f32, out: &mut Vec<u32>) {
        out.clear();
        let c = cell(p);
        let k = (r / CELL).ceil() as i32;
        for dx in -k..=k {
            for dy in -k..=k {
                if let Some(v) = self.cells.get(&(c.0 + dx, c.1 + dy)) {
                    out.extend(
                        v.iter()
                            .copied()
                            .filter(|&j| len(sub(self.birds[j as usize].p, p)) < r),
                    );
                }
            }
        }
    }

    /// Who is in the wind: across the wind the huddle is cut in strips a penguin wide; the first
    /// penguin met in each strip, coming from upwind, takes the wind, the second a little of it.
    fn shelter(&mut self) {
        let w = self.wind;
        let side = [-w[1], w[0]];
        let mut strips: HashMap<i32, Vec<(f32, usize)>> = HashMap::new();
        for (i, b) in self.birds.iter().enumerate() {
            let s = (dot(b.p, side) / (SPACING * 0.9)).floor() as i32;
            strips.entry(s).or_default().push((dot(b.p, w), i));
        }
        for b in &mut self.birds {
            b.exposed = 0.0;
            b.facing = 0.0;
        }
        for (_, mut v) in strips {
            v.sort_by(|a, b| a.0.total_cmp(&b.0));
            for (rank, &(_, i)) in v.iter().take(3).enumerate() {
                self.birds[i].exposed = [1.0, 0.35, 0.1][rank];
                self.birds[i].facing = [1.0, 0.25, 0.0][rank];
            }
        }
        // and anyone on the edge, wherever it faces, feels some of it
        let mut near = Vec::new();
        for i in 0..self.birds.len() {
            self.near(self.birds[i].p, SPACING * 1.3, &mut near);
            let open = (1.0 - (near.len() as f32 - 1.0) / 6.0).clamp(0.0, 1.0);
            let b = &mut self.birds[i];
            b.exposed = b.exposed.max(open * 0.45);
        }
    }

    fn warm(&mut self, i: usize, dt: f32) {
        let wind = (self.gust / 6.0).clamp(0.3, 2.0);
        let b = &mut self.birds[i];
        let e = if b.state == State::Packed {
            b.exposed
        } else {
            1.0
        };
        b.cold = (b.cold + dt * (e * wind / 22.0 - (1.0 - e) / 50.0)).clamp(0.0, 1.4);
        // the frost is the wind's own mark: it builds only on a back facing into it, fast, and
        // melts in the warmth inside; a walker keeps what it has a while
        let front = if b.state == State::Packed {
            b.facing
        } else {
            0.0
        };
        let melt = if b.state == State::Packed {
            (1.0 - front) * 0.07
        } else {
            0.01
        };
        b.frost = (b.frost + dt * (front * wind * 0.16 - melt)).clamp(0.0, 1.0);
    }
}

impl Huddle {
    /// The wind turns to blow toward `to`, at `speed` metres a second.
    pub fn blow(&mut self, to: [f32; 2], speed: f32) {
        self.wind = norm(to);
        self.gust = speed.clamp(2.0, 14.0);
    }

    /// Penguin `i` steps forward, and its neighbours will after it.
    pub fn nudge(&mut self, i: usize) {
        if self.birds[i].state == State::Packed && self.birds[i].rest <= 0.0 {
            self.birds[i].pending = Some(0.0);
        }
    }

    /// The nearest penguin to `p` within `r` metres.
    pub fn at(&self, p: [f32; 2], r: f32) -> Option<usize> {
        let mut best = (r, None);
        for (i, b) in self.birds.iter().enumerate() {
            let d = len(sub(b.p, p));
            if d < best.0 {
                best = (d, Some(i));
            }
        }
        best.1
    }

    /// The waves of small steps: one starts on its own every half minute to a minute, at the
    /// huddle's back; a step heard comes due and is passed on.
    fn waves(&mut self, dt: f32) {
        if self.t > self.next_wave {
            self.next_wave = self.t + self.rng.range(30.0, 60.0);
            let w = self.wind;
            let back = (0..self.birds.len())
                .filter(|&i| self.birds[i].state == State::Packed)
                .min_by(|&a, &b| dot(self.birds[a].p, w).total_cmp(&dot(self.birds[b].p, w)));
            if let Some(i) = back {
                self.nudge(i);
            }
        }
        let mut near = Vec::new();
        for i in 0..self.birds.len() {
            let b = &mut self.birds[i];
            b.rest = (b.rest - dt).max(0.0);
            let Some(wait) = b.pending else {
                continue;
            };
            if wait > dt {
                b.pending = Some(wait - dt);
                continue;
            }
            b.pending = None;
            if b.rest > 0.0 || b.state != State::Packed {
                continue;
            }
            b.step = 0.0;
            b.rest = STEP_REST;
            self.stepped += 1;
            let p = b.p;
            self.near(p, SPACING * 1.45, &mut near);
            for &j in &near {
                let n = &mut self.birds[j as usize];
                if n.rest <= 0.0 && n.pending.is_none() && n.state == State::Packed {
                    let d = len(sub(n.p, p));
                    n.pending = Some(d / WAVE_SPEED * self.rng.range(0.85, 1.15));
                }
            }
        }
    }

    fn move_one(&mut self, i: usize, c: [f32; 2], r: f32, dt: f32) {
        let w = self.wind;
        let mut b = self.birds[i];
        let before = b.p;
        match b.state {
            State::Packed => {
                // leaning in toward the middle, harder the farther out, and shuffling a little
                let to = sub(c, b.p);
                let d = len(to);
                if d > 0.05 {
                    let k = 0.07 * dt * (d / r.max(0.5)).powi(3).min(3.0);
                    b.p[0] += to[0] / d * k;
                    b.p[1] += to[1] / d * k;
                }
                let j = 0.012 * dt.sqrt();
                b.p[0] += self.rng.range(-j, j);
                b.p[1] += self.rng.range(-j, j);
                // a step: forward, eased
                if b.step < STEP_S {
                    let e0 = ease(b.step / STEP_S);
                    b.step += dt;
                    let e1 = ease((b.step / STEP_S).min(1.0));
                    b.p[0] += b.face[0] * STEP * (e1 - e0);
                    b.p[1] += b.face[1] * STEP * (e1 - e0);
                }
                // the backs to the wind
                b.face = turn(b.face, w, dt * 0.05);
                // cold on the windward edge: off it goes, round the nearer side
                if b.cold > 1.0 && b.exposed > 0.5 {
                    let side = [-w[1], w[0]];
                    // on the left of the wind it goes clockwise to the lee, on the right anticlockwise,
                    // and stops on its own side of the lee
                    let way = if dot(sub(b.p, c), side) >= 0.0 {
                        -1.0
                    } else {
                        1.0
                    };
                    let aim = -way * self.rng.range(0.0, 1.3);
                    b.state = State::Walking(way, aim);
                    b.pending = None;
                }
            }
            State::Walking(way, aim) => {
                // round the outside, a little clear of the packed edge
                let rel = sub(b.p, c);
                let a = rel[1].atan2(rel[0]);
                let lee = w[1].atan2(w[0]) + aim;
                let left = wrap(lee - a);
                let rr = r + SPACING * 1.4;
                let radial = (rr - len(rel)) * 1.5;
                let out = norm(rel);
                let round = [-out[1] * way, out[0] * way];
                let v = [
                    round[0] * WALK + out[0] * radial,
                    round[1] * WALK + out[1] * radial,
                ];
                b.p[0] += v[0] * dt;
                b.p[1] += v[1] * dt;
                b.face = turn(b.face, norm(v), (dt * 4.0).min(1.0));
                if left * way < 0.12 {
                    b.state = State::Joining;
                }
            }
            State::Joining => {
                let to = norm(sub(c, b.p));
                b.p[0] += to[0] * WALK * 0.6 * dt;
                b.p[1] += to[1] * WALK * 0.6 * dt;
                b.face = turn(b.face, to, (dt * 3.0).min(1.0));
            }
        }
        let moved = len(sub(b.p, before)) / dt.max(1e-4);
        b.moving += (moved.min(0.6) / 0.6 - b.moving) * (dt * 3.0).min(1.0);
        b.gait += moved * dt * 26.0;
        self.birds[i] = b;
    }

    /// Nobody passes through anybody: overlapping penguins are pushed apart (a walker gives way more),
    /// and a joiner that is packed in among others is packed.
    fn press(&mut self) {
        let mut near = Vec::new();
        for _ in 0..2 {
            for i in 0..self.birds.len() {
                let p = self.birds[i].p;
                self.near(p, SPACING, &mut near);
                let mut crowd = 0;
                for &j in &near {
                    let j = j as usize;
                    if j == i {
                        continue;
                    }
                    crowd += 1;
                    let d = sub(self.birds[i].p, self.birds[j].p);
                    let l = len(d).max(1e-4);
                    let over = (SPACING - l) * 0.5;
                    let (wi, wj) = (give(self.birds[i].state), give(self.birds[j].state));
                    let s = over / (wi + wj);
                    self.birds[i].p[0] += d[0] / l * s * wi;
                    self.birds[i].p[1] += d[1] / l * s * wi;
                    self.birds[j].p[0] -= d[0] / l * s * wj;
                    self.birds[j].p[1] -= d[1] / l * s * wj;
                }
                if self.birds[i].state == State::Joining && crowd >= 3 {
                    self.birds[i].state = State::Packed;
                    self.birds[i].cold = 0.0;
                    self.joined.push(self.birds[i].p);
                }
            }
        }
    }
}

/// How readily a penguin is pushed aside: one walking past gives way to the packed.
fn give(s: State) -> f32 {
    match s {
        State::Packed => 1.0,
        State::Joining => 1.5,
        State::Walking(..) => 4.0,
    }
}

fn ease(x: f32) -> f32 {
    x * x * (3.0 - 2.0 * x)
}

fn wrap(a: f32) -> f32 {
    let t = std::f32::consts::TAU;
    (a + std::f32::consts::PI).rem_euclid(t) - std::f32::consts::PI
}
