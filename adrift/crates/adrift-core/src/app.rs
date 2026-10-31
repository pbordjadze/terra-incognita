//! The app: the state (floats, the clock, the camera, undo), a frame, a pointer event, and applying
//! intents (Engineering; DECISIONS.md §2).

use crate::assets::Assets;
use crate::gpu::{Gpu, Textures, View, ViewUniform};
use crate::look::sprites::{KIND_BEAD, KIND_FLOAT, KIND_MAYBE};
use crate::look::{sea::Ripple, GroundPass, Instance, SeaPass, SpritePass};
use crate::model::*;
use crate::sea::Rules;
use crate::touch::{CameraControl, Gestures, Motion, Out};
use serde::{Deserialize, Serialize};

/// What the reader has done, to take back.
#[derive(Clone, Debug)]
enum Undo {
    Put(u32),
    Moved { id: u32, from: Float },
    Removed(Float),
}

/// Everything about the sea's state that a scene file can set (adrift-check) and a frame reads.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub camera: Camera,
    pub day: f64,
    #[serde(default = "yes")]
    pub running: bool,
    #[serde(default)]
    pub floats: Vec<SceneFloat>,
    /// A float held (its maybes shown), by its index in `floats`.
    #[serde(default)]
    pub hold: Option<usize>,
    /// The drawn motion's seconds.
    #[serde(default)]
    pub seconds: f64,
}
fn yes() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SceneFloat {
    pub lon: f64,
    pub lat: f64,
    /// Days before the scene's day it was put (its wake is simulated from there).
    #[serde(default)]
    pub days_ago: f64,
}

pub struct App {
    pub gpu: Gpu,
    assets: Box<dyn Assets>,
    textures: Textures,
    pub rules: Rules,
    view_buffer: wgpu::Buffer,
    view_bind: wgpu::BindGroup,
    ground: GroundPass,
    sea: SeaPass,
    sprites: SpritePass,
    pub floats: Vec<Float>,
    next_id: u32,
    pub clock: Clock,
    undo: Vec<Undo>,
    pub camera: CameraControl,
    gestures: Gestures,
    ripples: Vec<Ripple>,
    width: u32,
    height: u32,
    ppp: f64,
    pub reading: f64,
    /// The drawn motion's seconds (held while the date is).
    seconds: f64,
    last_t: Option<f64>,
    /// How far each float has been stepped (`id` → day).
    stepped: std::collections::HashMap<u32, f64>,
    pub reduce_motion: bool,
}

/// Days a second while the date runs.
pub const RATE: f64 = 2.0;
/// The longest step a float takes at once (a quarter day: the fastest currents cross less than a cell).
const SUBSTEP: f64 = 0.25;
/// Beads kept in a wake.
const WAKE_MAX: usize = 400;

impl App {
    pub fn new(gpu: Gpu, assets: Box<dyn Assets>, width: u32, height: u32, ppp: f64) -> anyhow::Result<App> {
        let clock = Clock { day: 0.0, running: true, rate: RATE };
        let textures = Textures::load(&gpu, assets.as_ref(), clock.months().0)?;
        let rules = Rules {
            currents: crate::sea::Currents::load(assets.as_ref())?,
            coast: crate::sea::Coast::load(assets.as_ref())?,
            ice: crate::sea::Ice::load(assets.as_ref())?,
        };
        let view_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("view"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let view_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("view"),
            size: std::mem::size_of::<ViewUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let view_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("view"),
            layout: &view_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: view_buffer.as_entire_binding() }],
        });
        let ground = GroundPass::new(&gpu, &view_layout, &textures);
        let sea = SeaPass::new(&gpu, &view_layout, &textures);
        let sprites = SpritePass::new(&gpu, &view_layout);
        let reading = Self::reading_for(width, height, ppp);
        Ok(App {
            gpu,
            assets,
            textures,
            rules,
            view_buffer,
            view_bind,
            ground,
            sea,
            sprites,
            floats: Vec::new(),
            next_id: 1,
            clock,
            undo: Vec::new(),
            camera: CameraControl::new(Camera { lon: -30.0, lat: 25.0, zoom: 1.0 }),
            gestures: Gestures::new(),
            ripples: Vec::new(),
            width,
            height,
            ppp,
            reading,
            seconds: 0.0,
            last_t: None,
            stepped: Default::default(),
            reduce_motion: false,
        })
    }

    /// The reading scale: 1.0 on a phone, up to 1.2 on a 13-inch iPad (Terra's rule).
    fn reading_for(width: u32, height: u32, ppp: f64) -> f64 {
        let short = (width.min(height) as f64) / ppp;
        (1.0 + 0.2 * ((short - 430.0) / (1032.0 - 430.0)).clamp(0.0, 1.0) * 1.0).clamp(1.0, 1.2)
    }

    pub fn resize(&mut self, width: u32, height: u32, ppp: f64) {
        self.width = width;
        self.height = height;
        self.ppp = ppp;
        self.reading = Self::reading_for(width, height, ppp);
    }

    pub fn disc(&self) -> Disc {
        View::layout(self.width, self.height, self.ppp, self.camera.camera)
    }

    pub fn view(&self) -> View {
        View {
            width: self.width,
            height: self.height,
            ppp: self.ppp,
            reading: self.reading,
            disc: self.disc(),
            camera: self.camera.camera,
            seconds: self.seconds,
            clock: self.clock,
        }
    }

    /// What is under a point of the screen (points).
    pub fn hit(&self, x: f64, y: f64) -> Hit {
        let disc = self.disc();
        let reach = 14.0 * self.reading;
        let mut best: Option<(f64, u32)> = None;
        for f in &self.floats {
            let (p, facing) = disc.project(&self.camera.camera, f.at);
            if facing < 0.05 {
                continue;
            }
            let d = ((p[0] - x).powi(2) + (p[1] - y).powi(2)).sqrt();
            if d < reach && best.map_or(true, |b| d < b.0) {
                best = Some((d, f.id));
            }
        }
        let place = disc.unproject(&self.camera.camera, x, y);
        if let Some((_, id)) = best {
            let f = self.floats.iter().find(|f| f.id == id).unwrap();
            return Hit::Float(id, place.unwrap_or(f.at));
        }
        match place {
            Some(p) if self.rules.coast.is_land(p) => Hit::Land(p),
            Some(p) => Hit::Sea(p),
            None => Hit::Off,
        }
    }

    pub fn pointer(&mut self, ev: PointerEvent) {
        let mut gestures = std::mem::take(&mut self.gestures);
        let out = gestures.feed(ev, &*self);
        self.gestures = gestures;
        self.apply(out, ev.t);
    }

    fn motion(&self) -> Motion {
        Motion { reduce: self.reduce_motion }
    }

    fn apply(&mut self, out: Out, now: f64) {
        let disc = self.disc();
        let motion = self.motion();
        for m in out.camera {
            self.camera.apply(&disc, m, now, motion);
        }
        for i in out.intents {
            self.intend(i);
        }
    }

    /// Applies one intent to the state.
    pub fn intend(&mut self, intent: Intent) {
        match intent {
            Intent::Ring { at } => self.ripples.push(Ripple { at, when: self.seconds, strength: 1.0 }),
            Intent::Put { at } => {
                let id = self.put(at);
                self.gestures.float_put(id);
                self.undo.push(Undo::Put(id));
                self.ripples.push(Ripple { at, when: self.seconds, strength: 0.6 });
            }
            Intent::Hold { float } => {
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == float) {
                    f.hand = Hand::Held;
                    f.lit_for = 0.0;
                    self.undo.push(Undo::Moved { id: float, from: f.clone() });
                }
            }
            Intent::Carry { float, to } => {
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == float) {
                    f.hand = Hand::Carried;
                    f.at = to;
                }
            }
            Intent::Release { float, at } => {
                let Some(i) = self.floats.iter().position(|f| f.id == float) else { return };
                match at {
                    Some(p) => {
                        let moved = self.floats[i].hand == Hand::Carried;
                        let f = &mut self.floats[i];
                        f.hand = Hand::Free;
                        f.lit_for = 0.0;
                        if moved {
                            f.at = p;
                            f.put_day = self.clock.day;
                            f.wake.clear();
                            f.held = Held::No;
                            self.stepped.insert(f.id, self.clock.day);
                            self.ripples.push(Ripple { at: p, when: self.seconds, strength: 0.6 });
                        } else {
                            // held and let go where it was: nothing changed, nothing to undo
                            if matches!(self.undo.last(), Some(Undo::Moved { id, .. }) if *id == float) {
                                self.undo.pop();
                            }
                        }
                    }
                    None => {
                        let f = self.floats.remove(i);
                        // taken out: undo gives it back as it was before the touch began (the
                        // held snapshot), not where the hand last carried it
                        let before = match self.undo.last() {
                            Some(Undo::Moved { id, from }) if *id == float => {
                                let from = from.clone();
                                self.undo.pop();
                                from
                            }
                            _ => f,
                        };
                        self.undo.push(Undo::Removed(before));
                    }
                }
            }
            Intent::Undo => self.undo(),
            Intent::Ride { float } => {
                if self.camera.riding == Some(float) {
                    self.camera.riding = None;
                } else if self.floats.iter().any(|f| f.id == float) {
                    self.camera.riding = Some(float);
                    if let Some(f) = self.floats.iter_mut().find(|f| f.id == float) {
                        f.lit_for = 0.0;
                    }
                }
            }
            Intent::Scrub { days } => {
                self.clock.day += days;
                for f in &self.floats {
                    let s = self.stepped.entry(f.id).or_insert(self.clock.day);
                    if *s > self.clock.day {
                        *s = self.clock.day;
                    }
                }
            }
            Intent::SetRunning { running } => self.clock.running = running,
        }
    }

    fn put(&mut self, at: LonLat) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let seed = (self.clock.day.to_bits() ^ (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).rotate_left(17);
        self.floats.push(Float { id, at, put_day: self.clock.day, seed, held: Held::No, hand: Hand::Held, wake: Vec::new(), lit_for: 0.0 });
        self.stepped.insert(id, self.clock.day);
        id
    }

    fn undo(&mut self) {
        match self.undo.pop() {
            Some(Undo::Put(id)) => self.floats.retain(|f| f.id != id),
            Some(Undo::Moved { id, from }) => {
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == id) {
                    let mut back = from;
                    back.hand = Hand::Free;
                    back.lit_for = 0.0;
                    *f = back;
                    self.stepped.insert(id, self.clock.day);
                }
            }
            Some(Undo::Removed(mut f)) => {
                f.hand = Hand::Free;
                f.lit_for = 0.0;
                self.stepped.insert(f.id, self.clock.day);
                self.floats.push(f);
            }
            None => {}
        }
    }

    /// Time passes: `now` in the shell's seconds.
    pub fn advance(&mut self, now: f64) {
        let dt = match self.last_t {
            Some(t) => (now - t).clamp(0.0, 0.25),
            None => 0.0,
        };
        self.last_t = Some(now);
        let mut gestures = std::mem::take(&mut self.gestures);
        let out = gestures.tick(now, &*self);
        self.gestures = gestures;
        self.apply(out, now);
        if self.clock.running {
            self.clock.day += self.clock.rate * dt;
            if !self.reduce_motion {
                self.seconds += dt;
            }
        }
        self.step_floats();
        for f in &mut self.floats {
            f.lit_for += dt;
        }
        let ride_at = self.camera.riding.and_then(|id| self.floats.iter().find(|f| f.id == id).map(|f| f.at));
        if self.camera.riding.is_some() && ride_at.is_none() {
            self.camera.riding = None;
        }
        self.camera.tick(now, dt, ride_at, self.floats.is_empty(), self.gestures.touching(), self.motion());
        let horizon = self.seconds - 6.0;
        self.ripples.retain(|r| r.when > horizon);
    }

    /// Every free float catches up with the clock, a quarter day at a time, a bead at each new day.
    fn step_floats(&mut self) {
        let day = self.clock.day;
        for f in &mut self.floats {
            if f.hand == Hand::Carried {
                self.stepped.insert(f.id, day);
                continue;
            }
            let mut t = *self.stepped.get(&f.id).unwrap_or(&day);
            let mut n = 0;
            while t < day - 1e-9 && n < 240 {
                let dt = SUBSTEP.min(day - t);
                let next_day_mark = t.floor() + 1.0;
                let dt = dt.min(next_day_mark - t).max(1e-6);
                let s = self.rules.step(f.at, t, dt, f.seed, 0);
                f.at = s.at;
                f.held = s.held;
                t += dt;
                if (t - next_day_mark).abs() < 1e-6 {
                    t = next_day_mark;
                    if f.wake.last().map_or(true, |b| b.day < next_day_mark - 0.5) {
                        f.wake.push(Bead { at: f.at, day: t });
                        if f.wake.len() > WAKE_MAX {
                            f.wake.remove(0);
                        }
                    }
                }
                n += 1;
            }
            self.stepped.insert(f.id, t);
        }
    }

    /// Sets the whole scene (the checks): the camera, the date, floats with their wakes simulated.
    pub fn set_scene(&mut self, scene: &Scene) {
        self.floats.clear();
        self.undo.clear();
        self.ripples.clear();
        self.stepped.clear();
        self.camera = CameraControl::new(scene.camera);
        self.clock = Clock { day: scene.day, running: scene.running, rate: RATE };
        self.seconds = scene.seconds;
        for (i, sf) in scene.floats.iter().enumerate() {
            self.clock.day = scene.day - sf.days_ago;
            let id = self.put(LonLat::new(sf.lon, sf.lat));
            if let Some(f) = self.floats.iter_mut().find(|f| f.id == id) {
                f.hand = if scene.hold == Some(i) { Hand::Held } else { Hand::Free };
                f.lit_for = 10.0;
            }
        }
        self.clock.day = scene.day;
        self.step_floats();
    }

    /// Draws a frame into `target`.
    pub fn render(&mut self, target: &wgpu::TextureView) {
        let view = self.view();
        let (ma, mb, _) = self.clock.months();
        match self.textures.set_months(&self.gpu, self.assets.as_ref(), ma, mb) {
            Ok(true) => self.ground.rebind(&self.gpu, &self.textures),
            Ok(false) => {}
            Err(e) => log::error!("land for month {ma}: {e}"),
        }
        self.gpu.queue.write_buffer(&self.view_buffer, 0, bytemuck::bytes_of(&view.uniform()));
        self.sea.prepare(&self.gpu, &self.ripples);
        let instances = self.instances(&view);
        self.sprites.prepare(&self.gpu, &instances);
        let mut enc = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.96, g: 0.95, b: 0.92, a: 1.0 }), store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.ground.draw(&mut rp, &self.view_bind);
            self.sea.draw(&mut rp, &self.view_bind);
            self.sprites.draw(&mut rp, &self.view_bind);
        }
        self.gpu.queue.submit([enc.finish()]);
    }

    /// A frame: time passes, then it is drawn.
    pub fn frame(&mut self, target: &wgpu::TextureView, now: f64) {
        self.advance(now);
        self.render(target);
    }

    /// The things on the sea, as sprites on the screen: wakes first, then the maybes, then the floats.
    fn instances(&self, view: &View) -> Vec<Instance> {
        let disc = view.disc;
        let cam = &view.camera;
        let mut out = Vec::new();
        let r = self.reading as f32;
        let amber = [1.0, 0.68, 0.22];
        for f in &self.floats {
            let n = f.wake.len().max(1) as f32;
            for (i, b) in f.wake.iter().enumerate() {
                let (p, facing) = disc.project(cam, b.at);
                if facing < 0.0 {
                    continue;
                }
                let age = 1.0 - (i as f32 + 1.0) / n; // 0 newest
                let a = 0.85 * (1.0 - age).powf(1.6);
                out.push(Instance { at: [p[0] as f32, p[1] as f32], size: (1.1 + 0.5 * (1.0 - age)) * r, kind: KIND_BEAD, color: [amber[0], amber[1], amber[2], a], t: 0.0, _pad: [0.0; 3] });
            }
        }
        for f in &self.floats {
            if f.hand == Hand::Held {
                // the maybes: R0 draws none (Truth's sampler is to come); a faint halo says it is held
                let (p, facing) = disc.project(cam, f.at);
                if facing >= 0.0 {
                    out.push(Instance { at: [p[0] as f32, p[1] as f32], size: 16.0 * r, kind: KIND_MAYBE, color: [1.0, 0.95, 0.85, 0.18], t: 0.0, _pad: [0.0; 3] });
                }
            }
        }
        for f in &self.floats {
            let (p, facing) = disc.project(cam, f.at);
            if facing < 0.0 {
                continue;
            }
            let lit = if f.lit_for < 3.0 { 1.0 } else { 0.35 };
            let size = (6.0 + if f.hand != Hand::Free { 2.0 } else { 0.0 }) * r;
            out.push(Instance { at: [p[0] as f32, p[1] as f32], size, kind: KIND_FLOAT, color: [1.0, 1.0, 1.0, 1.0], t: lit, _pad: [0.0; 3] });
        }
        out
    }
}
