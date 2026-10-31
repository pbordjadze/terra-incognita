//! Where a float goes in a step. Deterministic for (place, day, dt, seed, k): the same on the CPU and
//! in `rules.wgsl` once Truth writes it. R0: the month's mean current, held by the coast and by ice.

use super::{Coast, Currents, Ice};
use crate::model::{Clock, Held, LonLat, EARTH_RADIUS_M};

pub struct Rules {
    pub currents: Currents,
    pub coast: Coast,
    pub ice: Ice,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub at: LonLat,
    pub held: Held,
}

const SECONDS_A_DAY: f64 = 86_400.0;

impl Rules {
    /// Ice at or above this concentration holds a float (the ground's drawn edge: Terra's 0.30–0.55).
    pub const ICE_HOLDS: f64 = 0.45;

    /// One step of `dt_days` from `at` on day `day`. `k = 0` is the float's real path, `k ≥ 1` its
    /// maybes (R0: the current alone, so every k is the same; Truth's eddy wander parts them).
    pub fn step(&self, at: LonLat, day: f64, dt_days: f64, _seed: u64, _k: u32) -> Step {
        let clock = Clock { day, running: true, rate: 0.0 };
        let months = clock.months();
        if self.coast.is_land(at) {
            return Step { at, held: Held::Coast };
        }
        if self.ice.at(months, at) >= Self::ICE_HOLDS {
            return Step { at, held: Held::Ice };
        }
        // Heun's method on the sphere: the current at the start, a trial step, the current there, the
        // mean of the two.
        let v0 = self.velocity(months, at);
        let trial = Self::advance(at, v0, dt_days);
        let v1 = self.velocity(months, trial);
        let v = [(v0[0] + v1[0]) / 2.0, (v0[1] + v1[1]) / 2.0];
        let next = Self::advance(at, v, dt_days).normalized();
        if self.coast.is_land(next) {
            // the coast holds it: it stays at the last water
            return Step { at, held: Held::Coast };
        }
        Step { at: next, held: Held::No }
    }

    /// East and north velocity in m/s.
    fn velocity(&self, months: (u32, u32, f64), at: LonLat) -> [f64; 2] {
        let c = self.currents.at(months, at);
        [c[0], c[1]]
    }

    /// A place moved by a velocity (m/s) for so many days, along the ground.
    fn advance(at: LonLat, v: [f64; 2], dt_days: f64) -> LonLat {
        let s = dt_days * SECONDS_A_DAY / EARTH_RADIUS_M; // radians per m/s
        let dlat = v[1] * s;
        let cos = at.lat.to_radians().cos().max(0.02);
        let dlon = v[0] * s / cos;
        LonLat::new(at.lon + dlon.to_degrees(), (at.lat + dlat.to_degrees()).clamp(-89.0, 89.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_east_at_equator() {
        let p = Rules::advance(LonLat::new(0.0, 0.0), [1.0, 0.0], 1.0);
        // a metre a second for a day is 86.4 km, 0.777° at the equator
        assert!((p.lon - 0.777).abs() < 0.01, "{p:?}");
        assert_eq!(p.lat, 0.0);
    }
}
