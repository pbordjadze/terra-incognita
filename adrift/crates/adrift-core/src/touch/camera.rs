//! The camera under a finger: orbit, fling, pinch anchored to the fingers, the double tap's flight,
//! riding a float, and the slow turn when left alone. `CameraRig` owns the camera's truth and
//! reports it as `Intent::Camera` whenever it moves; the state mirrors it and the look draws it.
//! The frame is Terra's `GlobeView`: the centre of the screen is the point (lon, lat) of the globe,
//! `zoom` how far in; one point of finger moves one point of the globe's surface at the disc's edge.

use super::motion::{self, Motion};
use crate::model::{Camera, LonLat};

/// Where the globe is drawn this frame, in points: the screen, its reading scale, and the disc.
/// Engineering builds it from the `View` uniform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub width: f64,
    pub height: f64,
    /// Terra's reading scale: 1 on a phone, 1.2 on the 13-inch iPad; slop and reach grow with it.
    pub reading: f64,
    /// The disc's centre and radius, points, at this zoom.
    pub cx: f64,
    pub cy: f64,
    pub radius: f64,
}

impl Frame {
    /// A frame whose disc fills the shorter side at zoom 1, as the checks draw it.
    pub fn simple(width: f64, height: f64, zoom: f64) -> Frame {
        Frame { width, height, reading: if width.min(height) >= 700.0 { 1.2 } else { 1.0 }, cx: width / 2.0, cy: height / 2.0, radius: width.min(height) * 0.46 * zoom }
    }
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
pub struct CameraRig {
    camera: Camera,
    motion: Motion,
    /// A fling, points a second.
    spin: (f64, f64),
    flight: Option<Flight>,
    /// Riding: the float, and the spring's velocity (degrees a second) toward it.
    ride: Option<(u32, (f64, f64))>,
    /// When a finger last touched, for the idle turn.
    last_touch: f64,
    /// The deepest zoom the look allows (`ZOOM_MAX` unless asked for more).
    zoom_limit: f64,
    changed: bool,
}

impl CameraRig {
    pub fn new(camera: Camera, motion: Motion, now: f64) -> CameraRig {
        CameraRig { camera, motion, spin: (0.0, 0.0), flight: None, ride: None, last_touch: now, zoom_limit: motion::ZOOM_MAX, changed: true }
    }

    pub fn camera(&self) -> Camera {
        self.camera
    }
    pub fn riding(&self) -> Option<u32> {
        self.ride.map(|r| r.0)
    }
    pub fn set_zoom_limit(&mut self, z: f64) {
        self.zoom_limit = z.clamp(motion::ZOOM_MAX, 32.0);
    }
    /// Whether anything of the camera's own is moving (a fling or a flight): the shell draws at
    /// the full rate while it is.
    pub fn moving(&self) -> bool {
        self.flight.is_some() || self.spin.0.abs() + self.spin.1.abs() > motion::FLING_REST || self.ride.is_some()
    }

    // --- the sphere ---

    /// The rotation taking local coordinates (x right, y up, z toward the viewer) to the globe's
    /// (`lat = asin(y)`, `lon = atan2(x, z)`), as `Globe.metal` has it.
    fn rotation(&self) -> [[f64; 3]; 3] {
        let (sl, cl) = self.camera.lon.to_radians().sin_cos();
        let (sb, cb) = self.camera.lat.to_radians().sin_cos();
        // world = R_y(lon) · R_x(-lat) · local; columns are the images of the local axes
        [[cl, 0.0, -sl], [-sl * sb, cb, -cl * sb], [sl * cb, sb, cl * cb]]
    }

    fn to_xyz(p: LonLat) -> [f64; 3] {
        let (sl, cl) = p.lon.to_radians().sin_cos();
        let (sb, cb) = p.lat.to_radians().sin_cos();
        [sl * cb, sb, cl * cb]
    }

    /// The point of the globe under a point of the screen, if it is on the disc.
    pub fn unproject(&self, x: f64, y: f64, f: &Frame) -> Option<LonLat> {
        if f.radius <= 0.0 {
            return None;
        }
        let lx = (x - f.cx) / f.radius;
        let ly = (f.cy - y) / f.radius;
        let r2 = lx * lx + ly * ly;
        if r2 >= 1.0 {
            return None;
        }
        let lz = (1.0 - r2).sqrt();
        let m = self.rotation();
        let w = [
            m[0][0] * lx + m[1][0] * ly + m[2][0] * lz,
            m[0][1] * lx + m[1][1] * ly + m[2][1] * lz,
            m[0][2] * lx + m[1][2] * ly + m[2][2] * lz,
        ];
        Some(LonLat { lon: w[0].atan2(w[2]).to_degrees(), lat: w[1].clamp(-1.0, 1.0).asin().to_degrees() })
    }

    /// Where a point of the globe falls on the screen, and how much it faces the viewer (negative:
    /// behind the globe).
    pub fn project(&self, p: LonLat, f: &Frame) -> (f64, f64, f64) {
        let w = Self::to_xyz(p);
        let m = self.rotation();
        // local = Rᵀ · world: rows of the transpose are the columns of R
        let lx = m[0][0] * w[0] + m[0][1] * w[1] + m[0][2] * w[2];
        let ly = m[1][0] * w[0] + m[1][1] * w[1] + m[1][2] * w[2];
        let lz = m[2][0] * w[0] + m[2][1] * w[1] + m[2][2] * w[2];
        (f.cx + lx * f.radius, f.cy - ly * f.radius, lz)
    }

    // --- the hands ---

    /// A finger landed: whatever the camera was doing of its own stops at once.
    pub fn touched(&mut self, now: f64) {
        self.last_touch = now;
        self.flight = None;
        self.spin = (0.0, 0.0);
    }

    /// Turns the globe as a finger moving (dx, dy) points would drag the point at the screen's
    /// centre: the longitude's turn is scaled by the centre's latitude, so the surface there moves
    /// as far as the finger (Terra's turn without the scaling slows toward the poles).
    pub fn turn(&mut self, dx: f64, dy: f64, f: &Frame) {
        if f.radius <= 0.0 {
            return;
        }
        let c = self.camera.lat.to_radians().cos().max(0.17);
        self.camera.lon = motion::wrap_lon(self.camera.lon - (dx / (f.radius * c)).to_degrees());
        self.camera.lat = (self.camera.lat + (dy / f.radius).to_degrees()).clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT);
        self.changed = true;
    }

    /// Turns the globe so that `grab`, the point of the globe the finger took, lies under the
    /// finger at (x, y): Newton's method on the camera's (lon, lat), a few steps, each measuring
    /// how the point moves on screen for a small turn. Off the disc, or where the pole stops it,
    /// the finger's own move turns the globe by `turn` alone.
    fn turn_to(&mut self, grab: LonLat, x: f64, y: f64, dx: f64, dy: f64, f: &Frame) {
        if self.unproject(x, y, f).is_none() {
            self.turn(dx, dy, f);
            return;
        }
        let before = self.camera;
        for _ in 0..6 {
            let (px, py, facing) = self.project(grab, f);
            if facing <= 0.0 {
                self.camera = before;
                self.turn(dx, dy, f);
                return;
            }
            let (rx, ry) = (x - px, y - py);
            if rx.abs() < 1e-4 && ry.abs() < 1e-4 {
                break;
            }
            // the Jacobian of the point's screen position in the camera's lon and lat
            let h = 0.01;
            let mut probe = *self;
            probe.camera.lon = motion::wrap_lon(self.camera.lon + h);
            let (ax, ay, _) = probe.project(grab, f);
            probe.camera.lon = self.camera.lon;
            probe.camera.lat = self.camera.lat + h;
            let (bx, by, _) = probe.project(grab, f);
            let (j11, j21) = ((ax - px) / h, (ay - py) / h);
            let (j12, j22) = ((bx - px) / h, (by - py) / h);
            let det = j11 * j22 - j12 * j21;
            if det.abs() < 1e-9 {
                self.camera = before;
                self.turn(dx, dy, f);
                return;
            }
            let dlon = (rx * j22 - j12 * ry) / det;
            let dlat = (j11 * ry - j21 * rx) / det;
            self.camera.lon = motion::wrap_lon(self.camera.lon + dlon);
            self.camera.lat = (self.camera.lat + dlat).clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT);
        }
        self.changed = true;
    }

    /// A one-finger drag: the globe turns so the point taken stays under the finger, and any ride is
    /// left. `grab` is the point under the finger when the drag began (None off the disc).
    pub fn drag(&mut self, grab: Option<LonLat>, x: f64, y: f64, dx: f64, dy: f64, f: &Frame) {
        self.ride = None;
        match grab {
            Some(g) => self.turn_to(g, x, y, dx, dy, f),
            None => self.turn(dx, dy, f),
        }
    }

    /// The finger let go moving: a fling, points a second (none under Reduce Motion).
    pub fn fling(&mut self, vx: f64, vy: f64, now: f64) {
        self.last_touch = now;
        self.spin = if self.motion.reduce { (0.0, 0.0) } else { (vx, vy) };
    }

    /// Two fingers: zoom by `scale` about the anchor `(ax, ay)` and move the point of the globe
    /// that was under `(ax, ay)` to `(bx, by)`, so what the fingers hold stays under them. Two
    /// passes, as Terra's: the turn is a little curved.
    pub fn pinch(&mut self, scale: f64, ax: f64, ay: f64, bx: f64, by: f64, f: &Frame) {
        self.ride = None;
        let under = self.unproject(ax, ay, f);
        let z0 = self.camera.zoom;
        self.camera.zoom = (z0 * scale).clamp(motion::ZOOM_MIN, self.zoom_limit);
        // the disc grows with the zoom
        let g = Frame { radius: f.radius * self.camera.zoom / z0, ..*f };
        match under {
            Some(u) => self.turn_to(u, bx, by, bx - ax, by - ay, &g),
            None => self.turn(bx - ax, by - ay, &g),
        }
        self.changed = true;
    }

    /// A double tap: fly in about the point (toward it below zoom 4, about the centre above), or
    /// out to the whole globe from deeper than half the deepest. A cut under Reduce Motion.
    pub fn double_tap(&mut self, at: Option<LonLat>, now: f64) {
        let z = self.camera.zoom;
        let target_zoom = if z < self.zoom_limit / 2.0 { (z * 2.0).min(self.zoom_limit) } else { 1.0 };
        let (lon, lat) = match at {
            Some(p) if z < 4.0 && target_zoom > z => (p.lon, p.lat),
            _ => (self.camera.lon, self.camera.lat),
        };
        self.fly_to(Camera { lon, lat: lat.clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT), zoom: target_zoom }, now);
    }

    pub fn fly_to(&mut self, to: Camera, now: f64) {
        self.spin = (0.0, 0.0);
        let duration = if self.motion.reduce { 0.0 } else { motion::FLIGHT_S };
        self.flight = Some(Flight { from: self.camera, to, start: now, duration });
        if duration == 0.0 {
            self.camera = to;
            self.flight = None;
            self.changed = true;
        }
    }

    /// Ride a float: the camera's centre springs toward it every frame; zoom is untouched.
    pub fn ride(&mut self, id: u32) {
        self.ride = Some((id, (0.0, 0.0)));
    }
    pub fn leave(&mut self) -> bool {
        self.ride.take().is_some()
    }

    // --- every frame ---

    /// Advances the camera's own motion: a flight, a fling, the ride (given where the ridden float
    /// is now, or None if it is gone), or the idle turn (only when `sea_is_empty` and nothing is
    /// ridden). Returns the camera if it moved since the last report.
    pub fn frame(&mut self, now: f64, dt: f64, f: &Frame, ridden_at: Option<LonLat>, sea_is_empty: bool, touching: bool) -> Option<Camera> {
        let dt = dt.clamp(0.0, 0.05);
        if let Some(fl) = self.flight {
            let u = if fl.duration > 0.0 { ((now - fl.start) / fl.duration).min(1.0) } else { 1.0 };
            let e = motion::ease_in_out(u);
            self.camera.lon = motion::wrap_lon(fl.from.lon + motion::lon_delta(fl.from.lon, fl.to.lon) * e);
            self.camera.lat = fl.from.lat + (fl.to.lat - fl.from.lat) * e;
            self.camera.zoom = (fl.from.zoom.ln() + (fl.to.zoom.ln() - fl.from.zoom.ln()) * e).exp();
            if u >= 1.0 {
                self.flight = None;
            }
            self.changed = true;
        } else if !touching && self.spin.0.abs() + self.spin.1.abs() > motion::FLING_REST {
            self.turn(self.spin.0 * dt, self.spin.1 * dt, f);
            self.spin = (motion::fling_decay(self.spin.0, dt), motion::fling_decay(self.spin.1, dt));
        } else if let Some((id, v)) = self.ride {
            match ridden_at {
                Some(p) => {
                    if self.motion.reduce {
                        self.camera.lon = p.lon;
                        self.camera.lat = p.lat.clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT);
                    } else {
                        let target_lon = self.camera.lon + motion::lon_delta(self.camera.lon, p.lon);
                        let (lon, vl) = motion::spring(self.camera.lon, v.0, target_lon, motion::RIDE_TAU, dt);
                        let (lat, vb) = motion::spring(self.camera.lat, v.1, p.lat, motion::RIDE_TAU, dt);
                        self.camera.lon = motion::wrap_lon(lon);
                        self.camera.lat = lat.clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT);
                        self.ride = Some((id, (vl, vb)));
                    }
                    self.changed = true;
                }
                None => self.ride = None,
            }
        } else if !touching && sea_is_empty && !self.motion.reduce && now - self.last_touch > motion::IDLE_AFTER {
            // left alone, it turns as the Earth does, west to east
            self.camera.lon = motion::wrap_lon(self.camera.lon - motion::IDLE_RATE * dt);
            self.changed = true;
        }
        if self.changed {
            self.changed = false;
            Some(self.camera)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rig(lon: f64, lat: f64, zoom: f64) -> CameraRig {
        CameraRig::new(Camera { lon, lat, zoom }, Motion::default(), 0.0)
    }

    #[test]
    fn the_centre_is_the_camera_and_projection_round_trips() {
        let f = Frame::simple(800.0, 600.0, 1.0);
        for &(lon, lat) in &[(0.0, 0.0), (-40.0, 22.0), (170.0, -60.0), (-179.0, 79.0)] {
            let r = rig(lon, lat, 1.0);
            let c = r.unproject(f.cx, f.cy, &f).unwrap();
            assert!((c.lon - lon).abs() < 1e-9 && (c.lat - lat).abs() < 1e-9, "{:?}", c);
            for &(x, y) in &[(f.cx + 100.0, f.cy - 50.0), (f.cx - 200.0, f.cy + 120.0), (f.cx + 1.0, f.cy + 250.0)] {
                let p = r.unproject(x, y, &f).unwrap();
                let (px, py, facing) = r.project(p, &f);
                assert!((px - x).abs() < 1e-6 && (py - y).abs() < 1e-6, "{} {} -> {:?} -> {} {}", x, y, p, px, py);
                assert!(facing > 0.0);
            }
        }
        // north is up: a point above the centre has a higher latitude
        let r = rig(10.0, 20.0, 1.0);
        let up = r.unproject(f.cx, f.cy - 50.0, &f).unwrap();
        assert!(up.lat > 20.0 && (up.lon - 10.0).abs() < 1e-9);
        // east is right
        let right = r.unproject(f.cx + 50.0, f.cy, &f).unwrap();
        assert!(right.lon > 10.0);
        assert!(r.unproject(f.cx + f.radius + 1.0, f.cy, &f).is_none());
    }

    #[test]
    fn a_drag_keeps_the_point_under_the_finger() {
        let f = Frame::simple(800.0, 600.0, 1.0);
        let mut r = rig(-40.0, 22.0, 1.0);
        let (x, y) = (f.cx + 30.0, f.cy - 10.0);
        let under = r.unproject(x, y, &f).unwrap();
        // a drag, even a long one: the point taken stays under the finger
        r.drag(Some(under), x + 8.0, y + 5.0, 8.0, 5.0, &f);
        let (nx, ny, _) = r.project(under, &f);
        assert!((nx - (x + 8.0)).abs() < 0.01 && (ny - (y + 5.0)).abs() < 0.01, "{} {}", nx, ny);
        r.drag(Some(under), x + 150.0, y - 120.0, 142.0, -125.0, &f);
        let (nx, ny, _) = r.project(under, &f);
        assert!((nx - (x + 150.0)).abs() < 0.05 && (ny - (y - 120.0)).abs() < 0.05, "{} {}", nx, ny);
        // high up, the surface still moves with the finger
        let mut h = rig(0.0, 70.0, 1.0);
        let under = h.unproject(f.cx, f.cy, &f).unwrap();
        h.drag(Some(under), f.cx + 60.0, f.cy, 60.0, 0.0, &f);
        let (nx, _, _) = h.project(under, &f);
        assert!((nx - (f.cx + 60.0)).abs() < 0.05, "{}", nx);
        // the pole never flips
        for _ in 0..200 {
            r.drag(None, f.cx, f.cy, 0.0, 100.0, &f);
        }
        assert!(r.camera().lat <= motion::LAT_LIMIT);
    }

    #[test]
    fn a_pinch_holds_what_the_fingers_hold() {
        let f = Frame::simple(800.0, 600.0, 1.0);
        let mut r = rig(-40.0, 22.0, 1.0);
        let (ax, ay) = (f.cx + 120.0, f.cy - 60.0);
        let under = r.unproject(ax, ay, &f).unwrap();
        r.pinch(1.5, ax, ay, ax + 20.0, ay + 10.0, &f);
        assert!((r.camera().zoom - 1.5).abs() < 1e-12);
        let g = Frame { radius: f.radius * 1.5, ..f };
        let (nx, ny, _) = r.project(under, &g);
        assert!((nx - (ax + 20.0)).abs() < 0.05 && (ny - (ay + 10.0)).abs() < 0.05, "{} {}", nx, ny);
        // clamped
        r.pinch(100.0, ax, ay, ax, ay, &g);
        assert_eq!(r.camera().zoom, motion::ZOOM_MAX);
        r.pinch(0.0001, ax, ay, ax, ay, &g);
        assert_eq!(r.camera().zoom, motion::ZOOM_MIN);
    }

    #[test]
    fn a_fling_decays_and_a_touch_stops_it() {
        let f = Frame::simple(800.0, 600.0, 1.0);
        let mut r = rig(0.0, 0.0, 1.0);
        r.fling(600.0, 0.0, 0.0);
        let mut t = 0.0;
        let mut frames = 0;
        while r.frame(t, 1.0 / 120.0, &f, None, true, false).is_some() {
            t += 1.0 / 120.0;
            frames += 1;
            if frames > 2000 {
                break;
            }
        }
        // stopped by itself, west of where it was, in a couple of seconds
        assert!(r.camera().lon < -20.0 && r.camera().lon > -90.0, "{}", r.camera().lon);
        assert!(t > 1.0 && t < 3.0, "{}", t);
        r.fling(600.0, 0.0, t);
        r.touched(t);
        assert!(r.frame(t, 1.0 / 120.0, &f, None, true, true).is_none());
        // reduce motion: no fling at all
        let mut q = CameraRig::new(Camera { lon: 0.0, lat: 0.0, zoom: 1.0 }, Motion { reduce: true }, 0.0);
        q.frame(0.0, 0.01, &f, None, true, false);
        q.fling(600.0, 0.0, 0.0);
        assert!(q.frame(0.01, 0.01, &f, None, true, false).is_none());
    }

    #[test]
    fn a_flight_arrives_eased_and_a_cut_under_reduce_motion() {
        let f = Frame::simple(800.0, 600.0, 1.0);
        let mut r = rig(0.0, 0.0, 1.0);
        r.double_tap(Some(LonLat { lon: 30.0, lat: 10.0 }), 0.0);
        let mid = r.frame(motion::FLIGHT_S / 2.0, 0.01, &f, None, true, false).unwrap();
        assert!((mid.lon - 15.0).abs() < 1e-9 && (mid.zoom - 2.0f64.sqrt()).abs() < 1e-9);
        let end = r.frame(motion::FLIGHT_S, 0.01, &f, None, true, false).unwrap();
        assert_eq!((end.lon, end.lat, end.zoom), (30.0, 10.0, 2.0));
        assert!(!r.moving());
        // from deep in, out to the whole globe about the centre
        let mut d = rig(30.0, 10.0, 6.0);
        d.double_tap(Some(LonLat { lon: 50.0, lat: 0.0 }), 0.0);
        let end = d.frame(1.0, 0.01, &f, None, true, false).unwrap();
        assert_eq!((end.lon, end.lat, end.zoom), (30.0, 10.0, 1.0));
        let mut q = CameraRig::new(Camera { lon: 0.0, lat: 0.0, zoom: 1.0 }, Motion { reduce: true }, 0.0);
        q.double_tap(Some(LonLat { lon: 30.0, lat: 10.0 }), 0.0);
        assert_eq!(q.camera().zoom, 2.0);
    }

    #[test]
    fn a_ride_catches_up_without_overshoot_and_a_drag_leaves_it() {
        let f = Frame::simple(800.0, 600.0, 1.0);
        let mut r = rig(0.0, 0.0, 1.0);
        r.ride(7);
        let target = LonLat { lon: 10.0, lat: 5.0 };
        let mut t = 0.0;
        let mut max_lon = 0.0;
        for _ in 0..600 {
            t += 1.0 / 120.0;
            let c = r.frame(t, 1.0 / 120.0, &f, Some(target), false, false).unwrap();
            max_lon = c.lon.max(max_lon);
        }
        assert!((r.camera().lon - 10.0).abs() < 0.01 && (r.camera().lat - 5.0).abs() < 0.01);
        assert!(max_lon <= 10.0 + 1e-6, "overshot to {}", max_lon);
        assert_eq!(r.riding(), Some(7));
        r.drag(None, f.cx, f.cy, 1.0, 0.0, &f);
        assert_eq!(r.riding(), None);
        // a ridden float that is gone ends the ride
        r.ride(7);
        r.frame(t, 0.01, &f, None, false, false);
        assert_eq!(r.riding(), None);
    }

    #[test]
    fn the_idle_turn_waits_and_holds_still_for_a_float() {
        let f = Frame::simple(800.0, 600.0, 1.0);
        let mut r = rig(0.0, 0.0, 1.0);
        r.frame(0.0, 0.01, &f, None, true, false);
        assert!(r.frame(5.0, 0.01, &f, None, true, false).is_none());
        assert!(r.frame(9.0, 0.01, &f, None, false, false).is_none(), "a float on the sea: still");
        let c = r.frame(9.0, 0.05, &f, None, true, false).unwrap();
        assert!((c.lon - (-0.2)).abs() < 1e-9, "{}", c.lon);
    }
}
