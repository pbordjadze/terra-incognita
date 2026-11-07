//! The preset seam (R21, Engineering; painting-company row 199; `sea/MEDIA.md` › The wind preset's
//! seam): the few calls both media answer, so a float's path, a conductor's worlds and the clock
//! take the sea's lamps and the wind's seeds alike, without a branch at every call. The sea's
//! `Rules` and the wind's `AirRules` each answer them; `Media::of` picks by a leg's `Medium`.
//!
//! A state is the sea's `Drift` in both: its place and the wander's velocity it carries (the sea's
//! eddy, the air's gust, m/s east and north). What holds it is `Held`: the sea's coast and ice, the
//! air's `Landed` and `Ditched` (come down, for good).

use super::rules::{Drift, Flight, Put, Rules};
use super::wind::{Air, AirRules, Aloft};
use crate::model::{Held, LonLat, Medium};

pub trait MediumRules {
    /// A thing as it is put (or lifted) at `at` on `day` in world k: its first wander velocity.
    fn put(&self, at: LonLat, day: f64, seed: u64, k: u32) -> Drift;
    /// One step of `dt_days` under `law` (the sea's laws; the wind has one) from `d`, which `held`
    /// held after the step before (a thing come down stays down).
    fn flight(
        &self,
        law: u32,
        d: Drift,
        held: Held,
        day: f64,
        dt_days: f64,
        seed: u64,
        k: u32,
    ) -> Flight;
    /// The velocity between steps (m/s east, north), for a glide.
    fn velocity(&self, d: Drift, held: Held, day: f64) -> (f32, f32);
    /// Whether a thing may be put here.
    fn may_put(&self, at: LonLat) -> Put;
}

impl MediumRules for Rules {
    fn put(&self, at: LonLat, day: f64, seed: u64, k: u32) -> Drift {
        Rules::put(self, at, day, seed, k)
    }
    fn flight(
        &self,
        law: u32,
        d: Drift,
        _held: Held,
        day: f64,
        dt_days: f64,
        seed: u64,
        k: u32,
    ) -> Flight {
        self.flight_under(law, d, day, dt_days, seed, k)
    }
    fn velocity(&self, d: Drift, _held: Held, day: f64) -> (f32, f32) {
        Rules::velocity(self, d, day)
    }
    fn may_put(&self, at: LonLat) -> Put {
        Rules::may_put(self, at)
    }
}

fn aloft(held: Held) -> Aloft {
    match held {
        Held::Landed => Aloft::Land,
        Held::Ditched => Aloft::Sea,
        _ => Aloft::Flying,
    }
}

fn held(a: Aloft) -> Held {
    match a {
        Aloft::Flying => Held::No,
        Aloft::Land => Held::Landed,
        Aloft::Sea => Held::Ditched,
    }
}

impl MediumRules for AirRules {
    fn put(&self, at: LonLat, day: f64, _seed: u64, k: u32) -> Drift {
        let a = self.rise(at, day, k);
        Drift {
            at: a.at,
            eddy: a.gust,
        }
    }
    fn flight(
        &self,
        _law: u32,
        d: Drift,
        h: Held,
        day: f64,
        dt_days: f64,
        seed: u64,
        k: u32,
    ) -> Flight {
        let a = self.step(
            Air {
                at: d.at,
                gust: d.eddy,
                aloft: aloft(h),
            },
            day,
            dt_days,
            seed,
            k,
        );
        Flight {
            drift: Drift {
                at: a.at,
                eddy: a.gust,
            },
            held: held(a.aloft),
            measured: 1.0,
        }
    }
    fn velocity(&self, d: Drift, h: Held, day: f64) -> (f32, f32) {
        AirRules::velocity(
            self,
            Air {
                at: d.at,
                gust: d.eddy,
                aloft: aloft(h),
            },
            day,
        )
    }
    fn may_put(&self, _at: LonLat) -> Put {
        // the wind takes anything, anywhere: over land as over the sea
        Put::Sea
    }
}

/// Both media's rules, borrowed.
#[derive(Clone, Copy)]
pub struct Media<'a> {
    pub sea: &'a Rules,
    pub air: &'a AirRules,
}

impl<'a> Media<'a> {
    pub fn of(&self, m: Medium) -> &'a dyn MediumRules {
        match m {
            Medium::Sea => self.sea,
            Medium::Wind => self.air,
        }
    }
}
