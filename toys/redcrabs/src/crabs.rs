//! The crabs, seen from above on Christmas Island's forest floor (metres, from the top left): the
//! rainforest at the top, a road across it with an underpass beneath, the limestone shore and the sea
//! at the bottom. With the wet season's rain, red crabs (Gecarcoidea natalis) leave their burrows and
//! walk downhill to the sea; they rest again as the forest dries, hide in shade from the midday sun,
//! and follow a barrier fence along until it ends, which is how the island leads them to the tunnels
//! under its roads. Each crab only knows the slope, its neighbours, the fence it touches, the sun and
//! the rain (SOURCES.md).

use crate::fence::Fences;

/// The rules as switches (the sheet's), in order.
pub const RULES: [&str; 3] = [
    "Rain sets them walking",
    "They follow a fence",
    "The sun sends them to shade",
];

/// How many crabs are on the screen.
pub const CRABS: usize = 2600;
/// A crab is this wide (metres, a carapace and legs).
pub const BODY: f32 = 0.05;
/// Walking speed (metres a second), before each crab's own pace.
const SPEED: f32 = 0.36;
/// Two crabs keep about this far apart (metres).
const ROOM: f32 = 0.085;
/// A day goes round in this long (seconds), and the sun first rises this long after the toy opens.
pub const DAY_S: f32 = 50.0;
const DAWN_S: f32 = -2.0;
/// Above this much sun a crab that minds it walks to shade; below `COOL`, it walks on.
const HOT: f32 = 0.85;
const COOL: f32 = 0.7;
/// The forest dries in this long (seconds, a time constant).
const DRY_S: f32 = 30.0;
/// A shower wets the forest this far round (metres) and lasts this long (seconds).
pub const SHOWER_M: f32 = 3.2;
pub const SHOWER_S: f32 = 4.0;
/// The sky rains of its own accord this often (seconds).
const SKY_RAIN_S: f32 = 26.0;

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

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    /// in its burrow or under the leaf litter
    Rest,
    Walk,
    /// in the shade, waiting out the sun
    Shade,
    /// at the water's edge, wetting itself and (a female) letting go her eggs
    Sea,
}

pub struct Crab {
    pub p: [f32; 2],
    pub heading: f32,
    pub state: State,
    /// its own pace (0.7..1.3) and gait's phase
    pub pace: f32,
    pub gait: f32,
    pub size: f32,
    /// the way it follows the fence it is touching (-1, 1; 0 when free)
    pub along: f32,
    pub timer: f32,
}

/// A tree's shadow on the floor (centre, radius, metres).
pub struct Tree {
    pub c: [f32; 2],
    pub r: f32,
}

/// A shower falling (centre, seconds left).
pub struct Shower {
    pub c: [f32; 2],
    pub left: f32,
}

pub struct Island {
    pub w: f32,
    pub h: f32,
    pub road: [f32; 2],
    pub gap: [f32; 2],
    pub shore: f32,
    pub sea: f32,
    pub crabs: Vec<Crab>,
    pub trees: Vec<Tree>,
    pub showers: Vec<Shower>,
    /// wet ground left by showers (centre, wetness 0..1)
    pub puddles: Vec<([f32; 2], f32)>,
    pub fences: Fences,
    pub on: [bool; 3],
    /// how wet the forest is (0..1)
    pub wet: f32,
    pub t: f32,
    sky_rain: f32,
    rng: Rng,
    grid: Grid,
}

/// The crabs by where they are, for finding neighbours.
struct Grid {
    cell: f32,
    gw: usize,
    gh: usize,
    head: Vec<u32>,
    next: Vec<u32>,
}

const NONE: u32 = u32::MAX;

impl Grid {
    fn new(w: f32, h: f32, cell: f32) -> Grid {
        let gw = (w / cell).ceil() as usize + 1;
        let gh = (h / cell).ceil() as usize + 3;
        Grid {
            cell,
            gw,
            gh,
            head: vec![NONE; gw * gh],
            next: Vec::new(),
        }
    }
    fn at(&self, p: [f32; 2]) -> (usize, usize) {
        let x = (p[0] / self.cell).floor().clamp(0.0, (self.gw - 1) as f32) as usize;
        let y = ((p[1] / self.cell).floor() + 2.0).clamp(0.0, (self.gh - 1) as f32) as usize;
        (x, y)
    }
    fn build(&mut self, crabs: &[Crab]) {
        self.head.fill(NONE);
        self.next.clear();
        self.next.resize(crabs.len(), NONE);
        for (i, c) in crabs.iter().enumerate() {
            let (x, y) = self.at(c.p);
            let k = y * self.gw + x;
            self.next[i] = self.head[k];
            self.head[k] = i as u32;
        }
    }
}

/// The slope's streams: the way downhill wanders a little, so the crabs braid into rivers.
fn stream(p: [f32; 2], t: f32) -> f32 {
    0.55 * (p[0] * 0.9 + p[1] * 0.35 + t * 0.03).sin() + 0.3 * (p[0] * 2.3 - p[1] * 0.8).sin()
}

fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

impl Island {
    pub fn new(w: f32, h: f32, seed: u64) -> Island {
        let mut rng = Rng(seed | 1);
        let road_top = h * 0.42;
        let road = [road_top, road_top + 1.9];
        let gap = [w * 0.66 - 0.4, w * 0.66 + 0.4];
        let shore = h * 0.84;
        let sea = h * 0.9;
        let mut trees = Vec::new();
        for _ in 0..((w * h) / 4.5) as usize {
            let c = [rng.f() * w, rng.f() * shore - 0.5];
            if c[1] > road[0] - 1.0 && c[1] < road[1] + 1.0 {
                continue;
            }
            trees.push(Tree {
                c,
                r: 0.7 + rng.f() * 0.9,
            });
        }
        let mut island = Island {
            w,
            h,
            road,
            gap,
            shore,
            sea,
            crabs: Vec::with_capacity(CRABS),
            trees,
            showers: Vec::new(),
            puddles: Vec::new(),
            fences: Fences::default(),
            on: [true; 3],
            wet: 0.9,
            t: 0.0,
            sky_rain: SKY_RAIN_S * 0.6,
            rng,
            grid: Grid::new(w, h, ROOM * 1.4),
        };
        island.opening_fences();
        for i in 0..CRABS {
            let walking = i % 5 < 2;
            let y = if walking {
                let y = island.rng.f() * island.shore;
                if y > road[0] - 0.3 && y < road[1] {
                    y - 2.5
                } else {
                    y
                }
            } else {
                island.rng.f() * (road[0] - 0.6)
            };
            let p = [island.rng.f() * w, y];
            let crab = island.crab(p, if walking { State::Walk } else { State::Rest });
            island.crabs.push(crab);
        }
        island
    }

    /// The island's own fence along the road's uphill side, open over the underpass; it runs on past
    /// the screen's edges, so a crab finds the tunnel the nearer way.
    fn opening_fences(&mut self) {
        let y = self.road[0] - 0.15;
        self.fences.add_line(vec![[-6.0, y], [self.gap[0], y]]);
        self.fences
            .add_line(vec![[self.w + 6.0, y], [self.gap[1], y]]);
    }

    fn crab(&mut self, p: [f32; 2], state: State) -> Crab {
        let r = &mut self.rng;
        Crab {
            p,
            heading: std::f32::consts::FRAC_PI_2 + r.s() * 0.6,
            state,
            pace: 0.7 + r.f() * 0.6,
            gait: r.f() * std::f32::consts::TAU,
            size: 0.85 + r.f() * 0.3,
            along: 0.0,
            timer: r.f() * 4.0,
        }
    }

    /// The sun's height (0 at night, 1 overhead).
    pub fn sun(&self) -> f32 {
        let a = (self.t - DAWN_S) / DAY_S * std::f32::consts::TAU;
        a.sin().max(0.0).powf(0.8)
    }

    pub fn in_tunnel(&self, p: [f32; 2]) -> bool {
        p[0] > self.gap[0] && p[0] < self.gap[1] && p[1] > self.road[0] && p[1] < self.road[1]
    }

    /// Rain over `c`: the crabs it falls on set off walking.
    pub fn rain(&mut self, c: [f32; 2]) {
        self.showers.push(Shower { c, left: SHOWER_S });
        self.puddles.push((c, 1.0));
        if self.puddles.len() > 12 {
            self.puddles.remove(0);
        }
        self.wet = (self.wet + 0.45).min(1.0);
        if !self.on[0] {
            return;
        }
        for c2 in &mut self.crabs {
            if c2.state == State::Rest && dist(c2.p, c) < SHOWER_M {
                c2.state = State::Walk;
                c2.timer = 0.0;
            }
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        self.wet *= (-dt / DRY_S).exp();
        self.sky_rain -= dt;
        if self.sky_rain <= 0.0 {
            self.sky_rain = SKY_RAIN_S;
            let c = [self.rng.f() * self.w, self.rng.f() * self.road[0]];
            self.rain(c);
        }
        for s in &mut self.showers {
            s.left -= dt;
        }
        self.showers.retain(|s| s.left > 0.0);
        for p in &mut self.puddles {
            p.1 *= (-dt / 18.0).exp();
        }
        self.puddles.retain(|p| p.1 > 0.05);
        self.grid.build(&self.crabs);
        let sun = self.sun();
        for i in 0..self.crabs.len() {
            self.change_state(i, dt, sun);
            if matches!(self.crabs[i].state, State::Walk | State::Sea) {
                self.walk(i, dt, sun);
            }
        }
    }

    /// Whether a crab sets off, stops, hides or goes back up the hill.
    fn change_state(&mut self, i: usize, dt: f32, sun: f32) {
        let (wet, on) = (self.wet, self.on);
        let roll = self.rng.f();
        let p = self.crabs[i].p;
        let in_forest = p[1] < self.shore && !self.in_tunnel(p);
        let c = &mut self.crabs[i];
        c.timer += dt;
        match c.state {
            State::Rest => {
                if on[0] && roll < wet * 0.35 * dt {
                    c.state = State::Walk;
                }
            }
            State::Walk => {
                let dry = !on[0] || wet < 0.12;
                let rate = if on[0] { 0.25 } else { 1.6 };
                if in_forest && dry && roll < rate * dt {
                    c.state = State::Rest;
                } else if c.p[1] >= self.sea - 0.05 {
                    c.state = State::Sea;
                    c.timer = 0.0;
                }
            }
            State::Shade => {
                if !on[2] || sun < COOL {
                    c.state = State::Walk;
                } else if !on[0] && roll < 1.6 * dt {
                    c.state = State::Rest;
                }
            }
            State::Sea => {
                if c.timer > 3.0 + c.pace * 3.0 {
                    self.home(i);
                }
            }
        }
    }

    /// A crab done at the sea is gone from the screen; another comes down out of the forest.
    fn home(&mut self, i: usize) {
        let x = self.rng.f() * self.w;
        let walking = self.on[0] && self.wet > 0.2;
        let p = if walking {
            [x, -0.3 - self.rng.f() * 1.5]
        } else {
            [x, self.rng.f() * (self.road[0] - 0.6)]
        };
        let state = if walking { State::Walk } else { State::Rest };
        self.crabs[i] = self.crab(p, state);
    }

    /// The nearest tree's shade to `p`, when it is in the forest.
    fn shade_near(&self, p: [f32; 2]) -> Option<([f32; 2], f32)> {
        self.trees
            .iter()
            .map(|t| (t.c, t.r, dist(t.c, p)))
            .filter(|(_, _, d)| *d < 6.0)
            .min_by(|a, b| (a.2 - a.1).total_cmp(&(b.2 - b.1)))
            .map(|(c, r, _)| (c, r))
    }

    /// Where a walking crab wants to go: downhill, or into shade when the sun is high.
    fn wish(&self, c: &Crab, sun: f32) -> (f32, f32) {
        let down = std::f32::consts::FRAC_PI_2;
        if c.state == State::Sea {
            let side = if (c.gait as i32) % 2 == 0 {
                0.0
            } else {
                std::f32::consts::PI
            };
            return (side + (c.timer * 1.3).sin() * 0.5, 0.25);
        }
        if self.in_tunnel(c.p) {
            return (down, 1.2);
        }
        let open = c.p[1] < self.shore && !(c.p[1] > self.road[0] - 0.4 && c.p[1] < self.road[1]);
        if self.on[2] && sun > HOT && open {
            if let Some((t, r)) = self.shade_near(c.p) {
                let d = dist(t, c.p);
                if d < r * 0.75 {
                    return (c.heading, 0.0);
                }
                return ((t[1] - c.p[1]).atan2(t[0] - c.p[0]), 1.1);
            }
        }
        (down + stream(c.p, self.t), 1.0)
    }

    fn walk(&mut self, i: usize, dt: f32, sun: f32) {
        let c = &self.crabs[i];
        let (aim, go) = self.wish(c, sun);
        if go == 0.0 && c.state == State::Walk {
            self.crabs[i].state = State::Shade;
            return;
        }
        // turn toward the wish, a little at a time
        let mut turn = (aim - c.heading).rem_euclid(std::f32::consts::TAU);
        if turn > std::f32::consts::PI {
            turn -= std::f32::consts::TAU;
        }
        let heading = c.heading + turn.clamp(-3.0 * dt, 3.0 * dt);
        let gait = c.gait + dt * 7.0 * c.pace;
        let stride = SPEED * c.pace * go * (0.55 + 0.45 * gait.sin().max(-0.4));
        let mut v = [heading.cos() * stride, heading.sin() * stride];
        let push = self.crowding(i);
        v[0] += push[0];
        v[1] += push[1];
        let mut p = c.p;
        let mut along = c.along;
        let tunnel = self.in_tunnel(p);
        if tunnel {
            v = [0.0, SPEED * c.pace * 1.2];
        } else {
            let hit = self.fences.touch(&mut p, &mut v, &mut along, self.on[1]);
            if !hit {
                along = 0.0;
            }
        }
        p[0] = (p[0] + v[0] * dt).clamp(0.02, self.w - 0.02);
        p[1] = (p[1] + v[1] * dt).min(self.sea + 0.1);
        let c = &mut self.crabs[i];
        c.p = p;
        c.heading = heading;
        c.gait = gait;
        c.along = along;
    }

    /// A shove away from crabs too close (metres a second).
    fn crowding(&self, i: usize) -> [f32; 2] {
        let p = self.crabs[i].p;
        let g = &self.grid;
        let (cx, cy) = g.at(p);
        let mut push = [0.0f32; 2];
        for y in cy.saturating_sub(1)..(cy + 2).min(g.gh) {
            for x in cx.saturating_sub(1)..(cx + 2).min(g.gw) {
                let mut j = g.head[y * g.gw + x];
                while j != NONE {
                    let o = &self.crabs[j as usize];
                    j = g.next[j as usize];
                    if o.state == State::Rest && self.crabs[i].state != State::Rest {
                        continue;
                    }
                    let d = [p[0] - o.p[0], p[1] - o.p[1]];
                    let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
                    if l > 1e-5 && l < ROOM {
                        let k = (ROOM - l) / ROOM * 1.4 / l;
                        push[0] += d[0] * k;
                        push[1] += d[1] * k;
                    }
                }
            }
        }
        push
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crabs_reach_the_sea_and_the_fence_leads_them_under_the_road() {
        let mut island = Island::new(7.0, 15.0, 7);
        island.on[2] = false;
        let mut crossed_open_road = 0;
        for n in 0..600 {
            island.step(1.0 / 30.0);
            for c in island.crabs.iter().filter(|_| n > 150) {
                let on_road = c.p[1] > island.road[0] + 0.3 && c.p[1] < island.road[1] - 0.3;
                if on_road && !island.in_tunnel(c.p) {
                    crossed_open_road += 1;
                }
            }
        }
        let at_sea = island
            .crabs
            .iter()
            .filter(|c| c.state == State::Sea)
            .count();
        assert!(at_sea > 0, "no crab reached the sea");
        assert!(
            crossed_open_road < 4000,
            "{crossed_open_road} crab-steps on the open road"
        );
    }

    #[test]
    fn without_rain_they_stop() {
        let mut island = Island::new(7.0, 15.0, 7);
        island.on[0] = false;
        for _ in 0..150 {
            island.step(1.0 / 30.0);
        }
        let walking = island
            .crabs
            .iter()
            .filter(|c| c.state == State::Walk && c.p[1] < island.road[0])
            .count();
        assert!(walking < 40, "{walking} still walking in the forest");
    }
}
