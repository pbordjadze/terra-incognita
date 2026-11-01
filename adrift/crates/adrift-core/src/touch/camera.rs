//! The camera under a finger: orbit anchored to the point the finger took, fling, pinch anchored to
//! the fingers, the double tap's flight, riding a float, and the slow turn when left alone.
//! `CameraControl` is the camera's truth (the app holds it and the look draws it); `Gestures` asks
//! it to move with `CameraMove`s, which `apply` carries out against the disc as drawn.

use super::motion::{self, Motion};
use crate::model::{Camera, Disc, LonLat};

/// What the gestures ask of the camera.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraMove {
    /// A finger landed: whatever the camera was doing of its own stops at once.
    Stop,
    /// A one-finger drag: the finger is at (x, y), having moved (dx, dy); `grab` is the point of
    /// the globe it took when the drag began (None off the disc), kept under it.
    Drag { grab: Option<LonLat>, x: f64, y: f64, dx: f64, dy: f64 },
    /// The finger let go moving, points a second.
    Fling { vx: f64, vy: f64 },
    /// Two fingers: zoom by `factor`; `grab`, the point under their midpoint before, goes from
    /// (ax, ay) to (bx, by), so what the fingers hold stays under them.
    Pinch { grab: Option<LonLat>, ax: f64, ay: f64, bx: f64, by: f64, factor: f64 },
    /// A double tap at a place of the globe (None off it): fly in about it, or out.
    Fly { at: Option<LonLat> },
    /// The ride ends (a tap on the open sea lets the float go); the camera stays where it is.
    Leave,
    /// A lamp just chosen (R6, from the panel): if it is out of sight, or out toward the globe's
    /// rim where its cloud would lie along the limb, the globe turns (at the same zoom, the
    /// flight's ease) just far enough to bring it within `motion::FACE_WITHIN` of the disc's radius
    /// from the centre; where it already reads well, nothing moves.
    Face { at: LonLat },
    /// Follow the choice (R8): the camera holds what is chosen in view (the app's `riding` names
    /// the float in front); the zoom it has now is the deepest the framing will come back to.
    Follow { float: u32 },
    /// What is chosen, as the camera should hold it this frame (R8): the centre of every place to
    /// frame (`App::frame_points`) and the widest angle of any from it, radians. While the camera
    /// follows, it turns toward the centre and zooms out (never in past where the follow began) to
    /// hold them all within `motion::FRAME_WITHIN` of the room; a single place is the old ride.
    Frame { centre: LonLat, spread: f64, n: usize },
    /// Two fingers turned about each other by `angle` radians on the screen (y down, so positive
    /// is clockwise) about their midpoint (x, y): the orb rolls about the place under the midpoint
    /// (the screen's centre off the disc) by as much, so what the fingers hold turns with them (R8).
    Twist { x: f64, y: f64, angle: f64 },
}

#[derive(Clone, Copy, Debug)]
struct Flight {
    from: Camera,
    to: Camera,
    start: f64,
    duration: f64,
}

/// The camera and everything that moves it.
#[derive(Clone, Copy, Debug)]
pub struct CameraControl {
    pub camera: Camera,
    /// The float the camera drifts with, if any (the app sets it on `Intent::Ride`; any drag clears it).
    pub riding: Option<u32>,
    pub zoom_range: (f64, f64),
    /// A fling, points a second.
    spin: (f64, f64),
    flight: Option<Flight>,
    /// The ride's spring velocity, degrees a second.
    ride_v: f64,
    /// The ridden float's last place and its smoothed speed (degrees a second): the spring aims
    /// ahead by the lag a steady speed would leave, so a fast float is kept centred, not trailed.
    ride_seen: Option<(LonLat, [f64; 3])>,
    /// When a finger last touched (the shell's seconds), for the idle turn.
    last_touch: f64,
    /// The disc the last move was measured in (the fling turns by it between frames).
    disc: Disc,
    /// What to frame while following (R8): centre, spread (radians), how many places.
    frame: Option<(LonLat, f64, usize)>,
    /// The zoom when the follow began: the framing zooms out from it to hold everything and back in
    /// to it, never deeper (only the hand zooms in).
    frame_zoom: f64,
    /// The framing's zoom spring velocity (log zoom a second).
    zoom_v: f64,
    /// Whether the camera followed last frame: a follow that begins (a tap's `Ride`, a `Follow`)
    /// takes the zoom it finds as the framing's deepest.
    was_following: bool,
}

impl CameraControl {
    pub fn new(camera: Camera) -> CameraControl {
        CameraControl {
            camera,
            riding: None,
            zoom_range: (motion::ZOOM_MIN, motion::ZOOM_MAX),
            spin: (0.0, 0.0),
            flight: None,
            ride_v: 0.0,
            ride_seen: None,
            last_touch: f64::NEG_INFINITY,
            disc: Disc { cx: 0.0, cy: 0.0, r: 0.0 },
            frame: None,
            frame_zoom: 1.0,
            zoom_v: 0.0,
            was_following: false,
        }
    }

    /// Whether anything of the camera's own is moving: the shell draws at the full rate while it is.
    pub fn moving(&self) -> bool {
        self.flight.is_some() || self.spin.0.abs() + self.spin.1.abs() > motion::FLING_REST || self.riding.is_some()
    }

    /// Carries out one move against the disc as drawn.
    pub fn apply(&mut self, disc: &Disc, m: CameraMove, now: f64, motion: Motion) {
        self.disc = *disc;
        match m {
            CameraMove::Stop => self.stop_at(now),
            CameraMove::Drag { grab, x, y, dx, dy } => {
                self.riding = None;
                match grab {
                    Some(g) => self.turn_to(disc, g, x, y, dx, dy),
                    None => self.turn(disc, dx, dy),
                }
            }
            CameraMove::Fling { vx, vy } => {
                self.last_touch = now;
                self.spin = if motion.reduce { (0.0, 0.0) } else { (vx, vy) };
            }
            CameraMove::Pinch { grab, ax, ay, bx, by, factor } => self.pinch(disc, grab, ax, ay, bx, by, factor),
            CameraMove::Fly { at } => self.double_tap(at, now, motion),
            CameraMove::Leave => self.riding = None,
            CameraMove::Face { at } => self.face(at, now, motion),
            CameraMove::Follow { float } => self.riding = Some(float),
            CameraMove::Frame { centre, spread, n } => self.frame = Some((centre, spread, n)),
            CameraMove::Twist { x, y, angle } => self.twist(disc, x, y, angle),
        }
    }

    /// The zoom that holds places spread `spread` radians about the centre within `FRAME_WITHIN` of
    /// the room (orthographic: a place θ from the centre lies r·sin θ out), no deeper than the follow
    /// began, no shallower than the least.
    fn frame_zoom_for(&self, spread: f64) -> f64 {
        let d = self.disc;
        let r1 = d.r / self.camera.zoom.max(1e-6);
        let room = d.cx.min(d.cy).max(1.0);
        let needed = if spread >= motion::FRAME_WIDEST { self.zoom_range.0 } else { motion::FRAME_WITHIN * room / (r1 * spread.sin()).max(1e-6) };
        needed.min(self.frame_zoom.max(self.zoom_range.0)).clamp(self.zoom_range.0, self.zoom_range.1)
    }

    /// Turns the globe (no zoom) so `at` lies within `FACE_WITHIN` of the room from the centre:
    /// along the great circle from the camera's centre toward it, no further than needed.
    fn face(&mut self, at: LonLat, now: f64, motion: Motion) {
        if self.riding.is_some() {
            return; // the ride keeps its own float centred
        }
        let d = self.disc;
        // the room: the disc, or the screen about it when zoomed in past it (cx, cy its middle)
        let room = d.r.min(d.cx).min(d.cy).max(1.0);
        let within = motion::FACE_WITHIN * room;
        let (p, facing) = d.project(&self.camera, at);
        let off = (p[0] - d.cx).hypot(p[1] - d.cy);
        if facing > 0.0 && off <= within {
            return;
        }
        // the angle from the centre at which a place lies `within` points out; the globe turns about
        // the axis that carries the lamp straight toward the centre, by no more than it must
        let keep = (within / d.r).clamp(0.0, 1.0).asin();
        let v = self.camera.view(at.to_xyz());
        let ang = v[2].clamp(-1.0, 1.0).acos();
        if ang <= keep + 1e-9 {
            return;
        }
        let to = toward_centre(&self.camera, at, ang - keep);
        self.fly_to(to, now, motion);
    }

    // --- the sphere ---

    /// Turns the globe as a finger moving (dx, dy) points would drag the point at the screen's
    /// centre: a trackball (R6), about the view's axis (dy, dx, 0) by hypot(dx, dy) / r, so the orb
    /// turns the same way under the hand over a pole as anywhere else, with no north to stop at.
    pub fn turn(&mut self, disc: &Disc, dx: f64, dy: f64) {
        if disc.r <= 0.0 {
            return;
        }
        self.camera = self.camera.turned([dy, dx, 0.0], dx.hypot(dy) / disc.r);
        self.riding = None;
    }

    /// Turns the globe so that `grab`, the point of the globe the finger took, lies exactly under
    /// the finger at (x, y): the one rotation carrying its view direction to the finger's (R6: no
    /// solving in lon and lat, so no pole to stall on). Off the disc, or with the point gone round
    /// the back, the finger's own move turns the globe by `turn` alone.
    fn turn_to(&mut self, disc: &Disc, grab: LonLat, x: f64, y: f64, dx: f64, dy: f64) {
        let vx = (x - disc.cx) / disc.r;
        let vy = (disc.cy - y) / disc.r;
        let r2 = vx * vx + vy * vy;
        let a = self.camera.view(grab.to_xyz());
        if r2 >= 1.0 || a[2] <= 0.0 {
            self.turn(disc, dx, dy);
            return;
        }
        let b = [vx, vy, (1.0 - r2).sqrt()];
        let axis = cross(a, b);
        let angle = dot(a, b).clamp(-1.0, 1.0).acos();
        self.camera = self.camera.turned(axis, angle);
    }

    /// Two fingers: zoom by `factor`, and move the point of the globe that was under (ax, ay) to
    /// (bx, by), so what the fingers hold stays under them.
    fn pinch(&mut self, disc: &Disc, grab: Option<LonLat>, ax: f64, ay: f64, bx: f64, by: f64, factor: f64) {
        self.riding = None;
        let z0 = self.camera.zoom;
        self.camera.zoom = (z0 * factor).clamp(self.zoom_range.0, self.zoom_range.1);
        // the disc grows with the zoom
        let grown = Disc { r: disc.r * self.camera.zoom / z0, ..*disc };
        match grab {
            Some(g) => self.turn_to(&grown, g, bx, by, bx - ax, by - ay),
            None => self.turn(&grown, bx - ax, by - ay),
        }
    }

    /// Rolls the orb about the place under (x, y) by a twist of the fingers (R8): a clockwise twist
    /// on the screen turns the globe clockwise about that place, which stays where it is.
    fn twist(&mut self, disc: &Disc, x: f64, y: f64, angle: f64) {
        if angle == 0.0 || disc.r <= 0.0 {
            return;
        }
        self.riding = None;
        let vx = (x - disc.cx) / disc.r;
        let vy = (disc.cy - y) / disc.r;
        let r2 = vx * vx + vy * vy;
        let axis = if r2 < 1.0 { [vx, vy, (1.0 - r2).sqrt()] } else { [0.0, 0.0, 1.0] };
        // view z toward the reader: a positive turn about it is counterclockwise on the screen
        self.camera = self.camera.turned(axis, -angle);
    }

    /// Zoom by a factor about the screen's centre (the compatibility of R0's `zoom_by`).
    pub fn zoom_by(&mut self, factor: f64) {
        self.camera.zoom = (self.camera.zoom * factor).clamp(self.zoom_range.0, self.zoom_range.1);
    }

    /// A finger landed: whatever the camera was doing of its own stops at once.
    fn stop_at(&mut self, now: f64) {
        self.last_touch = now;
        self.flight = None;
        self.spin = (0.0, 0.0);
    }
    pub fn stop(&mut self) {
        self.flight = None;
        self.spin = (0.0, 0.0);
    }

    /// A double tap: fly in about the point (toward it below zoom 4, about the centre above), or
    /// out to the whole globe from deeper than half the deepest. A cut under Reduce Motion.
    fn double_tap(&mut self, at: Option<LonLat>, now: f64, motion: Motion) {
        let z = self.camera.zoom;
        let deepest = self.zoom_range.1;
        let target_zoom = if z < deepest / 2.0 { (z * 2.0).min(deepest) } else { 1.0 };
        let mut to = match at {
            Some(p) if z < 4.0 && target_zoom > z => {
                let ang = self.camera.view(p.to_xyz())[2].clamp(-1.0, 1.0).acos();
                toward_centre(&self.camera, p, ang)
            }
            _ => self.camera,
        };
        to.zoom = target_zoom;
        // a following camera frames no deeper than the hand last asked (R8)
        self.frame_zoom = target_zoom;
        self.fly_to(to, now, motion);
    }

    pub fn fly_to(&mut self, to: Camera, now: f64, motion: Motion) {
        self.spin = (0.0, 0.0);
        let duration = if motion.reduce { 0.0 } else { motion::FLIGHT_S };
        if duration == 0.0 {
            self.camera = to;
            self.flight = None;
        } else {
            self.flight = Some(Flight { from: self.camera, to, start: now, duration });
        }
    }

    // --- every frame ---

    /// Advances the camera's own motion: a flight, a fling, the ride (given where the ridden float
    /// is now), or the idle turn (only when `sea_is_empty`, nothing is ridden and no finger is
    /// down). Returns whether the camera moved.
    pub fn tick(&mut self, now: f64, dt: f64, ride_at: Option<LonLat>, sea_is_empty: bool, touching: bool, motion: Motion) -> bool {
        let dt = dt.clamp(0.0, 0.05);
        if self.last_touch == f64::NEG_INFINITY {
            // the first frame counts as a touch: the idle turn waits from the start
            self.last_touch = now;
        }
        let before = self.camera;
        if self.riding.is_some() && !self.was_following {
            self.frame_zoom = self.camera.zoom;
        }
        self.was_following = self.riding.is_some();
        if let Some(fl) = self.flight {
            let u = if fl.duration > 0.0 { ((now - fl.start) / fl.duration).min(1.0) } else { 1.0 };
            let e = motion::ease_in_out(u);
            // the orientation slerped (R6: the shortest turn from one to the other, a pole
            // included, the roll with it, so Home comes back upright), the zoom by its log
            let zoom = (fl.from.zoom.ln() + (fl.to.zoom.ln() - fl.from.zoom.ln()) * e).exp();
            self.camera = if u >= 1.0 { Camera { zoom, ..fl.to } } else { Camera::from_rotation(quat_mat(slerp(mat_quat(fl.from.rotation()), mat_quat(fl.to.rotation()))(e)), zoom) };
            if u >= 1.0 {
                self.flight = None;
            }
        } else if !touching && self.spin.0.abs() + self.spin.1.abs() > motion::FLING_REST {
            let disc = self.disc;
            self.turn(&disc, self.spin.0 * dt, self.spin.1 * dt);
            self.spin = (motion::fling_decay(self.spin.0, dt), motion::fling_decay(self.spin.1, dt));
        } else if let (Some((centre, spread, n)), true) = (self.frame.filter(|f| f.2 > 1), self.riding.is_some()) {
            // several places to hold (R8): turn toward their centre and zoom out to hold the
            // widest within FRAME_WITHIN of the room, back in no deeper than the follow began
            let _ = n;
            let target_zoom = self.frame_zoom_for(spread);
            if motion.reduce {
                let ang = self.camera.view(centre.to_xyz())[2].clamp(-1.0, 1.0).acos();
                self.camera = toward_centre(&self.camera, centre, ang);
                self.camera.zoom = target_zoom;
            } else {
                let ang = self.camera.view(centre.to_xyz())[2].clamp(-1.0, 1.0).acos();
                let (left, v) = motion::spring(ang, self.ride_v, 0.0, motion::FRAME_TAU, dt);
                self.camera = toward_centre(&self.camera, centre, ang - left.max(0.0));
                self.ride_v = v;
                let (lz, vz) = motion::spring(self.camera.zoom.ln(), self.zoom_v, target_zoom.ln(), motion::FRAME_TAU, dt);
                self.camera.zoom = lz.exp().clamp(self.zoom_range.0, self.zoom_range.1);
                self.zoom_v = vz;
            }
        } else if let Some(p) = ride_at.filter(|_| self.riding.is_some()) {
            let px = p.to_xyz();
            if motion.reduce {
                let ang = self.camera.view(px)[2].clamp(-1.0, 1.0).acos();
                self.camera = toward_centre(&self.camera, p, ang);
            } else {
                // the float's velocity on the sphere, smoothed over about a third of a second (it
                // steps, not glides)
                let speed = match self.ride_seen {
                    Some((last, v)) if dt > 0.0 => {
                        let l = last.to_xyz();
                        let now_v = [(px[0] - l[0]) / dt, (px[1] - l[1]) / dt, (px[2] - l[2]) / dt];
                        let k = 1.0 - (-dt / motion::RIDE_SPEED_TAU).exp();
                        [v[0] + (now_v[0] - v[0]) * k, v[1] + (now_v[1] - v[1]) * k, v[2] + (now_v[2] - v[2]) * k]
                    }
                    Some((_, v)) => v,
                    None => [0.0; 3],
                };
                self.ride_seen = Some((p, speed));
                // aim ahead by the lag a steady speed leaves a critically damped spring (2τ·v), and
                // close the angle to the aim on the spring, turning about the axis that carries it
                // straight to the centre (R6: in rotation, so a pole is no different)
                let ahead = 2.0 * motion::RIDE_TAU;
                let aim = LonLat::from_xyz(norm([px[0] + speed[0] * ahead, px[1] + speed[1] * ahead, px[2] + speed[2] * ahead]));
                let ang = self.camera.view(aim.to_xyz())[2].clamp(-1.0, 1.0).acos();
                let (left, v) = motion::spring(ang, self.ride_v, 0.0, motion::RIDE_TAU, dt);
                self.camera = toward_centre(&self.camera, aim, ang - left.max(0.0));
                self.ride_v = v;
            }
        } else if !touching && sea_is_empty && self.riding.is_none() && !motion.reduce && now - self.last_touch > motion::IDLE_AFTER {
            // left alone, it turns as the Earth does, west to east
            self.camera.lon = motion::wrap_lon(self.camera.lon - motion::IDLE_RATE * dt);
        }
        if self.riding.is_none() {
            self.frame = None;
            self.zoom_v = 0.0;
            self.ride_v = 0.0;
            self.ride_seen = None;
        }
        self.camera != before
    }
}

// --- rotations (R6: the orb has no north to stop at) ---

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn norm(a: [f64; 3]) -> [f64; 3] {
    let n = dot(a, a).sqrt().max(1e-15);
    [a[0] / n, a[1] / n, a[2] / n]
}

/// The camera turned by `angle` radians about the axis that carries `at` straight toward the
/// screen's centre (the whole angle centres it); the roll goes with the turn, nothing else changes.
/// Away from the poles the roll is kept as it was (a turn about a tilted axis would otherwise
/// creep it), so a reader who has never gone over a pole keeps north up.
fn toward_centre(cam: &Camera, at: LonLat, angle: f64) -> Camera {
    let v = cam.view(at.to_xyz());
    let axis = cross(v, [0.0, 0.0, 1.0]);
    let t = if dot(axis, axis) < 1e-20 {
        // at the centre already, or right behind it: any axis in the screen's plane
        if v[2] > 0.0 { *cam } else { cam.turned([0.0, 1.0, 0.0], angle) }
    } else {
        cam.turned(axis, angle)
    };
    if t.lat.abs() < 85.0 && cam.lat.abs() < 85.0 {
        Camera { roll: cam.roll, ..t }
    } else {
        t
    }
}

/// A rotation (columns) as a unit quaternion (w, x, y, z).
fn mat_quat(c: [[f64; 3]; 3]) -> [f64; 4] {
    // m[i][j] = row i, column j = c[j][i]
    let m = |i: usize, j: usize| c[j][i];
    let tr = m(0, 0) + m(1, 1) + m(2, 2);
    let q = if tr > 0.0 {
        let s = (tr + 1.0).sqrt() * 2.0;
        [0.25 * s, (m(2, 1) - m(1, 2)) / s, (m(0, 2) - m(2, 0)) / s, (m(1, 0) - m(0, 1)) / s]
    } else if m(0, 0) > m(1, 1) && m(0, 0) > m(2, 2) {
        let s = (1.0 + m(0, 0) - m(1, 1) - m(2, 2)).sqrt() * 2.0;
        [(m(2, 1) - m(1, 2)) / s, 0.25 * s, (m(0, 1) + m(1, 0)) / s, (m(0, 2) + m(2, 0)) / s]
    } else if m(1, 1) > m(2, 2) {
        let s = (1.0 + m(1, 1) - m(0, 0) - m(2, 2)).sqrt() * 2.0;
        [(m(0, 2) - m(2, 0)) / s, (m(0, 1) + m(1, 0)) / s, 0.25 * s, (m(1, 2) + m(2, 1)) / s]
    } else {
        let s = (1.0 + m(2, 2) - m(0, 0) - m(1, 1)).sqrt() * 2.0;
        [(m(1, 0) - m(0, 1)) / s, (m(0, 2) + m(2, 0)) / s, (m(1, 2) + m(2, 1)) / s, 0.25 * s]
    };
    let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
    [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
}

/// A unit quaternion as a rotation (columns).
fn quat_mat(q: [f64; 4]) -> [[f64; 3]; 3] {
    let [w, x, y, z] = q;
    let r = [
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y - w * z), 2.0 * (x * z + w * y)],
        [2.0 * (x * y + w * z), 1.0 - 2.0 * (x * x + z * z), 2.0 * (y * z - w * x)],
        [2.0 * (x * z - w * y), 2.0 * (y * z + w * x), 1.0 - 2.0 * (x * x + y * y)],
    ];
    // columns
    [[r[0][0], r[1][0], r[2][0]], [r[0][1], r[1][1], r[2][1]], [r[0][2], r[1][2], r[2][2]]]
}

/// The shortest turn from `a` to `b`, a share `t` of the way.
fn slerp(a: [f64; 4], b: [f64; 4]) -> impl Fn(f64) -> [f64; 4] {
    let mut b = b;
    let mut d = a[0] * b[0] + a[1] * b[1] + a[2] * b[2] + a[3] * b[3];
    if d < 0.0 {
        b = [-b[0], -b[1], -b[2], -b[3]];
        d = -d;
    }
    move |t| {
        let (wa, wb) = if d > 0.9995 {
            (1.0 - t, t)
        } else {
            let th = d.clamp(-1.0, 1.0).acos();
            (((1.0 - t) * th).sin() / th.sin(), (t * th).sin() / th.sin())
        };
        let q = [wa * a[0] + wb * b[0], wa * a[1] + wb * b[1], wa * a[2] + wb * b[2], wa * a[3] + wb * b[3]];
        let n = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        [q[0] / n, q[1] / n, q[2] / n, q[3] / n]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const M: Motion = Motion { reduce: false };
    const R: Motion = Motion { reduce: true };

    fn disc(zoom: f64) -> Disc {
        Disc { cx: 400.0, cy: 300.0, r: 276.0 * zoom }
    }
    fn rig(lon: f64, lat: f64, zoom: f64) -> CameraControl {
        let mut c = CameraControl::new(Camera::new(lon, lat, zoom));
        c.apply(&disc(zoom), CameraMove::Stop, 0.0, M);
        c
    }

    #[test]
    fn a_lamp_at_the_rim_is_turned_inward_and_one_on_the_face_is_left() {
        let d = disc(1.0);
        let mut r = rig(0.0, 0.0, 1.0);
        r.apply(&d, CameraMove::Face { at: LonLat { lon: 20.0, lat: 0.0 } }, 0.0, M);
        assert!(!r.moving());
        r.apply(&d, CameraMove::Face { at: LonLat { lon: 85.0, lat: 0.0 } }, 0.0, M);
        r.tick(motion::FLIGHT_S, 0.01, None, true, false, M);
        let keep = motion::FACE_WITHIN.asin().to_degrees();
        assert!((r.camera.lon - (85.0 - keep)).abs() < 1e-6 && r.camera.lat.abs() < 1e-6 && r.camera.zoom == 1.0, "{:?}", r.camera);
        // behind the globe: turned the long way just as far as needed
        let mut b = rig(0.0, 0.0, 1.0);
        b.apply(&d, CameraMove::Face { at: LonLat { lon: 150.0, lat: 0.0 } }, 0.0, R);
        assert!((b.camera.lon - (150.0 - keep)).abs() < 1e-6, "{:?}", b.camera);
    }

    #[test]
    fn a_drag_carries_the_orb_over_the_pole_smoothly_and_home_comes_back_upright() {
        // the finger takes a point on the meridian above the centre and drags it down, carrying the
        // north pole through the middle and on: no lock, every step as small as the finger's
        let d = disc(1.0);
        let mut r = rig(10.0, 60.0, 1.0);
        let (x, mut y) = (d.cx, d.cy - 150.0);
        let grab = d.unproject(&r.camera, x, y).unwrap();
        let mut last = r.camera;
        let mut north = 0.0f64;
        for _ in 0..60 {
            y += 4.0;
            r.apply(&d, CameraMove::Drag { grab: Some(grab), x, y, dx: 0.0, dy: 4.0 }, 0.0, M);
            let (p, _) = d.project(&r.camera, grab);
            assert!((p[0] - x).abs() < 1e-6 && (p[1] - y).abs() < 1e-6, "{p:?} {y}");
            // the centre moves on the sphere by about the finger's step over the radius, never a jump
            let a = LonLat::new(last.lon, last.lat).to_xyz();
            let b = LonLat::new(r.camera.lon, r.camera.lat).to_xyz();
            let step = dot(a, b).clamp(-1.0, 1.0).acos().to_degrees();
            assert!(step < 1.5, "jumped {step}° at {:?}", r.camera);
            north = north.max(r.camera.lat);
            last = r.camera;
        }
        assert!(north > 89.0, "never crossed: {north}");
        assert!(r.camera.lat < 70.0 && r.camera.roll.abs() > 90.0, "{:?}", r.camera);
        // home: the flight turns it the short way and lands upright
        r.fly_to(Camera::new(-30.0, 25.0, 1.0), 0.0, M);
        r.tick(motion::FLIGHT_S / 2.0, 0.01, None, true, false, M);
        r.tick(motion::FLIGHT_S, 0.01, None, true, false, M);
        assert_eq!((r.camera.lon, r.camera.lat, r.camera.roll), (-30.0, 25.0, 0.0));
        // a fling over the pole: the trackball, no lock
        let mut f = rig(0.0, 80.0, 1.0);
        f.apply(&d, CameraMove::Fling { vx: 0.0, vy: 800.0 }, 0.0, M);
        let mut t = 0.0;
        while f.tick(t, 1.0 / 60.0, None, true, false, M) && t < 5.0 {
            t += 1.0 / 60.0;
        }
        assert!(f.camera.roll.abs() > 90.0, "{:?}", f.camera);
    }

    #[test]
    fn a_twist_rolls_the_orb_about_the_fingers_and_keeps_what_they_hold() {
        let d = disc(1.0);
        let mut r = rig(-30.0, 25.0, 1.0);
        // two fingers 80 points either side of a midpoint, turned a quarter clockwise about it
        let (mx, my) = (d.cx + 40.0, d.cy - 30.0);
        let a = d.unproject(&r.camera, mx + 80.0, my).unwrap();
        let mid = d.unproject(&r.camera, mx, my).unwrap();
        let q = std::f64::consts::FRAC_PI_2;
        for _ in 0..30 {
            r.apply(&d, CameraMove::Twist { x: mx, y: my, angle: q / 30.0 }, 0.0, M);
        }
        let (p, _) = d.project(&r.camera, mid);
        assert!((p[0] - mx).abs() < 1e-6 && (p[1] - my).abs() < 1e-6, "{p:?}");
        // the place under the right finger has gone round below the midpoint (y down: clockwise)
        let (p, _) = d.project(&r.camera, a);
        assert!((p[0] - mx).abs() < 8.0 && p[1] > my + 60.0, "{p:?}");
        assert!(r.camera.roll.abs() > 60.0, "{:?}", r.camera);
    }

    #[test]
    fn a_drag_keeps_the_point_under_the_finger() {
        let d = disc(1.0);
        let mut r = rig(-40.0, 22.0, 1.0);
        let (x, y) = (d.cx + 30.0, d.cy - 10.0);
        let under = d.unproject(&r.camera, x, y).unwrap();
        r.apply(&d, CameraMove::Drag { grab: Some(under), x: x + 8.0, y: y + 5.0, dx: 8.0, dy: 5.0 }, 0.0, M);
        let (p, _) = d.project(&r.camera, under);
        assert!((p[0] - (x + 8.0)).abs() < 0.01 && (p[1] - (y + 5.0)).abs() < 0.01, "{:?}", p);
        r.apply(&d, CameraMove::Drag { grab: Some(under), x: x + 150.0, y: y - 120.0, dx: 142.0, dy: -125.0 }, 0.0, M);
        let (p, _) = d.project(&r.camera, under);
        assert!((p[0] - (x + 150.0)).abs() < 0.05 && (p[1] - (y - 120.0)).abs() < 0.05, "{:?}", p);
        // high up, the surface still moves with the finger
        let mut h = rig(0.0, 70.0, 1.0);
        let under = h.camera;
        let under = LonLat { lon: under.lon, lat: under.lat };
        h.apply(&d, CameraMove::Drag { grab: Some(under), x: d.cx + 60.0, y: d.cy, dx: 60.0, dy: 0.0 }, 0.0, M);
        let (p, _) = d.project(&h.camera, under);
        assert!((p[0] - (d.cx + 60.0)).abs() < 0.05, "{:?}", p);
        // off the disc the surface still goes the finger's way: down brings the north in, right
        // the west (R6: a trackball, nothing clamps it)
        let mut o = rig(0.0, 0.0, 1.0);
        o.apply(&d, CameraMove::Drag { grab: None, x: 0.0, y: 0.0, dx: 0.0, dy: 20.0 }, 0.0, M);
        assert!(o.camera.lat > 4.0 && o.camera.lon.abs() < 1e-9, "{:?}", o.camera);
        let mut o = rig(0.0, 0.0, 1.0);
        o.apply(&d, CameraMove::Drag { grab: None, x: 0.0, y: 0.0, dx: 20.0, dy: 0.0 }, 0.0, M);
        assert!(o.camera.lon < -4.0 && o.camera.lat.abs() < 1e-9, "{:?}", o.camera);
        // a drag leaves a ride
        r.riding = Some(3);
        r.apply(&d, CameraMove::Drag { grab: None, x: d.cx, y: d.cy, dx: 1.0, dy: 0.0 }, 0.0, M);
        assert_eq!(r.riding, None);
    }

    #[test]
    fn a_pinch_holds_what_the_fingers_hold() {
        let d = disc(1.0);
        let mut r = rig(-40.0, 22.0, 1.0);
        let (ax, ay) = (d.cx + 120.0, d.cy - 60.0);
        let under = d.unproject(&r.camera, ax, ay).unwrap();
        r.apply(&d, CameraMove::Pinch { grab: Some(under), ax, ay, bx: ax + 20.0, by: ay + 10.0, factor: 1.5 }, 0.0, M);
        assert!((r.camera.zoom - 1.5).abs() < 1e-12);
        let g = disc(1.5);
        let (p, _) = g.project(&r.camera, under);
        assert!((p[0] - (ax + 20.0)).abs() < 0.05 && (p[1] - (ay + 10.0)).abs() < 0.05, "{:?}", p);
        r.apply(&g, CameraMove::Pinch { grab: Some(under), ax, ay, bx: ax, by: ay, factor: 100.0 }, 0.0, M);
        assert_eq!(r.camera.zoom, motion::ZOOM_MAX);
        r.apply(&g, CameraMove::Pinch { grab: Some(under), ax, ay, bx: ax, by: ay, factor: 0.0001 }, 0.0, M);
        assert_eq!(r.camera.zoom, motion::ZOOM_MIN);
    }

    #[test]
    fn a_fling_decays_and_a_touch_stops_it() {
        let d = disc(1.0);
        let mut r = rig(0.0, 0.0, 1.0);
        r.apply(&d, CameraMove::Fling { vx: 600.0, vy: 0.0 }, 0.0, M);
        let mut t = 0.0;
        let mut frames = 0;
        while r.tick(t, 1.0 / 120.0, None, true, false, M) {
            t += 1.0 / 120.0;
            frames += 1;
            if frames > 2000 {
                break;
            }
        }
        assert!(r.camera.lon < -20.0 && r.camera.lon > -90.0, "{}", r.camera.lon);
        assert!(t > 1.0 && t < 3.0, "{}", t);
        r.apply(&d, CameraMove::Fling { vx: 600.0, vy: 0.0 }, t, M);
        r.apply(&d, CameraMove::Stop, t, M);
        assert!(!r.tick(t, 1.0 / 120.0, None, true, true, M));
        let mut q = rig(0.0, 0.0, 1.0);
        q.apply(&d, CameraMove::Fling { vx: 600.0, vy: 0.0 }, 0.0, R);
        assert!(!q.tick(0.01, 0.01, None, true, false, R));
    }

    #[test]
    fn a_flight_arrives_eased_and_a_cut_under_reduce_motion() {
        let d = disc(1.0);
        let mut r = rig(0.0, 0.0, 1.0);
        r.apply(&d, CameraMove::Fly { at: Some(LonLat { lon: 30.0, lat: 10.0 }) }, 0.0, M);
        r.tick(motion::FLIGHT_S / 2.0, 0.01, None, true, false, M);
        // (R6: halfway along the great circle, the orientation slerped)
        assert!((r.camera.lon - 15.0).abs() < 0.5 && (r.camera.lat - 5.0).abs() < 0.5 && (r.camera.zoom - 2.0f64.sqrt()).abs() < 1e-9, "{:?}", r.camera);
        r.tick(motion::FLIGHT_S, 0.01, None, true, false, M);
        assert!((r.camera.lon - 30.0).abs() < 1e-9 && (r.camera.lat - 10.0).abs() < 1e-9 && r.camera.zoom == 2.0 && r.camera.roll == 0.0, "{:?}", r.camera);
        assert!(!r.moving());
        let mut deep = rig(30.0, 10.0, 6.0);
        deep.apply(&disc(6.0), CameraMove::Fly { at: Some(LonLat { lon: 50.0, lat: 0.0 }) }, 0.0, M);
        deep.tick(1.0, 0.01, None, true, false, M);
        assert_eq!((deep.camera.lon, deep.camera.lat, deep.camera.zoom), (30.0, 10.0, 1.0));
        let mut q = rig(0.0, 0.0, 1.0);
        q.apply(&d, CameraMove::Fly { at: Some(LonLat { lon: 30.0, lat: 10.0 }) }, 0.0, R);
        assert_eq!(q.camera.zoom, 2.0);
    }

    #[test]
    fn a_ride_catches_up_without_overshoot() {
        let mut r = rig(0.0, 0.0, 1.0);
        r.riding = Some(7);
        let target = LonLat { lon: 10.0, lat: 5.0 };
        let mut t = 0.0;
        let mut max_lon = 0.0;
        for _ in 0..600 {
            t += 1.0 / 120.0;
            r.tick(t, 1.0 / 120.0, Some(target), false, false, M);
            max_lon = r.camera.lon.max(max_lon);
        }
        assert!((r.camera.lon - 10.0).abs() < 0.01 && (r.camera.lat - 5.0).abs() < 0.01);
        assert!(max_lon <= 10.0 + 1e-6, "overshot to {}", max_lon);
    }

    #[test]
    fn a_ride_keeps_a_fast_float_centred() {
        let mut r = rig(0.0, 0.0, 1.0);
        r.riding = Some(7);
        // a float at 2 degrees a second (a Gulf Stream float at the clock's two days a second)
        let mut t = 0.0;
        let mut lag: f64 = 0.0;
        for i in 0..600 {
            t += 1.0 / 120.0;
            let p = LonLat { lon: 2.0 * t, lat: 0.0 };
            r.tick(t, 1.0 / 120.0, Some(p), false, false, M);
            if i > 360 {
                lag = lag.max((p.lon - r.camera.lon).abs());
            }
        }
        // without aiming ahead the lag would be 2τ·v = 1 degree
        assert!(lag < 0.15, "lag {lag}");
    }

    #[test]
    fn the_idle_turn_waits_and_holds_still_for_a_float() {
        let mut r = rig(0.0, 0.0, 1.0);
        assert!(!r.tick(5.0, 0.01, None, true, false, M));
        assert!(!r.tick(9.0, 0.01, None, false, false, M), "a float on the sea: still");
        assert!(r.tick(9.0, 0.05, None, true, false, M));
        assert!((r.camera.lon - (-0.2)).abs() < 1e-9, "{}", r.camera.lon);
        let mut q = rig(0.0, 0.0, 1.0);
        assert!(!q.tick(9.0, 0.05, None, true, false, R), "reduce motion: no idle turn");
    }
}
