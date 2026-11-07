//! The salt and its drivers. Cars on open ground with no roads, lanes or signs (the Bonneville Salt
//! Flats at night). Each driver does three things and nothing else:
//!
//! 1. **Follow the tail lights ahead.** Of the cars in front of you, inside your headlights' reach and
//!    cone and going roughly your way, steer after the nearest, unless someone else is already right
//!    behind it (a car has one follower). With no one ahead, wander, a little slower, and turn back from
//!    the dark at the edge.
//! 2. **Keep a safe gap** to whatever is in front, by the Intelligent Driver Model (Treiber, Hennecke &
//!    Helbing 2000), the same rule as the 22 cars on the Nagoya ring (Sugiyama et al. 2008): speed up
//!    toward your own speed, ease off by how close the car ahead is and how fast you are closing.
//! 3. **Turn no sharper than a car turns** (a small car's turning circle, about 5 m radius), and
//!    slower the slower you go.
//!
//! And people get bored: after a while behind the same car a driver peels off and wanders. From these:
//! lone cars string into convoys, convoys snake, merge and split, and a convoy whose head finds its own
//! tail closes into a ring by itself (as ants do), which in time breaks open again. Brake one car hard
//! and the cars behind it brake harder, until some stop: the jam that runs backward down a convoy.

pub const CAR: f64 = 4.4;
/// IDM: acceleration a and comfortable braking b (m/s²), time gap T (s), standstill gap s0 (m). Each
/// car's own desired speed is drawn between V_LO and V_HI.
pub const A: f64 = 1.6;
pub const B: f64 = 2.6;
pub const T: f64 = 0.8;
pub const S0: f64 = 2.2;
pub const V_LO: f64 = 7.5;
pub const V_HI: f64 = 10.5;
/// The tightest turn (m radius).
pub const TURN: f64 = 5.5;
/// How far and how wide a driver looks for tail lights (m, radians off the nose).
pub const SEE: f64 = 42.0;
pub const CONE: f64 = 0.95;
/// A tap: brake this hard (m/s²) to a stop, stand this long (s).
pub const TAP_BRAKE: f64 = 7.0;
pub const TAP_HOLD: f64 = 2.0;
/// Brake lights ahead: a driver who sees them within this distance (m) dabs the brakes this hard
/// (m/s²) for this long (s), after a reaction time (s). So a braking car's red passes back down its
/// convoy one car after another, a ripple, and dies out at the end of it.
pub const DAB_SEE: f64 = 30.0;
pub const DAB: f64 = 3.0;
pub const DAB_FOR: f64 = 0.7;
pub const REACT: f64 = 0.35;
/// A held finger on open ground: a light the cars nearby circle at this radius (m), seen this far.
pub const LAMP_R: f64 = 13.0;
pub const LAMP_SEE: f64 = 110.0;

const STEP: f64 = 1.0 / 60.0;

#[derive(Clone, Debug)]
pub struct Car {
    pub p: [f64; 2],
    /// Heading (radians, world: x right, y up).
    pub h: f64,
    pub v: f64,
    pub acc: f64,
    /// Steering curvature now (1/m), for the wheels' look.
    pub k: f64,
    pub v0: f64,
    pub leader: Option<usize>,
    /// Seconds behind the present leader's convoy, and how long this driver stands it.
    pub following: f64,
    pub patience: f64,
    /// Seconds left wandering off on its own (bored), and the way it swerved.
    pub bored: f64,
    pub swerve: f64,
    pub stopping: bool,
    pub brake: f64,
    /// The brake lights, eased 0..1.
    pub lights: f64,
    /// Seconds held up by a car that isn't the one it follows (a car across its nose).
    pub blocked: f64,
    /// What set the speed last tick: 0 the car ahead it follows (or none), 1 another car across or
    /// ahead of it, 2 a tap.
    pub cause: u8,
    /// Brake lights ahead seen (the leader's were on last tick); seconds until this driver reacts,
    /// and seconds left of its own dab on the brakes.
    seen: bool,
    react: f64,
    dab: f64,
    wander: f64,
    /// The way a driver on its own means to go (radians).
    aim: f64,
}

impl Car {
    pub fn dir(&self) -> [f64; 2] {
        [self.h.cos(), self.h.sin()]
    }
    pub fn rear(&self) -> [f64; 2] {
        let d = self.dir();
        [self.p[0] - d[0] * CAR * 0.5, self.p[1] - d[1] * CAR * 0.5]
    }
    pub fn front(&self) -> [f64; 2] {
        let d = self.dir();
        [self.p[0] + d[0] * CAR * 0.5, self.p[1] + d[1] * CAR * 0.5]
    }
}

/// What a finger is doing to the salt.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hand {
    /// A car led by the finger at a point.
    Lead(usize, [f64; 2]),
    /// A light held on open ground.
    Lamp([f64; 2]),
}

pub struct Flat {
    pub cars: Vec<Car>,
    /// Half the field's width and height (m): what is on screen. Cars turn back from its edge.
    pub half: [f64; 2],
    pub hands: Vec<Hand>,
    pub t: f64,
    rng: u64,
    left: f64,
}

fn wrap(a: f64) -> f64 {
    let mut a = a % std::f64::consts::TAU;
    if a > std::f64::consts::PI {
        a -= std::f64::consts::TAU;
    }
    if a < -std::f64::consts::PI {
        a += std::f64::consts::TAU;
    }
    a
}

fn idm(v: f64, v0: f64, gap: f64, dv: f64) -> f64 {
    let ss = S0 + (v * T + v * dv / (2.0 * (A * B).sqrt())).max(0.0);
    let free = 1.0 - (v / v0).powi(4);
    A * (free - (ss / gap.max(0.3)).powi(2))
}

impl Flat {
    /// A field of `half` (m) with about one car to `area` m², some already strung into convoys, the
    /// night already under way.
    pub fn new(seed: u64, half: [f64; 2], n: usize) -> Flat {
        let mut f = Flat {
            cars: Vec::with_capacity(n),
            half,
            hands: Vec::new(),
            t: 0.0,
            rng: seed.max(1),
            left: 0.0,
        };
        // the night under way: most cars already strung into a few long convoys spread over the salt,
        // each laid along a gentle curve from its own part of the field, a few loners between
        let mut placed = 0;
        let convoys = (n / 11).clamp(2, 8);
        let len = (n * 8 / 10 / convoys).max(4);
        let cols = if convoys > 3 { 2 } else { 1 };
        let rows = convoys.div_ceil(cols);
        for c in 0..convoys {
            let (cx, cy) = (
                ((c % cols) as f64 + 0.5) / cols as f64,
                ((c / cols) as f64 + 0.5) / rows as f64,
            );
            let mut p = [
                (cx - 0.5) * 1.3 * half[0] + (f.uni() - 0.5) * 20.0,
                (cy - 0.5) * 1.3 * half[1] + (f.uni() - 0.5) * 20.0,
            ];
            let mut h = f.uni() * std::f64::consts::TAU;
            let bend = (f.uni() - 0.5) * 0.05;
            // the convoy laid back from its middle, so it sits in its own part of the salt
            let gap = CAR + S0 + 5.0;
            p = [
                p[0] + h.cos() * gap * len as f64 * 0.5,
                p[1] + h.sin() * gap * len as f64 * 0.5,
            ];
            for _ in 0..len {
                if placed >= n {
                    break;
                }
                f.push_car(p, h);
                placed += 1;
                p = [p[0] - h.cos() * gap, p[1] - h.sin() * gap];
                h -= bend * gap;
            }
        }
        while placed < n {
            let p = [
                (f.uni() - 0.5) * 1.8 * half[0],
                (f.uni() - 0.5) * 1.8 * half[1],
            ];
            let h = f.uni() * std::f64::consts::TAU;
            f.push_car(p, h);
            placed += 1;
        }
        for c in f.cars.iter_mut() {
            c.p = [
                c.p[0].clamp(-0.85 * half[0], 0.85 * half[0]),
                c.p[1].clamp(-0.85 * half[1], 0.85 * half[1]),
            ];
        }
        f.separate(60);
        f
    }

    fn push_car(&mut self, p: [f64; 2], h: f64) {
        let v0 = V_LO + (V_HI - V_LO) * self.uni();
        let patience = self.patience();
        let wander = self.uni() * 100.0;
        let following = self.uni() * patience * 0.6;
        self.cars.push(Car {
            p,
            h,
            v: v0 * 0.8,
            acc: 0.0,
            k: 0.0,
            v0,
            leader: None,
            following,
            patience,
            bored: 0.0,
            swerve: 0.0,
            stopping: false,
            brake: 0.0,
            lights: 0.0,
            blocked: 0.0,
            cause: 0,
            seen: false,
            react: 0.0,
            dab: 0.0,
            wander,
            aim: h,
        });
    }

    fn patience(&mut self) -> f64 {
        25.0 + 45.0 * self.uni()
    }

    pub fn uni(&mut self) -> f64 {
        self.rng ^= self.rng >> 12;
        self.rng ^= self.rng << 25;
        self.rng ^= self.rng >> 27;
        ((self.rng.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 + 0.5) / (1u64 << 53) as f64
    }

    /// Pushes overlapping cars apart (`rounds` times).
    fn separate(&mut self, rounds: usize) {
        let n = self.cars.len();
        let min = CAR * 1.05;
        for _ in 0..rounds {
            let mut moved = false;
            for i in 0..n {
                for j in i + 1..n {
                    let d = [
                        self.cars[j].p[0] - self.cars[i].p[0],
                        self.cars[j].p[1] - self.cars[i].p[1],
                    ];
                    let l = d[0].hypot(d[1]);
                    if l < min {
                        let u = if l > 1e-6 {
                            [d[0] / l, d[1] / l]
                        } else {
                            [1.0, 0.0]
                        };
                        let push = 0.5 * (min - l);
                        self.cars[i].p[0] -= u[0] * push;
                        self.cars[i].p[1] -= u[1] * push;
                        self.cars[j].p[0] += u[0] * push;
                        self.cars[j].p[1] += u[1] * push;
                        moved = true;
                    }
                }
            }
            if !moved {
                break;
            }
        }
    }

    pub fn tap(&mut self, i: usize) {
        if let Some(c) = self.cars.get_mut(i) {
            c.stopping = true;
            c.brake = TAP_HOLD;
        }
    }

    pub fn step(&mut self, dt: f64) {
        self.left += dt;
        while self.left >= STEP {
            self.left -= STEP;
            self.tick(STEP);
        }
    }

    /// The cars behind car `i`, in order, while each follows the one before.
    pub fn behind(&self, i: usize) -> Vec<usize> {
        let mut follower = vec![usize::MAX; self.cars.len()];
        for (k, c) in self.cars.iter().enumerate() {
            if let Some(j) = c.leader {
                follower[j] = k;
            }
        }
        let mut out = Vec::new();
        let mut at = i;
        while follower[at] != usize::MAX && follower[at] != i && out.len() < self.cars.len() {
            at = follower[at];
            out.push(at);
        }
        out
    }

    /// Whether car `i` is in a ring: its leaders lead back to it.
    pub fn in_ring(&self, i: usize) -> bool {
        let mut at = i;
        for _ in 0..self.cars.len() {
            match self.cars[at].leader {
                Some(j) if j == i => return true,
                Some(j) => at = j,
                None => return false,
            }
        }
        false
    }

    fn led(&self, i: usize) -> Option<[f64; 2]> {
        self.hands.iter().find_map(|h| match *h {
            Hand::Lead(k, p) if k == i => Some(p),
            _ => None,
        })
    }

    fn choose_leaders(&mut self) {
        let n = self.cars.len();
        // each car's best few candidates, scored by distance and how far off its nose
        let mut pairs: Vec<(f64, usize, usize)> = Vec::new();
        for i in 0..n {
            let c = &self.cars[i];
            if c.bored > 0.0 || self.led(i).is_some() {
                continue;
            }
            let d = c.dir();
            let mut best: Vec<(f64, usize)> = Vec::new();
            for j in 0..n {
                if j == i {
                    continue;
                }
                let o = &self.cars[j];
                let r = o.rear();
                let w = [r[0] - c.p[0], r[1] - c.p[1]];
                let dist = w[0].hypot(w[1]);
                let keep = c.leader == Some(j);
                let reach = if keep { SEE * 1.25 } else { SEE };
                if dist > reach || dist < 0.5 {
                    continue;
                }
                // not one cut in right under your nose (you'd have to stamp on the brake)
                if !keep && dist < CAR + S0 + c.v * T * 0.8 {
                    continue;
                }
                let off = ((w[0] * d[0] + w[1] * d[1]) / dist).clamp(-1.0, 1.0).acos();
                let cone = if keep { CONE * 1.5 } else { CONE };
                if off > cone {
                    continue;
                }
                // going roughly the same way (no chasing a car coming at you)
                if (o.h - c.h).cos() < if keep { -0.2 } else { 0.4 } {
                    continue;
                }
                let mut s = dist * (1.0 + 1.6 * off);
                if keep {
                    s *= 0.45;
                }
                best.push((s, j));
            }
            best.sort_by(|a, b| a.0.total_cmp(&b.0));
            for &(s, j) in best.iter().take(3) {
                pairs.push((s, i, j));
            }
        }
        pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut has_leader = vec![false; n];
        let mut taken = vec![false; n];
        let old: Vec<Option<usize>> = self.cars.iter().map(|c| c.leader).collect();
        for c in self.cars.iter_mut() {
            c.leader = None;
        }
        for (_, i, j) in pairs {
            if has_leader[i] || taken[j] {
                continue;
            }
            has_leader[i] = true;
            taken[j] = true;
            self.cars[i].leader = Some(j);
        }
        for (c, was) in self.cars.iter_mut().zip(old) {
            // alone again: patience recovers a little
            if c.leader.is_none() && was.is_some() {
                c.following = (c.following - 2.0).max(0.0);
            }
        }
    }

    fn tick(&mut self, dt: f64) {
        self.t += dt;
        // leaders are chosen a few times a second (a driver's glance), not every tick
        if (self.t / dt).round() as i64 % 6 == 0 {
            self.choose_leaders();
        }
        let n = self.cars.len();
        let lamps: Vec<[f64; 2]> = self
            .hands
            .iter()
            .filter_map(|h| {
                if let Hand::Lamp(p) = h {
                    Some(*p)
                } else {
                    None
                }
            })
            .collect();
        // who is in a train behind a car the finger leads
        let mut pulled = vec![false; n];
        for h in &self.hands {
            if let Hand::Lead(k, _) = *h {
                for j in self.behind(k) {
                    pulled[j] = true;
                }
            }
        }
        let mut out: Vec<(f64, f64, bool, u8)> = Vec::with_capacity(n); // (acc, desired heading change, held up)
        for i in 0..n {
            let c = &self.cars[i];
            let d = c.dir();
            let mut want;
            let mut acc;
            let led = self.led(i);
            if let Some(f) = led {
                // led by a finger: to the point, slowing as it nears
                let w = [f[0] - c.p[0], f[1] - c.p[1]];
                let dist = w[0].hypot(w[1]);
                want = w[1].atan2(w[0]);
                // as fast as the finger is far, slower while it turns toward it (no orbiting the finger)
                // on a string: as fast as the finger is far (a finger moves faster than any car)
                let off = wrap(want - c.h).cos().max(0.5);
                let vdes = (dist * 3.0).min(26.0) * off;
                acc = (vdes - c.v) * 6.0;
            } else if let Some(j) = c.leader {
                let o = &self.cars[j];
                // after the leader's rear, a little behind it so the follower takes the leader's line
                let r = o.rear();
                let aim = [r[0] - o.h.cos() * 2.0, r[1] - o.h.sin() * 2.0];
                want = (aim[1] - c.p[1]).atan2(aim[0] - c.p[0]);
                let f = c.front();
                let gap = (r[0] - f[0]).hypot(r[1] - f[1]);
                let closing = c.v - o.v * (o.h - c.h).cos();
                // a train behind a car the finger pulls hurries to keep up
                let v0 = if pulled[i] { c.v0 * 1.8 } else { c.v0 };
                acc = idm(c.v, v0, gap, closing);
            } else {
                // wandering: a slow meander, a little slower than the driver would go behind someone
                // a heading of its own that drifts slowly (never a steady turn: no curling up)
                let w = (c.wander + self.t * 0.05).sin() * 0.5
                    + (c.wander * 1.7 + self.t * 0.11).sin() * 0.3;
                want = c.aim + w + c.swerve * (c.bored / 4.0).min(1.0);
                acc = idm(c.v, c.v0 * 0.78, 1e3, 0.0);
            }
            // a held light: drivers without anyone to follow (and heads of convoys) circle it
            if led.is_none() && c.leader.is_none() {
                for l in &lamps {
                    let w = [l[0] - c.p[0], l[1] - c.p[1]];
                    let dist = w[0].hypot(w[1]);
                    if dist < LAMP_SEE {
                        let to = w[1].atan2(w[0]);
                        // the tangent, leaning in or out toward the circle
                        // each driver its own orbit, so they circle in lanes rather than pile up
                        let r = LAMP_R * (1.0 + 0.9 * (c.wander * 7.3).sin().abs());
                        let lean = ((dist - r) / r).clamp(-1.0, 1.0) * 0.9;
                        want = to - std::f64::consts::FRAC_PI_2 + lean;
                        acc = idm(c.v, c.v0 * 0.85, 1e3, 0.0);
                    }
                }
            }
            // the dark at the edge: turn back toward the middle
            // a driver on its own drifts back toward the middle well before the edge; a convoy's
            // follower only once it is nearly off the salt
            let u = (c.p[0].abs() / self.half[0]).max(c.p[1].abs() / self.half[1]);
            let (from, over) = if c.leader.is_none() {
                (0.72, 0.3)
            } else {
                (0.9, 0.1)
            };
            if u > from && led.is_none() {
                let home = (-c.p[1]).atan2(-c.p[0]);
                let wgt = ((u - from) / over).clamp(0.0, 1.0);
                let cur = wrap(want - c.h);
                let mut to_home = wrap(home - c.h);
                // facing straight out, keep turning the way it already turns (no dithering at ±π)
                if to_home.abs() > 2.4 {
                    to_home = to_home.abs() * if c.k >= 0.0 { 1.0 } else { -1.0 };
                }
                let mix = cur * (1.0 - wgt) + to_home * wgt;
                want = c.h + mix;
            }
            // whatever else is in front: keep the gap and steer a little away; held up a while by
            // someone across its nose, a driver creeps round them
            let follow_acc = acc;
            let mut cause = 0u8;
            let mut held = false;
            for (j, o) in self.cars.iter().enumerate() {
                // a car the finger leads goes where it is led (it nudges through; nothing overlaps)
                if j == i || Some(j) == c.leader || led.is_some() {
                    continue;
                }
                let w = [o.p[0] - c.p[0], o.p[1] - c.p[1]];
                let along = w[0] * d[0] + w[1] * d[1];
                if along <= 0.0 || along > 26.0 {
                    continue;
                }
                let side = -w[0] * d[1] + w[1] * d[0];
                if side.abs() > 5.0 {
                    continue;
                }
                // only what is truly ahead (not alongside) holds a driver up
                let ahead = side.abs() < 2.1 && along > CAR * 0.6;
                let same = (o.h - c.h).cos();
                let gap = along - CAR;
                // open ground: drivers go round one another, and brake only when about to touch
                if ahead && gap < 2.5 {
                    let closing = c.v - o.v * same.max(0.0);
                    let brake = idm(c.v, c.v0, gap.max(0.2), closing.max(0.0) + 0.5);
                    // across your nose: ease off and swerve; only about to touch is a hard stop
                    acc = acc.min(if same > 0.3 || gap < 0.8 {
                        brake
                    } else {
                        brake.max(-2.5)
                    });
                }
                held |= ahead && gap < 3.0 && !(same > 0.3 && o.v > 1.0);
                let push = (1.0 - along / 26.0)
                    * if same > 0.3 { 0.8 } else { 2.0 }
                    * if c.blocked > 0.6 { 2.5 } else { 1.0 };
                // weave behind a crossing car: away from where it is, toward where it came from
                let s = if side.abs() < 0.3 && same <= 0.3 {
                    -((o.h - c.h).sin()).signum()
                } else {
                    side.signum()
                };
                want -= s * push;
            }
            if c.blocked > 0.6 && acc < follow_acc {
                // creep: no faster than a walk, wheel turned away
                acc = follow_acc.min(idm(c.v, 1.6, 1e3, 0.0));
            }
            if acc < follow_acc - 0.01 {
                cause = 1;
            }
            // a dab on the brakes for the brake lights ahead
            if c.dab > 0.0 && c.v > 1.0 {
                acc = acc.min(-DAB);
                cause = 0;
            }
            if c.stopping || c.brake > 0.0 && c.v < 0.05 {
                cause = 2;
                acc = if c.v > 0.0 { -TAP_BRAKE } else { 0.0 };
            }
            let held = held && c.v < 1.0;
            let mut dh = wrap(want - c.h);
            if dh.abs() > 2.6 {
                dh = dh.abs() * if c.k >= 0.0 { 1.0 } else { -1.0 };
            }
            out.push((acc, dh, held, cause));
        }
        let half = self.half;
        let pulled = pulled;
        let ahead_lit: Vec<bool> = (0..n)
            .map(|i| match self.cars[i].leader {
                Some(j) => {
                    let (a, b) = (&self.cars[i], &self.cars[j]);
                    b.lights > 0.6 && (a.p[0] - b.p[0]).hypot(a.p[1] - b.p[1]) < DAB_SEE
                }
                None => false,
            })
            .collect();
        for i in 0..n {
            let (acc, dh, held, cause) = out[i];
            let led = self.led(i).is_some();
            let c = &mut self.cars[i];
            c.cause = cause;
            // brake lights ahead come on: react, then dab
            if ahead_lit[i] && !c.seen && c.react <= 0.0 && c.dab <= 0.0 {
                c.react = REACT;
            }
            c.seen = ahead_lit[i];
            if c.react > 0.0 {
                c.react -= dt;
                if c.react <= 0.0 {
                    c.dab = DAB_FOR;
                }
            }
            if c.dab > 0.0 {
                c.dab -= dt;
            }
            c.blocked = if held {
                c.blocked + dt
            } else {
                (c.blocked - dt * 2.0).max(0.0)
            };
            let wobble = 0.0;
            c.acc = (acc + wobble).clamp(
                if led { -40.0 } else { -9.0 },
                if led {
                    40.0
                } else if pulled[i] {
                    A * 2.5
                } else {
                    A * 1.2
                },
            );
            c.v = (c.v + c.acc * dt).max(0.0);
            if c.stopping && c.v <= 0.0 {
                c.stopping = false;
            }
            if !c.stopping && c.v <= 0.0 && c.brake > 0.0 {
                c.brake -= dt;
            }
            // steer: toward the wanted heading, never sharper than the turning circle
            // a led car swings round on its string; a driven one turns as a car turns
            let kmax = if led { 1.0 / 1.2 } else { 1.0 / TURN };
            let gain = if led { 4.0 } else { 1.6 };
            let k = (dh * gain / c.v.max(2.0)).clamp(-kmax, kmax);
            c.k += (k - c.k) * (dt * if led { 20.0 } else { 6.0 }).min(1.0);
            c.h = wrap(c.h + c.v * c.k * dt);
            c.p[0] += c.h.cos() * c.v * dt;
            c.p[1] += c.h.sin() * c.v * dt;
            // brake lights for stamping on it behind the car you follow; easing off round a stranger barely shows
            let hard = if c.cause == 1 { -6.5 } else { -3.4 };
            let braking = (c.dab > 0.0 && !led)
                || (c.acc < hard && !led)
                || (c.v < 0.4
                    && (c.brake > 0.0 || c.stopping || (c.leader.is_some() && c.cause != 1)));
            let target = if braking { 1.0 } else { 0.0 };
            c.lights += (target - c.lights) * (dt * if braking { 14.0 } else { 3.0 }).min(1.0);
            // patience: a while behind the same convoy, and the driver peels off
            if c.leader.is_some() || led {
                c.following += dt;
            }
            // whoever isn't wandering freely (following, led, turned back at the edge, circling a light)
            // carries on the way it was going when it is next on its own
            let edge = (c.p[0].abs() / half[0]).max(c.p[1].abs() / half[1]) > 0.72;
            if c.leader.is_some()
                || led
                || edge
                || lamps
                    .iter()
                    .any(|l| (l[0] - c.p[0]).hypot(l[1] - c.p[1]) < LAMP_SEE)
            {
                c.aim = c.h;
            }
            if c.bored > 0.0 {
                c.bored -= dt;
            }
        }
        // in a ring, drivers tire of it sooner (round and round)
        if (self.t / dt).round() as i64 % 30 == 0 {
            for i in 0..n {
                if self.in_ring(i) {
                    self.cars[i].following += 30.0 * dt * 2.0;
                }
            }
        }
        // the bored
        for i in 0..n {
            if self.cars[i].leader.is_some()
                && self.cars[i].following > self.cars[i].patience
                && self.cars[i].v > 1.0
            {
                let sw = if self.uni() < 0.5 { -1.0 } else { 1.0 } * (0.5 + 0.5 * self.uni());
                let bored = 4.0 + 4.0 * self.uni();
                let patience = self.patience();
                let c = &mut self.cars[i];
                c.leader = None;
                c.bored = bored;
                c.swerve = sw;
                c.following = 0.0;
                c.patience = patience;
            }
        }
        // no two cars in one place
        self.separate(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(f: &mut Flat, s: f64) {
        f.step(s);
    }

    fn following(f: &Flat) -> f64 {
        f.cars.iter().filter(|c| c.leader.is_some()).count() as f64 / f.cars.len() as f64
    }

    #[test]
    fn convoys_form_and_keep_forming() {
        let mut f = Flat::new(7, [75.0, 160.0], 100);
        let mut low: f64 = 1.0;
        let mut high: f64 = 0.0;
        for _ in 0..12 {
            run(&mut f, 10.0);
            let q = following(&f);
            low = low.min(q);
            high = high.max(q);
        }
        // most cars are in some convoy most of the time, and it never all locks up or all falls apart
        assert!(low > 0.35, "low {low}");
        assert!(high < 0.97, "high {high}");
    }

    #[test]
    fn never_all_stuck() {
        // no gridlock: after the first minutes, the cars keep moving on the whole
        for seed in [1, 2, 3] {
            let mut f = Flat::new(seed, [92.0, 199.0], 43);
            let mut slow = 0.0f64;
            for _ in 0..30 {
                run(&mut f, 10.0);
                let mean = f.cars.iter().map(|c| c.v / c.v0).sum::<f64>() / f.cars.len() as f64;
                let still =
                    f.cars.iter().filter(|c| c.v < 0.5).count() as f64 / f.cars.len() as f64;
                slow = slow.max(still);
                assert!(mean > 0.4, "seed {seed} at {:.0}: mean {mean:.2}", f.t);
            }
            assert!(slow < 0.35, "seed {seed}: {slow} standing at worst");
        }
    }

    #[test]
    fn no_car_runs_through_another() {
        let mut f = Flat::new(3, [75.0, 160.0], 110);
        let mut worst = f64::MAX;
        for _ in 0..600 {
            run(&mut f, 0.1);
            for i in 0..f.cars.len() {
                for j in i + 1..f.cars.len() {
                    let d =
                        (f.cars[i].p[0] - f.cars[j].p[0]).hypot(f.cars[i].p[1] - f.cars[j].p[1]);
                    worst = worst.min(d);
                }
            }
        }
        assert!(worst > CAR * 0.7, "closest {worst}");
    }

    #[test]
    fn cars_stay_on_the_field() {
        let mut f = Flat::new(11, [75.0, 160.0], 100);
        for _ in 0..300 {
            run(&mut f, 1.0);
            for c in &f.cars {
                assert!(
                    c.p[0].abs() < 75.0 + 30.0 && c.p[1].abs() < 160.0 + 30.0,
                    "{:?}",
                    c.p
                );
            }
        }
    }

    #[test]
    fn a_tap_stops_the_cars_behind() {
        // the longest convoy after a while; tap its head and count the cars behind that come to a stop
        let mut f = Flat::new(5, [75.0, 160.0], 100);
        run(&mut f, 20.0);
        let head = (0..f.cars.len())
            .filter(|&i| f.cars[i].leader.is_none())
            .max_by_key(|&i| f.behind(i).len())
            .unwrap();
        let tail = f.behind(head);
        assert!(tail.len() >= 4, "a convoy of {}", tail.len());
        f.tap(head);
        let mut stopped = std::collections::BTreeSet::new();
        for _ in 0..80 {
            run(&mut f, 0.1);
            for &k in &tail {
                if f.cars[k].v < 1.0 {
                    stopped.insert(k);
                }
            }
        }
        assert!(
            stopped.len() >= 3,
            "only {} of {} stopped",
            stopped.len(),
            tail.len()
        );
    }

    #[test]
    fn rings_come_and_go() {
        let mut f = Flat::new(9, [75.0, 160.0], 110);
        let mut ever = false;
        let mut longest = 0.0f64;
        let mut cur = 0.0f64;
        for _ in 0..1200 {
            run(&mut f, 0.5);
            let any = (0..f.cars.len()).any(|i| f.in_ring(i));
            ever |= any;
            cur = if any { cur + 0.5 } else { 0.0 };
            longest = longest.max(cur);
        }
        eprintln!("ring ever {ever}, longest {longest} s");
        assert!(longest < 240.0, "a ring held {longest} s");
    }

    #[test]
    fn a_held_light_gathers_cars() {
        let mut f = Flat::new(13, [75.0, 160.0], 100);
        run(&mut f, 5.0);
        f.hands.push(Hand::Lamp([0.0, 0.0]));
        run(&mut f, 30.0);
        let near = f
            .cars
            .iter()
            .filter(|c| c.p[0].hypot(c.p[1]) < LAMP_R * 2.2)
            .count();
        assert!(near >= 5, "{near} near the light");
    }
}

#[cfg(test)]
mod lead {
    use super::*;

    #[test]
    fn a_led_car_comes_to_the_finger() {
        let mut f = Flat::new(5, [110.0, 240.0], 120);
        f.step(5.0);
        let i = 7;
        let to = [f.cars[i].p[0] + 30.0, f.cars[i].p[1] - 40.0];
        f.hands = vec![Hand::Lead(i, to)];
        let d0 = (f.cars[i].p[0] - to[0]).hypot(f.cars[i].p[1] - to[1]);
        // even with the finger behind it, the car turns round and comes within six seconds
        f.step(6.0);
        let d = (f.cars[i].p[0] - to[0]).hypot(f.cars[i].p[1] - to[1]);
        assert!(d < 8.0, "from {d0:.0} m to {d:.0} m");
    }

    #[test]
    fn a_led_car_pulls_its_convoy() {
        let mut f = Flat::new(5, [92.0, 199.0], 43);
        f.step(12.0);
        let head = (0..f.cars.len())
            .filter(|&i| f.cars[i].leader.is_none())
            .max_by_key(|&i| f.behind(i).len())
            .unwrap();
        let before = f.behind(head).len();
        // the finger walks the head on a slow curve, about 12 m/s (a finger crossing the phone in 6 s)
        let p0 = f.cars[head].p;
        let d = f.cars[head].dir();
        for k in 0..80 {
            let s = k as f64 * 0.1 * 12.0;
            let a = s / 60.0;
            let to = [
                p0[0] + d[0] * 30.0 * a.sin() + -d[1] * 30.0 * (1.0 - a.cos()),
                p0[1] + d[1] * 30.0 * a.sin() + d[0] * 30.0 * (1.0 - a.cos()),
            ];
            f.hands = vec![Hand::Lead(head, to)];
            f.step(0.1);
        }
        let after = f.behind(head).len();
        eprintln!("train {before} -> {after}");
        assert!(after >= 2.min(before), "train of {before} left {after}");
    }
}

#[cfg(test)]
mod quiet {
    use super::*;

    /// Untouched, brake lights are rare (so a tap's chain of stops is the red on screen).
    #[test]
    fn untouched_brake_lights_are_rare() {
        let (mut lit, mut hard, mut stand, mut blocked, mut n) = (0, 0, 0, 0, 0);
        for seed in [21, 22, 23, 24] {
            let mut f = Flat::new(seed, [110.0, 240.0], 62);
            f.step(20.0);
            for _ in 0..120 {
                f.step(0.5);
                for c in &f.cars {
                    n += 1;
                    if c.lights > 0.5 {
                        lit += 1;
                        if c.acc < -1.6 {
                            hard += 1;
                        }
                        if c.v < 0.4 {
                            stand += 1;
                        }
                        if c.cause == 1 {
                            blocked += 1;
                        }
                    }
                }
            }
        }
        let q = lit as f64 / n as f64;
        eprintln!("lit {q:.3} (hard {hard} standing {stand} blocked {blocked} of {lit})");
        assert!(q < 0.06, "lit {q:.3}");
    }
}

#[cfg(test)]
mod chain {
    use super::*;

    /// A tap on a convoy's head: the cars behind it light up one after another, in order.
    #[test]
    fn a_tap_lights_the_convoy_in_turn() {
        let mut f = Flat::new(5, [92.0, 199.0], 43);
        f.step(12.0);
        let head = (0..f.cars.len())
            .filter(|&i| f.cars[i].leader.is_none())
            .max_by_key(|&i| f.behind(i).len())
            .unwrap();
        let tail = f.behind(head);
        f.tap(head);
        let mut first = vec![f64::NAN; tail.len()];
        for k in 0..100 {
            f.step(0.05);
            for (n, &c) in tail.iter().enumerate() {
                if first[n].is_nan() && f.cars[c].lights > 0.6 {
                    first[n] = k as f64 * 0.05;
                }
            }
        }
        eprintln!("convoy of {}: lit at {:?}", tail.len(), first);
        let lit = first.iter().take(4).filter(|t| !t.is_nan()).count();
        assert!(
            lit >= 3.min(tail.len()),
            "only {lit} of the first cars behind lit"
        );
    }
}
