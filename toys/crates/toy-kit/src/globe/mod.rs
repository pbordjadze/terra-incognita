//! The globe a toy may stand on: the real ground of the month (or the night and its lights) drawn in
//! one pass, a camera a finger turns and pinches, and the `View` uniform every drawing shader takes as
//! its group 0. A toy that wants no globe never makes one.
//!
//! Drawing over it: write a pass whose WGSL starts with `toy_kit::globe::wgsl(own)` (the `View`
//! struct and the shared helpers of `common.wgsl`), bind `globe.view_bind()` at group 0, and use
//! `globe.view()` on the CPU to place things (`disc.project(&camera, lonlat)`).

pub mod ground;

use crate::assets::Assets;
use crate::gpu::{Gpu, Textures, View, ViewUniform, VIEW_WGSL};
use crate::model::{Camera, Clock, Disc, LonLat, Phase, PointerEvent};
use std::collections::HashMap;

pub const COMMON_WGSL: &str = include_str!("common.wgsl");

/// A pass's WGSL with the `View` struct and the shared helpers before it.
pub fn wgsl(own: &str) -> String {
    format!("{VIEW_WGSL}\n{COMMON_WGSL}\n{own}")
}

/// The view's bind group layout (group 0 of every pass): one uniform buffer, `ViewUniform`.
pub fn view_layout(gpu: &Gpu) -> wgpu::BindGroupLayout {
    gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("view"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }],
    })
}

/// The ground, its textures, the camera and the hand that turns it.
pub struct Globe {
    pub textures: Textures,
    pub ground: ground::GroundPass,
    pub view_layout: wgpu::BindGroupLayout,
    view_buf: wgpu::Buffer,
    view_bind: wgpu::BindGroup,
    pub camera: Camera,
    pub clock: Clock,
    /// 1 by night (the lights), 0 by day; a toy may ease it.
    pub night: f32,
    /// The drawn motion's seconds (the shimmer of the lights by night).
    pub seconds: f64,
    /// The hand: on by default; a toy that reads fingers itself turns it off.
    pub hand: Hand,
    width: u32,
    height: u32,
    ppp: f64,
}

impl Globe {
    /// The globe over the month `month` (0 = January).
    pub fn new(gpu: &Gpu, assets: &dyn Assets, width: u32, height: u32, ppp: f64, month: u32) -> anyhow::Result<Globe> {
        let textures = Textures::load(gpu, assets, month)?;
        let view_layout = view_layout(gpu);
        let ground = ground::GroundPass::new(gpu, &view_layout, &textures);
        let view_buf = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            size: std::mem::size_of::<ViewUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view"),
            layout: &view_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: view_buf.as_entire_binding() }],
        });
        let day = month as f64 * 30.44 + 15.0;
        Ok(Globe {
            textures,
            ground,
            view_layout,
            view_buf,
            view_bind,
            camera: Camera::new(-30.0, 25.0, 1.0),
            clock: Clock { day, running: false, rate: 0.0 },
            night: 0.0,
            seconds: 0.0,
            hand: Hand::default(),
            width,
            height,
            ppp,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32, ppp: f64) {
        (self.width, self.height, self.ppp) = (width, height, ppp);
    }

    /// The reading scale: 1.0 on a phone, 1.2 on the 13-inch iPad (Terra's `reading`).
    pub fn reading(&self) -> f64 {
        let short = (self.width.min(self.height) as f64 / self.ppp).max(1.0);
        (1.0 + (short - 390.0) / (1032.0 - 390.0) * 0.2).clamp(1.0, 1.2)
    }

    pub fn disc(&self) -> Disc {
        View::layout(self.width, self.height, self.ppp, self.camera)
    }

    pub fn view(&self) -> View {
        View {
            width: self.width,
            height: self.height,
            ppp: self.ppp,
            reading: self.reading(),
            disc: self.disc(),
            camera: self.camera,
            seconds: self.seconds,
            clock: self.clock,
            rest: 1.0,
            night: self.night,
            export: 0.0,
        }
    }

    /// Where a place is on screen (points) and how far it faces the reader (1 centre, 0 rim, <0 behind).
    pub fn project(&self, at: LonLat) -> ([f64; 2], f64) {
        self.disc().project(&self.camera, at)
    }

    /// The place under a point of the screen, if it is on the globe.
    pub fn unproject(&self, x: f64, y: f64) -> Option<LonLat> {
        self.disc().unproject(&self.camera, x, y)
    }

    pub fn view_bind(&self) -> &wgpu::BindGroup {
        &self.view_bind
    }

    /// Advances the hand's fling and the date; keeps the land textures on the date's months.
    pub fn update(&mut self, gpu: &Gpu, assets: &dyn Assets, t: f64, dt: f64) {
        self.seconds = t;
        if self.clock.running {
            self.clock.day += self.clock.rate * dt;
        }
        let (ma, mb, _) = self.clock.months();
        if let Ok(true) = self.textures.set_months(gpu, assets, ma, mb) {
            self.ground.rebind(gpu, &self.textures);
        }
        let disc = self.disc();
        self.camera = self.hand.coast(self.camera, disc, dt);
    }

    /// A finger for the hand (drag turns, pinch zooms, a flick carries on). Says whether it took it.
    pub fn pointer(&mut self, e: PointerEvent) -> bool {
        if !self.hand.on {
            return false;
        }
        let disc = self.disc();
        self.camera = self.hand.pointer(self.camera, disc, e);
        true
    }

    /// Writes the view uniform (call once a frame before drawing anything that reads it).
    pub fn upload(&self, gpu: &Gpu) {
        gpu.queue.write_buffer(&self.view_buf, 0, bytemuck::bytes_of(&self.view().uniform()));
    }

    /// The ground, filling the target (it clears nothing: it draws every pixel).
    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>) {
        self.ground.draw(rp, &self.view_bind);
    }
}

/// A plain globe hand: one finger turns the globe under it (a trackball, over the poles too), two
/// pinch, a flick keeps turning and slows. Zoom is kept within `min_zoom..max_zoom`.
pub struct Hand {
    pub on: bool,
    pub min_zoom: f64,
    pub max_zoom: f64,
    /// Below this zoom the globe eases back up once the fingers lift (a pinch out springs back to
    /// fill its room); set it to `min_zoom` to keep wherever the pinch left it.
    pub rest_zoom: f64,
    fingers: HashMap<u64, [f64; 2]>,
    /// The flick's turn: axis (view space) times radians a second.
    spin: [f64; 3],
    last_t: f64,
}

impl Default for Hand {
    fn default() -> Hand {
        Hand { on: true, min_zoom: 0.6, max_zoom: 40.0, rest_zoom: 1.0, fingers: HashMap::new(), spin: [0.0; 3], last_t: 0.0 }
    }
}

impl Hand {
    /// How many fingers are down.
    pub fn fingers(&self) -> usize {
        self.fingers.len()
    }

    pub fn pointer(&mut self, cam: Camera, disc: Disc, e: PointerEvent) -> Camera {
        let mut cam = cam;
        match e.phase {
            Phase::Down => {
                self.fingers.insert(e.id, [e.x, e.y]);
                self.spin = [0.0; 3];
            }
            Phase::Move => {
                let Some(&prev) = self.fingers.get(&e.id) else { return cam };
                let n = self.fingers.len();
                if n == 1 {
                    let (dx, dy) = (e.x - prev[0], e.y - prev[1]);
                    // screen y is down, view y up: a drag (dx, dy) turns the globe about (dy, dx, 0)
                    let angle = dx.hypot(dy) / disc.r;
                    cam = cam.turned([dy, dx, 0.0], angle);
                    let dt = (e.t - self.last_t).max(1.0 / 240.0);
                    if dt < 0.1 {
                        let k = angle / dt;
                        let l = dx.hypot(dy).max(1e-9);
                        let s = [dy / l * k, dx / l * k, 0.0];
                        self.spin = [0.6 * s[0] + 0.4 * self.spin[0], 0.6 * s[1] + 0.4 * self.spin[1], 0.0];
                    }
                } else if n >= 2 {
                    let other = self.fingers.iter().find(|(id, _)| **id != e.id).map(|(_, p)| *p).unwrap_or(prev);
                    let d0 = (prev[0] - other[0]).hypot(prev[1] - other[1]).max(1.0);
                    let d1 = (e.x - other[0]).hypot(e.y - other[1]).max(1.0);
                    cam.zoom = (cam.zoom * d1 / d0).clamp(self.min_zoom, self.max_zoom);
                }
                self.fingers.insert(e.id, [e.x, e.y]);
                self.last_t = e.t;
            }
            Phase::Up | Phase::Cancel => {
                self.fingers.remove(&e.id);
                if e.t - self.last_t > 0.08 || !self.fingers.is_empty() {
                    self.spin = [0.0; 3];
                }
            }
        }
        cam
    }

    /// The flick carrying on, slowing by half every 0.35 s.
    pub fn coast(&mut self, cam: Camera, _disc: Disc, dt: f64) -> Camera {
        if !self.fingers.is_empty() {
            return cam;
        }
        let mut cam = cam;
        if cam.zoom < self.rest_zoom {
            // a pinch out springs back: halfway every 0.25 s
            let k = 0.5f64.powf(dt / 0.25);
            cam.zoom = self.rest_zoom - (self.rest_zoom - cam.zoom) * k;
            if self.rest_zoom - cam.zoom < 1e-3 {
                cam.zoom = self.rest_zoom;
            }
        }
        let w = self.spin[0].hypot(self.spin[1]);
        if w < 1e-3 {
            self.spin = [0.0; 3];
            return cam;
        }
        let out = cam.turned(self.spin, w * dt / 1.0);
        let k = 0.5f64.powf(dt / 0.35);
        self.spin = [self.spin[0] * k, self.spin[1] * k, 0.0];
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drag_keeps_the_place_under_the_finger() {
        let disc = Disc { cx: 200.0, cy: 400.0, r: 180.0 };
        let mut hand = Hand::default();
        let mut cam = Camera::new(10.0, 20.0, 1.0);
        let (x0, y0) = (230.0, 380.0);
        let under = disc.unproject(&cam, x0, y0).unwrap();
        cam = hand.pointer(cam, disc, PointerEvent { id: 1, phase: Phase::Down, x: x0, y: y0, t: 0.0 });
        let (mut x, mut y) = (x0, y0);
        for i in 1..=20 {
            x += 1.5;
            y += 1.0;
            cam = hand.pointer(cam, disc, PointerEvent { id: 1, phase: Phase::Move, x, y, t: i as f64 / 60.0 });
        }
        let ([px, py], _) = disc.project(&cam, under);
        assert!((px - x).abs() < 1.5 && (py - y).abs() < 1.5, "the place is at {px},{py}, the finger at {x},{y}");
    }

    #[test]
    fn a_pinch_out_springs_back() {
        let disc = Disc { cx: 200.0, cy: 400.0, r: 180.0 };
        let mut hand = Hand::default();
        let mut cam = Camera::new(0.0, 0.0, 0.6);
        for _ in 0..120 {
            cam = hand.coast(cam, disc, 1.0 / 60.0);
        }
        assert!((cam.zoom - 1.0).abs() < 0.01, "zoom {}", cam.zoom);
    }
}
