//! The `View` uniform: the first binding of every drawing shader (DECISIONS.md §3), Terra's
//! `OceanUniforms` layout so its shaders port line for line.

use crate::model::{Camera, Clock, Disc};

/// What the shaders need to know about the screen and the camera, on the CPU side.
#[derive(Clone, Copy, Debug)]
pub struct View {
    /// The target's size in pixels.
    pub width: u32,
    pub height: u32,
    /// Pixels a point.
    pub ppp: f64,
    /// The reading scale (1.0 on a phone, 1.2 on the 13-inch iPad).
    pub reading: f64,
    pub disc: Disc,
    pub camera: Camera,
    /// The drawn motion's seconds.
    pub seconds: f64,
    pub clock: Clock,
    /// How far the sea has settled to rest with the date held, 0..1 (eased by the app).
    pub rest: f32,
    /// 1 by night (the system's dark appearance, or a scene's).
    pub night: f32,
}

impl View {
    /// The globe's disc: as large as the room allows, 14 points in from the shorter side, times the zoom.
    pub fn layout(width: u32, height: u32, ppp: f64, camera: Camera) -> Disc {
        let w = width as f64 / ppp;
        let h = height as f64 / ppp;
        let r = (w.min(h) / 2.0 - 14.0).max(40.0) * camera.zoom;
        Disc { cx: w / 2.0, cy: h / 2.0, r }
    }

    pub fn uniform(&self) -> ViewUniform {
        let m = self.camera.rotation();
        let (ma, mb, mix) = self.clock.months();
        ViewUniform {
            frame: [self.width as f32, self.height as f32, self.ppp as f32, self.reading as f32],
            disc: [self.disc.cx as f32, self.disc.cy as f32, self.disc.r as f32, self.camera.zoom as f32],
            rot0: [m[0][0] as f32, m[0][1] as f32, m[0][2] as f32, 0.0],
            rot1: [m[1][0] as f32, m[1][1] as f32, m[1][2] as f32, 0.0],
            rot2: [m[2][0] as f32, m[2][1] as f32, m[2][2] as f32, 0.0],
            time: [self.seconds as f32, ma as f32, mb as f32, mix as f32],
            tone: [1.0, if self.clock.running { 1.0 } else { 0.0 }, self.seconds as f32, self.rest],
            _pad: [self.night, 0.0, 0.0, 0.0],
        }
    }
}

/// The uniform as the shaders read it (`struct View` in every `.wgsl`).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewUniform {
    /// width, height (pixels), pixels a point, the reading scale
    pub frame: [f32; 4],
    /// the globe's centre x, y and radius (points, y down), the zoom
    pub disc: [f32; 4],
    /// world from view, by columns
    pub rot0: [f32; 4],
    pub rot1: [f32; 4],
    pub rot2: [f32; 4],
    /// seconds; this month (0 = January), the next, how far between
    pub time: [f32; 4],
    /// x how much shows; y 1 while the date runs, 0 held; z the drawn motion's seconds; w how far
    /// the sea has settled to rest (eased)
    pub tone: [f32; 4],
    /// x 1 by night
    pub _pad: [f32; 4],
}

pub const VIEW_WGSL: &str = r#"
struct View {
    frame: vec4f,   // width, height (pixels), pixels a point, the reading scale
    disc: vec4f,    // the globe's centre x, y and radius (points, y down), the zoom
    rot0: vec4f,    // world from view, by columns
    rot1: vec4f,
    rot2: vec4f,
    time: vec4f,    // seconds; this month (0 = January), the next, how far between
    tone: vec4f,    // x how much shows; y 1 while the date runs; z the drawn motion's seconds; w the sea's rest
    pad: vec4f,     // x 1 by night
};
"#;
