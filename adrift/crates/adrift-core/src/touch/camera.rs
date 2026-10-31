//! The camera under the finger: orbit by drag, inertia, pinch to zoom, riding a float.

use crate::model::{Camera, Disc, LonLat};

pub struct CameraControl {
    pub camera: Camera,
    /// Degrees a second, after a fling.
    velocity: (f64, f64),
    /// The float the camera drifts with, if any.
    pub riding: Option<u32>,
    pub zoom_range: (f64, f64),
}

impl CameraControl {
    pub fn new(camera: Camera) -> CameraControl {
        CameraControl { camera, velocity: (0.0, 0.0), riding: None, zoom_range: (0.8, 12.0) }
    }

    /// Turns the globe as a finger moving (dx, dy) points would drag its centre.
    pub fn turn(&mut self, disc: &Disc, dx: f64, dy: f64) {
        if disc.r <= 0.0 {
            return;
        }
        self.camera.lon -= dx / disc.r * 180.0 / std::f64::consts::PI;
        self.camera.lat = (self.camera.lat + dy / disc.r * 180.0 / std::f64::consts::PI).clamp(-85.0, 85.0);
        self.camera.lon = (self.camera.lon + 180.0).rem_euclid(360.0) - 180.0;
        self.riding = None;
    }

    /// A fling: the finger's last speed, points a second.
    pub fn fling(&mut self, disc: &Disc, vx: f64, vy: f64) {
        if disc.r <= 0.0 {
            return;
        }
        let k = 180.0 / std::f64::consts::PI / disc.r;
        self.velocity = (-vx * k, vy * k);
    }

    pub fn zoom_by(&mut self, factor: f64) {
        self.camera.zoom = (self.camera.zoom * factor).clamp(self.zoom_range.0, self.zoom_range.1);
    }

    pub fn stop(&mut self) {
        self.velocity = (0.0, 0.0);
    }

    /// A frame: inertia runs down; riding, the camera eases toward the float.
    pub fn tick(&mut self, dt: f64, ride_at: Option<LonLat>) {
        if self.velocity != (0.0, 0.0) {
            self.camera.lon += self.velocity.0 * dt;
            self.camera.lat = (self.camera.lat + self.velocity.1 * dt).clamp(-85.0, 85.0);
            let decay = (-dt / 0.6).exp();
            self.velocity = (self.velocity.0 * decay, self.velocity.1 * decay);
            if self.velocity.0.abs() + self.velocity.1.abs() < 0.05 {
                self.velocity = (0.0, 0.0);
            }
            self.camera.lon = (self.camera.lon + 180.0).rem_euclid(360.0) - 180.0;
        }
        if let Some(at) = ride_at {
            let k = 1.0 - (-dt / 0.8).exp();
            let mut dlon = at.lon - self.camera.lon;
            if dlon > 180.0 {
                dlon -= 360.0;
            }
            if dlon < -180.0 {
                dlon += 360.0;
            }
            self.camera.lon += dlon * k;
            self.camera.lat += (at.lat - self.camera.lat) * k;
            self.camera.lon = (self.camera.lon + 180.0).rem_euclid(360.0) - 180.0;
        }
    }
}
