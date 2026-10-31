//! Where a float goes in a step: the month's mean current, its leeway on the month's wind, the
//! eddy wander drifters measured there (a random flight with their measured memory, or the plainer
//! random walk), a coast that holds and ice that holds till it retreats. Deterministic: the same
//! (at, day, dt, seed, k) always gives the same answer, here and in `rules.wgsl` (to the rounding
//! of a multiply; see `sheets`). A float's real path is k = 0; its maybes are k = 1..n, each a
//! different draw of the same rules.
//!
//! The sources and the honest limits of every number are in `adrift/SOURCES.md`.

use super::sheets::{Coast, Currents, Eddy, EddyTime, Ice, Wind};
use crate::assets::Assets;
use crate::model::{Held, LonLat};

/// The Earth's radius the step converts metres to degrees with (the sphere the data are on).
pub const EARTH_RADIUS: f32 = 6_371_000.0;
pub const SECONDS_A_DAY: f32 = 86_400.0;

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
    pub currents: Currents,
    pub wind: Wind,
    pub eddy: Eddy,
    pub eddy_time: EddyTime,
    pub ice: Ice,
    pub coast: Coast,
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

    /// Every sheet the rules read, from the assets (the names `adrift/data/manifest.json` maps).
    pub fn load(assets: &dyn Assets) -> anyhow::Result<Rules> {
        Ok(Rules {
            currents: Currents::load(assets)?,
            wind: Wind::load(assets)?,
            eddy: Eddy::load(assets)?,
            eddy_time: EddyTime::load(assets)?,
            ice: Ice::load(assets)?,
            coast: Coast::load(assets)?,
            leeway: Self::LEEWAY,
            ice_hold: Self::ICE_HOLD,
            wander: 1.0,
            shore: 0.5,
        })
    }

    /// One step of `dt_days` from `at` on `day`: the new position and what held it.
    pub fn step(&self, at: LonLat, day: f64, dt_days: f64, seed: u64, k: u32) -> Step {
        let (lon, lat) = (at.lon as f32, at.lat as f32);
        let day = day as f32;
        let dt = dt_days as f32;
        let measured = self.currents.at(day, lon, lat).2;
        if self.ice.at(day, lon, lat) >= self.ice_hold {
            return Step { at, held: Held::Ice, measured };
        }
        let (u, v, _) = self.currents.at(day, lon, lat);
        let (wu, wv) = self.wind.at(day, lon, lat);
        let (ku, kv, _) = self.eddy.at(lon, lat);
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
        if self.ice.at(day, nlon, nlat) >= self.ice_hold {
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
        let coast = &self.coast;
        let d = coast.distance(nlon, nlat);
        if d < -self.shore {
            return (nlon, nlat, Held::No);
        }
        let px = Coast::DEGREES_PER_PIXEL;
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
                let mut last = Step { at, held: Held::No, measured: 1.0 };
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

/// A float with the memory of its wander: where it is and the eddy velocity it carries (m/s,
/// zonal and meridional; zero when first put). The answer to D1 (DEMANDS.md): with this carried
/// through the step, the wander is a random flight with the drifters' measured time scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drift {
    pub at: LonLat,
    pub eddy: (f32, f32),
}

impl Drift {
    pub fn put(at: LonLat) -> Drift {
        Drift { at, eddy: (0.0, 0.0) }
    }
}

/// The result of a flight step.
#[derive(Clone, Copy, Debug)]
pub struct Flight {
    pub drift: Drift,
    pub held: Held,
    pub measured: f32,
}

impl Rules {
    /// One step of `dt_days` with memory: the eddy velocity decays over the measured time scale
    /// T and is kicked so its variance stays K / T (an Ornstein-Uhlenbeck velocity: Taylor's
    /// random flight), and the float moves with the mean current, its leeway and that velocity.
    /// Over times long against T the spread is 2 K t, as `step`'s; within T a float keeps going
    /// the way it was going, as a drifter does. Deterministic for (drift, day, dt, seed, k).
    pub fn flight(&self, d: Drift, day: f64, dt_days: f64, seed: u64, k: u32) -> Flight {
        let (lon, lat) = (d.at.lon as f32, d.at.lat as f32);
        let day = day as f32;
        let dt = dt_days as f32;
        let measured = self.currents.at(day, lon, lat).2;
        if self.ice.at(day, lon, lat) >= self.ice_hold {
            return Flight { drift: Drift { at: d.at, eddy: (0.0, 0.0) }, held: Held::Ice, measured };
        }
        let (u, v, _) = self.currents.at(day, lon, lat);
        let (wu, wv) = self.wind.at(day, lon, lat);
        let (ku, kv, _) = self.eddy.at(lon, lat);
        let (tu, tv) = self.eddy_time.at(lon, lat);
        let secs = dt * SECONDS_A_DAY;
        let h = hash_seed(seed, k, day.to_bits());
        let gx = gaussian(h, 0);
        let gy = gaussian(h, 12);
        let (tu, tv) = (tu.max(0.25) * SECONDS_A_DAY, tv.max(0.25) * SECONDS_A_DAY);
        let (au, av) = ((-secs / tu).exp(), (-secs / tv).exp());
        let eu = d.eddy.0 * au + gx * (ku / tu * (1.0 - au * au)).sqrt() * self.wander;
        let ev = d.eddy.1 * av + gy * (kv / tv * (1.0 - av * av)).sqrt() * self.wander;
        let dx = (u + self.leeway * wu + eu) * secs;
        let dy = (v + self.leeway * wv + ev) * secs;
        let coslat = (lat.to_radians()).cos().max(0.01);
        let dlon = (dx / (EARTH_RADIUS * coslat)).to_degrees();
        let dlat = (dy / EARTH_RADIUS).to_degrees();
        let nlon = wrap(lon + dlon);
        let nlat = (lat + dlat).clamp(-89.5, 89.5);
        if self.ice.at(day, nlon, nlat) >= self.ice_hold {
            return Flight { drift: Drift { at: d.at, eddy: (0.0, 0.0) }, held: Held::Ice, measured };
        }
        let (slon, slat, held) = self.ashore(lon, lat, nlon, nlat);
        let eddy = if held == Held::Coast { (0.0, 0.0) } else { (eu, ev) };
        Flight { drift: Drift { at: LonLat { lon: slon as f64, lat: slat as f64 }, eddy }, held, measured }
    }

    /// A float's maybes with memory, as `maybes` (k = 1..=n), each the flight reached.
    pub fn flights(&self, from: Drift, day0: f64, until: f64, dt_days: f64, seed: u64, n: u32) -> Vec<Flight> {
        (1..=n)
            .map(|k| {
                let mut d = from;
                let mut day = day0;
                let mut last = Flight { drift: d, held: Held::No, measured: 1.0 };
                while day < until - 1e-9 {
                    let dt = dt_days.min(until - day);
                    last = self.flight(d, day, dt, seed, k);
                    d = last.drift;
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
