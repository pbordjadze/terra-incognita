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
    ride_v: (f64, f64),
    /// When a finger last touched (the shell's seconds), for the idle turn.
    last_touch: f64,
    /// The disc the last move was measured in (the fling turns by it between frames).
    disc: Disc,
}

impl CameraControl {
    pub fn new(camera: Camera) -> CameraControl {
        CameraControl {
            camera,
            riding: None,
            zoom_range: (motion::ZOOM_MIN, motion::ZOOM_MAX),
            spin: (0.0, 0.0),
            flight: None,
            ride_v: (0.0, 0.0),
            last_touch: f64::NEG_INFINITY,
            disc: Disc { cx: 0.0, cy: 0.0, r: 0.0 },
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
        }
    }

    // --- the sphere ---

    /// Turns the globe as a finger moving (dx, dy) points would drag the point at the screen's
    /// centre; the longitude's turn is scaled by the centre's latitude so the surface there moves
    /// as far as the finger (Terra's turn without that slows toward the poles).
    pub fn turn(&mut self, disc: &Disc, dx: f64, dy: f64) {
        if disc.r <= 0.0 {
            return;
        }
        let c = self.camera.lat.to_radians().cos().max(0.17);
        self.camera.lon = motion::wrap_lon(self.camera.lon - (dx / (disc.r * c)).to_degrees());
        self.camera.lat = (self.camera.lat + (dy / disc.r).to_degrees()).clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT);
        self.riding = None;
    }

    /// Turns the globe so that `grab`, the point of the globe the finger took, lies under the
    /// finger at (x, y): Newton's method on the camera's (lon, lat), a few steps, each measuring
    /// how the point moves on screen for a small turn. Off the disc, or where the pole stops it,
    /// the finger's own move turns the globe by `turn` alone.
    fn turn_to(&mut self, disc: &Disc, grab: LonLat, x: f64, y: f64, dx: f64, dy: f64) {
        if disc.unproject(&self.camera, x, y).is_none() {
            self.turn(disc, dx, dy);
            return;
        }
        let before = self.camera;
        for _ in 0..6 {
            let (p, facing) = disc.project(&self.camera, grab);
            if facing <= 0.0 {
                self.camera = before;
                self.turn(disc, dx, dy);
                return;
            }
            let (rx, ry) = (x - p[0], y - p[1]);
            if rx.abs() < 1e-4 && ry.abs() < 1e-4 {
                break;
            }
            // the Jacobian of the point's screen position in the camera's lon and lat
            let h = 0.01;
            let mut cam = self.camera;
            cam.lon = motion::wrap_lon(self.camera.lon + h);
            let (a, _) = disc.project(&cam, grab);
            cam.lon = self.camera.lon;
            cam.lat = self.camera.lat + h;
            let (b, _) = disc.project(&cam, grab);
            let (j11, j21) = ((a[0] - p[0]) / h, (a[1] - p[1]) / h);
            let (j12, j22) = ((b[0] - p[0]) / h, (b[1] - p[1]) / h);
            let det = j11 * j22 - j12 * j21;
            if det.abs() < 1e-9 {
                self.camera = before;
                self.turn(disc, dx, dy);
                return;
            }
            let dlon = (rx * j22 - j12 * ry) / det;
            let dlat = (j11 * ry - j21 * rx) / det;
            self.camera.lon = motion::wrap_lon(self.camera.lon + dlon);
            self.camera.lat = (self.camera.lat + dlat).clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT);
        }
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
        let (lon, lat) = match at {
            Some(p) if z < 4.0 && target_zoom > z => (p.lon, p.lat),
            _ => (self.camera.lon, self.camera.lat),
        };
        self.fly_to(Camera { lon, lat: lat.clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT), zoom: target_zoom }, now, motion);
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
        if let Some(fl) = self.flight {
            let u = if fl.duration > 0.0 { ((now - fl.start) / fl.duration).min(1.0) } else { 1.0 };
            let e = motion::ease_in_out(u);
            self.camera.lon = motion::wrap_lon(fl.from.lon + motion::lon_delta(fl.from.lon, fl.to.lon) * e);
            self.camera.lat = fl.from.lat + (fl.to.lat - fl.from.lat) * e;
            self.camera.zoom = (fl.from.zoom.ln() + (fl.to.zoom.ln() - fl.from.zoom.ln()) * e).exp();
            if u >= 1.0 {
                self.flight = None;
            }
        } else if !touching && self.spin.0.abs() + self.spin.1.abs() > motion::FLING_REST {
            let disc = self.disc;
            self.turn(&disc, self.spin.0 * dt, self.spin.1 * dt);
            self.spin = (motion::fling_decay(self.spin.0, dt), motion::fling_decay(self.spin.1, dt));
        } else if let Some(p) = ride_at.filter(|_| self.riding.is_some()) {
            if motion.reduce {
                self.camera.lon = p.lon;
                self.camera.lat = p.lat.clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT);
            } else {
                let target_lon = self.camera.lon + motion::lon_delta(self.camera.lon, p.lon);
                let (lon, vl) = motion::spring(self.camera.lon, self.ride_v.0, target_lon, motion::RIDE_TAU, dt);
                let (lat, vb) = motion::spring(self.camera.lat, self.ride_v.1, p.lat, motion::RIDE_TAU, dt);
                self.camera.lon = motion::wrap_lon(lon);
                self.camera.lat = lat.clamp(-motion::LAT_LIMIT, motion::LAT_LIMIT);
                self.ride_v = (vl, vb);
            }
        } else if !touching && sea_is_empty && self.riding.is_none() && !motion.reduce && now - self.last_touch > motion::IDLE_AFTER {
            // left alone, it turns as the Earth does, west to east
            self.camera.lon = motion::wrap_lon(self.camera.lon - motion::IDLE_RATE * dt);
        }
        if self.riding.is_none() {
            self.ride_v = (0.0, 0.0);
        }
        self.camera != before
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
        let mut c = CameraControl::new(Camera { lon, lat, zoom });
        c.apply(&disc(zoom), CameraMove::Stop, 0.0, M);
        c
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
        // the pole never flips
        for _ in 0..200 {
            r.apply(&d, CameraMove::Drag { grab: None, x: d.cx, y: d.cy, dx: 0.0, dy: 100.0 }, 0.0, M);
        }
        assert!(r.camera.lat <= motion::LAT_LIMIT);
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
        assert!((r.camera.lon - 15.0).abs() < 1e-9 && (r.camera.zoom - 2.0f64.sqrt()).abs() < 1e-9);
        r.tick(motion::FLIGHT_S, 0.01, None, true, false, M);
        assert_eq!((r.camera.lon, r.camera.lat, r.camera.zoom), (30.0, 10.0, 2.0));
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
