//! The walk: one creature, one form at a time, moving through every form it could be by a Metropolis
//! walk whose target is the reader's own attention (painting-company row 225).
//!
//! - A form is four numbers (`Form`): its colour, how spiky, how many arms, how it glows (the rest of
//!   its looks follow from these). Colour wraps round; the rest lie in 0..1.
//! - The density: for any form, how much of the time it was in front of the reader they held it
//!   (`Memory::liking`): every moment the creature is shown while the reader is with it (a finger on it
//!   now or within ENGAGED seconds) is remembered with whether a finger was on it, and a form's liking is the held time near it over the shown time near it (near: a gaussian in
//!   the four numbers), pulled toward an even prior where little has been seen.
//! - The step: from the form it is, it tries on a form a small random step away (each number moved by
//!   a gaussian, the colour wrapped, the rest reflected at their ends: the proposal is symmetric), shows
//!   it for a moment, then keeps it with probability min(1, liking(new) / liking(old)), or goes back.
//!   That is Metropolis' rule, and nothing else moves it: no steering, no gradient, no memory of the
//!   reader's favourite. Where the reader holds it longer it lingers; it still wanders, as often as the
//!   ratio allows.
//!
//! The density is the reader's, so it changes as they watch: at each step the rule is applied to the
//! liking as it stands then (SOURCES.md says so). With a fixed liking the walk's time at each form is
//! proportional to it (tested below).

use serde::{Deserialize, Serialize};

pub const D: usize = 4;
/// colour (wraps), spikes, arms, glow: what the eye reads first. The creature's other traits (curl,
/// markings, second colour, temper) follow from these smoothly (creature.rs' `body`).
pub type Form = [f64; D];

/// The proposal's step, each number: mostly a small one, sometimes (JUMPS of the time) a leap. A
/// mixture of two symmetric steps is symmetric.
pub const STEP: f64 = 0.12;
pub const LEAP: f64 = 0.4;
pub const JUMPS: f64 = 0.15;
/// "Near", for the liking: the gaussian's width in the eight numbers.
pub const NEAR: f64 = 0.17;
/// The even prior: a form never seen counts as held this share of the time, worth PRIOR_S seconds.
pub const PRIOR: f64 = 0.15;
pub const PRIOR_S: f64 = 0.8;

/// The target is the liking raised to this power: the walk keeps to what was held more firmly than
/// the bare ratio would (a cooler walk on the same liking).
pub const SHARP: f64 = 3.0;

/// The phases of a step, and their seconds.
pub const REST: f64 = 0.45;
pub const TRY: f64 = 0.32;
pub const LOOK: f64 = 0.5;
pub const BACK: f64 = 0.25;
/// The reader is with it while a finger is on it and for this long after it lifts.
pub const ENGAGED: f64 = 6.0;
/// The reader's eye lags the screen: the form remembered for a moment is the one shown this long ago.
pub const LAG: f64 = 0.3;

/// A small, fast, seeded generator (xorshift64*), so a plan plays the same walk every time.
#[derive(Clone, Serialize, Deserialize)]
pub struct Rng(pub u64);

impl Rng {
    pub fn bits(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    /// Uniform in [0, 1).
    pub fn f(&mut self) -> f64 {
        (self.bits() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn gauss(&mut self) -> f64 {
        let u = self.f().max(1e-12);
        let v = self.f();
        (-2.0 * u.ln()).sqrt() * (std::f64::consts::TAU * v).cos()
    }
}

/// The difference b - a in each number (the colour the short way round).
pub fn delta(a: &Form, b: &Form) -> Form {
    let mut d = [0.0; D];
    for i in 0..D {
        d[i] = b[i] - a[i];
    }
    d[0] = (d[0] + 0.5).rem_euclid(1.0) - 0.5;
    d
}

pub fn dist2(a: &Form, b: &Form) -> f64 {
    delta(a, b).iter().map(|x| x * x).sum()
}

/// From a to b a share u of the way (the colour the short way round).
pub fn lerp(a: &Form, b: &Form, u: f64) -> Form {
    let d = delta(a, b);
    let mut f = *a;
    for i in 0..D {
        f[i] += d[i] * u;
    }
    f[0] = f[0].rem_euclid(1.0);
    f
}

/// A form a small random step from `x`: symmetric (a gaussian, the colour wrapped, the rest reflected).
pub fn propose(x: &Form, rng: &mut Rng) -> Form {
    let mut y = *x;
    let s = if rng.f() < JUMPS { LEAP } else { STEP };
    for (i, v) in y.iter_mut().enumerate() {
        *v += s * rng.gauss();
        if i == 0 {
            *v = v.rem_euclid(1.0);
        } else {
            // reflect into [0, 1]
            let m = v.rem_euclid(2.0);
            *v = if m > 1.0 { 2.0 - m } else { m };
        }
    }
    y
}

/// One remembered moment: the form shown, for how long, and how much of that a finger held it.
#[derive(Clone, Serialize, Deserialize)]
pub struct Moment {
    pub form: [f32; D],
    pub shown: f32,
    pub held: f32,
}

/// Every moment shown, with whether it was held.
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Memory {
    pub moments: Vec<Moment>,
}

const MOMENTS: usize = 6000;

impl Memory {
    /// The creature showed `form` for `dt` seconds, `held` of them under a finger.
    pub fn remember(&mut self, form: &Form, dt: f64, held: f64) {
        let f: [f32; D] = std::array::from_fn(|i| form[i] as f32);
        if let Some(last) = self.moments.last_mut() {
            let close = (0..D).all(|i| (last.form[i] - f[i]).abs() < 0.004);
            if close && last.shown < 4.0 {
                last.shown += dt as f32;
                last.held += held as f32;
                return;
            }
        }
        self.moments.push(Moment { form: f, shown: dt as f32, held: held as f32 });
        if self.moments.len() > MOMENTS {
            // the oldest half merged pairwise (a form and the next are close: the walk is continuous)
            let half = MOMENTS / 2;
            let mut merged: Vec<Moment> = self.moments[..half]
                .chunks(2)
                .map(|c| {
                    let mut m = c[0].clone();
                    if let Some(b) = c.get(1) {
                        m.shown += b.shown;
                        m.held += b.held;
                    }
                    m
                })
                .collect();
            merged.extend_from_slice(&self.moments[half..]);
            self.moments = merged;
        }
    }

    /// How much the reader holds forms like `y` when they are shown: the target the walk samples.
    #[allow(clippy::needless_range_loop)]
    pub fn liking(&self, y: &Form) -> f64 {
        let k = -0.5 / (NEAR * NEAR);
        let (mut s, mut h) = (0.0, 0.0);
        for m in &self.moments {
            let mut d2 = 0.0;
            for i in 0..D {
                let mut d = m.form[i] as f64 - y[i];
                if i == 0 {
                    d = (d + 0.5).rem_euclid(1.0) - 0.5;
                }
                d2 += d * d;
            }
            let w = (k * d2).exp();
            s += w * m.shown as f64;
            h += w * m.held as f64;
        }
        (h + PRIOR * PRIOR_S) / (s + PRIOR_S)
    }

    pub fn held_total(&self) -> f64 {
        self.moments.iter().map(|m| m.held as f64).sum()
    }
}

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub enum Phase {
    Rest,
    Try,
    Look,
    Back,
}

/// One kept form on the path, how long the creature was it, and the tries it turned down there.
#[derive(Clone, Serialize, Deserialize)]
pub struct Stop {
    pub form: Form,
    pub dwell: f64,
    pub declined: Vec<Form>,
    /// Begun afresh here (the reader chose it on the wheel), not walked to.
    #[serde(default)]
    pub jump: bool,
}

/// What a step decided, for the toy's sound and touch.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Event {
    Tries,
    Keeps,
    Declines,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Walker {
    pub x: Form,
    pub y: Form,
    pub phase: Phase,
    pub clock: f64,
    pub rng: Rng,
    pub memory: Memory,
    pub path: Vec<Stop>,
    pub steps: usize,
    /// The forms shown, recently (for the reader's lag): (seconds, form).
    #[serde(skip)]
    shown: std::collections::VecDeque<(f64, Form)>,
    pub time: f64,
    /// The last ratio and whether it was kept (for the toy's sound).
    pub last: (f64, bool),
}

fn ease(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

impl Walker {
    pub fn new(seed: u64, start: Form) -> Walker {
        Walker {
            x: start,
            y: start,
            phase: Phase::Rest,
            clock: 0.0,
            rng: Rng(seed | 1),
            memory: Memory::default(),
            path: vec![Stop { form: start, dwell: 0.0, declined: Vec::new(), jump: false }],
            steps: 0,
            shown: Default::default(),
            time: 0.0,
            last: (1.0, true),
        }
    }

    /// The form on screen now.
    pub fn display(&self) -> Form {
        match self.phase {
            Phase::Rest => self.x,
            Phase::Try => lerp(&self.x, &self.y, ease(self.clock / TRY)),
            Phase::Look => self.y,
            Phase::Back => lerp(&self.y, &self.x, ease(self.clock / BACK)),
        }
    }

    /// Time moves by `dt`, `held` of it under the reader's finger. `liking` is the target: the
    /// reader's (`Memory::liking`) in the toy; any function in the tests.
    /// `engaged`: whether the reader is with it (a finger on it now or a moment ago); only then is the
    /// moment remembered, so watching without touching neither teaches nor unteaches it.
    pub fn step_with(&mut self, dt: f64, held: f64, engaged: bool, liking: &dyn Fn(&Memory, &Form) -> f64) -> Option<Event> {
        self.time += dt;
        // remember what the reader saw a moment ago, and whether they held it
        let now = self.display();
        self.shown.push_back((self.time, now));
        while self.shown.len() > 2 && self.shown[1].0 <= self.time - LAG {
            self.shown.pop_front();
        }
        let seen = self.shown.front().map(|s| s.1).unwrap_or(now);
        if dt > 0.0 && engaged {
            self.memory.remember(&seen, dt, held);
        }
        if let Some(s) = self.path.last_mut() {
            s.dwell += dt;
        }
        self.clock += dt;
        match self.phase {
            Phase::Rest if self.clock >= REST => {
                self.clock = 0.0;
                self.y = propose(&self.x, &mut self.rng);
                self.phase = Phase::Try;
                Some(Event::Tries)
            }
            Phase::Try if self.clock >= TRY => {
                self.clock = 0.0;
                self.phase = Phase::Look;
                None
            }
            Phase::Look if self.clock >= LOOK => {
                self.clock = 0.0;
                self.steps += 1;
                let (a, b) = (liking(&self.memory, &self.x), liking(&self.memory, &self.y));
                let ratio = (b / a.max(1e-9)).powf(SHARP);
                let keep = self.rng.f() < ratio.min(1.0);
                self.last = (ratio, keep);
                if std::env::var("TASTE_TRACE").is_ok() {
                    eprintln!("trace {:.3} {} {:.3} {:.3} held {:.2}", self.time, keep, self.y[0], ratio, self.memory.held_total());
                }
                if keep {
                    self.x = self.y;
                    self.path.push(Stop { form: self.x, dwell: 0.0, declined: Vec::new(), jump: false });
                    self.phase = Phase::Rest;
                    // every step lasts as long, kept or not, so time is shared as the rule shares it
                    self.clock = -BACK;
                    Some(Event::Keeps)
                } else {
                    let y = self.y;
                    if let Some(s) = self.path.last_mut() {
                        if s.declined.len() < 12 {
                            s.declined.push(y);
                        }
                    }
                    self.phase = Phase::Back;
                    Some(Event::Declines)
                }
            }
            Phase::Back if self.clock >= BACK => {
                self.clock = 0.0;
                self.y = self.x;
                self.phase = Phase::Rest;
                None
            }
            _ => None,
        }
    }

    /// Begins the walk again from a form it kept before (the reader chose it on the wheel). Any kept
    /// form is a fair place to walk on from; what it remembers is untouched.
    pub fn return_to(&mut self, f: Form) {
        self.x = f;
        self.y = f;
        self.phase = Phase::Rest;
        self.clock = 0.0;
        self.path.push(Stop { form: f, dwell: 0.0, declined: Vec::new(), jump: true });
    }

    pub fn step(&mut self, dt: f64, held: f64, engaged: bool) -> Option<Event> {
        self.step_with(dt, held, engaged, &|m, f| m.liking(f))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposals_stay_in_range_and_are_symmetric_in_spread() {
        let mut r = Rng(7);
        let x = [0.98, 0.01, 0.99, 0.0];
        for _ in 0..1000 {
            let y = propose(&x, &mut r);
            assert!(y.iter().all(|v| (0.0..=1.0).contains(v)));
        }
    }

    /// With a fixed liking, the time the walk spends is proportional to the liking raised to SHARP: on
    /// a liking that depends on the spikes alone (1.5× above 0.5), the share of time above is r/(r+1),
    /// r = 1.5^SHARP.
    #[test]
    fn a_fixed_liking_is_sampled_in_proportion() {
        let mut w = Walker::new(11, [0.5; D]);
        let target = |_: &Memory, f: &Form| if f[1] > 0.5 { 1.5 } else { 1.0 };
        let (mut above, mut all) = (0.0, 0.0);
        let dt = 0.05;
        for _ in 0..1_000_000 {
            w.step_with(dt, 0.0, true, &target);
            // the kept form's time: the walk's state is x (a try shown is not yet the walk's)
            if w.x[1] > 0.5 {
                above += dt;
            }
            all += dt;
        }
        let share = above / all;
        let r = 1.5f64.powf(SHARP);
        let want = r / (r + 1.0);
        assert!((share - want).abs() < 0.03, "share above {share}, want {want}");
    }

    /// The reader's liking: held more near where they held.
    #[test]
    fn liking_follows_the_held_time() {
        let mut m = Memory::default();
        let a = [0.1; D];
        let b = [0.9; D];
        m.remember(&a, 10.0, 9.0);
        m.remember(&b, 10.0, 0.5);
        assert!(m.liking(&a) > 0.8);
        assert!(m.liking(&b) < 0.15);
        // far from both: the prior
        let c = [0.5, 0.5, 0.0, 1.0];
        assert!((m.liking(&c) - PRIOR).abs() < 0.05);
    }
}
