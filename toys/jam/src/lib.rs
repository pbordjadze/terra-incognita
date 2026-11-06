//! Salt Flat (a working title; the crate keeps its first name, `jam`): cars at night on the
//! Bonneville Salt Flats, where there are no roads. Every driver only follows the tail lights ahead and
//! keeps a safe gap (`flat.rs`); from that, lone cars string into convoys that snake across the salt,
//! merge, split and sometimes close into a ring, and their tyres draw on the crust where they went.
//! Tap a car and it brakes: the cars behind it stop one by one, the jam running back down its convoy.
//! Drag a car and its convoy follows your finger. Hold the open salt and the cars nearby come to circle
//! your light.

pub mod flat;
mod sound;

use flat::{Flat, Hand, CAR};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::layer::Layer;
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Gpu, Phase, PointerEvent, Toy};

/// The cars' paint (the ordinary colours of cars; drawn).
const PAINT: [[f32; 3]; 9] = [
    [0.86, 0.87, 0.88], // white
    [0.60, 0.62, 0.65], // silver
    [0.09, 0.10, 0.11], // black
    [0.14, 0.20, 0.36], // navy
    [0.55, 0.12, 0.10], // red
    [0.47, 0.60, 0.72], // pale blue
    [0.74, 0.68, 0.56], // champagne
    [0.86, 0.87, 0.88], // white again (the commonest)
    [0.20, 0.32, 0.26], // green
];

/// Cars drawn larger than life so they read on a phone (length, width).
const LONG: f64 = 1.3;
const WIDE: f64 = 1.4;
/// About one car to this many square metres of salt on screen.
const AREA: f64 = 1700.0;
/// Seconds of night already run before the toy opens (so it opens mid-story).
const BEFORE: f64 = 12.0;
/// Tyre tracks: stamped every this many seconds, fading by half in HALF_LIFE.
const STAMP: f64 = 0.1;
const HALF_LIFE: f64 = 12.0;
const FADE_EVERY: f64 = 1.0;

pub struct Jam {
    flat: Flat,
    ground: Backdrop,
    painter: Painter,
    tracks: Option<Layer>,
    fade: wgpu::RenderPipeline,
    fade_bind: wgpu::BindGroup,
    last_fade: f64,
    sound: sound::Engines,
    points: [f64; 2],
    ppp: f64,
    t: f64,
    paint: Vec<usize>,
    /// Each car's rear wheels where they last marked the salt (m).
    wheels: Vec<[[f64; 2]; 2]>,
    next_stamp: f64,
    /// Track segments to mark (world m), with their age in seconds.
    marks: Vec<([f64; 2], [f64; 2], f64)>,
    /// Fingers: id, where it went down (points), when, and what it is doing.
    fingers: Vec<Finger>,
    /// Where taps landed (points, when), for the touch's ring.
    taps: Vec<([f64; 2], f64)>,
    /// Lights that fade after the finger lifts: where (m), when let go.
    embers: Vec<([f64; 2], f64)>,
    touched: bool,
    invite: Option<usize>,
    still: bool,
    born: Option<f64>,
}

#[derive(Clone, Copy, Debug)]
struct Finger {
    id: u64,
    down: [f64; 2],
    at: [f64; 2],
    t: f64,
    car: Option<usize>,
    mode: Mode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Pending,
    Lead,
    Lamp,
}

struct Layout {
    c: [f64; 2],
    /// points a metre
    s: f64,
}

/// The reading scale: 1 on a phone, up to 1.35 on a large iPad.
fn reading(points: [f64; 2]) -> f64 {
    (points[0].min(points[1]) / 393.0).clamp(1.0, 1.35)
}

fn layout_of(points: [f64; 2]) -> Layout {
    Layout { c: [points[0] * 0.5, points[1] * 0.5], s: 3.1 * reading(points) }
}

/// The salt the cars drive on reaches past the screen on every side: convoys turn back out in the dark
/// and come in again, so the screen's edge is never a road.
const BEYOND: f64 = 1.45;

fn half_of(points: [f64; 2]) -> [f64; 2] {
    let l = layout_of(points);
    [points[0] * 0.5 / l.s * BEYOND, points[1] * 0.5 / l.s * BEYOND]
}

impl Jam {
    fn layout(&self) -> Layout {
        layout_of(self.points)
    }

    fn to_screen(&self, l: &Layout, p: [f64; 2]) -> [f64; 2] {
        [l.c[0] + l.s * p[0], l.c[1] - l.s * p[1]]
    }

    fn to_world(&self, l: &Layout, q: [f64; 2]) -> [f64; 2] {
        [(q[0] - l.c[0]) / l.s, -(q[1] - l.c[1]) / l.s]
    }

    fn car_at(&self, q: [f64; 2], reach: f64) -> Option<usize> {
        let l = self.layout();
        let mut best = None;
        let mut bd = reach;
        for (i, c) in self.flat.cars.iter().enumerate() {
            let p = self.to_screen(&l, c.p);
            let d = (p[0] - q[0]).hypot(p[1] - q[1]);
            if d < bd {
                bd = d;
                best = Some(i);
            }
        }
        best
    }

    fn reach(&self) -> f64 {
        26.0 * reading(self.points)
    }

    /// The head of the longest convoy (favouring one near the middle), and how many follow it.
    fn longest(&self) -> (usize, usize) {
        // on screen (the salt reaches past it), well inside its edges
        let h = [self.flat.half[0] / BEYOND, self.flat.half[1] / BEYOND];
        (0..self.flat.cars.len())
            .filter(|&i| self.flat.cars[i].leader.is_none())
            .filter(|&i| {
                let p = self.flat.cars[i].p;
                p[0].abs() < 0.75 * h[0] && p[1].abs() < 0.8 * h[1]
            })
            .map(|i| {
                let p = self.flat.cars[i].p;
                let mid = 1.0 - 0.6 * (p[0] / h[0]).hypot(p[1] / h[1]).min(1.0);
                (i, self.flat.behind(i).len(), mid)
            })
            .max_by(|a, b| (a.1 as f64 * (0.75 + 0.25 * a.2)).total_cmp(&(b.1 as f64 * (0.75 + 0.25 * b.2))))
            .map(|(i, n, _)| (i, n))
            .unwrap_or((0, 0))
    }

    fn sync_hands(&mut self) {
        let l = self.layout();
        self.flat.hands = self
            .fingers
            .iter()
            .filter_map(|f| match (f.mode, f.car) {
                (Mode::Lead, Some(i)) => Some(Hand::Lead(i, self.to_world(&l, f.at))),
                (Mode::Lamp, _) => Some(Hand::Lamp(self.to_world(&l, f.at))),
                _ => None,
            })
            .collect();
    }

    fn stamp(&mut self, age: f64) {
        for (i, c) in self.flat.cars.iter().enumerate() {
            let d = c.dir();
            let n = [-d[1], d[0]];
            let back = CAR * 0.3 * LONG;
            let side = 0.75 * WIDE;
            let w = [
                [c.p[0] - d[0] * back + n[0] * side, c.p[1] - d[1] * back + n[1] * side],
                [c.p[0] - d[0] * back - n[0] * side, c.p[1] - d[1] * back - n[1] * side],
            ];
            let was = self.wheels[i];
            let moved = (w[0][0] - was[0][0]).hypot(w[0][1] - was[0][1]);
            if moved > 0.4 && moved < 6.0 {
                self.marks.push((was[0], w[0], age));
                self.marks.push((was[1], w[1], age));
            }
            if moved >= 0.4 || moved.is_nan() {
                self.wheels[i] = w;
            }
        }
    }
}

const FADE_WGSL: &str = r#"
struct VO { @builtin(position) pos: vec4<f32> };
@group(0) @binding(0) var<uniform> unused: vec4<f32>;
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); return o;
}
@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> { return vec4<f32>(1.5 / 255.0) + unused * 0.0; }
"#;

/// The tracks' fading: what is in the layer times the blend constant, less a hair, so old tracks go
/// all the way rather than sticking at the last 8-bit step.
fn fade_pipeline(gpu: &Gpu) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
    let shader = gpu.shader("tracks fade", FADE_WGSL);
    let bgl = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("tracks fade"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }],
    });
    use wgpu::util::DeviceExt;
    let buf = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("tracks fade"), contents: &[0u8; 16], usage: wgpu::BufferUsages::UNIFORM });
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("tracks fade"), layout: &bgl, entries: &[wgpu::BindGroupEntry { binding: 0, resource: buf.as_entire_binding() }] });
    let comp = wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::Constant, operation: wgpu::BlendOperation::ReverseSubtract };
    let pipe = gpu.fullscreen_pipeline("tracks fade", &shader, &[&bgl], Some(wgpu::BlendState { color: comp, alpha: comp }));
    (pipe, bind)
}

impl Toy for Jam {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let ground = Backdrop::new(&ctx.gpu, "salt", GROUND_WGSL);
        let painter = Painter::new(&ctx.gpu);
        let (fade, fade_bind) = fade_pipeline(&ctx.gpu);
        let points = ctx.points();
        let half = half_of(points);
        let n = ((4.0 * half[0] * half[1]) / AREA).round().clamp(40.0, 400.0) as usize;
        let mut flat = Flat::new(0x5a17_f1a7, half, n);
        let mut h: u32 = 0x9e3779b9;
        let paint = (0..n)
            .map(|_| {
                h ^= h << 13;
                h ^= h >> 17;
                h ^= h << 5;
                (h % PAINT.len() as u32) as usize
            })
            .collect();
        // the night already under way: the cars have been driving, and their tracks are on the salt
        let wheels = vec![[[f64::NAN; 2]; 2]; n];
        flat.step(0.0);
        let sound = sound::Engines::new();
        let mut toy = Jam {
            flat,
            ground,
            painter,
            tracks: None,
            fade,
            fade_bind,
            last_fade: 0.0,
            sound,
            points,
            ppp: ctx.ppp,
            t: 0.0,
            paint,
            wheels,
            next_stamp: 0.0,
            marks: Vec::new(),
            fingers: Vec::new(),
            taps: Vec::new(),
            embers: Vec::new(),
            touched: false,
            invite: None,
            still: ctx.reduce_motion,
            born: None,
        };
        toy_kit::step("salt: the night before");
        let mut s = 0.0;
        while s < BEFORE {
            toy.flat.step(STAMP);
            s += STAMP;
            toy.stamp(BEFORE - s);
        }
        Ok(toy)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.points = ctx.points();
        self.ppp = ctx.ppp;
        self.flat.half = half_of(self.points);
        self.tracks = None;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let q = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                self.touched = true;
                let car = self.car_at(q, self.reach());
                let mode = if car.is_some() { Mode::Pending } else { Mode::Lamp };
                self.fingers.retain(|f| f.id != e.id);
                self.fingers.push(Finger { id: e.id, down: q, at: q, t: e.t, car, mode });
                if mode == Mode::Lamp {
                    ctx.haptic();
                }
            }
            Phase::Move => {
                let slop = 10.0;
                if let Some(f) = self.fingers.iter_mut().find(|f| f.id == e.id) {
                    f.at = q;
                    if f.mode == Mode::Pending && (q[0] - f.down[0]).hypot(q[1] - f.down[1]) > slop {
                        f.mode = Mode::Lead;
                        if let Some(i) = f.car {
                            // a led car leaves its convoy (its followers stay with it)
                            self.flat.cars[i].leader = None;
                            self.flat.cars[i].stopping = false;
                            self.flat.cars[i].brake = 0.0;
                        }
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                if let Some(k) = self.fingers.iter().position(|f| f.id == e.id) {
                    let f = self.fingers.remove(k);
                    let l = self.layout();
                    match (f.mode, f.car) {
                        (Mode::Pending, Some(i)) if e.phase == Phase::Up => {
                            self.flat.tap(i);
                            let p = self.to_screen(&l, self.flat.cars[i].p);
                            self.taps.push((p, self.t));
                            if self.taps.len() > 8 {
                                self.taps.remove(0);
                            }
                            self.sound.thump();
                            ctx.haptic();
                        }
                        (Mode::Lamp, _) => self.embers.push((self.to_world(&l, f.at), self.t)),
                        _ => {}
                    }
                }
            }
        }
        self.sync_hands();
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.points = ctx.points();
        self.t = t;
        if self.born.is_none() {
            self.born = Some(t);
            self.next_stamp = t;
            self.invite = Some(self.longest().0);
        }
        // a finger held on a car without moving, past a moment, takes the wheel too
        for f in self.fingers.iter_mut() {
            if f.mode == Mode::Pending && t - f.t > 0.45 {
                f.mode = Mode::Lead;
                if let Some(i) = f.car {
                    self.flat.cars[i].leader = None;
                }
            }
        }
        self.sync_hands();
        self.flat.step(dt);
        while t >= self.next_stamp {
            self.next_stamp += STAMP;
            if self.next_stamp < t - 1.0 {
                self.next_stamp = t;
            }
            self.stamp(0.0);
        }
        self.taps.retain(|(_, at)| t - at < 2.0);
        self.embers.retain(|(_, at)| t - at < 3.0);
        // the engines heard: the cars nearest the middle of the screen
        let half = self.flat.half;
        let mut near: Vec<(f64, (f32, f32, f32))> = self
            .flat
            .cars
            .iter()
            .map(|c| {
                let d = (c.p[0] / half[0]).hypot(c.p[1] / half[1]);
                (d, (c.v as f32, c.acc as f32, (c.p[0] / half[0]).clamp(-1.0, 1.0) as f32))
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        let cars: Vec<(f32, f32, f32)> = near.iter().take(sound::VOICES).map(|x| x.1).collect();
        self.sound.set(&cars);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let k = ctx.ppp;
        let size = [ctx.width, ctx.height];
        let l = self.layout();
        let m = (l.s * k) as f32; // pixels a metre
        let px = |p: [f64; 2]| -> [f32; 2] { [((l.c[0] + l.s * p[0]) * k) as f32, ((l.c[1] - l.s * p[1]) * k) as f32] };
        let mut enc = gpu.device.create_command_encoder(&Default::default());

        // the tracks: a layer that keeps what the tyres draw, fading slowly
        if self.tracks.as_ref().is_none_or(|t| t.size != size) {
            let layer = Layer::new(gpu, size);
            layer.clear(&mut enc);
            self.tracks = Some(layer);
            self.last_fade = self.t;
        }
        let tracks = self.tracks.as_ref().unwrap();
        if self.t - self.last_fade >= FADE_EVERY {
            let steps = ((self.t - self.last_fade) / FADE_EVERY).floor();
            self.last_fade += steps * FADE_EVERY;
            let kf = 0.5f64.powf(steps * FADE_EVERY / HALF_LIFE);
            let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("tracks fade"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: &tracks.view, resolve_target: None, depth_slice: None, ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.fade);
            pass.set_bind_group(0, &self.fade_bind, &[]);
            pass.set_blend_constant(wgpu::Color { r: kf, g: kf, b: kf, a: kf });
            pass.draw(0..3, 0..1);
        }
        if !self.marks.is_empty() {
            let r = 0.17 * m;
            for (a, b, age) in self.marks.drain(..) {
                if a[0].is_nan() {
                    continue;
                }
                let f = 0.5f64.powf(age / HALF_LIFE) as f32;
                self.painter.line(px(a), px(b), r, 0.7 * m * 0.12 + 0.6, [0.02, 0.022, 0.03, 0.30 * f]);
            }
            self.painter.render(gpu, &mut enc, &tracks.view, size, None);
        }

        let params = BackdropParams { time: self.t as f32, scale: m, center: [0.0, 0.0], p: [(l.c[0] * k) as f32, (l.c[1] * k) as f32, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0] };
        self.ground.render(gpu, &mut enc, target, size, &params);
        tracks.composite(&mut enc, target);

        let half = CAR / 2.0;
        let cars = &self.flat.cars;
        // where a point on a car is: fwd along it, side to its right (m), drawn larger than life
        let at = |c: &flat::Car, fwd: f64, side: f64| -> [f32; 2] {
            let d = c.dir();
            let r = [d[1], -d[0]];
            let (fwd, side) = (fwd * LONG, side * WIDE);
            px([c.p[0] + d[0] * fwd + r[0] * side, c.p[1] + d[1] * fwd + r[1] * side])
        };

        // light on the salt: headlights ahead, tail lights behind, a held finger's light
        for c in cars.iter() {
            // headlights: a cone of warm light reaching ahead over the salt, the car's own light
            for (d, w, a) in [(3.0, 0.9, 0.08), (7.0, 1.5, 0.045), (12.0, 2.3, 0.028), (17.0, 3.1, 0.016)] {
                self.painter.glow(at(c, half + 0.8, 0.0), at(c, half + d, 0.0), w * m, w * 0.9 * m, [1.0, 0.88, 0.66, a]);
            }
            let red = c.lights as f32;
            self.painter.glow(at(c, -half - 0.6, 0.0), at(c, -half - 2.4, 0.0), 1.3 * m, 2.2 * m, [1.0, 0.07, 0.04, 0.03 + 0.6 * red]);
        }
        let lamp_glow = |p: &mut Painter, w: [f64; 2], a: f32| {
            let q = px(w);
            p.glow(q, q, 4.0 * m, 22.0 * m, [1.0, 0.80, 0.50, 0.16 * a]);
            p.glow(q, q, 1.2 * m, 5.0 * m, [1.0, 0.86, 0.62, 0.35 * a]);
            p.glow(q, q, 0.5 * m, 1.2 * m, [1.0, 0.96, 0.88, 0.9 * a]);
        };
        let held: Vec<([f64; 2], f32)> = self.fingers.iter().filter(|f| f.mode == Mode::Lamp).map(|f| (self.to_world(&l, f.at), ((self.t - f.t) / 0.3).clamp(0.0, 1.0) as f32)).collect();
        for (w, come) in held {
            lamp_glow(&mut self.painter, w, come);
        }
        for &(w, at_t) in &self.embers {
            let a = (1.0 - (self.t - at_t) / 3.0).clamp(0.0, 1.0) as f32;
            lamp_glow(&mut self.painter, w, a * a);
        }
        self.painter.render(gpu, &mut enc, target, size, None);

        // the cars
        let wd = WIDE as f32;
        for (i, c) in cars.iter().enumerate() {
            let col = PAINT[self.paint[i]];
            let dim = |c: [f32; 3], f: f32, a: f32| [c[0] * f, c[1] * f, c[2] * f, a];
            // shadow (the moon's, from the upper left)
            let sh = |q: [f32; 2]| [q[0] + 0.3 * m, q[1] + 0.4 * m];
            self.painter.line(sh(at(c, -1.75, 0.0)), sh(at(c, 1.75, 0.0)), 0.95 * m * wd, 0.6 * m, [0.0, 0.0, 0.03, 0.45]);
            for side in [-0.36, 0.36] {
                self.painter.line(at(c, -1.72, side), at(c, 1.72, side), 0.48 * m * wd, 0.6, dim(col, 0.80, 1.0));
            }
            self.painter.line(at(c, -0.95, 0.0), at(c, 0.45, 0.0), 0.56 * m * wd, 0.08 * m, dim(col, 0.95, 1.0));
            self.painter.line(at(c, 0.86, -0.50), at(c, 0.86, 0.50), 0.24 * m * wd, 0.6, [0.05, 0.07, 0.10, 0.95]);
            self.painter.line(at(c, -1.30, -0.46), at(c, -1.30, 0.46), 0.17 * m * wd, 0.6, [0.05, 0.07, 0.10, 0.9]);
            for side in [-0.6, 0.6] {
                self.painter.disc(at(c, -2.08, side), 0.17 * m * wd, 0.6, [0.35, 0.04, 0.04, 1.0]);
                self.painter.disc(at(c, 2.10, side), 0.16 * m * wd, 0.6, [0.85, 0.84, 0.78, 1.0]);
            }
        }
        // lamps alight
        for c in cars.iter() {
            for side in [-0.6, 0.6] {
                self.painter.glow(at(c, 2.15, side), at(c, 2.15, side), 0.12 * m, 0.35 * m, [1.0, 0.92, 0.75, 0.5]);
                let red = c.lights as f32;
                let q = at(c, -2.1, side);
                self.painter.glow(q, q, 0.2 * m * wd, 0.25 * m, [1.0, 0.25, 0.15, 0.18 + 0.82 * red]);
                if red > 0.01 {
                    self.painter.glow(q, q, 0.4 * m, 2.2 * m, [1.0, 0.07, 0.04, 0.55 * red]);
                }
            }
        }
        // a finger leading a car: a soft ring at the finger
        for f in &self.fingers {
            if f.mode == Mode::Lead {
                let q = [(f.at[0] * k) as f32, (f.at[1] * k) as f32];
                self.painter.glow_ring(q, 5.0 * m, 0.18 * m, 0.4 * m, [1.0, 0.95, 0.88, 0.3]);
            }
        }
        // the invitation: until the first touch, the head of the longest convoy breathes a soft halo
        let since = self.t - self.born.unwrap_or(self.t);
        if let (false, Some(i), true) = (self.touched, self.invite, since > 1.2) {
            let p = px(cars[i].p);
            let come = ((since - 1.2) / 0.6).min(1.0) as f32;
            let breathe = if self.still { 0.8 } else { 0.55 + 0.45 * (0.5 + 0.5 * ((since - 1.2) * 2.4).sin()) as f32 };
            let r = (4.6 + 0.5 * breathe) * m;
            self.painter.glow_ring(p, r, 0.22 * m, 0.35 * m, [1.0, 0.97, 0.90, 0.55 * come * breathe]);
            self.painter.glow(p, p, r * 0.8, r * 0.6, [1.0, 0.95, 0.85, 0.05 * come * breathe]);
        }
        // a tap: a soft ring from the car
        for &(p, at_t) in &self.taps {
            let a = ((self.t - at_t) / 0.9).clamp(0.0, 1.0) as f32;
            if a < 1.0 {
                let q = [(p[0] * k) as f32, (p[1] * k) as f32];
                self.painter.glow_ring(q, (3.0 + 6.0 * a) * m, 0.25 * m, 0.4 * m, [1.0, 0.95, 0.9, 0.5 * (1.0 - a)]);
            }
        }
        self.painter.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.still = ctx.reduce_motion;
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(self.sound.synth())
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let l = self.layout();
        let mut a: Vec<(String, [f64; 2])> = self.flat.cars.iter().enumerate().map(|(i, c)| (format!("car{i}"), self.to_screen(&l, c.p))).collect();
        let (head, _) = self.longest();
        a.push(("head".into(), self.to_screen(&l, self.flat.cars[head].p)));
        // a car well back in the longest convoy
        let behind = self.flat.behind(head);
        if let Some(&mid) = behind.get(behind.len() / 3) {
            a.push(("middle".into(), self.to_screen(&l, self.flat.cars[mid].p)));
        }
        a.push(("centre".into(), l.c));
        // the open salt farthest from any car, away from the screen's edges
        let mut best = (0.0, l.c);
        for gy in 3..10 {
            for gx in 1..5 {
                let q = [self.points[0] * gx as f64 / 5.0, self.points[1] * gy as f64 / 12.0];
                let w = self.to_world(&l, q);
                let d = self.flat.cars.iter().map(|c| (c.p[0] - w[0]).hypot(c.p[1] - w[1])).fold(f64::MAX, f64::min);
                if d > best.0 {
                    best = (d, q);
                }
            }
        }
        a.push(("open".into(), best.1));
        if let Some(i) = self.invite {
            a.push(("invited".into(), self.to_screen(&l, self.flat.cars[i].p)));
        }
        a
    }

    fn demo(&self, _ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        let l = self.layout();
        let (head, _) = self.longest();
        let c = &self.flat.cars[head];
        let d = c.dir();
        let p = self.to_screen(&l, [c.p[0] + d[0] * c.v * 0.5, c.p[1] + d[1] * c.v * 0.5]);
        Some(toy_kit::info::Demo::new("Tap a car, and everyone behind it stops.", vec![toy_kit::info::Stroke::tap(0.5, p)]))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::new(
            &[
                "No roads. Every driver follows the nearest tail lights ahead, keeps a safe gap, and turns no tighter than a car can.",
                "Behind the same car too long, a driver gets bored and wanders off.",
                "Tap a car and it brakes. Drag one and its convoy follows you. Hold the open salt and they come to your light.",
            ],
            "Real: the drivers' gap-keeping rule, the one that made a jam out of nothing on a test ring in Nagoya in 2008, and the salt crust's polygons. Drawn: the night, the cars, everything they do.",
        ))
    }
}

/// The Bonneville salt by moonlight: a white crust broken into polygons a few metres across by low
/// ridges, patches a little damper and darker, the light falling off to the edges. `p0.xy` the centre
/// (px), `u.scale` pixels a metre.
const GROUND_WGSL: &str = r#"
fn fbm(p: vec2<f32>) -> f32 {
  var a = 0.0; var w = 0.5; var q = p;
  for (var i = 0; i < 4; i = i + 1) { a = a + w * noise2(q); q = q * 2.03 + vec2<f32>(17.1, 9.3); w = w * 0.5; }
  return a;
}
fn h22(p: vec2<f32>) -> vec2<f32> {
  return vec2<f32>(hash2(p), hash2(p + vec2<f32>(41.3, 7.7)));
}
// distance to the nearest polygon edge (Voronoi F2 - F1), cells 1 unit across
fn ridge(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = fract(p);
  var d1 = 8.0; var d2 = 8.0;
  for (var y = -1; y <= 1; y = y + 1) {
    for (var x = -1; x <= 1; x = x + 1) {
      let g = vec2<f32>(f32(x), f32(y));
      let o = 0.15 + 0.7 * h22(i + g);
      let d = length(g + o - f);
      if (d < d1) { d2 = d1; d1 = d; } else if (d < d2) { d2 = d; }
    }
  }
  return d2 - d1;
}
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let m = (px - u.p0.xy) / u.scale;          // metres from the centre
  let pxm = 1.0 / u.scale;
  // the moonlit crust
  var col = vec3<f32>(0.150, 0.165, 0.205);
  let damp = smoothstep(0.35, 0.75, fbm(m * 0.018 + vec2<f32>(3.1, 1.7)));
  col = col * (1.0 - 0.22 * damp) * (0.93 + 0.12 * fbm(m * 0.21)) * (0.97 + 0.05 * noise2(m * 2.7));
  // the polygons: cells about 2.5 m across, their ridges raised and catching the light
  let cell = 2.6;
  let e = ridge(m / cell + vec2<f32>(0.3 * fbm(m * 0.5), 0.0));
  let w = 0.035 + 1.2 * pxm / cell;
  let rim = 1.0 - smoothstep(0.0, w, e);
  let shade = 1.0 - smoothstep(w, w * 3.5, e);
  col = col * (1.0 - 0.10 * shade) + vec3<f32>(0.10, 0.11, 0.12) * rim * (1.0 - 0.6 * damp);
  // the moon from the upper left, the dark coming in at the edges
  let uv = px / u.size;
  col = col * (1.04 - 0.16 * uv.y + 0.06 * (1.0 - uv.x));
  let v = length((uv - vec2<f32>(0.5, 0.5)) * vec2<f32>(1.0, u.size.y / u.size.x));
  col = col * (1.0 - 0.55 * smoothstep(0.30, 1.05, v));
  return vec4<f32>(col, 1.0);
}
"#;

toy_kit::ios_shell!(Jam);
