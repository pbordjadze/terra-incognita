//! The geometry every toy speaks in: places, the date, the camera, the globe's disc, a finger.
//! Copied from adrift-core/src/model.rs (Nov 2, 2026; the parts that are not Adrift's floats) so a
//! round of Adrift never breaks a toy: change it here freely.

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
        Self {
            lon,
            lat: self.lat.clamp(-90.0, 90.0),
        }
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
        Camera {
            lon,
            lat,
            zoom,
            roll: 0.0,
        }
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
            [
                cr * c0[0] + sr * c1[0],
                cr * c0[1] + sr * c1[1],
                cr * c0[2] + sr * c1[2],
            ],
            [
                -sr * c0[0] + cr * c1[0],
                -sr * c0[1] + cr * c1[1],
                -sr * c0[2] + cr * c1[2],
            ],
            c2,
        ]
    }

    /// The camera whose `rotation()` is `m` (columns, world from view), at this zoom: the centre's
    /// place from the third column, the roll from the first. At a pole the longitude is whatever
    /// `atan2` gives and the roll takes up the rest, so the picture is exact there too.
    pub fn from_rotation(m: [[f64; 3]; 3], zoom: f64) -> Camera {
        let c = m[2];
        let lat = c[1].clamp(-1.0, 1.0).asin().to_degrees();
        let lon = if c[0].abs() < 1e-12 && c[2].abs() < 1e-12 {
            0.0
        } else {
            c[0].atan2(c[2]).to_degrees()
        };
        let base = Camera {
            lon,
            lat,
            zoom,
            roll: 0.0,
        }
        .rotation();
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let roll = dot(m[0], base[1]).atan2(dot(m[0], base[0])).to_degrees();
        Camera {
            lon,
            lat,
            zoom,
            roll,
        }
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
