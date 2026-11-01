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
/// One leg of a float's life: put (or set down after a carry) at `at` on `day` with `seed`. Its
/// path from there is a pure function of these three and the rules (R5: time is reversible, so a
/// float's place at any date is read off the leg active then, never stepped from where it was).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Leg {
    pub at: LonLat,
    pub day: f64,
    pub seed: u64,
    /// Which of the sea's worlds this leg's path is (R11: 0 the real one, `k` the maybe a conductor's
    /// cue chose; `conduct.rs`).
    #[serde(default)]
    pub world: u32,
    /// The eddy velocity it starts with when it continues a flight (a conductor's cue, a take's
    /// start: R11), so the continuation is the same law as one unbroken flight; None for a put.
    #[serde(default)]
    pub eddy: Option<[f32; 2]>,
    /// The sea's law this leg was made under (R13): 0 the rules as shipped up to build 49 (no
    /// spinning eddies), `sea::rules::LAW` the current one. A leg continues the law of the leg it
    /// continues and a fresh put takes the current one, so a lamp saved on the phone keeps its
    /// dance across an update. A kept state from before the field reads 0.
    #[serde(default)]
    pub law: u32,
}

impl Leg {
    /// A leg put fresh: the real world, its own eddy, the current law.
    pub const fn new(at: LonLat, day: f64, seed: u64) -> Leg {
        Leg { at, day, seed, world: 0, eddy: None, law: crate::sea::rules::LAW }
    }

    /// The same, under `law` (a leg that continues another's).
    pub const fn under(self, law: u32) -> Leg {
        Leg { law, ..self }
    }
}

/// A conductor's gesture (R11, `conduct.rs`), in screen points and the hand's seconds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ConductGesture {
    /// A tap at a point: the world drawn nearest it.
    Tap { x: f64, y: f64 },
    /// A stroke `dx, dy` points over `secs`: the world that veered most that way over that stretch.
    Sweep { dx: f64, dy: f64, secs: f64 },
    /// A loop drawn, `turn` radians (the screen's y points down, so positive turns clockwise as seen) over `secs`: the world that
    /// turned most that way.
    Circle { turn: f64, secs: f64 },
}

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
    /// Kept for good (R13): a clear-all leaves it, and a journey clip or a copy takes the kept lamps
    /// when none is chosen.
    #[serde(default)]
    pub kept: bool,
    /// The same journey at every step of its path (the quarter-day grid), oldest first, ending at
    /// the step before now (R10: the line the look draws runs through the real steps, the eddy's
    /// stalls and turns, not a day's chords). Rebuilt with the wake; never stored.
    #[serde(skip)]
    pub trail: Vec<Bead>,
    /// The seconds since the reader last touched it (what was just done stays lit).
    pub lit_for: f64,
    /// Its legs, oldest first (R5); empty in a kept state from before legs, which `legs()` reads as
    /// one leg from the wake's first bead (or `at`) on `put_day` with `seed`.
    #[serde(default)]
    pub legs: Vec<Leg>,
}

impl Float {
    /// The legs, with the one a pre-leg state implies.
    pub fn legs(&self) -> Vec<Leg> {
        if self.legs.is_empty() {
            // a kept state from before legs (and before laws): the first law
            vec![Leg::new(self.wake.first().map_or(self.at, |b| b.at), self.put_day, self.seed).under(0)]
        } else {
            self.legs.clone()
        }
    }

    /// Whether the float exists on `day`: not before its first put.
    pub fn born(&self, day: f64) -> bool {
        self.legs.first().map_or(self.put_day, |l| l.day) <= day + 1e-9
    }

    /// The leg active on `day` (the last put on or before it), if any.
    pub fn leg_at(&self, day: f64) -> Option<usize> {
        self.legs.iter().rposition(|l| l.day <= day + 1e-9)
    }
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
    /// The picture turned about the screen's centre, degrees (R6: no pole the camera must keep
    /// up; with `roll` any orientation of the orb is a camera, so a drag can turn it freely over a
    /// pole). 0 is north up; positive turns the globe's north toward the screen's left.
    #[serde(default)]
    pub roll: f64,
}

impl Camera {
    pub const fn new(lon: f64, lat: f64, zoom: f64) -> Camera {
        Camera { lon, lat, zoom, roll: 0.0 }
    }

    /// World from view: Ry(lon) · Rx(-lat) · Rz(roll), as columns `[c0, c1, c2]` (Terra's
    /// `rotation`, then the roll about the view's axis).
    pub fn rotation(&self) -> [[f64; 3]; 3] {
        let a = self.lon.to_radians();
        let b = -self.lat.to_radians();
        let c0 = [a.cos(), 0.0, -a.sin()];
        let c1 = [a.sin() * b.sin(), b.cos(), a.cos() * b.sin()];
        let c2 = [a.sin() * b.cos(), -b.sin(), a.cos() * b.cos()];
        if self.roll == 0.0 {
            return [c0, c1, c2];
        }
        let (sr, cr) = self.roll.to_radians().sin_cos();
        [
            [cr * c0[0] + sr * c1[0], cr * c0[1] + sr * c1[1], cr * c0[2] + sr * c1[2]],
            [-sr * c0[0] + cr * c1[0], -sr * c0[1] + cr * c1[1], -sr * c0[2] + cr * c1[2]],
            c2,
        ]
    }

    /// The camera whose `rotation()` is `m` (columns, world from view), at this zoom: the centre's
    /// place from the third column, the roll from the first. At a pole the longitude is whatever
    /// `atan2` gives and the roll takes up the rest, so the picture is exact there too.
    pub fn from_rotation(m: [[f64; 3]; 3], zoom: f64) -> Camera {
        let c = m[2];
        let lat = c[1].clamp(-1.0, 1.0).asin().to_degrees();
        let lon = if c[0].abs() < 1e-12 && c[2].abs() < 1e-12 { 0.0 } else { c[0].atan2(c[2]).to_degrees() };
        let base = Camera { lon, lat, zoom, roll: 0.0 }.rotation();
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let roll = dot(m[0], base[1]).atan2(dot(m[0], base[0])).to_degrees();
        Camera { lon, lat, zoom, roll }
    }

    /// The globe turned on the screen by `angle` radians about `axis`, a direction in view space
    /// (x right, y up, z toward the reader): a drag of dx, dy points over a disc of radius r turns it
    /// about (-dy, dx, 0) by hypot(dx, dy) / r, and the place under the finger follows it, over a
    /// pole as anywhere else (R6; Motion's trackball).
    pub fn turned(&self, axis: [f64; 3], angle: f64) -> Camera {
        let n = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        if n < 1e-15 || angle == 0.0 {
            return *self;
        }
        let (x, y, z) = (axis[0] / n, axis[1] / n, axis[2] / n);
        let (s, c) = angle.sin_cos();
        let t = 1.0 - c;
        // Q (view-space rotation), row-major
        let q = [
            [t * x * x + c, t * x * y - s * z, t * x * z + s * y],
            [t * x * y + s * z, t * y * y + c, t * y * z - s * x],
            [t * x * z - s * y, t * y * z + s * x, t * z * z + c],
        ];
        // M' = M · Qᵀ: a world point's view place v = Mᵀ w becomes Q v
        let m = self.rotation();
        let mut out = [[0.0; 3]; 3];
        for j in 0..3 {
            // column j of M Qᵀ = Σ_k M[:,k] · Q[j][k]
            for i in 0..3 {
                out[j][i] = (0..3).map(|k| m[k][i] * q[j][k]).sum();
            }
        }
        Camera::from_rotation(out, self.zoom)
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

/// How a chosen float is shown (R8): as itself, its journey to follow, or as its cloud of maybes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shown {
    Lamp,
    Cloud,
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
    /// A float is chosen: its maybes and nearest drifter stay shown through zooming and panning
    /// until the sea is tapped or another is chosen (R5). `Ride` chooses its float too.
    Choose { float: u32 },
    /// This float joins the chosen, or leaves them if it is chosen already (R6: several clouds out at
    /// once, to compare; the last chosen is in front). `Choose` makes it the only one.
    ChooseAlso { float: u32 },
    /// The choice is let go (every chosen float, as lamp or as cloud).
    Unchoose,
    /// A float is shown as itself (`Shown::Lamp`: its journey, followed), as its cloud
    /// (`Shown::Cloud`: its maybes out, as `ChooseAlso` adds it), or neither (`None`), and comes to
    /// the front (R8: each lamp chosen its own way, mixed freely).
    Show { float: u32, shown: Option<Shown> },
    /// A take begins (`on`) or ends (R9: the studio's recording; `studio.rs`).
    Record { on: bool },
    /// The newest take is to be exported as a video: the shell polls `App::take_export_wanted`.
    Export,
    /// The date's pace, days a second (R14: a journey clip's pace varies, slowing where its lamps do
    /// something worth seeing; recorded in its take as intents).
    Rate { days_per_second: f64 },
    /// The studio's menu is asked for (R13: a rest on the record control): the shell polls
    /// `App::menu_wanted` and offers what can be made (a journey clip of the chosen lamps, the last
    /// take again, the lamps copied or pasted).
    Menu,
    /// A conductor's cue while a take is recorded (R11): the lamp `float` (None: every lamp out)
    /// takes the world that did what the gesture did.
    Conduct { float: Option<u32>, gesture: ConductGesture },
    /// The newest take's clip plays at this speed (R10: 0.5 is slow motion, the clip twice the take's
    /// length; clamped to 0.25..2). The take itself is unchanged.
    TakeSpeed { speed: f64 },
    /// The camera returns to the opening view (the whole Earth; the ride ends) (R4).
    Home,
}

#[cfg(test)]
mod camera_tests {
    use super::*;

    fn close(a: [[f64; 3]; 3], b: [[f64; 3]; 3]) -> bool {
        a.iter().flatten().zip(b.iter().flatten()).all(|(x, y)| (x - y).abs() < 1e-9)
    }

    #[test]
    fn a_rotation_reads_back_as_its_camera() {
        for &(lon, lat, roll) in &[(-30.0, 25.0, 0.0), (120.0, -60.0, 33.0), (10.0, 89.9, -150.0), (0.0, 90.0, 0.0), (45.0, -90.0, 70.0)] {
            let c = Camera { lon, lat, zoom: 2.0, roll };
            let back = Camera::from_rotation(c.rotation(), 2.0);
            assert!(close(c.rotation(), back.rotation()), "{c:?} -> {back:?}");
        }
    }

    #[test]
    fn a_turn_carries_the_globe_over_the_pole_without_a_jump() {
        // a drag straight up the screen from 80° N, in small steps: the centre crosses the pole and
        // comes down the far side, every step the same small arc (no pole lock, no jump)
        let mut c = Camera::new(-30.0, 80.0, 1.0);
        let step = 0.5f64.to_radians();
        let mut last = LonLat::new(c.lon, c.lat);
        for _ in 0..80 {
            c = c.turned([-1.0, 0.0, 0.0], -step);
            let now = LonLat::new(c.lon, c.lat);
            let arc = now.distance_m(last) / EARTH_RADIUS_M;
            assert!((arc - step).abs() < 1e-6, "{arc} at {c:?}");
            last = now;
        }
        assert!((c.lat - 60.0).abs() < 1e-6 && (c.lon - 150.0).abs() < 1e-6, "{c:?}");
        // the place under the screen's top moved by the same turn: what was 10° above the centre
        let v = c.view(LonLat::new(150.0, 70.0).to_xyz());
        assert!(v[2] > 0.0, "the far side came into view: {v:?}");
    }
}
