//! A stand-in for `adrift-core`'s `gpu::View`: the fields the touch module's probe reads.
use crate::model::{Camera, Clock, Disc};

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub width: u32,
    pub height: u32,
    pub ppp: f64,
    pub reading: f64,
    pub disc: Disc,
    pub camera: Camera,
    pub seconds: f64,
    pub clock: Clock,
}

impl View {
    /// `adrift-core`'s layout, the same numbers: the disc fills the shorter side less 14 points.
    pub fn layout(width: u32, height: u32, ppp: f64, camera: Camera) -> Disc {
        let w = width as f64 / ppp;
        let h = height as f64 / ppp;
        let r = (w.min(h) / 2.0 - 14.0).max(40.0) * camera.zoom;
        Disc { cx: w / 2.0, cy: h / 2.0, r }
    }
}
