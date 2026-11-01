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
    /// How far off the shore a held float sits, in the coast sheet's pixels: one, the coast's own
    /// resolution (about 10 km), so a float never sits inside the shoreline's own uncertainty.
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
            shore: 1.0,
        })
    }

    /// One step of `dt_days` from `at` on `day`: the new position and what held it.
    pub fn step(&self, at: LonLat, day: f64, dt_days: f64, seed: u64, k: u32) -> Step {
        let (lon, lat) = (at.lon as f32, at.lat as f32);
        let day = day as f32;
        let dt = dt_days as f32;
        let (u, v, measured) = self.currents.at(day, lon, lat);
        if self.ice.at(day, lon, lat) >= self.ice_hold {
            return Step { at, held: Held::Ice, measured };
        }
        let (u, v) = (u * reach(measured), v * reach(measured));
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

/// Whether the sea at a place can carry a float, by what the record holds there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Put {
    /// Land: no float.
    Land,
    /// Water, but no drifter came within about three degrees of it in any month (the Caspian,
    /// the Great Lakes, Hudson Bay, the Red Sea, the Persian Gulf, the seas under the ice): the
    /// sheets hold only a value smoothed in from far away, which is no record of this water. The
    /// sea refuses the float: nothing scripts where it goes, and here nothing measured could.
    NoRecord,
    /// Water the record reaches only from a distance (`measured` under 0.5): the float is carried
    /// as a marked guess.
    Guess,
    /// Water a drifter measured.
    Sea,
}

impl Rules {
    /// How far the record reaches a place, 0..1: the most measured month's `measured` there.
    pub fn record(&self, at: LonLat) -> f32 {
        let (lon, lat) = (at.lon as f32, at.lat as f32);
        (0..12).map(|m| self.currents.0.sample(m, 2, lon, lat) / 255.0).fold(0.0, f32::max)
    }

    /// Whether a float may be put at `at`, and on what footing.
    pub fn may_put(&self, at: LonLat) -> Put {
        if self.coast.is_land(at) {
            return Put::Land;
        }
        let r = self.record(at);
        if r < Self::NO_RECORD {
            Put::NoRecord
        } else if r < Self::GUESS {
            Put::Guess
        } else {
            Put::Sea
        }
    }

    /// Below this the record is nothing: no measured cell within about 2.9 degrees.
    pub const NO_RECORD: f32 = 0.05;
    /// Below this the current is a guess from more than 1.5 degrees away (the mark's threshold).
    pub const GUESS: f32 = 0.5;
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
    /// the way it was going, as a drifter does. The kicks are world k's (`world_kick`), so the k-th
    /// paths of floats near each other are kicked alike. Deterministic for (drift, day, dt, k); the
    /// seed is kept in the signature and no longer read (R6).
    pub fn flight(&self, d: Drift, day: f64, dt_days: f64, seed: u64, k: u32) -> Flight {
        self.flight_under(LAW, d, day, dt_days, seed, k)
    }

    /// `flight` under a given law (R13: a leg's own, `Leg::law`): law 0 is the flight as shipped up
    /// to build 49, without the spinning eddies, bit for bit (`sea::tests::law_zero_is_the_old_flight`).
    pub fn flight_under(&self, law: u32, d: Drift, day: f64, dt_days: f64, seed: u64, k: u32) -> Flight {
        let (lon, lat) = (d.at.lon as f32, d.at.lat as f32);
        let day = day as f32;
        let dt = dt_days as f32;
        let (u, v, measured) = self.currents.at(day, lon, lat);
        if self.ice.at(day, lon, lat) >= self.ice_hold {
            return Flight { drift: Drift { at: d.at, eddy: (0.0, 0.0) }, held: Held::Ice, measured };
        }
        let (u, v) = (u * reach(measured), v * reach(measured));
        let (wu, wv) = self.wind.at(day, lon, lat);
        let (ku, kv, _) = self.eddy.at(lon, lat);
        let (tu, tv) = self.eddy_time.at(lon, lat);
        let secs = dt * SECONDS_A_DAY;
        let _ = seed; // the wander is the world's (k's), not the float's: see `world_kick`
        let (gx, gy) = world_kick(k, day, dt, lon, lat);
        let (tu, tv) = (tu.max(0.25) * SECONDS_A_DAY, tv.max(0.25) * SECONDS_A_DAY);
        let (au, av) = ((-secs / tu).exp(), (-secs / tv).exp());
        // in world k's spinning eddies the velocity turns as it decays (R12: the spinning flight)
        let om = if law >= 1 { spin(k, day, lon, lat) } else { 0.0 };
        let (ru, rv) = if om == 0.0 {
            d.eddy
        } else {
            let phi = om * dt;
            let (c, s) = (phi.cos(), phi.sin());
            (d.eddy.0 * c - d.eddy.1 * s, d.eddy.0 * s + d.eddy.1 * c)
        };
        let eu = ru * au + gx * (ku / tu * (1.0 - au * au)).sqrt() * self.wander;
        let ev = rv * av + gy * (kv / tv * (1.0 - av * av)).sqrt() * self.wander;
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
        let (slon, slat, held, eddy) = if law >= 2 {
            self.slide(lon, lat, nlon, nlat, (eu, ev))
        } else {
            let (slon, slat, held) = self.ashore(lon, lat, nlon, nlat);
            (slon, slat, held, if held == Held::Coast { (0.0, 0.0) } else { (eu, ev) })
        };
        Flight { drift: Drift { at: LonLat { lon: slon as f64, lat: slat as f64 }, eddy }, held, measured }
    }

    /// The coast under law 2 (R15): no water flows through it. A step that would end ashore (or
    /// within `shore` pixels of it) loses only its landward part, the coast's normal taken from the
    /// distance field's gradient where the step began, so the float slides along the shore with the
    /// rest of its motion; its eddy velocity loses its landward part too, and keeps the rest. Law 1
    /// put the end back along the gradient and stopped the eddy, which drew a V or an elbow at every
    /// touch. If the slid end is still too near (a headland, a strait), law 1's hold is the fallback.
    pub fn slide(&self, lon: f32, lat: f32, nlon: f32, nlat: f32, eddy: (f32, f32)) -> (f32, f32, Held, (f32, f32)) {
        let coast = &self.coast;
        if coast.distance(nlon, nlat) < -self.shore {
            return (nlon, nlat, Held::No, eddy);
        }
        let px = Coast::DEGREES_PER_PIXEL;
        let c = lat.to_radians().cos().max(0.01);
        // the landward normal where the step began, east and north in metres' proportions
        let gx = (coast.distance(lon + px, lat) - coast.distance(lon - px, lat)) / c;
        let gy = coast.distance(lon, (lat + px).min(89.9)) - coast.distance(lon, (lat - px).max(-89.9));
        let g = (gx * gx + gy * gy).sqrt();
        if g > 1e-6 {
            let (nx, ny) = (gx / g, gy / g);
            let (sx, sy) = ((nlon - lon) * c, nlat - lat);
            let sn = sx * nx + sy * ny;
            let (sx, sy) = if sn > 0.0 { (sx - sn * nx, sy - sn * ny) } else { (sx, sy) };
            let (blon, blat) = (wrap(lon + sx / c), (lat + sy).clamp(-89.5, 89.5));
            if coast.distance(blon, blat) < -self.shore * 0.5 {
                let en = eddy.0 * nx + eddy.1 * ny;
                let e = if en > 0.0 { (eddy.0 - en * nx, eddy.1 - en * ny) } else { eddy };
                return (blon, blat, Held::Coast, e);
            }
        }
        let (blon, blat, held) = self.ashore(lon, lat, nlon, nlat);
        (blon, blat, held, if held == Held::Coast { (0.0, 0.0) } else { eddy })
    }

    /// A float as it is put: at `at`, with a first eddy velocity drawn from what the drifters
    /// measured there (a Gaussian of variance K / T on each axis: the velocity a water parcel
    /// has at any moment, not the zero of a parcel that has just started), deterministic for
    /// (at, day, k): world k's water there (R6). The real float is k = 0 and each maybe k = 1..n
    /// starts with its own draw, so the cloud is right from the first day instead of growing from a point held still
    /// for the memory's two or three days.
    pub fn put(&self, at: LonLat, day: f64, seed: u64, k: u32) -> Drift {
        let (lon, lat) = (at.lon as f32, at.lat as f32);
        let (ku, kv, _) = self.eddy.at(lon, lat);
        let (tu, tv) = self.eddy_time.at(lon, lat);
        let (tu, tv) = (tu.max(0.25) * SECONDS_A_DAY, tv.max(0.25) * SECONDS_A_DAY);
        // the velocity world k's sea has here as the float is put (its own stream of the field,
        // apart from the steps' kicks), so two floats put near each other start as that world's
        // water there moves
        let _ = seed;
        let (gu, gv) = world_field(k ^ PUT_STREAM, quarter_cell(day as f32), lon, lat);
        let eu = gu * (ku / tu).sqrt() * self.wander;
        let ev = gv * (kv / tv).sqrt() * self.wander;
        Drift { at, eddy: (eu, ev) }
    }

    /// The velocity a float has between steps (m/s east, north): the mean current, its leeway
    /// and the eddy velocity it carries. For the look: a float drawn at `at` plus this times the
    /// seconds since its last step glides instead of ticking a quarter day at a time (at the
    /// phone's closest zoom a Gulf Stream float's quarter-day step is five points).
    pub fn velocity(&self, d: Drift, day: f64) -> (f32, f32) {
        let (lon, lat) = (d.at.lon as f32, d.at.lat as f32);
        let day = day as f32;
        if self.ice.at(day, lon, lat) >= self.ice_hold {
            return (0.0, 0.0);
        }
        let (u, v, measured) = self.currents.at(day, lon, lat);
        let (u, v) = (u * reach(measured), v * reach(measured));
        let (wu, wv) = self.wind.at(day, lon, lat);
        (u + self.leeway * wu + d.eddy.0, v + self.leeway * wv + d.eddy.1)
    }

    /// Where a float drawn between steps is: `at` carried by `velocity` for `days` (a fraction
    /// of a step; the coast is not consulted, the step does that).
    pub fn glide(&self, d: Drift, day: f64, days: f64) -> LonLat {
        let (u, v) = self.velocity(d, day);
        let secs = days as f32 * SECONDS_A_DAY;
        let coslat = (d.at.lat as f32).to_radians().cos().max(0.01);
        LonLat {
            lon: wrap(d.at.lon as f32 + (u * secs / (EARTH_RADIUS * coslat)).to_degrees()) as f64,
            lat: (d.at.lat as f32 + (v * secs / EARTH_RADIUS).to_degrees()).clamp(-89.5, 89.5) as f64,
        }
    }

    /// A float's maybes with memory, as `maybes` (k = 1..=n), each the flight reached.
    pub fn flights(&self, from: Drift, day0: f64, until: f64, dt_days: f64, seed: u64, n: u32) -> Vec<Flight> {
        (1..=n)
            .map(|k| {
                // each maybe is put afresh at the float's place, with its own first draw
                let mut d = if from.eddy == (0.0, 0.0) { self.put(from.at, day0, seed, k) } else { from };
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

/// How much of the sheet's current a float rides, by how far the record reaches the place:
/// all of it where a drifter measured within 1.5 degrees (`measured` 0.5 and over), none where
/// none came within three (0), and between in proportion. Where the sheet's value is only
/// smoothed in from far away (the Caspian, the Great Lakes, Hudson Bay) the float goes on the
/// wind's leeway and the wander alone, the way a lake's surface drifts.
#[inline]
pub fn reach(measured: f32) -> f32 {
    (measured / Rules::GUESS).clamp(0.0, 1.0)
}

#[inline]
pub fn wrap(lon: f32) -> f32 {
    (lon + 180.0).rem_euclid(360.0) - 180.0
}

/// The worlds (R6). A float's eddy wander is not its own dice but the sea of a world: world k is
/// one random eddy field, shared by every float's k-th path, so lamp A's k-th maybe and lamp B's
/// k-th meet the same eddies. Two paths near each other in one world are kicked alike and keep
/// together; once an eddy's width apart they are kicked apart, and the jets and fronts of the
/// mean current carry them their own ways, as real drifter pairs part. One path alone sees what it
/// saw before: a unit-variance kick each step, so its velocity keeps the measured variance K / T
/// and its memory T (`flight`), and the single cloud is unchanged in law.
///
/// The field: on a lattice `WORLD_CELL` degrees square, each node of each quarter day holds two
/// standard draws for world k; between nodes they are mixed bilinearly and renormalised to unit
/// variance (a kick is never weaker between nodes). The lattice's width is the one number here
/// not measured per place: about the mesoscale eddies' size (SOURCES.md › The worlds). A step's
/// kick is the quarter days whose middles it spans, summed and renormalised, so a quarter-day
/// lamp and half-day maybes read the same field.
pub const WORLD_CELL: f32 = 1.0;
/// The quarter days a step may span (a longer one reads only the first eight).
pub const WORLD_SPAN: u32 = 8;
/// The put's stream of the field, apart from the steps'.
pub const PUT_STREAM: u32 = 0x8000_0000;

/// The sea's current law (R13, `Leg::law`): 2, the coast a slip (R15: `slide`). 1 added the spinning
/// eddies (R12) to 0, the law as shipped up to build 49; each is kept for the lamps made under it.
pub const LAW: u32 = 2;

/// The spinning eddies (R12). Drifters' daily tracks (the bundled 1979 runs) set against a
/// random flight with no spin: their velocity's turning from day to day is mostly what such a
/// flight shows by chance (41% of 10-day windows turn consistently, against 39% for the plain
/// flight), but two in three that do are anticyclonic where chance gives one in two: some share
/// of the sea, roughly a tenth to a fifth, is a coherent eddy that carries a drifter round. Those
/// are drawn here: world k's sea on a lattice `SPIN_CELL` wide (an eddy's width) has, in a share
/// `SPIN_SHARE` of its cells, an eddy spinning at `SPIN_RATE` (the measured loopers' median, a loop
/// in eleven days), anticyclonic in `SPIN_ANTICYCLONIC` of them (clockwise in the north); each cell
/// is drawn afresh every `SPIN_EPOCH` days, at its own phase (an eddy's life). The flight's
/// velocity turns at that rate as it decays: Veneziani, Griffa, Reynolds and Mariano (2004)'s
/// spinning random flight. A float carried by one loops (where the eddy outruns the mean current:
/// in a jet it wiggles instead) at the measured speed (the drifters' loopers run only a fifth
/// faster than the rest); its spread there is less, K / (1 + (Omega T)^2), as loopers' is.
/// `adrift/SOURCES.md` › The spinning eddies.
pub const SPIN_CELL: f32 = 1.0;
pub const SPIN_SHARE: f32 = 0.15;
pub const SPIN_RATE: f32 = 0.58;
pub const SPIN_ANTICYCLONIC: f32 = 0.8;
pub const SPIN_EPOCH: f32 = 30.0;
pub const SPIN_STREAM: u32 = 0x5b17_0000;

/// World k's eddy spin at (lon, lat) on `day`, radians a day (positive anticlockwise; 0 where its
/// sea does not spin).
#[inline]
pub fn spin(k: u32, day: f32, lon: f32, lat: f32) -> f32 {
    let n = (360.0 / SPIN_CELL) as i32;
    let ix = (((lon + 180.0) / SPIN_CELL).floor() as i32).rem_euclid(n) as u32;
    let iy = ((lat + 90.0) / SPIN_CELL).floor() as i32 as u32;
    let node = mix(mix(mix(k ^ SPIN_STREAM) ^ ix) ^ iy);
    let phase = (node >> 8) as f32 / 16_777_216.0 * SPIN_EPOCH;
    let epoch = ((day + phase) / SPIN_EPOCH).floor() as i32;
    let h = mix(node ^ epoch as u32);
    if ((h >> 8) as f32 / 16_777_216.0) >= SPIN_SHARE {
        return 0.0;
    }
    let anti = ((mix(h ^ 0x0a17_c1c1) >> 8) as f32 / 16_777_216.0) < SPIN_ANTICYCLONIC;
    let north = if lat >= 0.0 { 1.0 } else { -1.0 };
    if anti {
        -north * SPIN_RATE
    } else {
        north * SPIN_RATE
    }
}

/// The quarter day `day` falls in.
#[inline]
pub fn quarter_cell(day: f32) -> i32 {
    (day * 4.0).floor() as i32
}

/// Four standard-ish draws from one hash's bytes: (sum of four uniform bytes - 2) * sqrt 3,
/// mean 0, variance 1, within +-3.46; integer to the last step, so the CPU and GPU agree.
#[inline]
pub fn bytes_gaussian(h: u32) -> f32 {
    let s = (h & 255) + ((h >> 8) & 255) + ((h >> 16) & 255) + (h >> 24);
    ((s as f32 + 2.0) / 256.0 - 2.0) * 1.732_050_8
}

/// World k's two unit draws at (lon, lat) in quarter day `cell`.
#[inline]
pub fn world_field(k: u32, cell: i32, lon: f32, lat: f32) -> (f32, f32) {
    let n = (360.0 / WORLD_CELL) as i32;
    let fx = (lon + 180.0) / WORLD_CELL;
    let fy = (lat + 90.0) / WORLD_CELL;
    let (x0, y0) = (fx.floor(), fy.floor());
    let (tx, ty) = (fx - x0, fy - y0);
    let base = mix(mix(k ^ 0x5eed_c0de) ^ cell as u32);
    let (x0, y0) = (x0 as i32, y0 as i32);
    let (mut gu, mut gv) = (0.0f32, 0.0f32);
    for (dx, dy, w) in [(0, 0, (1.0 - tx) * (1.0 - ty)), (1, 0, tx * (1.0 - ty)), (0, 1, (1.0 - tx) * ty), (1, 1, tx * ty)] {
        let ix = (x0 + dx).rem_euclid(n) as u32;
        let h = mix(mix(base ^ ix) ^ (y0 + dy) as u32);
        gu += w * bytes_gaussian(h);
        gv += w * bytes_gaussian(mix(h ^ 0x68e3_1da4));
    }
    let a = (1.0 - tx) * (1.0 - tx) + tx * tx;
    let b = (1.0 - ty) * (1.0 - ty) + ty * ty;
    let norm = 1.0 / (a * b).sqrt();
    (gu * norm, gv * norm)
}

/// World k's kick over a step of `dt` days from `day`: the quarter days whose middles the step
/// spans (the one it starts in, if it spans none), summed and renormalised to unit variance.
#[inline]
pub fn world_kick(k: u32, day: f32, dt: f32, lon: f32, lat: f32) -> (f32, f32) {
    let first = ((day - 0.125) * 4.0).ceil() as i32;
    let end = ((day + dt - 0.125) * 4.0).ceil() as i32;
    let (first, count) = if end > first { (first, ((end - first) as u32).min(WORLD_SPAN)) } else { (quarter_cell(day), 1) };
    let (mut gu, mut gv) = (0.0f32, 0.0f32);
    for i in 0..count {
        let (u, v) = world_field(k, first + i as i32, lon, lat);
        gu += u;
        gv += v;
    }
    let norm = 1.0 / (count as f32).sqrt();
    (gu * norm, gv * norm)
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
