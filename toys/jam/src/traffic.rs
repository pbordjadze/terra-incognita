//! The ring and its drivers. 22 cars on a single-lane circle 230 m round (Sugiyama et al. 2008, New J.
//! Phys. 10 033001). Each driver follows one rule, the Intelligent Driver Model (Treiber, Hennecke &
//! Helbing 2000): speed up toward a desired speed, and ease off by how close the car ahead is and how
//! fast it is closing. Nothing else steers them: no lanes, no lights, no leader.
//!
//! The constants are chosen (tests below) so the ring matches what the paper measured: an even flow at
//! about 30 km/h; once jammed, a cluster of stopped cars travelling backward at about 20 km/h while
//! the cars outside it run at about 40 km/h. Unlike the real ring, the even flow here is held just on
//! the stable side (metastable): the drivers' own small wobbles never break it, and a single hard
//! brake does, so a jam starts only where you tap.

/// The ring's circumference (m) and its cars.
pub const RING: f64 = 230.0;
pub const N: usize = 22;
/// The ring's radius (m).
pub const RADIUS: f64 = RING / std::f64::consts::TAU;
/// A small car, bumper to bumper (m).
pub const CAR: f64 = 4.4;

/// The drivers' rule (IDM): desired speed v0 (m/s), acceleration a and comfortable braking b (m/s²),
/// time gap T (s), standstill gap s0 (m).
pub const V0: f64 = 15.0;
pub const A: f64 = 2.0;
pub const B: f64 = 3.0;
pub const T: f64 = 0.5;
pub const S0: f64 = 1.5;

/// A tap: the car brakes this hard (m/s²) to a stop, and stands this long (s) before it goes on.
pub const TAP_BRAKE: f64 = 6.5;
pub const TAP_HOLD: f64 = 0.8;
/// The wobble in each driver's foot (m/s² a √s): small enough that the flow holds.
pub const WOBBLE: f64 = 0.06;

const STEP: f64 = 1.0 / 120.0;

#[derive(Clone, Debug)]
pub struct Car {
    /// Distance along the ring (m), 0..RING, the direction of travel.
    pub x: f64,
    pub v: f64,
    pub acc: f64,
    /// A tap: braking to a stop.
    pub stopping: bool,
    /// Seconds still to stand after a tap's stop.
    pub brake: f64,
    /// The brake lights, eased 0..1.
    pub lights: f64,
}

pub struct Ring {
    pub cars: Vec<Car>,
    rng: u64,
    pub t: f64,
    left: f64,
}

/// The even flow's speed for the ring's spacing (m/s).
pub fn even_speed() -> f64 {
    let s = RING / N as f64 - CAR;
    let (mut lo, mut hi) = (0.0, V0);
    for _ in 0..60 {
        let m = 0.5 * (lo + hi);
        let ss = (S0 + m * T) / s;
        if 1.0 - (m / V0).powi(4) - ss * ss > 0.0 {
            lo = m;
        } else {
            hi = m;
        }
    }
    lo
}

impl Ring {
    pub fn new(seed: u64) -> Ring {
        let v = even_speed();
        let cars = (0..N)
            .map(|i| Car { x: i as f64 * RING / N as f64, v, acc: 0.0, stopping: false, brake: 0.0, lights: 0.0 })
            .collect();
        Ring { cars, rng: seed.max(1), t: 0.0, left: 0.0 }
    }

    fn gauss(&mut self) -> f64 {
        // xorshift64*, two uniforms, Box-Muller
        let mut u = || {
            self.rng ^= self.rng >> 12;
            self.rng ^= self.rng << 25;
            self.rng ^= self.rng >> 27;
            ((self.rng.wrapping_mul(0x2545F4914F6CDD1D) >> 11) as f64 + 0.5) / (1u64 << 53) as f64
        };
        let (a, b) = (u(), u());
        (-2.0 * a.ln()).sqrt() * (std::f64::consts::TAU * b).cos()
    }

    /// The car ahead of car `i` (the cars never pass, so it is always the next one).
    pub fn ahead(i: usize) -> usize {
        (i + 1) % N
    }

    /// Gap to the car ahead, bumper to bumper (m).
    pub fn gap(&self, i: usize) -> f64 {
        let j = Self::ahead(i);
        (self.cars[j].x - self.cars[i].x).rem_euclid(RING) - CAR
    }

    pub fn tap(&mut self, i: usize) {
        self.cars[i].stopping = true;
        self.cars[i].brake = TAP_HOLD;
    }

    pub fn step(&mut self, dt: f64) {
        self.left += dt;
        while self.left >= STEP {
            self.left -= STEP;
            self.tick(STEP);
        }
    }

    fn tick(&mut self, dt: f64) {
        let ab = 2.0 * (A * B).sqrt();
        let mut acc = [0.0; N];
        for (i, a) in acc.iter_mut().enumerate() {
            let c = &self.cars[i];
            let lead = &self.cars[Self::ahead(i)];
            let gap = self.gap(i).max(0.1);
            let want = S0 + (c.v * T + c.v * (c.v - lead.v) / ab).max(0.0);
            *a = A * (1.0 - (c.v / V0).powi(4) - (want / gap).powi(2));
        }
        let w = WOBBLE / dt.sqrt();
        let calm = acc; // the drivers' intent, without the wobble: what the brake lights and engines show
        for (i, a) in acc.iter_mut().enumerate() {
            *a += w * self.gauss();
            let c = &mut self.cars[i];
            if c.stopping {
                *a = a.min(-TAP_BRAKE);
                if c.v <= 0.0 {
                    c.stopping = false;
                }
            } else if c.brake > 0.0 {
                *a = a.min(0.0);
                c.brake -= dt;
            }
        }
        for (i, c) in self.cars.iter_mut().enumerate() {
            c.acc = if c.stopping { acc[i] } else { 0.8 * c.acc + 0.2 * calm[i] };
            c.v = (c.v + acc[i] * dt).max(0.0);
            if c.v == 0.0 {
                c.acc = c.acc.max(0.0);
            }
            c.x = (c.x + c.v * dt).rem_euclid(RING);
            // the driver's foot on the brake: braking, or held still
            let on = if c.acc < -1.8 || c.v < 0.8 || c.stopping || c.brake > 0.0 { 1.0 } else { 0.0 };
            let k = if on > c.lights { 14.0 } else { 5.0 };
            c.lights += (on - c.lights) * (k * dt).min(1.0);
        }
        self.t += dt;
    }

    /// The slowest car's index and speed.
    pub fn slowest(&self) -> (usize, f64) {
        self.cars.iter().enumerate().map(|(i, c)| (i, c.v)).fold((0, f64::MAX), |m, c| if c.1 < m.1 { c } else { m })
    }

    /// Cars standing (under 1.5 m/s).
    pub fn stopped(&self) -> usize {
        self.cars.iter().filter(|c| c.v < 1.5).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kmh(v: f64) -> f64 {
        v * 3.6
    }

    #[test]
    fn even_flow_is_about_30_kmh_and_holds() {
        assert!((28.0..34.0).contains(&kmh(even_speed())), "{}", kmh(even_speed()));
        for seed in 1..5 {
            let mut r = Ring::new(seed);
            let mut low = f64::MAX;
            for _ in 0..(600 * 10) {
                r.step(0.1);
                low = low.min(r.slowest().1);
            }
            assert!(low > 0.85 * even_speed(), "seed {seed}: the flow broke by itself ({} km/h)", kmh(low));
        }
    }

    #[test]
    fn a_tap_makes_a_jam_that_runs_backward_at_about_20_kmh_and_stays() {
        for seed in 1..4 {
            let mut r = Ring::new(seed);
            r.step(3.0);
            r.tap(seed as usize * 5 % N);
            // let it form
            for _ in 0..600 {
                r.step(0.1);
            }
            assert!(r.stopped() >= 3, "seed {seed}: no jam ({} stopped)", r.stopped());
            // follow the slowest car's place for a minute
            let mut last = r.cars[r.slowest().0].x;
            let mut moved = 0.0;
            for _ in 0..600 {
                r.step(0.1);
                let x = r.cars[r.slowest().0].x;
                let mut d = x - last;
                if d > RING / 2.0 {
                    d -= RING;
                }
                if d < -RING / 2.0 {
                    d += RING;
                }
                moved += d;
                last = x;
            }
            let v = kmh(moved / 60.0);
            assert!((-26.0..-15.0).contains(&v), "seed {seed}: the jam moves at {v} km/h");
            let fastest = r.cars.iter().map(|c| c.v).fold(0.0, f64::max);
            assert!((36.0..52.0).contains(&kmh(fastest)), "outside the jam {}", kmh(fastest));
            assert!(r.stopped() >= 3, "seed {seed}: the jam dissolved");
        }
    }

    /// A tap always starts a jam: on the even flow, on a ring still settling from earlier taps, and
    /// three at once spread round the ring.
    #[test]
    fn every_tap_starts_a_jam() {
        let jammed = |r: &mut Ring| {
            let mut most = 0;
            for _ in 0..300 {
                r.step(0.1);
                most = most.max(r.stopped());
            }
            most >= 3
        };
        for seed in 1..9u64 {
            let mut r = Ring::new(seed);
            r.step(1.0 + seed as f64);
            r.tap((seed as usize * 7) % N);
            assert!(jammed(&mut r), "seed {seed}: one tap on the even flow");
            let mut r = Ring::new(seed);
            for k in 0..3 {
                r.tap((k * 7 + seed as usize) % N);
            }
            assert!(jammed(&mut r), "seed {seed}: three at once");
            // then a long time on, another tap still jams (or the jam is still there)
            for _ in 0..1200 {
                r.step(0.1);
            }
            r.tap(seed as usize % N);
            assert!(jammed(&mut r), "seed {seed}: a later tap");
        }
    }

    #[test]
    fn cars_never_touch() {
        let mut r = Ring::new(7);
        let mut low = f64::MAX;
        for k in 0..3000 {
            if k % 97 == 0 {
                r.tap(k % N);
            }
            r.step(0.1);
            for i in 0..N {
                low = low.min(r.gap(i));
            }
        }
        assert!(low > 0.5, "gap {low}");
    }
}

#[cfg(test)]
mod probe {
    use super::*;
    /// `cargo test -p jam --release probe_many -- --ignored --nocapture`: a hand tapping eleven cars
    /// in three seconds; how many stand a minute and two minutes on.
    #[test]
    #[ignore]
    fn probe_many() {
        for seed in 1..7u64 {
            let mut r = Ring::new(seed);
            r.step(2.0);
            for k in 0..11 {
                r.tap((k * 2 + seed as usize) % N);
                r.step(0.27);
            }
            let mut at = Vec::new();
            for s in 0..120 {
                r.step(1.0);
                if s == 30 || s == 60 || s == 119 {
                    at.push(r.stopped());
                }
            }
            println!("seed {seed}: standing at 30/60/120 s: {at:?}");
        }
    }
}
