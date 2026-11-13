//! The colony: bats leaving the cave at dusk. Metres; x to the right of the watcher, y up, z away.
//!
//! Each bat climbs out of the cave mouth in a turning column (it circles the column's axis, pulled to
//! the column's radius, and rises), and at the column's top it takes a lane in one of two streams that
//! run off over the hills. A bat that sees the hawk swerves away from it and drops; a bat whose
//! neighbour swerves swerves too, a moment later, the same way, unless it swerved a moment ago. That
//! is all: the ripple that runs along the stream is those rules.

use std::collections::HashMap;

/// The cave mouth: ahead of the watcher, `CAVE_Z` metres away, at the ground (y = 0).
pub const CAVE_Z: f32 = 110.0;
/// and a little left of straight ahead, so the streams have the sky to the right to run into
pub const CAVE_X: f32 = -12.0;
/// Where the column ends and the streams begin, metres above the mouth.
pub const TOP: f32 = 125.0;
/// How far along a stream a bat is let go (and goes back to the cave to come out again), metres.
pub const STREAM_END: f32 = 2600.0;

/// A stream: its heading over the ground (x, z, a unit vector), how far it climbs above the top before levelling, its
/// half-width at the top and how it widens, and how much it wanders.
pub struct Stream {
    pub heading: [f32; 2],
    pub climb: f32,
    pub width: f32,
    pub spread: f32,
    pub wander: f32,
}

pub const STREAMS: [Stream; 2] = [
    Stream {
        heading: [0.50, 0.866],
        climb: 70.0,
        width: 9.0,
        spread: 0.02,
        wander: 60.0,
    },
    Stream {
        heading: [-0.42, 0.907],
        climb: 45.0,
        width: 6.0,
        spread: 0.016,
        wander: -100.0,
    },
];

#[derive(Clone, Copy, PartialEq)]
pub enum Stage {
    /// Inside, waiting to come out.
    Roost,
    Column,
    Stream,
}

#[derive(Clone, Copy)]
pub struct Bat {
    pub p: [f32; 3],
    pub v: [f32; 3],
    pub stage: Stage,
    /// which stream, its place across it (-1..1) and above it (-1..1), and how far along it is
    pub stream: u8,
    pub lane: [f32; 2],
    pub s: f32,
    /// its own pace (about 1) and the wingbeat's phase
    pub pace: f32,
    pub wing: f32,
    /// the swerve: seconds since it began (`QUIET` and over: none), the way it goes, how strong; and
    /// seconds until it may swerve again
    pub swerve: f32,
    pub away: [f32; 3],
    pub strength: f32,
    pub rest: f32,
    /// a swerve heard from a neighbour, not yet made: seconds to wait, the way, the strength
    pub pending: Option<(f32, [f32; 3], f32)>,
}

/// A small random source (xorshift), so a run is the same every time.
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

pub struct Colony {
    pub bats: Vec<Bat>,
    rng: Rng,
    /// bats out a second now, and the time since the emergence began
    pub rate: f32,
    pub t: f32,
    owed: f32,
    /// the bats near enough each other to hear a swerve, by cell (rebuilt while one is spreading)
    cells: HashMap<(i32, i32, i32), Vec<u32>>,
    /// bats that swerved this step (for the sound)
    pub swerved: u32,
    /// where the hawks are now: every bat keeps clear of them
    pub hawks: Vec<[f32; 3]>,
}

const CELL: f32 = 4.0;
/// How near a neighbour must be to pass a swerve on, metres; and how long it takes, seconds.
const HEAR: f32 = 4.5;
const REACT: [f32; 2] = [0.07, 0.16];
/// How much of a swerve's strength a neighbour takes on.
const PASS: f32 = 0.986;
/// How far a jink carries a bat aside at its strongest, metres; and how long a swerve lasts.
const JINK: f32 = 15.0;
const QUIET: f32 = 4.0;
/// How much of its distance from the flight's middle line a jinking bat closes.
const PINCH: f32 = 0.9;

/// How far into its jink a bat is (0 none .. 1 at its widest).
pub fn jinking(b: &Bat) -> f32 {
    if b.swerve < QUIET {
        (jink(b.swerve) * b.strength / 0.7).min(1.0)
    } else {
        0.0
    }
}

/// The jink's shape over time: out in a quarter second, back over a second and a half.
fn jink(t: f32) -> f32 {
    (1.0 - (-t / 0.11).exp()) * (-t / 1.1).exp()
}

/// How far bats keep from a hawk, metres.
const CLEAR: f32 = 13.0;

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: [f32; 3], k: f32) -> [f32; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}
pub fn len(a: [f32; 3]) -> f32 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    mul(a, 1.0 / len(a).max(1e-6))
}

/// The column's axis at height `y` and time `t` (it leans a little toward the streams as it rises,
/// and sways like smoke).
fn axis(y: f32, t: f32) -> [f32; 2] {
    let h = (y / TOP).clamp(0.0, 1.0);
    let sway = h * h * 15.0;
    [
        CAVE_X + 0.06 * y + sway * (y / 45.0 - t * 0.21).sin(),
        CAVE_Z + 0.04 * y + sway * 0.6 * (y / 60.0 - t * 0.17 + 1.3).cos(),
    ]
}

/// The column's radius at height `y`.
fn radius(y: f32, t: f32) -> f32 {
    // swelling and narrowing as the bats come out in pulses
    (2.5 + 0.085 * y.max(0.0)) * (1.0 + 0.22 * (y / 28.0 - t * 0.9).sin())
}

/// A point `s` metres along stream `k`'s middle, and its direction there.
pub fn stream_at(k: usize, s: f32) -> ([f32; 3], [f32; 3]) {
    let st = &STREAMS[k];
    let a = axis(TOP, 0.0);
    let side = [st.heading[1], -st.heading[0]];
    // one long curve out and away (it never turns back), with a little wander on it
    let bend = |s: f32| {
        st.wander * ((1.0 - (-s / 500.0).exp()) * 1.6 + 0.25 * (s / 260.0 + k as f32 * 2.0).sin())
    };
    let at = |s: f32| {
        let b = bend(s) * (s / 300.0).min(1.0);
        [
            a[0] + st.heading[0] * s + side[0] * b,
            // up off the column's top, then level: far off, a ribbon low over the hills
            TOP + st.climb * (1.0 - (-s / 350.0).exp()),
            a[1] + st.heading[1] * s + side[1] * b,
        ]
    };
    let p = at(s);
    let d = norm(sub(at(s + 5.0), p));
    (p, d)
}

impl Colony {
    pub fn new(n: usize, seed: u64) -> Colony {
        let mut rng = Rng::new(seed);
        let bats = (0..n)
            .map(|_| Bat {
                p: [0.0, -10.0, CAVE_Z],
                v: [0.0; 3],
                stage: Stage::Roost,
                stream: 0,
                lane: [0.0; 2],
                s: 0.0,
                pace: rng.range(0.85, 1.15),
                wing: rng.range(0.0, std::f32::consts::TAU),
                swerve: QUIET,
                away: [0.0; 3],
                strength: 0.0,
                rest: 0.0,
                pending: None,
            })
            .collect();
        Colony {
            bats,
            rng,
            rate: 0.0,
            t: 0.0,
            owed: 0.0,
            cells: HashMap::new(),
            swerved: 0,
            hawks: Vec::new(),
        }
    }

    /// How many are out (in the column or a stream).
    pub fn out(&self) -> usize {
        self.bats.iter().filter(|b| b.stage != Stage::Roost).count()
    }

    fn emerge(&mut self, dt: f32) {
        // a trickle, then the flood: the rate climbs over the first minute and a half
        let full = 330.0;
        // they come out in pulses, which the circling turns into rings climbing the column
        let pulse = 0.6 + 0.4 * (self.t * 0.9).sin() * (self.t * 0.37 + 1.0).sin();
        self.rate = full * (self.t / 90.0).clamp(0.04, 1.0).powf(1.3) * pulse;
        self.owed += self.rate * dt;
        let mut i = 0;
        while self.owed >= 1.0 && i < self.bats.len() {
            if self.bats[i].stage == Stage::Roost {
                let a = self.rng.range(0.0, std::f32::consts::TAU);
                let r = self.rng.range(0.0, 3.0);
                let b = &mut self.bats[i];
                b.p = [CAVE_X + r * a.cos(), -2.0, CAVE_Z + r * a.sin()];
                b.v = [0.0, 6.0, 0.0];
                b.stage = Stage::Column;
                b.swerve = QUIET;
                b.rest = 0.0;
                b.pending = None;
                self.owed -= 1.0;
            }
            i += 1;
        }
        self.owed = self.owed.min(4.0);
    }

    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        self.emerge(dt);
        self.swerved = 0;
        let spreading = self
            .bats
            .iter()
            .any(|b| b.swerve < 0.6 || b.pending.is_some());
        if spreading {
            self.index();
        }
        for i in 0..self.bats.len() {
            if self.bats[i].stage == Stage::Roost {
                continue;
            }
            self.hear(i, dt);
            self.fly(i, dt);
        }
    }

    fn index(&mut self) {
        for v in self.cells.values_mut() {
            v.clear();
        }
        for (i, b) in self.bats.iter().enumerate() {
            if b.stage != Stage::Roost {
                self.cells.entry(cell(b.p)).or_default().push(i as u32);
            }
        }
    }

    /// The bats within `r` metres of `p` (needs `index`).
    fn near(&self, p: [f32; 3], r: f32, out: &mut Vec<u32>) {
        out.clear();
        let c = cell(p);
        let k = (r / CELL).ceil() as i32;
        for dx in -k..=k {
            for dy in -k..=k {
                for dz in -k..=k {
                    if let Some(v) = self.cells.get(&(c.0 + dx, c.1 + dy, c.2 + dz)) {
                        for &j in v {
                            if len(sub(self.bats[j as usize].p, p)) < r {
                                out.push(j);
                            }
                        }
                    }
                }
            }
        }
    }

    /// A pending swerve comes due; a fresh swerve is passed to the neighbours.
    fn hear(&mut self, i: usize, dt: f32) {
        let b = &mut self.bats[i];
        b.rest = (b.rest - dt).max(0.0);
        let Some((wait, away, strength)) = b.pending else {
            return;
        };
        if wait > dt {
            b.pending = Some((wait - dt, away, strength));
            return;
        }
        b.pending = None;
        self.startle(i, away, strength);
    }

    /// Bat `i` swerves along `away` (and drops), and tells its neighbours.
    pub fn startle(&mut self, i: usize, away: [f32; 3], strength: f32) {
        let b = &mut self.bats[i];
        if b.rest > 0.0 || b.stage == Stage::Roost {
            return;
        }
        b.swerve = 0.0;
        b.rest = 1.8;
        b.away = away;
        b.strength = strength;
        b.v = add(b.v, mul(away, 3.0 * strength));
        self.swerved += 1;
        let next = strength * PASS;
        if next < 0.12 {
            return;
        }
        let p = b.p;
        let mut near = Vec::new();
        if self.cells.is_empty() {
            self.index();
        }
        self.near(p, HEAR, &mut near);
        for j in near {
            let wait = self.rng.range(REACT[0], REACT[1]);
            let n = &mut self.bats[j as usize];
            if n.rest <= 0.0 && n.pending.is_none() && j as usize != i {
                n.pending = Some((wait, away, next));
            }
        }
    }

    fn fly(&mut self, i: usize, dt: f32) {
        let b = self.bats[i];
        let (want, turn) = match b.stage {
            Stage::Column => self.climb(&b),
            Stage::Stream => stream_want(&b),
            Stage::Roost => return,
        };
        let follow = if b.swerve < 0.5 { 0.4 } else { turn };
        let mut want = want;
        // keep clear of the hawks: the nearer, the harder
        for h in &self.hawks {
            let d = sub(b.p, *h);
            let r = len(d);
            if r < CLEAR {
                want = add(want, mul(d, 26.0 * (CLEAR - r) / (CLEAR * r.max(0.5))));
            }
        }
        let mut v = add(b.v, mul(sub(want, b.v), (follow * dt).min(1.0)));
        // a little flutter of its own
        let j = 1.4 * dt.sqrt();
        v = add(
            v,
            [
                self.rng.range(-j, j),
                self.rng.range(-j, j) * 0.6,
                self.rng.range(-j, j),
            ],
        );
        // the line the bat flies along: the column's axis, or its stream's middle
        let centre = match b.stage {
            Stage::Column => {
                let a = axis(b.p[1], self.t);
                [a[0], b.p[1], a[1]]
            }
            _ => stream_at(b.stream as usize, b.s).0,
        };
        let b = &mut self.bats[i];
        b.v = v;
        b.p = add(b.p, mul(v, dt));
        // the jink: a quick sidestep and drop along `away`, eased back over a second or two
        if b.swerve < QUIET {
            let g0 = jink(b.swerve);
            b.swerve += dt;
            let g1 = jink(b.swerve);
            let dir = norm(add(b.away, [0.0, -0.6, 0.0]));
            b.p = add(b.p, mul(dir, JINK * b.strength * (g1 - g0)));
            // and the flight closes up round its line as the swerve passes: a dark knot runs on
            let mut inward = sub(centre, b.p);
            inward[1] *= 0.3;
            b.p = add(b.p, mul(inward, PINCH * b.strength.min(1.0) * (g1 - g0)));
        }
        b.wing += dt * std::f32::consts::TAU * 9.5 * b.pace;
        match b.stage {
            // each leaves the column at its own height (its pace says which)
            Stage::Column if b.p[1] > TOP * (0.62 + (b.pace - 0.85) * 1.3) => {
                b.stage = Stage::Stream;
                b.stream = if self.rng.unit() < 0.68 { 0 } else { 1 };
                b.lane = [self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0)];
                b.s = 0.0;
            }
            Stage::Stream => {
                let (_, d) = stream_at(b.stream as usize, b.s);
                b.s += (v[0] * d[0] + v[1] * d[1] + v[2] * d[2]).max(0.0) * dt;
                if b.s > STREAM_END {
                    b.stage = Stage::Roost;
                    b.p = [0.0, -10.0, CAVE_Z];
                }
            }
            _ => {}
        }
    }

    /// The column: round the axis (anticlockwise seen from above), to the column's radius, and up.
    fn climb(&self, b: &Bat) -> ([f32; 3], f32) {
        let a = axis(b.p[1], self.t);
        let rel = [b.p[0] - a[0], b.p[2] - a[1]];
        let r = (rel[0] * rel[0] + rel[1] * rel[1]).sqrt().max(0.3);
        let out = [rel[0] / r, rel[1] / r];
        let round = [-out[1], out[0]];
        let pull = (radius(b.p[1], self.t) - r).clamp(-6.0, 6.0) * 1.3;
        let speed = 9.5 * b.pace;
        let up = 4.0 * b.pace + if b.p[1] < 4.0 { 4.0 } else { 0.0 };
        (
            [
                round[0] * speed + out[0] * pull,
                up,
                round[1] * speed + out[1] * pull,
            ],
            2.2,
        )
    }
}

/// A stream: toward a point a little ahead on its middle, at the bat's own lane.
fn stream_want(b: &Bat) -> ([f32; 3], f32) {
    let k = b.stream as usize;
    let st = &STREAMS[k];
    let (c, d) = stream_at(k, b.s + 28.0);
    let side = norm([d[2], 0.0, -d[0]]);
    let w = st.width + st.spread * b.s;
    let target = add(
        c,
        add(mul(side, b.lane[0] * w), [0.0, b.lane[1] * w * 0.35, 0.0]),
    );
    let speed = 16.0 * b.pace;
    (mul(norm(sub(target, b.p)), speed), 1.6)
}

fn cell(p: [f32; 3]) -> (i32, i32, i32) {
    (
        (p[0] / CELL).floor() as i32,
        (p[1] / CELL).floor() as i32,
        (p[2] / CELL).floor() as i32,
    )
}

impl Colony {
    /// The hawk at `p`, moving at `v`: every bat within `reach` metres that can swerve swerves away
    /// from it. Returns how many did, and the index of a bat it caught, if it was going fast and
    /// passed close enough.
    pub fn hawk(&mut self, p: [f32; 3], v: [f32; 3], reach: f32) -> (u32, Option<usize>) {
        self.index();
        let mut near = Vec::new();
        self.near(p, reach, &mut near);
        let mut n = 0;
        let mut caught = None;
        let fast = len(v) > 9.0;
        for j in near {
            let j = j as usize;
            let d = sub(self.bats[j].p, p);
            if fast && len(d) < 1.6 && caught.is_none() && self.rng.unit() < 0.25 {
                caught = Some(j);
                continue;
            }
            if self.bats[j].rest <= 0.0 {
                // away from the hawk, mostly across its path
                let away = norm(add(norm(d), mul(norm(v), -0.3)));
                self.startle(j, away, 1.0);
                n += 1;
            }
        }
        if let Some(j) = caught {
            let b = &mut self.bats[j];
            b.stage = Stage::Roost;
            b.p = [0.0, -10.0, CAVE_Z];
        }
        (n, caught)
    }
}
