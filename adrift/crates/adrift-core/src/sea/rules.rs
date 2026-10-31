//! Where a float goes in a step: the month's mean current, its leeway on the month's wind, the
//! eddy wander drifters measured there as a random walk, a coast that holds and ice that holds
//! till it retreats. Deterministic: the same (at, day, dt, seed, k) always gives the same answer,
//! here and in `rules.wgsl` (to the rounding of a multiply; see `sheets`). A float's real path is
//! k = 0; its maybes are k = 1..n, each a different draw of the same rules.
//!
//! The sources and the honest limits of every number are in `adrift/SOURCES.md`.

use super::sheets::Sea;
use crate::model::LonLat;
use std::path::Path;

/// The Earth's radius the step converts metres to degrees with (the sphere the data are on).
pub const EARTH_RADIUS: f32 = 6_371_000.0;
pub const SECONDS_A_DAY: f32 = 86_400.0;

/// What held a float in a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    /// It took its whole step on open water.
    Free,
    /// The step would have put it on land: it is at the water's edge instead.
    Coast,
    /// It is in the month's ice, where it was or where it would have gone: it stays.
    Ice,
}

/// The result of a step.
#[derive(Clone, Copy, Debug)]
pub struct Step {
    pub at: LonLat,
    pub held: Held,
    /// How measured the current it rode is, 0..1: 1 where a drifter measured this month's
    /// current here, 0 where the sheet's value is smoothed in from far away (a guess).
    pub measured: f32,
}

pub struct Rules {
    pub sea: Sea,
    /// A float's leeway as a fraction of the 10 m wind, downwind (SOURCES.md: a glass float
    /// three fifths under, by the scaling of Daniel et al. 2002 on Breivik et al.'s measured
    /// sphere).
    pub leeway: f32,
    /// The ice concentration that holds a float (the Sea Ice Index's own edge).
    pub ice_hold: f32,
    /// The eddy wander's scale: 1 is the measured diffusivity; 0 switches it off (for tests).
    pub wander: f32,
    /// How far off the shore a held float sits, in the coast sheet's pixels.
    pub shore: f32,
}

impl Rules {
    pub const LEEWAY: f32 = 0.03;
    pub const ICE_HOLD: f32 = 0.15;

    pub fn new(sea: Sea) -> Rules {
        Rules { sea, leeway: Self::LEEWAY, ice_hold: Self::ICE_HOLD, wander: 1.0, shore: 0.5 }
    }

    pub fn load(dir: &Path) -> Result<Rules, String> {
        Ok(Rules::new(Sea::load(dir)?))
    }

    /// One step of `dt_days` from `at` on `day`: the new position and what held it.
    pub fn step(&self, at: LonLat, day: f64, dt_days: f64, seed: u64, k: u32) -> Step {
        let (lon, lat) = (at.lon as f32, at.lat as f32);
        let day = day as f32;
        let dt = dt_days as f32;
        let measured = self.sea.currents.at(day, lon, lat).2;
        if self.sea.ice.at(day, lon, lat) >= self.ice_hold {
            return Step { at, held: Held::Ice, measured };
        }
        let (u, v, _) = self.sea.currents.at(day, lon, lat);
        let (wu, wv) = self.sea.wind.at(day, lon, lat);
        let (ku, kv, _) = self.sea.eddy.at(lon, lat);
        let secs = dt * SECONDS_A_DAY;
        let h = hash_seed(seed, k, day.to_bits());
        let gx = gaussian(h, 0);
        let gy = gaussian(h, 12);
        let dx = (u + self.leeway * wu) * secs + gx * (2.0 * ku * secs).sqrt() * self.wander;
        let dy = (v + self.leeway * wv) * secs + gy * (2.0 * kv * secs).sqrt() * self.wander;
        let coslat = (lat.to_radians()).cos().max(0.01);
        let dlon = (dx / (EARTH_RADIUS * coslat)).to_degrees();
        let dlat = (dy / EARTH_RADIUS).to_degrees();
        let nlon = wrap(lon + dlon);
        let nlat = (lat + dlat).clamp(-89.5, 89.5);
        if self.sea.ice.at(day, nlon, nlat) >= self.ice_hold {
            return Step { at, held: Held::Ice, measured };
        }
        let (slon, slat, held) = self.ashore(lon, lat, nlon, nlat);
        Step { at: LonLat { lon: slon as f64, lat: slat as f64 }, held, measured }
    }

    /// Keeps a step off the land: a point that would be ashore (or within `shore` pixels of it) is
    /// put back along the distance field's gradient to `shore` pixels off; if that fails too (a
    /// pixel-thin strait, a bay the step jumped), the float stays where it was. Public for the
    /// tests.
    pub fn ashore(&self, lon: f32, lat: f32, nlon: f32, nlat: f32) -> (f32, f32, Held) {
        let coast = &self.sea.coast;
        let d = coast.distance(nlon, nlat);
        if d < -self.shore {
            return (nlon, nlat, Held::Free);
        }
        let px = super::sheets::Coast::DEGREES_PER_PIXEL;
        let gx = (coast.distance(nlon + px, nlat) - coast.distance(nlon - px, nlat)) / 2.0;
        let gy = (coast.distance(nlon, nlat - px) - coast.distance(nlon, nlat + px)) / 2.0;
        let g = (gx * gx + gy * gy).sqrt();
        if g > 1e-3 {
            let back = (d + self.shore + 0.5) / g;
            let blon = wrap(nlon - back * gx * px);
            let blat = (nlat - back * gy * px).clamp(-89.5, 89.5);
            if coast.distance(blon, blat) < -self.shore * 0.5 {
                return (blon, blat, Held::Coast);
            }
        }
        (lon, lat, Held::Coast)
    }

    /// A float's maybes on the CPU: n paths from `from` on `day0`, stepping `dt_days` until
    /// `until`, each the position reached (k = 1..=n). The GPU does the same in `rules.wgsl`.
    pub fn maybes(&self, from: LonLat, day0: f64, until: f64, dt_days: f64, seed: u64, n: u32) -> Vec<Step> {
        (1..=n)
            .map(|k| {
                let mut at = from;
                let mut day = day0;
                let mut last = Step { at, held: Held::Free, measured: 1.0 };
                while day < until - 1e-9 {
                    let dt = dt_days.min(until - day);
                    last = self.step(at, day, dt, seed, k);
                    at = last.at;
                    day += dt;
                }
                last
            })
            .collect()
    }
}

#[inline]
pub fn wrap(lon: f32) -> f32 {
    (lon + 180.0).rem_euclid(360.0) - 180.0
}

/// A 32-bit mixer (the "lowbias32" of Chris Wellons), the same in WGSL.
#[inline]
pub fn mix(mut x: u32) -> u32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    x
}

/// The hash a step's draws come from: the seed, the path (k) and the day.
#[inline]
pub fn hash_seed(seed: u64, k: u32, day_bits: u32) -> u32 {
    let mut h = mix(seed as u32 ^ 0x9e37_79b9);
    h = mix(h ^ (seed >> 32) as u32);
    h = mix(h ^ k);
    mix(h ^ day_bits)
}

/// Draw i of a step, uniform in [0, 1).
#[inline]
pub fn uniform(h: u32, i: u32) -> f32 {
    (mix(h ^ i.wrapping_mul(0x9e37_79b9)) >> 8) as f32 / 16_777_216.0
}

/// A standard normal from twelve uniforms (Irwin-Hall: mean 0, variance 1, within +-6), draws
/// i..i+12; no logarithm or cosine, so the CPU and the GPU agree.
#[inline]
pub fn gaussian(h: u32, i: u32) -> f32 {
    let mut s = 0.0;
    for j in 0..12 {
        s += uniform(h, i + j);
    }
    s - 6.0
}
