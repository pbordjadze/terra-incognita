//! The crowd as an excitable medium: I. Farkas, D. Helbing & T. Vicsek, "Mexican waves in an
//! excitable medium", *Nature* 419 (2002) 131–132, their n-state model with their parameters.
//!
//! Each person is resting, active (standing, `NA` stages) or refractory (sitting back down, `NR`
//! stages). A resting person stands when the weighted share of active people within `R` seats is over
//! their own threshold (drawn from `C ± DC`); the weights fall off exponentially with distance and
//! lean one way, so the wave runs one way round (clockwise, seen from above, as most do). A person
//! who has stood goes through every stage and rests again. That is all.

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

pub struct Crowd {
    /// 0 resting, 1..=NA active, NA+1..=NA+NR refractory; by seat, row-major (row * U + seat)
    pub stage: Vec<u8>,
    threshold: Vec<f32>,
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
                } else if hot[u] && self.excited(u, v) {
                    1
                } else {
                    0
                };
                active += Self::is_active(self.next[i]) as usize;
            }
        }
        std::mem::swap(&mut self.stage, &mut self.next);
        self.active = active;
    }

    fn excited(&self, u: usize, v: usize) -> bool {
        let mut sum = 0.0;
        let mut seen = self.total;
        for &(du, dv, w) in &self.kernel {
            let vv = v as i32 + dv;
            if !(0..V as i32).contains(&vv) {
                seen -= w;
                continue;
            }
            let uu = (u as i32 + du).rem_euclid(U as i32) as usize;
            if Self::is_active(self.stage[vv as usize * U + uu]) {
                sum += w;
            }
        }
        sum > self.threshold[v * U + u] * seen
    }

    /// The resting people within `r` seats of seat `u`, row `v` stand up (a group trying to start a
    /// wave). Returns how many did.
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
                    n += 1;
                }
            }
        }
        n
    }

    /// Somewhere in the crowd a few dozen try to start one.
    pub fn somebody_tries(&mut self) -> (f32, f32) {
        let u = self.rng.range(0.0, U as f32);
        let v = self.rng.range(8.0, V as f32 - 8.0);
        let r = self.rng.range(1.9, 3.0);
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
            c.stand(300.0, 29.0, 1.6);
            for _ in 0..60 {
                c.tick();
            }
            ok += (c.active > 0) as usize;
        }
        eprintln!("{ok} of 20 tiny groups made a wave");
        assert!(ok < 10);
    }
}
