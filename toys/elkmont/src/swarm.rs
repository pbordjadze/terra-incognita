//! The swarm: Photinus carolinus males flying in the woods at Elkmont, each flashing in bursts and
//! resting in the dark between them, and the females in the leaf litter answering.
//!
//! The rule is Sarfati et al.'s (eLife 12:e78908, 2023): alone, a male has no rhythm of his own; after
//! a burst he rests, then waits a random while (broad, often tens of seconds) before the next. But once
//! rested, any flash he sees sets him off at once. So whoever starts first starts them all, and the
//! swarm's beat is the shortest wait among many: a steady rhythm no one of them keeps. Within a burst
//! each flashes about twice a second and is pulled into step with the flashes he sees.
//!
//! Units: metres and seconds. x right, y up, z away from the watcher (who stands at the origin).

/// The woods the flies keep to: x within ±(z · HALF_WIDE + 2), z within NEAR..FAR, y within LOW..HIGH.
pub const NEAR: f32 = 1.6;
pub const FAR: f32 = 36.0;
pub const HALF_WIDE: f32 = 0.62;
pub const LOW: f32 = 0.25;
pub const HIGH: f32 = 12.0;

/// The males out once the evening is under way, and the first few at dusk.
pub const MALES: usize = 1100;
pub const FIRST_OUT: usize = 90;
/// Seconds over which they come out (the swarm grows to MALES, so its rhythm emerges as it grows).
pub const EMERGE_S: f64 = 70.0;
pub const FEMALES: usize = 90;

/// A flash's length (s): its light rises and falls within this.
pub const FLASH_S: f64 = 0.16;
/// The time between flashes within a burst (s), and its spread from fly to fly.
pub const INTERFLASH_S: f64 = 0.52;
const INTERFLASH_SPREAD: f64 = 0.03;
/// Flashes in a burst (inclusive).
const BURST_MIN: u32 = 4;
const BURST_MAX: u32 = 8;
/// After a burst, the rest in which nothing sets him off (s, mean and spread), measured from the
/// burst's last flash.
const REST_S: f64 = 2.0;
const REST_SPREAD: f64 = 0.45;
/// After the rest, the mean of his own random wait (s; exponential): alone he would flash rarely.
const WAIT_MEAN_S: f64 = 38.0;
/// How far a male sees another's flash (m): the leaves hide farther ones. A rested male is set off
/// by the start of another's burst (its first two flashes), not by one under way.
pub const SIGHT_M: f32 = 30.0;
/// A rested male is ready only once he has seen no flash for this long (s): so a burst's stragglers
/// hold the rest back, and the woods' rests end together.
const DARK_S: f64 = 8.2;
const DARK_SPREAD: f64 = 0.1;
/// From seeing a flash to his own first (s, least and spread).
const ANSWER_S: f64 = 0.22;
const ANSWER_SPREAD: f64 = 0.10;
/// Within a burst, a flash seen this close before his own next one (s) pulls his next one this much
/// of the way toward it.
const PULL_WINDOW_S: f64 = 0.2;
const PULL: f64 = 0.5;
/// Flight: cruising speed (m/s) and how fast his heading wanders (rad/s).
const SPEED: f32 = 0.55;
const WANDER: f32 = 1.4;
/// A female answers the males' burst with a doublet about this long after their last flash (s), this
/// often (per burst she sees).
const FEMALE_DELAY_S: f64 = 2.8;
const FEMALE_ANSWERS: f64 = 0.28;
const DOUBLET_GAP_S: f64 = 0.24;
/// The females who take the watcher's light for a male's: within this of it (m).
const LAMP_FEMALES_M: f32 = 7.0;
/// A male who sees a female answer within this (m) flies to her for this long (s).
const LURE_M: f32 = 9.0;
const LURE_S: f64 = 6.0;

pub fn rand(seed: &mut u64) -> f64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;
    (*seed >> 11) as f64 / (1u64 << 53) as f64
}

fn rand_f(seed: &mut u64, a: f32, b: f32) -> f32 {
    a + (b - a) * rand(seed) as f32
}

fn gauss(seed: &mut u64) -> f64 {
    let u = rand(seed).max(1e-9);
    let v = rand(seed);
    (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Not out yet.
    Hidden,
    /// Resting after a burst: blind to others until `until`.
    Resting { until: f64 },
    /// Rested: flashes on his own at `own`, or at once when he sees a flash.
    Ready { own: f64 },
    /// In a burst: his next flash at `next`, the flashes given, and the most he will give.
    Burst { next: f64, done: u32, most: u32 },
}

#[derive(Clone, Debug)]
pub struct Male {
    pub pos: [f32; 3],
    pub heading: f32,
    pub climb: f32,
    pub state: State,
    /// His own time between flashes.
    pub interflash: f64,
    /// When his last flash began.
    pub last_flash: f64,
    /// Where he was when it began (a flash is drawn as the short streak he flies while it lasts).
    pub flash_from: [f32; 3],
    /// When he last saw another firefly's flash.
    pub saw: f64,
    /// A female he saw answer: where, and until when he flies to her.
    pub lure: Option<([f32; 3], f64)>,
    /// How long the woods must be dark before he is rested (DARK_S, his own).
    pub dark: f64,
}

#[derive(Clone, Debug)]
pub struct Female {
    pub pos: [f32; 3],
    /// Her doublet's two flashes, when scheduled or last given.
    pub flashes: [f64; 2],
}

/// A flash seen this step: where, when.
#[derive(Clone, Copy)]
struct Seen {
    pos: [f32; 3],
    t: f64,
    /// How far it is seen (m).
    sight: f32,
    /// A firefly's (not the watcher's light): only theirs keep the woods from being dark.
    firefly: bool,
}

pub struct Swarm {
    pub males: Vec<Male>,
    pub females: Vec<Female>,
    pub t: f64,
    seed: u64,
    /// The flashes of the last step (seen by others in this one).
    seen: Vec<Seen>,
    /// The watcher's light: its flashes (where, when) to be seen like a male's.
    pending: Vec<Seen>,
    /// When the last flash of any male was (for the females' answers).
    last_any: f64,
    /// A collective burst is under way (some male in a burst).
    bursting: bool,
    /// Males out now (grows to MALES over EMERGE_S).
    pub out: usize,
    /// The light of the whole swarm this step (the sum of the flashes' light, for the woods' glow).
    pub light: f32,
}

pub fn in_woods(p: [f32; 3]) -> bool {
    let half = p[2] * HALF_WIDE + 2.0;
    p[2] >= NEAR && p[2] <= FAR && p[0].abs() <= half && p[1] >= LOW && p[1] <= HIGH
}

fn dist2(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2]
}

impl Swarm {
    pub fn new(seed: u64) -> Swarm {
        let mut s = seed | 1;
        let mut males = Vec::with_capacity(MALES);
        for _ in 0..MALES {
            // more of them near than an even spread would put there: the watcher stands among them
            let z = NEAR + (FAR - NEAR) * (rand(&mut s) as f32).powf(2.0);
            let half = z * HALF_WIDE + 2.0;
            let pos = [
                rand_f(&mut s, -half, half),
                // most fly low, within a metre or so of the ground; some go up among the trees
                if rand(&mut s) < 0.45 {
                    LOW + 1.3 * rand(&mut s) as f32
                } else {
                    LOW + (HIGH - LOW) * (rand(&mut s) as f32).powf(1.5)
                },
                z,
            ];
            males.push(Male {
                pos,
                heading: rand_f(&mut s, 0.0, std::f32::consts::TAU),
                climb: 0.0,
                state: State::Hidden,
                interflash: INTERFLASH_S * (1.0 + INTERFLASH_SPREAD * gauss(&mut s)),
                last_flash: -100.0,
                flash_from: pos,
                saw: -100.0,
                lure: None,
                dark: DARK_S,
            });
        }
        let mut females = Vec::with_capacity(FEMALES);
        for _ in 0..FEMALES {
            // more of them near: the watcher's own stretch of the trail
            let z = 3.0 + 23.0 * (rand(&mut s) as f32).powf(1.8);
            let half = z * HALF_WIDE;
            females.push(Female {
                pos: [rand_f(&mut s, -half, half), rand_f(&mut s, 0.02, 0.3), z],
                flashes: [-100.0, -100.0],
            });
        }
        // the first few are out already, rested
        let mut sw = Swarm {
            males,
            females,
            t: 0.0,
            seed: s,
            seen: Vec::new(),
            pending: Vec::new(),
            last_any: -100.0,
            bursting: false,
            out: 0,
            light: 0.0,
        };
        sw.emerge();
        for m in sw.males[..sw.out].iter_mut() {
            m.saw = -100.0;
        }
        sw
    }

    /// Lets out the males due by now: FIRST_OUT at once, the rest over EMERGE_S, the nearer ones
    /// first in no order (the order is the shuffle of their making).
    fn emerge(&mut self) {
        let u = (self.t / EMERGE_S).clamp(0.0, 1.0);
        let due = FIRST_OUT + ((MALES - FIRST_OUT) as f64 * u * u) as usize;
        while self.out < due.min(self.males.len()) {
            let m = &mut self.males[self.out];
            // out, and resting until the woods round him have been dark a while
            m.state = State::Resting { until: self.t };
            m.saw = self.t;
            self.out += 1;
        }
    }

    /// Everyone out at once (a restored evening).
    pub fn all_out(&mut self) {
        self.t = self.t.max(EMERGE_S);
        self.emerge();
    }

    /// The watcher's light flashes at `pos`: seen as a male's flash is, a little farther (it is bright).
    pub fn lamp(&mut self, pos: [f32; 3]) {
        self.pending.push(Seen {
            pos,
            t: self.t,
            sight: SIGHT_M * 1.4,
            firefly: false,
        });
    }

    /// The watcher's light has given a burst (its last flash now, at `pos`): the females near it take
    /// it for a male's and answer, a few seconds on, with their doublet.
    pub fn lamp_burst_done(&mut self, pos: [f32; 3]) {
        let t = self.t;
        if std::env::var_os("ELKMONT_CENSUS").is_some() {
            let n = self
                .females
                .iter()
                .filter(|f| dist2(f.pos, pos) < LAMP_FEMALES_M * LAMP_FEMALES_M)
                .count();
            println!("lamp burst at {pos:?}: {n} females near");
        }
        let mut seed = self.seed;
        // the nearest few, each perhaps
        let mut near: Vec<(f32, usize)> = (0..self.females.len())
            .map(|i| (dist2(self.females[i].pos, pos), i))
            .filter(|&(d, _)| d < LAMP_FEMALES_M * LAMP_FEMALES_M)
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        for &(_, i) in near.iter().take(4) {
            if rand(&mut seed) < 0.85 {
                let at = t + FEMALE_DELAY_S + 0.5 * rand(&mut seed);
                self.females[i].flashes = [at, at + DOUBLET_GAP_S];
                if std::env::var_os("ELKMONT_CENSUS").is_some() {
                    println!("female {i} answers at {at:.2}");
                }
            }
        }
        self.seed = seed;
    }

    fn start_burst(m: &mut Male, first: f64, seed: &mut u64) {
        let n = BURST_MIN + (rand(seed) * (BURST_MAX - BURST_MIN + 1) as f64) as u32;
        m.state = State::Burst {
            next: first,
            done: 0,
            most: n.min(BURST_MAX),
        };
    }

    pub fn step(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        self.emerge();
        let seen = std::mem::take(&mut self.seen);
        let mut seen_now: Vec<Seen> = std::mem::take(&mut self.pending);
        let mut seed = self.seed;
        let mut any_burst = false;
        for m in self.males[..self.out].iter_mut() {
            // fly: a heading that wanders, turning back into the woods at its edges
            let turn = (rand(&mut seed) as f32 - 0.5) * 2.0 * WANDER * dt as f32 * 3.0;
            m.heading += turn;
            // a female's answer seen: he flies down to her
            if let Some((at, until)) = m.lure {
                if t > until {
                    m.lure = None;
                } else {
                    let want = (at[2] - m.pos[2]).atan2(0.6 * (at[0] - m.pos[0]));
                    let d = (want - m.heading + std::f32::consts::PI)
                        .rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    m.heading += d.clamp(-1.0, 1.0) * 2.5 * dt as f32;
                    m.climb +=
                        ((at[1] + 0.3 - m.pos[1]).clamp(-0.4, 0.4) - m.climb) * dt as f32 * 2.0;
                }
            }
            m.climb =
                (m.climb + (rand(&mut seed) as f32 - 0.5) * dt as f32 * 1.2).clamp(-0.25, 0.25);
            let v = [
                m.heading.cos() * SPEED,
                m.climb,
                m.heading.sin() * SPEED * 0.6,
            ];
            let next = [
                m.pos[0] + v[0] * dt as f32,
                m.pos[1] + v[1] * dt as f32,
                m.pos[2] + v[2] * dt as f32,
            ];
            if in_woods(next) {
                m.pos = next;
            } else {
                m.heading += std::f32::consts::PI * 0.7;
                m.climb = -m.climb;
            }
            // see, and flash
            let sees = |s: &[Seen], p: [f32; 3], fireflies: bool| -> Option<f64> {
                s.iter()
                    .filter(|f| {
                        (f.firefly || !fireflies)
                            && dist2(f.pos, p) < f.sight * f.sight
                            && dist2(f.pos, p) > 1e-6
                    })
                    .map(|f| f.t)
                    .reduce(f64::min)
            };
            let saw = sees(&seen, m.pos, false);
            if let Some(at) = sees(&seen, m.pos, true) {
                m.saw = at;
            }
            match m.state {
                State::Hidden => {}
                State::Resting { until } => {
                    // rested, and the woods around him dark a moment
                    // rested once the woods round him have been dark a while
                    if t >= until && t - m.saw.max(m.last_flash) > m.dark {
                        let wait = WAIT_MEAN_S * -rand(&mut seed).max(1e-9).ln();
                        m.state = State::Ready { own: t + wait };
                    }
                }
                State::Ready { own } => {
                    if let Some(at) = saw {
                        let first = at + ANSWER_S + ANSWER_SPREAD * rand(&mut seed);
                        Self::start_burst(m, first.max(t), &mut seed);
                    } else if t >= own {
                        Self::start_burst(m, t, &mut seed);
                    }
                }
                State::Burst { next, done, most } => {
                    any_burst = true;
                    // a flash seen just before his own next pulls it earlier (into step)
                    let mut next = next;
                    if let Some(at) = saw {
                        let mine = next - m.interflash;
                        let ahead = mine - (at + ANSWER_S * 0.5);
                        if ahead > 0.0 && ahead < PULL_WINDOW_S && t - m.last_flash > FLASH_S {
                            next -= ahead * PULL;
                        }
                    }
                    if t >= next {
                        m.last_flash = next;
                        m.flash_from = m.pos;
                        seen_now.push(Seen {
                            pos: m.pos,
                            t: next,
                            sight: SIGHT_M,
                            firefly: true,
                        });
                        // he keeps on while he sees the others flashing, from his fourth flash to his
                        // most; so a burst ends together, and the rest after it starts together
                        let done = done + 1;
                        let alone = next - m.saw > m.interflash * 1.6;
                        m.state = if done >= most || (done >= BURST_MIN && alone) {
                            m.dark = DARK_S + DARK_SPREAD * gauss(&mut seed);
                            let rest = REST_S + REST_SPREAD * gauss(&mut seed);
                            State::Resting {
                                until: next + rest.max(3.0),
                            }
                        } else {
                            State::Burst {
                                next: next + m.interflash,
                                done,
                                most,
                            }
                        };
                    } else {
                        m.state = State::Burst { next, done, most };
                    }
                }
            }
        }
        // the females answer a burst they saw, a few seconds after its end, with a doublet
        if !seen_now.is_empty() {
            self.last_any = t;
        }
        if self.bursting && !any_burst {
            for f in self.females.iter_mut() {
                let near = self.males[..self.out]
                    .iter()
                    .any(|m| t - m.last_flash < 3.0 && dist2(m.pos, f.pos) < 100.0);
                if near && rand(&mut seed) < FEMALE_ANSWERS {
                    let at = self.last_any + FEMALE_DELAY_S + 0.6 * rand(&mut seed);
                    f.flashes = [at, at + DOUBLET_GAP_S];
                }
            }
        }
        // the males near a female who answers see her, and come
        for f in &self.females {
            if f.flashes[0] <= t && f.flashes[0] > t - dt {
                for m in self.males[..self.out].iter_mut() {
                    if dist2(m.pos, f.pos) < LURE_M * LURE_M && rand(&mut seed) < 0.7 {
                        m.lure = Some((f.pos, t + LURE_S));
                    }
                }
            }
        }
        self.bursting = any_burst;
        self.seed = seed;
        self.seen = seen_now;
        self.light = (0..self.out).map(|i| self.male_glow(i)).sum::<f32>()
            + (0..self.females.len())
                .map(|i| self.female_glow(i))
                .sum::<f32>()
                * 0.4;
    }

    /// A flash's light at `since` seconds after it began: a quick rise, a softer fall.
    pub fn pulse(since: f64) -> f32 {
        if !(0.0..FLASH_S * 1.6).contains(&since) {
            return 0.0;
        }
        let u = since / FLASH_S;
        let rise = (u / 0.25).min(1.0);
        let fall = (-(u - 0.25).max(0.0) * 3.2).exp();
        (rise * rise * (3.0 - 2.0 * rise) * fall) as f32
    }

    pub fn male_glow(&self, i: usize) -> f32 {
        Swarm::pulse(self.t - self.males[i].last_flash)
    }

    pub fn female_glow(&self, i: usize) -> f32 {
        let f = &self.females[i];
        0.85 * (Swarm::pulse(self.t - f.flashes[0]) + Swarm::pulse(self.t - f.flashes[1]))
    }

    /// How many males are out and in a burst, resting or ready (for the checks).
    pub fn census(&self) -> (usize, usize, usize) {
        let mut c = (0, 0, 0);
        for m in &self.males[..self.out] {
            match m.state {
                State::Burst { .. } => c.0 += 1,
                State::Resting { .. } => c.1 += 1,
                State::Ready { .. } => c.2 += 1,
                State::Hidden => {}
            }
        }
        c
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The swarm's bursts, once all are out, come at a steady beat near twelve seconds (the measured
    /// collective period), though alone a male has no rhythm.
    #[test]
    fn a_steady_beat_emerges() {
        let mut s = Swarm::new(7);
        s.all_out();
        let dt = 1.0 / 60.0;
        let mut starts = Vec::new();
        let mut was = false;
        for _ in 0..(240.0 / dt) as usize {
            s.step(dt);
            let (b, _, _) = s.census();
            if !was && b > MALES / 8 {
                starts.push(s.t);
                was = true;
            } else if was && b < MALES / 40 {
                was = false;
            }
        }
        let mut gaps: Vec<f64> = starts.windows(2).map(|w| w[1] - w[0]).skip(4).collect();
        assert!(gaps.len() > 8, "bursts {starts:?}");
        gaps.sort_by(f64::total_cmp);
        let median = gaps[gaps.len() / 2];
        assert!(
            (10.5..14.0).contains(&median),
            "beat {median:.2} s, gaps {gaps:?}"
        );
        // now and then a burst is skipped (too few rested); the rest keep the beat
        let steady = gaps.iter().filter(|g| (*g - median).abs() < 1.5).count();
        assert!(steady * 5 >= gaps.len() * 4, "unsteady: gaps {gaps:?}");
    }

    /// Alone, a male's bursts come at no steady beat.
    #[test]
    fn alone_no_rhythm() {
        let mut s = Swarm::new(3);
        s.out = 0;
        s.males.truncate(1);
        s.out = 1;
        s.males[0].state = State::Ready { own: 0.0 };
        s.t = EMERGE_S; // no more emerge
        let dt = 1.0 / 60.0;
        let mut starts = Vec::new();
        let mut was = false;
        for _ in 0..(2000.0 / dt) as usize {
            s.step(dt);
            let now = matches!(s.males[0].state, State::Burst { .. });
            if now && !was {
                starts.push(s.t);
            }
            was = now;
        }
        let gaps: Vec<f64> = starts.windows(2).map(|w| w[1] - w[0]).collect();
        let mean = gaps.iter().sum::<f64>() / gaps.len() as f64;
        let sd = (gaps.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / gaps.len() as f64).sqrt();
        assert!(mean > 25.0 && sd > 15.0, "alone: mean {mean:.1} sd {sd:.1}");
    }
}

#[cfg(test)]
mod probe {
    use super::*;
    #[test]
    #[ignore]
    fn print_series() {
        let mut s = Swarm::new(0x5eed_e1c5);
        let dt = 1.0 / 120.0;
        let mut line = String::new();
        for i in 0..(200.0 / dt) as usize {
            s.step(dt);
            if i % 30 == 0 {
                let (b, r, w) = s.census();
                line.push_str(&format!("{:.0}:{}/{}/{} ", s.t, b, r, w));
            }
        }
        println!("{line}");
    }
}
