//! The crowd as an excitable medium: I. Farkas, D. Helbing & T. Vicsek, "Mexican waves in an
//! excitable medium", *Nature* 419 (2002) 131–132, their n-state model with their parameters.
//!
//! Each person is resting, active (standing, `NA` stages) or refractory (sitting back down, `NR`
//! stages). A resting person stands when the weighted share of active people within `R` seats is over
//! their own threshold (drawn from `C ± DC`); the weights fall off exponentially with distance and
//! lean one way, so the wave runs one way round (clockwise, seen from above, as most do). A person
//! who has stood goes through every stage and rests again.
//!
//! Three things the paper's model leaves out, so that a wave can end: each person, having stood,
//! is less keen to again for the next minute or so (so a second wave close behind the first
//! fizzles); nobody is quite the same twice (each second a person's threshold wobbles a little, and
//! the whole crowd's mood drifts); and people tire of a wave the longer it has been going (a wave
//! carries its age from person to person, and the older it is the more it takes to join it). An
//! old wave dims, as the least keen only half stand, then thins and dies, usually within a lap or
//! two; a new one started by a fresh group is as young as it ever was.

/// Seats round a row, and rows (about the stadium's 87,000).
pub const U: usize = 1500;
pub const V: usize = 58;
/// The model's parameters. The paper's figure 1 has na = nr = 5, c = 0.25, Δc = 0.05, R = 3 in its own
/// units; here a cell is a seat and a step 1/28 of a second, so the stages are lengthened to
/// give the measured band (about 15 seats wide at 20 seats a second) and the threshold raised so a few
/// dozen, not a handful, are needed to start one.
const NA: u8 = 20;
const NR: u8 = 20;
const C: f32 = 0.42;
const DC: f32 = 0.05;
const R: i32 = 3;
/// How much more a neighbour behind (against the wave's way) counts than one ahead.
const LEAN: f32 = 2.0;
/// The weights' fall-off with distance, seats.
const FALL: f32 = 1.5;
/// Each time a person stands their threshold rises by about `FATIGUE` (so a second wave close
/// behind the first fizzles), and that falls away with a time constant of `RECOVER_S` seconds.
pub const FATIGUE: f32 = 0.10;
pub const RECOVER_S: f32 = 45.0;
/// Each second, each threshold wobbles by up to this much either way.
pub const NOISE: f32 = 0.06;
/// And the whole crowd's mood drifts, keener or flatter, by about this much over about `DRIFT_S`.
const SWAY: f32 = 0.015;
const DRIFT_S: f32 = 40.0;
/// The older a wave, the more it takes to join it: this much more threshold for each second of its
/// age. A group that starts one is, by luck, keener or flatter: its wave starts as if up to
/// `VERVE_S` seconds younger, or a third of that older.
const BORED: f32 = 0.0007;
const VERVE_S: f32 = 40.0;
/// About how bored people are when a wave can no longer go on (the tests measure it).
const BORED_END: f32 = 0.09;

#[derive(Clone)]
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

#[derive(Clone)]
pub struct Crowd {
    /// 0 resting, 1..=NA active, NA+1..=NA+NR refractory; by seat, row-major (row * U + seat)
    pub stage: Vec<u8>,
    threshold: Vec<f32>,
    /// how tired of it each person is, added to their threshold
    tired: Vec<f32>,
    /// each person's mood this second, added to their threshold
    mood: Vec<f32>,
    /// how keen on waves each person is, 0..1: as the crowd tires of one, the least keen only half
    /// stand
    keen: Vec<f32>,
    /// the whole crowd's mood, added to every threshold (below zero, keener)
    pub sway: f32,
    /// the step at which the wave each person last stood in began (older: less keen to join)
    born: Vec<i64>,
    /// when the wave the group standing now (`stand`) starts is counted to have begun
    group: i64,
    ticks: i64,
    /// the kernel: (seat offset, row offset, weight)
    kernel: Vec<(i32, i32, f32)>,
    total: f32,
    next: Vec<u8>,
    rng: Rng,
    /// people standing now
    pub active: usize,
}

impl Crowd {
    pub fn new(seed: u64) -> Crowd {
        let mut rng = Rng::new(seed);
        let threshold = (0..U * V).map(|_| rng.range(C - DC, C + DC)).collect();
        // the wave runs toward higher seat numbers (clockwise from above, bowl.rs); those at lower
        // seat numbers, behind it, count more
        let mut kernel = Vec::new();
        for dv in -R..=R {
            for du in -R..=R {
                let d = ((du * du + dv * dv) as f32).sqrt();
                if (du == 0 && dv == 0) || d > R as f32 + 0.01 {
                    continue;
                }
                let cos = -du as f32 / d;
                let lean = (LEAN - 1.0) / (LEAN + 1.0);
                kernel.push((du, dv, (-d / FALL).exp() * (1.0 + lean * cos)));
            }
        }
        let total = kernel.iter().map(|k| k.2).sum();
        Crowd {
            stage: vec![0; U * V],
            threshold,
            tired: vec![0.0; U * V],
            mood: vec![0.0; U * V],
            keen: (0..U * V).map(|_| rng.unit()).collect(),
            sway: 0.0,
            born: vec![0; U * V],
            group: 0,
            ticks: 0,
            kernel,
            total,
            next: vec![0; U * V],
            rng,
            active: 0,
        }
    }

    pub fn is_active(s: u8) -> bool {
        (1..=NA).contains(&s)
    }

    /// How far up a person is (0 seated .. 1 standing, arms up), at `frac` of the way to the next
    /// stage.
    pub fn up(s: u8, frac: f32) -> f32 {
        if Self::is_active(s) {
            let x = (s as f32 - 1.0 + frac) / NA as f32;
            (x * std::f32::consts::PI).sin().max(0.0)
        } else {
            0.0
        }
    }

    /// How old the wave person `i` last stood in is, seconds.
    fn age(&self, born: i64) -> f32 {
        (self.ticks - born).max(0) as f32 / crate::TICK_HZ as f32
    }

    /// How high person `i` stands (0 seated .. 1 standing, arms up): in a young wave everyone all
    /// the way; as it ages, more and more, the least keen first, only half rise.
    pub fn height(&self, i: usize, frac: f32) -> f32 {
        let up = Self::up(self.stage[i], frac);
        if up == 0.0 {
            return 0.0;
        }
        let tired = (self.age(self.born[i]) * BORED / BORED_END).min(1.0);
        up * ((self.keen[i] + 0.3 - tired) / 0.3).clamp(0.3, 1.0)
    }

    /// One step of the model.
    pub fn tick(&mut self) {
        // only seats within reach of someone standing can stand: find those columns first
        let mut hot = vec![false; U];
        for u in 0..U {
            if (0..V).any(|v| Self::is_active(self.stage[v * U + u])) {
                for du in -R..=R {
                    hot[(u as i32 + du).rem_euclid(U as i32) as usize] = true;
                }
            }
        }
        let mut active = 0;
        for v in 0..V {
            for u in 0..U {
                let i = v * U + u;
                let s = self.stage[i];
                self.next[i] = if s > 0 {
                    if s >= NA + NR {
                        0
                    } else {
                        s + 1
                    }
                } else if let Some(born) = hot[u].then(|| self.excited(u, v)).flatten() {
                    self.tired[i] += FATIGUE * self.rng.range(0.3, 1.7);
                    self.born[i] = born;
                    1
                } else {
                    0
                };
                active += Self::is_active(self.next[i]) as usize;
            }
        }
        std::mem::swap(&mut self.stage, &mut self.next);
        self.active = active;
        // once a second, everyone gets a little keener again
        self.ticks += 1;
        let hz = crate::TICK_HZ as i64;
        if self.ticks % hz == 0 {
            let k = (-1.0 / RECOVER_S).exp();
            self.tired.iter_mut().for_each(|t| *t *= k);
            let keep = (-1.0 / DRIFT_S).exp();
            let kick = (self.rng.unit() + self.rng.unit() + self.rng.unit() - 1.5) * 2.0;
            self.sway = self.sway * keep + SWAY * (1.0 - keep * keep).sqrt() * kick;
            let sway = self.sway;
            for m in self.mood.iter_mut() {
                *m = sway + self.rng.range(-NOISE, NOISE);
            }
        }
    }

    /// Whether the resting person at seat `u`, row `v` stands now, and if so when the wave they join
    /// began (the oldest of the standing people round them).
    fn excited(&self, u: usize, v: usize) -> Option<i64> {
        let mut sum = 0.0;
        let mut seen = self.total;
        let mut born = i64::MAX;
        for &(du, dv, w) in &self.kernel {
            let vv = v as i32 + dv;
            if !(0..V as i32).contains(&vv) {
                seen -= w;
                continue;
            }
            let uu = (u as i32 + du).rem_euclid(U as i32) as usize;
            let j = vv as usize * U + uu;
            if Self::is_active(self.stage[j]) {
                sum += w;
                born = born.min(self.born[j]);
            }
        }
        if sum == 0.0 {
            return None;
        }
        let i = v * U + u;
        let bored = BORED * self.age(born);
        (sum > (self.threshold[i] + self.tired[i] + self.mood[i] + bored) * seen).then_some(born)
    }

    /// A new group gets together to try to start a wave: how keen it is, by luck.
    pub fn new_group(&mut self) {
        let hz = crate::TICK_HZ as f32;
        self.group = self.ticks + (self.rng.range(-VERVE_S / 3.0, VERVE_S) * hz) as i64;
    }

    /// The resting people within `r` seats of seat `u`, row `v` stand up (the group of the last
    /// `new_group` trying to start a wave). Returns how many did.
    pub fn stand(&mut self, u: f32, v: f32, r: f32) -> usize {
        let mut n = 0;
        let k = r.ceil() as i32;
        for dv in -k..=k {
            for du in -k..=k {
                let (fu, fv) = (u + du as f32, v + dv as f32);
                let d = ((fu.floor() + 0.5 - u).powi(2) + (fv.floor() + 0.5 - v).powi(2)).sqrt();
                if d > r || fv < 0.0 || fv >= V as f32 {
                    continue;
                }
                let i = fv as usize * U + (fu.floor() as i32).rem_euclid(U as i32) as usize;
                if self.stage[i] == 0 {
                    self.stage[i] = 1;
                    self.tired[i] += FATIGUE;
                    self.born[i] = self.group;
                    n += 1;
                }
            }
        }
        n
    }

    /// Somewhere in the crowd a handful try to start one; now and then a bigger group. Almost always
    /// too few.
    pub fn somebody_tries(&mut self) -> (f32, f32) {
        let u = self.rng.range(0.0, U as f32);
        let v = self.rng.range(8.0, V as f32 - 8.0);
        self.new_group();
        let r = if self.rng.unit() < 0.04 {
            self.rng.range(2.2, 3.0)
        } else {
            self.rng.range(1.2, 2.0)
        };
        self.stand(u, v, r);
        (u, v)
    }

    pub fn rand(&mut self) -> f32 {
        self.rng.unit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The front's seat on the middle row after `ticks` steps from a group at seat 100.
    fn front(c: &Crowd) -> Option<usize> {
        let v = V / 2;
        (0..U / 2)
            .rev()
            .find(|&u| Crowd::is_active(c.stage[v * U + u]))
    }

    #[test]
    fn a_wave_runs_clockwise_about_twenty_seats_a_second() {
        let mut c = Crowd::new(7);
        c.new_group();
        c.stand(100.0, V as f32 * 0.5, 6.0);
        for _ in 0..10 {
            c.tick();
        }
        let a = front(&c).expect("a wave");
        for _ in 0..35 {
            c.tick();
        }
        let b = front(&c).expect("still a wave");
        let per_tick = (b - a) as f32 / 35.0;
        eprintln!(
            "front {a} -> {b}: {per_tick:.2} seats a tick, {:.1} a second",
            per_tick * crate::TICK_HZ as f32
        );
        let width = (0..U)
            .filter(|&u| Crowd::is_active(c.stage[(V / 2) * U + u]))
            .count();
        eprintln!("standing across the middle row: {width}");
        let per_s = per_tick * crate::TICK_HZ as f32;
        assert!(
            (16.0..25.0).contains(&per_s),
            "about 20 seats a second, clockwise"
        );
        assert!((10..22).contains(&width), "about 15 seats wide");
    }

    #[test]
    fn a_few_people_fizzle() {
        let mut ok = 0;
        for seed in 0..20 {
            let mut c = Crowd::new(seed);
            c.new_group();
            c.stand(300.0, 29.0, 1.6);
            for _ in 0..60 {
                c.tick();
            }
            ok += (c.active > 0) as usize;
        }
        eprintln!("{ok} of 20 tiny groups made a wave");
        assert!(ok < 10);
    }

    /// Seconds until the crowd is still again, starting from `starts` (seat, radius) groups.
    fn lifetime(seed: u64, starts: &[(f32, f32)]) -> f32 {
        let mut c = Crowd::new(seed);
        for &(u, r) in starts {
            c.new_group();
            c.stand(u, V as f32 * 0.5, r);
        }
        let hz = crate::TICK_HZ as f32;
        let mut n = 0;
        while c.active > 0 || n == 0 {
            c.tick();
            n += 1;
            if n as f32 > hz * 900.0 {
                break;
            }
        }
        n as f32 / hz
    }

    #[test]
    #[ignore = "a measurement"]
    fn how_often_a_wave_starts_by_itself() {
        let hz = crate::TICK_HZ as f32;
        let minutes = 30.0;
        let mut c = Crowd::new(3);
        let (mut next, mut started, mut tries, mut was) = (5.0f32, 0, 0, 0usize);
        let mut long = 0usize;
        for n in 0..(minutes * 60.0 * hz) as usize {
            let t = n as f32 / hz;
            c.tick();
            if t > next {
                next = t + crate::TRY_S[0] + (crate::TRY_S[1] - crate::TRY_S[0]) * c.rand();
                if c.active == 0 {
                    c.somebody_tries();
                    tries += 1;
                }
            }
            long = if c.active > 0 { long + 1 } else { 0 };
            if long as f32 == 20.0 * hz {
                started += 1;
            }
            was = was.max(c.active);
        }
        eprintln!("{minutes} minutes: {tries} tries, {started} waves that lasted 20 s");
    }

    #[test]
    #[ignore = "a measurement"]
    fn how_soon_after_one_dies_another_can_start() {
        let hz = crate::TICK_HZ as f32;
        for seed in 0..4 {
            let mut c = Crowd::new(2 * seed);
            c.new_group();
            c.stand(100.0, V as f32 * 0.5, 6.0);
            let mut n = 0;
            while c.active > 0 || n == 0 {
                c.tick();
                n += 1;
            }
            let mut after = Vec::new();
            for wait in 0..12 {
                let mut d = c.clone();
                d.new_group();
                d.stand(700.0, V as f32 * 0.5, 6.0);
                for _ in 0..(hz * 15.0) as usize {
                    d.tick();
                }
                after.push((wait * 15, d.active > 200));
                for _ in 0..(hz * 15.0) as usize {
                    c.tick();
                }
            }
            eprintln!(
                "died after {:.0} s; a swipe so many seconds later runs: {after:?}",
                n as f32 / hz
            );
        }
    }

    /// A finger drawn `len` seats along row 30 (as `Wave::touch` does), on a fresh crowd: how many of
    /// 20 start a wave.
    fn swipes(len: f32) -> usize {
        (0..20)
            .filter(|&seed| {
                let mut c = Crowd::new(2 * seed + 1);
                let n = (len / 2.2).ceil() as usize;
                c.new_group();
                for k in 0..=n {
                    c.stand(400.0 + len * k as f32 / n as f32, 30.0, 2.8);
                }
                for _ in 0..(crate::TICK_HZ * 15.0) as usize {
                    c.tick();
                }
                c.active > 200
            })
            .count()
    }

    #[test]
    #[ignore = "a measurement"]
    fn how_often_a_swipe_starts_one() {
        for len in [10.0, 20.0, 40.0, 80.0] {
            eprintln!("swipe {len} seats: {} of 20", swipes(len));
        }
    }

    #[test]
    #[ignore = "a measurement"]
    fn trace_the_opening_wave() {
        let mut c = Crowd::new(0x0_1986_A2EC);
        for _ in 0..U * V {
            if c.rand() < 0.6 {
                c.rand();
            }
            c.rand();
        }
        c.new_group();
        c.stand(U as f32 * 0.10, V as f32 * 0.5, 6.0);
        for n in 0..(crate::TICK_HZ * 300.0) as usize {
            c.tick();
            if n % (crate::TICK_HZ as usize * 15) == 0 {
                eprintln!(
                    "{:>4} s: active {:>5}, sway {:.3}, oldest age {:.0}",
                    n / crate::TICK_HZ as usize,
                    c.active,
                    c.sway,
                    (0..U * V)
                        .filter(|&i| Crowd::is_active(c.stage[i]))
                        .map(|i| c.age(c.born[i]))
                        .fold(0.0, f32::max)
                );
            }
        }
    }

    #[test]
    #[ignore = "a measurement"]
    fn how_tired_kills_a_wave() {
        for x in [0.0, 0.03, 0.06, 0.09, 0.12, 0.15] {
            let mut alive = 0;
            for seed in 0..4 {
                let mut c = Crowd::new(seed);
                c.tired.fill(x);
                c.new_group();
                c.stand(100.0, V as f32 * 0.5, 6.0);
                for _ in 0..(crate::TICK_HZ * 8.0) as usize {
                    c.tick();
                }
                alive += (c.active > 200) as usize;
                eprintln!("tired {x}: seed {seed} active {}", c.active);
            }
            eprintln!("tired {x}: {alive}/4 alive after 8 s");
        }
    }

    #[test]
    fn a_wave_dies_out_after_a_lap_or_few() {
        let lap = U as f32 / 20.0;
        for seed in 0..3 {
            let laps = lifetime(2 * seed, &[(100.0, 6.0)]) / lap;
            eprintln!("seed {seed}: {laps:.2} laps");
            assert!((0.8..6.0).contains(&laps), "a lap or a few, not for ever");
        }
    }

    #[test]
    fn a_second_wave_close_behind_fizzles() {
        let mut c = Crowd::new(5);
        c.new_group();
        c.stand(100.0, V as f32 * 0.5, 6.0);
        for _ in 0..(crate::TICK_HZ * 6.0) as usize {
            c.tick();
        }
        c.new_group();
        c.stand(100.0, V as f32 * 0.5, 6.0);
        for _ in 0..(crate::TICK_HZ * 20.0) as usize {
            c.tick();
        }
        // count the separate runs of standing people along the middle row
        let row = |u: usize| Crowd::is_active(c.stage[(V / 2) * U + u % U]);
        let runs = (0..U).filter(|&u| row(u) && !row(u + U - 1)).count();
        eprintln!("{runs} waves 20 s later");
        assert!(
            runs == 1 && c.active > 200,
            "the first runs on; the crowd behind it is too tired for a second"
        );
    }

    #[test]
    fn a_swipe_ahead_of_an_old_wave_starts_a_new_one() {
        let mut ok = 0;
        for seed in 0..6 {
            let mut c = Crowd::new(2 * seed + 1);
            c.new_group();
            c.stand(100.0, V as f32 * 0.5, 6.0);
            for _ in 0..(crate::TICK_HZ * 60.0) as usize {
                c.tick();
            }
            let front = (0..U)
                .filter(|&u| Crowd::is_active(c.stage[(V / 2) * U + u]))
                .max()
                .unwrap_or(0);
            c.new_group();
            for k in 0..=12 {
                c.stand((front + 150 + 3 * k) as f32, 30.0, 2.8);
            }
            for _ in 0..(crate::TICK_HZ * 15.0) as usize {
                c.tick();
            }
            let row = |u: usize| Crowd::is_active(c.stage[(V / 2) * U + u % U]);
            ok += ((0..U).filter(|&u| row(u) && !row(u + U - 1)).count() >= 1
                && (front + 350..front + 650).any(row)) as usize;
        }
        eprintln!("{ok} of 6 swipes ahead of a minute-old wave started one");
        assert!(
            ok >= 5,
            "a fresh group's wave is young, whatever the crowd is doing"
        );
    }

    #[test]
    #[ignore = "a measurement: cargo test -p wave --release -- --ignored --nocapture"]
    fn how_long_waves_last() {
        let lap = U as f32 / 20.0;
        let one: Vec<f32> = (0..12)
            .map(|s| lifetime(2 * s, &[(100.0, 6.0)]) / lap)
            .collect();
        eprintln!("one wave, laps: {one:.2?}");
        let five: Vec<f32> = (0..8)
            .map(|s| {
                lifetime(
                    2 * s,
                    &[
                        (100.0, 6.0),
                        (250.0, 6.0),
                        (400.0, 6.0),
                        (550.0, 6.0),
                        (700.0, 6.0),
                    ],
                ) / lap
            })
            .collect();
        eprintln!("five waves, laps till all are gone: {five:.2?}");
    }
}
