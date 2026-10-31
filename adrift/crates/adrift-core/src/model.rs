//! The shared types every module speaks in (DECISIONS.md §3). Changed only by a ruling there.

use serde::{Deserialize, Serialize};

/// A place on the Earth, degrees, WGS84 as the data has it. The globe is a unit sphere.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct LonLat {
    pub lon: f64,
    pub lat: f64,
}

impl LonLat {
    pub const fn new(lon: f64, lat: f64) -> Self {
        Self { lon, lat }
    }

    /// The unit vector of this place, in the globe's frame: y north, z toward longitude 0, x toward
    /// longitude 90° E (so `lat = asin(y)`, `lon = atan2(x, z)`, as Terra's `Globe.metal` reads it).
    pub fn to_xyz(self) -> [f64; 3] {
        let (la, lo) = (self.lat.to_radians(), self.lon.to_radians());
        [la.cos() * lo.sin(), la.sin(), la.cos() * lo.cos()]
    }

    pub fn from_xyz(v: [f64; 3]) -> Self {
        let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
        Self {
            lon: v[0].atan2(v[2]).to_degrees(),
            lat: (v[1] / n).clamp(-1.0, 1.0).asin().to_degrees(),
        }
    }

    /// Longitude folded into [-180, 180).
    pub fn normalized(self) -> Self {
        let mut lon = (self.lon + 180.0).rem_euclid(360.0) - 180.0;
        if lon >= 180.0 {
            lon -= 360.0;
        }
        Self { lon, lat: self.lat.clamp(-90.0, 90.0) }
    }

    /// Great-circle distance, in metres (the Earth taken as a sphere of 6 371 km).
    pub fn distance_m(self, other: LonLat) -> f64 {
        let a = self.to_xyz();
        let b = other.to_xyz();
        let dot = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
        dot.acos() * EARTH_RADIUS_M
    }
}

pub const EARTH_RADIUS_M: f64 = 6_371_000.0;

/// What holds a float where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Held {
    /// Free: the sea has it.
    No,
    /// Aground: a coast holds it.
    Coast,
    /// Frozen in: sea ice holds it till the ice retreats.
    Ice,
}

/// What the reader is doing to a float, if anything.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Hand {
    /// The sea's.
    Free,
    /// Under a resting finger: its maybes show.
    Held,
    /// Carried: moving with the finger, not the sea.
    Carried,
}

/// One bead of the wake: where the float was at the start of a day.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bead {
    pub at: LonLat,
    pub day: f64,
    /// How measured the sea was there (0..1): under 1 the sheet's smoothed-in guess (Truth's D3).
    #[serde(default = "one")]
    pub measured: f32,
}
fn one() -> f32 {
    1.0
}

/// A glass float.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Float {
    pub id: u32,
    pub at: LonLat,
    /// The day it was put (or last set down), `Clock.day`.
    pub put_day: f64,
    /// Its seed: with the day and the place it fixes its real path (`k = 0`) and its maybes.
    pub seed: u64,
    pub held: Held,
    pub hand: Hand,
    /// The eddy velocity it carries (zonal, meridional, m/s): the wander's memory, the drifters'
    /// measured time scale (Truth's D1). Zero when put or set down.
    #[serde(default)]
    pub eddy: [f32; 2],
    /// How measured the sea is where it is now (0..1; Truth's rules say).
    #[serde(default = "one")]
    pub measured: f32,
    /// One bead a day, oldest first, since it was put or last set down.
    pub wake: Vec<Bead>,
    /// The seconds since the reader last touched it (what was just done stays lit).
    pub lit_for: f64,
}

/// The date: days since 2000-01-01 00:00 UTC, running or held.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clock {
    pub day: f64,
    pub running: bool,
    /// Days a second while running.
    pub rate: f64,
}

impl Clock {
    /// The day of the year, 0 at January 1st, on the climatology's typical year of 365.25 days.
    pub fn year_day(&self) -> f64 {
        self.day.rem_euclid(365.25)
    }

    /// The month this day lies in (0 = January) and the next, and how far between their middles:
    /// Terra's reading of a date against a monthly climatology.
    pub fn months(&self) -> (u32, u32, f64) {
        let m = (self.year_day() / 365.25 * 12.0 - 0.5).rem_euclid(12.0);
        let a = m.floor();
        (a as u32 % 12, (a as u32 + 1) % 12, m - a)
    }

    /// The calendar month and day (a typical year: 2001 is used for the calendar).
    pub fn calendar(&self) -> (u32, u32) {
        const DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let mut d = (self.year_day() * 365.0 / 365.25).floor() as u32;
        for (i, n) in DAYS.iter().enumerate() {
            if d < *n {
                return (i as u32 + 1, d + 1);
            }
            d -= n;
        }
        (12, 31)
    }
}

/// Where the reader looks: the point of the globe at the screen's centre and how close.
/// Zoom 1 is the whole globe filling its room; 8 a region.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub lon: f64,
    pub lat: f64,
    pub zoom: f64,
}

impl Camera {
    /// World from view: Ry(lon) · Rx(-lat), as columns `[c0, c1, c2]` (Terra's `rotation`).
    pub fn rotation(&self) -> [[f64; 3]; 3] {
        let a = self.lon.to_radians();
        let b = -self.lat.to_radians();
        [
            [a.cos(), 0.0, -a.sin()],
            [a.sin() * b.sin(), b.cos(), a.cos() * b.sin()],
            [a.sin() * b.cos(), -b.sin(), a.cos() * b.cos()],
        ]
    }

    /// The point of the globe at `v` in view space.
    pub fn world(&self, v: [f64; 3]) -> [f64; 3] {
        let m = self.rotation();
        [
            m[0][0] * v[0] + m[1][0] * v[1] + m[2][0] * v[2],
            m[0][1] * v[0] + m[1][1] * v[1] + m[2][1] * v[2],
            m[0][2] * v[0] + m[1][2] * v[1] + m[2][2] * v[2],
        ]
    }

    /// A point of the world in view space (the rotation's transpose).
    pub fn view(&self, w: [f64; 3]) -> [f64; 3] {
        let m = self.rotation();
        [
            m[0][0] * w[0] + m[0][1] * w[1] + m[0][2] * w[2],
            m[1][0] * w[0] + m[1][1] * w[1] + m[1][2] * w[2],
            m[2][0] * w[0] + m[2][1] * w[1] + m[2][2] * w[2],
        ]
    }
}

/// The globe's disc on the screen, in points: centre and radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Disc {
    pub cx: f64,
    pub cy: f64,
    pub r: f64,
}

impl Disc {
    /// Where a place is on screen and how far it faces the viewer (1 at the centre, 0 at the rim,
    /// negative behind).
    pub fn project(&self, cam: &Camera, at: LonLat) -> ([f64; 2], f64) {
        let v = cam.view(at.to_xyz());
        ([self.cx + v[0] * self.r, self.cy - v[1] * self.r], v[2])
    }

    /// The place under a point of the screen, if it is on the globe.
    pub fn unproject(&self, cam: &Camera, x: f64, y: f64) -> Option<LonLat> {
        let vx = (x - self.cx) / self.r;
        let vy = (self.cy - y) / self.r;
        let r2 = vx * vx + vy * vy;
        if r2 >= 1.0 {
            return None;
        }
        Some(LonLat::from_xyz(cam.world([vx, vy, (1.0 - r2).sqrt()])))
    }
}

/// A finger (or a mouse button) on the screen, in points from the top left.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PointerEvent {
    pub id: u64,
    pub phase: Phase,
    pub x: f64,
    pub y: f64,
    /// Seconds, the shell's clock.
    pub t: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    Down,
    Move,
    Up,
    Cancel,
}

/// What is under a point of the screen, for the gestures to decide by.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    /// A float, and the place under the finger.
    Float(u32, LonLat),
    Sea(LonLat),
    Land(LonLat),
    /// Off the globe.
    Off,
}

/// The only way touch reaches the state: Motion produces these, Engineering applies them.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Intent {
    /// A tap on the sea: it rings there.
    Ring { at: LonLat },
    /// The finger rests on the sea: the ring thickens from `since` (Motion's D1).
    Press { at: LonLat, since: f64 },
    /// It lifted or moved before the beat: the ring lets go (and a pressed float is let be).
    Unpress,
    /// The finger lands on a float and rests from `since`: its maybes rise from the landing, before
    /// the beat that holds it (Motion's D7, R4). Cleared by `Unpress`, superseded by `Hold`.
    PressFloat { float: u32, since: f64 },
    /// A finger has rested a beat on the sea: a float is put there (and is held while the finger stays).
    Put { at: LonLat },
    /// A finger rests on a float: its maybes show.
    Hold { float: u32 },
    /// A held float moves with the finger.
    Carry { float: u32, to: LonLat },
    /// The finger lifts: the float is set down there, or (`None`) taken out past the rim.
    Release { float: u32, at: Option<LonLat> },
    /// The last touch is taken back.
    Undo,
    /// A touch taken back is given back.
    Redo,
    /// The page of credits opens (a tap on the credit line) or closes (any tap while it is open).
    ToggleCredits,
    /// The camera drifts with this float.
    Ride { float: u32 },
    /// The date is moved by so many days.
    Scrub { days: f64 },
    /// The date stops or runs.
    SetRunning { running: bool },
    /// Every float is taken out at once, as one touch: undo gives them all back (R4).
    ClearAll,
    /// The camera returns to the opening view (the whole Earth; the ride ends) (R4).
    Home,
}
