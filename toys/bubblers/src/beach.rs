//! The beach at low tide, in centimetres: sand-bubbler crabs (Scopimera, Dotilla) each at its burrow.
//! A crab feeds out along a spoke from its hole, scooping the surface, sifting what it can eat and
//! rolling the rest into a ball it leaves behind; at the spoke's end it runs home and sets off along
//! a new spoke beside the last, on the side where the sand is least worked (its own or its
//! neighbours'). Nothing more, and the beach fills with starbursts that meet but rarely cross.
//!
//! A crab that sees a neighbour bolt bolts too; a footstep sends the near ones running at once. In
//! the burrow a while, it peeks, then comes out. The tide, when it comes, takes the balls and the
//! trenches and leaves the sand smooth and wet; the crabs wait it out below.

/// The grid's cell, cm (the trenches, the wet and the finger's furrows are kept on it).
pub const CELL: f32 = 0.25;

#[derive(Clone)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    pub fn range(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.f()
    }
}

#[derive(Clone, Copy)]
pub struct Pellet {
    pub p: [f32; 2],
    pub r: f32,
    pub born: f32,
    pub shade: f32,
    /// when the tide took it (it melts away)
    pub gone: f32,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    /// down the burrow until then
    Hidden(f32),
    /// eyes up at the mouth until then
    Peek(f32),
    /// out along its spoke, feeding
    Feed,
    /// running home at the spoke's end
    Home,
    /// running home in fear
    Flee,
}

#[derive(Clone, Copy)]
pub struct Crab {
    pub home: [f32; 2],
    pub p: [f32; 2],
    /// the spoke's direction (radians) and how far out it goes, cm
    pub heading: f32,
    pub reach: f32,
    /// which way round it works its spokes (+1 or -1)
    pub turn: f32,
    pub state: State,
    pub next_ball: f32,
    pub side: f32,
    /// it will bolt then, having seen or felt something
    pub scared: Option<f32>,
    pub facing: f32,
    /// its legs' phase, for drawing
    pub gait: f32,
    pub size: f32,
    pub moving: f32,
    /// its burrow is open (it has dug out since the last tide sealed it)
    pub dug: bool,
}

pub struct Beach {
    pub w: f32,
    pub h: f32,
    pub gw: usize,
    pub gh: usize,
    pub crabs: Vec<Crab>,
    pub pellets: Vec<Pellet>,
    /// per cell: the worked (sifted) sand 0..1, the wet 0..1, the finger's furrow 0..1
    pub worked: Vec<f32>,
    pub wet: Vec<f32>,
    pub furrow: Vec<f32>,
    pub t: f32,
    pub tide: Option<f32>,
    pub rng: Rng,
    pub dirty: bool,
    /// what happened this step, for the sound and the hand: balls laid, crabs that bolted
    pub laid: Vec<[f32; 2]>,
    pub bolted: Vec<[f32; 2]>,
    /// The rule, on or off (the switches mark; `RULES`): each spoke starts beside the last, on the
    /// least worked sand (off: anywhere at random). (Bolting with a neighbour shows only after the
    /// reader's footstep, so it isn't a switch.)
    pub on: [bool; 1],
}

/// The rules as the switches mark names them, in `Beach::on`'s order.
pub const RULES: [&str; 1] = ["Feed beside the last"];

/// The tide: it rises over the beach in this long, stands, and falls back (seconds).
pub const TIDE_IN: f32 = 6.0;
pub const TIDE_STAND: f32 = 1.5;
pub const TIDE_OUT: f32 = 7.0;
/// A crab sees a neighbour bolt this far off (cm); a footstep is felt this far.
const SEE_CM: f32 = 9.0;
const FEEL_CM: f32 = 13.0;
const FEED_CM_S: f32 = 1.5;
const RUN_CM_S: f32 = 5.0;
const FLEE_CM_S: f32 = 14.0;

impl Beach {
    pub fn new(w: f32, h: f32, seed: u64) -> Beach {
        let gw = (w / CELL).ceil() as usize;
        let gh = (h / CELL).ceil() as usize;
        let mut rng = Rng(seed | 1);
        // burrows: a jittered scatter, no two closer than 6 cm
        let mut homes: Vec<[f32; 2]> = Vec::new();
        for _ in 0..4000 {
            let p = [rng.range(1.0, w - 1.0), rng.range(1.0, h - 1.0)];
            if homes
                .iter()
                .all(|q| (q[0] - p[0]).powi(2) + (q[1] - p[1]).powi(2) > 6.2f32.powi(2))
            {
                homes.push(p);
            }
        }
        let crabs = homes
            .iter()
            .map(|&home| Crab {
                home,
                p: home,
                heading: rng.range(0.0, std::f32::consts::TAU),
                reach: rng.range(2.5, 4.0),
                turn: if rng.f() < 0.5 { 1.0 } else { -1.0 },
                state: State::Hidden(rng.range(0.5, 7.0)),
                next_ball: 0.0,
                side: 1.0,
                scared: None,
                facing: 0.0,
                gait: rng.range(0.0, 6.0),
                size: rng.range(0.85, 1.15),
                moving: 0.0,
                dug: false,
            })
            .collect();
        Beach {
            w,
            h,
            gw,
            gh,
            crabs,
            pellets: Vec::new(),
            worked: vec![0.0; gw * gh],
            // the tide has just gone out: the beach is wet and smooth, every burrow sealed
            wet: vec![1.0; gw * gh],
            furrow: vec![0.0; gw * gh],
            t: 0.0,
            tide: None,
            rng,
            dirty: true,
            laid: Vec::new(),
            bolted: Vec::new(),
            on: [true; 1],
        }
    }

    fn cell(&self, p: [f32; 2]) -> Option<usize> {
        let (x, y) = ((p[0] / CELL) as i64, (p[1] / CELL) as i64);
        (x >= 0 && y >= 0 && (x as usize) < self.gw && (y as usize) < self.gh)
            .then(|| y as usize * self.gw + x as usize)
    }

    /// Marks a disc of cells (radius cm) in one of the grids, toward `to`.
    fn mark(grid: &mut [f32], gw: usize, gh: usize, c: [f32; 2], r: f32, to: f32) {
        let (x0, x1) = (
            ((c[0] - r) / CELL).floor().max(0.0) as usize,
            (((c[0] + r) / CELL).ceil() as usize).min(gw),
        );
        let (y0, y1) = (
            ((c[1] - r) / CELL).floor().max(0.0) as usize,
            (((c[1] + r) / CELL).ceil() as usize).min(gh),
        );
        for y in y0..y1 {
            for x in x0..x1 {
                let q = [(x as f32 + 0.5) * CELL, (y as f32 + 0.5) * CELL];
                let d = ((q[0] - c[0]).powi(2) + (q[1] - c[1]).powi(2)).sqrt();
                if d < r {
                    let k = 1.0 - d / r;
                    let i = y * gw + x;
                    grid[i] = grid[i].max(to * k * k * (3.0 - 2.0 * k));
                }
            }
        }
    }

    /// How much worked sand lies along a spoke from `home` at angle `a`, out to `len`.
    fn worked_along(&self, home: [f32; 2], a: f32, len: f32) -> f32 {
        let mut s = 0.0;
        let mut n = 0.0f32;
        let mut d = 1.0;
        while d < len {
            let p = [home[0] + a.cos() * d, home[1] + a.sin() * d];
            match self.cell(p) {
                Some(i) => s += self.worked[i],
                None => s += 1.5,
            }
            n += 1.0;
            d += 0.5;
        }
        s / n.max(1.0)
    }

    /// The next spoke: a few steps round from the last, the way the crab works, onto the least sifted
    /// sand among a few tries.
    fn next_spoke(&mut self, k: usize) {
        let c = self.crabs[k];
        let reach = (c.reach + self.rng.range(-0.3, 1.2)).clamp(2.5, 9.0);
        let mut best = (f32::MAX, c.heading);
        for _ in 0..5 {
            if !self.on[0] {
                best = (0.0, self.rng.range(0.0, std::f32::consts::TAU));
                break;
            }
            let a = c.heading + c.turn * self.rng.range(0.45, 0.8);
            let w = self.worked_along(c.home, a, reach) + self.rng.range(0.0, 0.05);
            if w < best.0 {
                best = (w, a);
            }
        }
        let crab = &mut self.crabs[k];
        crab.heading = best.1;
        // everything round already sifted: it wanders less far
        crab.reach = if best.0 > 0.6 {
            (reach * 0.7).max(2.0)
        } else {
            reach
        };
    }

    /// A footstep (or a finger) at `p`: the crabs near feel it at once, the nearest first.
    pub fn footstep(&mut self, p: [f32; 2], reach: f32) {
        let t = self.t;
        for c in &mut self.crabs {
            let d = ((c.p[0] - p[0]).powi(2) + (c.p[1] - p[1]).powi(2)).sqrt();
            if d < FEEL_CM * reach && matches!(c.state, State::Feed | State::Home | State::Peek(_))
            {
                let at = t + d / 80.0;
                c.scared = Some(c.scared.map_or(at, |s| s.min(at)));
            }
            // one already down stays down a while longer
            if d < FEEL_CM * reach {
                if let State::Hidden(u) = c.state {
                    c.state = State::Hidden(u.max(t + 1.5));
                }
            }
        }
    }

    /// A finger dragged through the sand leaves a furrow.
    pub fn furrow(&mut self, a: [f32; 2], b: [f32; 2]) {
        let d = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
        let n = (d / 0.2).ceil().max(1.0) as usize;
        for s in 0..=n {
            let f = s as f32 / n as f32;
            let p = [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f];
            Self::mark(&mut self.furrow, self.gw, self.gh, p, 0.6, 1.0);
        }
        // the balls in its way are pushed aside, out to its banks
        let (ux, uy) = (b[0] - a[0], b[1] - a[1]);
        let l2 = (ux * ux + uy * uy).max(1e-6);
        for pl in &mut self.pellets {
            let f = (((pl.p[0] - a[0]) * ux + (pl.p[1] - a[1]) * uy) / l2).clamp(0.0, 1.0);
            let q = [a[0] + ux * f, a[1] + uy * f];
            let (dx, dy) = (pl.p[0] - q[0], pl.p[1] - q[1]);
            let dd = (dx * dx + dy * dy).sqrt();
            if dd < 0.75 {
                let (nx, ny) = if dd > 1e-4 {
                    (dx / dd, dy / dd)
                } else {
                    (-uy / l2.sqrt(), ux / l2.sqrt())
                };
                pl.p[0] = q[0] + nx * (0.8 + 0.1 * pl.shade.abs());
                pl.p[1] = q[1] + ny * (0.8 + 0.1 * pl.shade.abs());
            }
        }
        self.dirty = true;
    }

    pub fn bring_tide(&mut self) {
        if self.tide.is_none() {
            self.tide = Some(self.t);
            for c in &mut self.crabs {
                if !matches!(c.state, State::Hidden(_)) {
                    c.scared = Some(self.t + self.rng.range(0.0, 0.8));
                }
            }
        }
    }

    /// The water's edge (cm from the top): below it, the swash. None when the tide is out.
    pub fn front(&self) -> Option<f32> {
        let s = self.tide?;
        let a = self.t - s;
        let reach = self.h + 3.0;
        let ease = |x: f32| x * x * (3.0 - 2.0 * x);
        let up = if a < TIDE_IN {
            ease(a / TIDE_IN)
        } else if a < TIDE_IN + TIDE_STAND {
            1.0
        } else {
            1.0 - ease(((a - TIDE_IN - TIDE_STAND) / TIDE_OUT).min(1.0))
        };
        Some(self.h + 2.0 - up * reach)
    }

    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        let t = self.t;
        self.laid.clear();
        self.bolted.clear();
        self.tide_step();
        let n = self.crabs.len();
        for k in 0..n {
            self.crab_step(k, dt);
        }
        // the wet dries
        if self.tide.is_none() {
            let dry = dt / 90.0;
            for w in &mut self.wet {
                if *w > 0.35 {
                    *w = (*w - dry).max(0.35);
                }
            }
        }
        self.pellets.retain(|p| p.gone < 0.0 || t - p.gone < 1.5);
        if self.pellets.len() > 14000 {
            let extra = self.pellets.len() - 14000;
            self.pellets.drain(..extra);
        }
    }

    fn tide_step(&mut self) {
        let Some(s) = self.tide else { return };
        let t = self.t;
        if let Some(front) = self.front() {
            // under the water: the balls melt, the trenches and furrows smooth, the sand is soaked
            let row = ((front / CELL).floor().max(0.0) as usize).min(self.gh);
            for y in row..self.gh {
                for x in 0..self.gw {
                    let i = y * self.gw + x;
                    self.worked[i] *= 0.9;
                    self.furrow[i] *= 0.9;
                    self.wet[i] = 1.0;
                }
            }
            // the burrows under it are sealed
            for c in &mut self.crabs {
                if c.home[1] > front {
                    c.dug = false;
                }
            }
            for p in &mut self.pellets {
                if p.gone < 0.0 && p.p[1] > front {
                    p.gone = t;
                }
            }
            self.dirty = true;
        }
        if t - s > TIDE_IN + TIDE_STAND + TIDE_OUT {
            self.tide = None;
            for c in &mut self.crabs {
                c.state = State::Hidden(t + self.rng.range(1.0, 5.0));
                c.reach = self.rng.range(2.5, 3.5);
                c.dug = false;
            }
        }
    }

    /// A crab that has seen or felt something bolts when its moment comes, and its neighbours see it go.
    fn fright(&mut self, k: usize, c: &mut Crab) {
        let t = self.t;
        if let Some(at) = c.scared {
            if t >= at && !matches!(c.state, State::Hidden(_)) {
                c.scared = None;
                if c.state != State::Flee {
                    c.state = State::Flee;
                    self.bolted.push(c.p);
                    // the neighbours see it go
                    for (j, o) in self.crabs.iter_mut().enumerate() {
                        if j == k || matches!(o.state, State::Hidden(_) | State::Flee) {
                            continue;
                        }
                        let d = ((o.p[0] - c.p[0]).powi(2) + (o.p[1] - c.p[1]).powi(2)).sqrt();
                        if d < SEE_CM {
                            let when = t + self.rng.range(0.12, 0.3);
                            o.scared = Some(o.scared.map_or(when, |s| s.min(when)));
                        }
                    }
                }
            } else if matches!(c.state, State::Hidden(_)) {
                c.scared = None;
            }
        }
    }

    /// Out along its spoke: eating the sand under it, rolling a ball behind it now and then, home at the
    /// spoke's end.
    fn feed(&mut self, c: &mut Crab, dt: f32) {
        let t = self.t;
        let dir = [c.heading.cos(), c.heading.sin()];
        let step = FEED_CM_S * dt;
        c.p = [c.p[0] + dir[0] * step, c.p[1] + dir[1] * step];
        c.facing = c.heading;
        c.moving = 0.4;
        Self::mark(&mut self.worked, self.gw, self.gh, c.p, 0.45, 1.0);
        self.dirty = true;
        if t >= c.next_ball {
            c.next_ball = t + self.rng.range(0.19, 0.23);
            c.side = -c.side;
            let perp = [-dir[1], dir[0]];
            let r = self.rng.range(0.11, 0.2) * c.size;
            let back = self.rng.range(0.35, 0.45);
            let lat = c.side * self.rng.range(0.0, 0.07) + self.rng.range(-0.05, 0.05);
            let p = [
                c.p[0] - dir[0] * back + perp[0] * lat,
                c.p[1] - dir[1] * back + perp[1] * lat,
            ];
            self.pellets.push(Pellet {
                p,
                r,
                born: t,
                shade: self.rng.range(-1.0, 1.0),
                gone: -1.0,
            });
            self.laid.push(p);
        }
        let out = ((c.p[0] - c.home[0]).powi(2) + (c.p[1] - c.home[1]).powi(2)).sqrt();
        let off = c.p[0] < 0.5 || c.p[1] < 0.5 || c.p[0] > self.w - 0.5 || c.p[1] > self.h - 0.5;
        if out >= c.reach || off {
            c.state = State::Home;
        }
    }

    fn crab_step(&mut self, k: usize, dt: f32) {
        let t = self.t;
        let mut c = self.crabs[k];
        c.moving = 0.0;
        self.fright(k, &mut c);
        match c.state {
            State::Hidden(until) => {
                c.p = c.home;
                if t >= until && self.tide.is_none() {
                    c.state = State::Peek(t + self.rng.range(0.6, 1.4));
                    c.dug = true;
                }
            }
            State::Peek(until) => {
                c.p = c.home;
                if t >= until {
                    self.crabs[k] = c;
                    self.next_spoke(k);
                    c = self.crabs[k];
                    c.state = State::Feed;
                    c.next_ball = t + self.rng.range(0.6, 1.2);
                }
            }
            State::Feed => self.feed(&mut c, dt),
            State::Home | State::Flee => {
                let speed = if c.state == State::Flee {
                    FLEE_CM_S
                } else {
                    RUN_CM_S
                };
                let d = [c.home[0] - c.p[0], c.home[1] - c.p[1]];
                let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
                c.moving = if c.state == State::Flee { 1.0 } else { 0.7 };
                if l > 1e-3 {
                    c.facing = d[1].atan2(d[0]);
                }
                if l <= speed * dt {
                    c.p = c.home;
                    if c.state == State::Flee {
                        c.state = State::Hidden(t + self.rng.range(2.5, 5.5));
                    } else {
                        self.crabs[k] = c;
                        self.next_spoke(k);
                        c = self.crabs[k];
                        c.state = State::Feed;
                        c.next_ball = t + self.rng.range(0.4, 0.9);
                    }
                } else {
                    c.p = [
                        c.p[0] + d[0] / l * speed * dt,
                        c.p[1] + d[1] / l * speed * dt,
                    ];
                }
            }
        }
        c.gait += dt * c.moving * 22.0;
        self.crabs[k] = c;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crabs_lay_balls_in_spokes_and_a_step_sends_them_home() {
        let mut b = Beach::new(30.0, 65.0, 7);
        for _ in 0..1200 {
            b.step(0.05);
        }
        assert!(
            b.pellets.len() > b.crabs.len() * 10,
            "{} balls",
            b.pellets.len()
        );
        let out = b.crabs.iter().filter(|c| c.state == State::Feed).count();
        assert!(out > b.crabs.len() / 3);
        let c = b.crabs[0].p;
        b.footstep(c, 1.0);
        for _ in 0..20 {
            b.step(0.05);
        }
        assert!(matches!(b.crabs[0].state, State::Hidden(_)));
    }

    #[test]
    fn the_tide_takes_everything() {
        let mut b = Beach::new(30.0, 65.0, 3);
        for _ in 0..600 {
            b.step(0.05);
        }
        b.bring_tide();
        for _ in 0..((TIDE_IN + TIDE_STAND + 2.0) / 0.05) as usize {
            b.step(0.05);
        }
        assert!(b.pellets.is_empty());
        assert!(b.crabs.iter().all(|c| matches!(c.state, State::Hidden(_))));
    }
}
