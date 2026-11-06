//! Terra Incognita: the toy that holds the others (painting-company row 217). The whole Earth, turning
//! slowly, and on it a light for each toy, alive with a small likeness of its rule at the place where it
//! is real (`doors`). Touch a light: it stirs and rings, the camera flies there (the Earth falling into
//! night for a night toy), and the toy itself opens where the camera arrived, already going (`rooms`).
//! The Earth mark, bottom left, brings you back to where you were, and the Earth comes up again.
//!
//! One device, one hand, one ground: every toy's globe is the kit's, so the dive is one picture becoming
//! another. Only one toy is open at a time; closing it frees it.

pub mod doors;
pub mod rooms;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use toy_kit::info::{Demo, Rules, Stroke};
use toy_kit::paint::Painter;
use toy_kit::sound::{pentatonic, Bells, Note};
use toy_kit::{Arrival, Assets, Camera, Ctx, Globe, LonLat, MemoryAssets, Phase, PointerEvent, Synth, Toy};

use doors::Life;
use rooms::{Room, Shared};

/// The Earth mark: bottom left, as the info and rules marks are bottom right (toy-kit info.rs).
fn earth_spot(points: [f64; 2]) -> [f64; 2] {
    [26.0, points[1] - 34.0]
}
const EARTH_REACH: f64 = 24.0;
/// Seconds of the crossfade between the Earth and a toy.
const FADE: f64 = 0.7;

enum State {
    /// The Earth and its lights.
    Atlas,
    /// Flying to a light; its toy already made, waiting under the Earth.
    Diving { from: Camera, to: Camera, t0: f64, dur: f64 },
    /// The toy coming up through the Earth.
    Opening { t0: f64 },
    /// In the toy.
    Inside,
    /// The toy going down into the Earth, at its camera.
    Closing { t0: f64 },
    /// The Earth coming back up to the whole of it.
    Rising { from: Camera, to: Camera, t0: f64, dur: f64 },
    /// A toy that takes passages (`Toy::arrival`), carried in or out without a cut: one fall in scale
    /// from the whole Earth down into the toy's own eye straight above its place, then that eye swinging
    /// up into the toy's first view (or all of it backward, going out).
    Passing(Passage),
}

/// One passage's timeline (seconds from `t0`). In: the scale (metres a point at the middle of the glass)
/// falls from the whole Earth's to the toy's low look, eased in its logarithm over `fall`; the globe's
/// camera travels to the light and turns its top to the toy's bearing while the globe is still the
/// picture; once the scale passes the globe's deepest (`hand`), the toy's eye takes over under a short
/// crossfade, still falling; then `swing` seconds of the eye swinging into its own view. Out: the same,
/// read backward.
#[derive(Clone, Copy)]
struct Passage {
    inn: bool,
    t0: f64,
    fall: f64,
    swing: f64,
    /// The globe's camera at the start (in) or at the end (out), and over the light, turned (the hand-off).
    far: Camera,
    near: Camera,
    /// Metres a point: the whole Earth's (at `far`), the globe's deepest (`near`), the toy's low look.
    s_far: f64,
    s_hand: f64,
    s_low: f64,
}

/// The globe's deepest in a passage, and the toy's lowest look before its eye swings up (metres a point).
const PASS_ZOOM: f64 = 34.0;
const PASS_LOW: f64 = 3.0;
/// Seconds the swing begins before the fall has quite ended (one motion, not two).
const PASS_OVERLAP: f64 = 0.6;
/// Seconds: the toy's picture crossing the globe's, and the eye's swing.
const PASS_FADE: f64 = 0.6;
const PASS_SWING: f64 = 2.4;
/// The flock's (or the toy's subject's) half-width, metres: the light's likeness grows to it.
const PASS_SUBJECT: f64 = 70.0;
/// How far the Earth falls into night as it hands the picture to a toy's dusk.
const PASS_DUSK: f64 = 0.8;

impl Passage {
    /// Time since the start, the other way round going out (so both read as "in").
    fn clock(&self, t: f64) -> f64 {
        let s = t - self.t0;
        if self.inn {
            s
        } else {
            self.total() - s
        }
    }
    fn total(&self) -> f64 {
        self.fall + self.swing - PASS_OVERLAP
    }
    /// 0..1 along the fall, eased (fast in the middle of the logarithm, gentle at both ends).
    fn fall_at(&self, c: f64) -> f64 {
        // slow while the globe travels, quickest as it hands over, gentle at the end
        ease((c / self.fall).clamp(0.0, 1.0)).powi(3)
    }
    /// Metres a point now.
    fn scale(&self, c: f64) -> f64 {
        let u = self.fall_at(c);
        (self.s_far.ln() * (1.0 - u) + self.s_low.ln() * u).exp()
    }
    /// How far along the fall (its own clock) the globe reaches its deepest.
    fn hand_clock(&self) -> f64 {
        let want = (self.s_far / self.s_hand).ln() / (self.s_far / self.s_low).ln();
        // invert the ease by bisection
        let (mut a, mut b) = (0.0, self.fall);
        for _ in 0..40 {
            let m = 0.5 * (a + b);
            if self.fall_at(m) < want {
                a = m;
            } else {
                b = m;
            }
        }
        0.5 * (a + b)
    }
    /// The toy's picture over the globe's: 0 before the hand-off, 1 after the crossfade.
    fn toy_alpha(&self, c: f64) -> f64 {
        ease((c - self.hand_clock()) / PASS_FADE)
    }
    /// The swing (0 looking straight down .. 1 the toy's own view).
    fn swing_at(&self, c: f64) -> f64 {
        ((c - self.fall + PASS_OVERLAP) / self.swing).clamp(0.0, 1.0)
    }
    /// The globe's camera now: travelling (in) to the light with the turn, its zoom the scale's.
    fn camera(&self, c: f64, r_per_zoom: f64) -> Camera {
        let zoom = toy_kit::model::EARTH_RADIUS_M / (self.scale(c) * r_per_zoom);
        let h = self.hand_clock();
        let u = (c / h).clamp(0.0, 1.0);
        // the place and the turn arrive with the globe's deepest, the zoom's own fall carrying on
        let mut cam = fly(self.far, self.near, u);
        cam.zoom = zoom.max(0.6);
        cam
    }
}

#[derive(Serialize, Deserialize)]
struct Saved {
    camera: Camera,
}

/// Terra's sound: its own bells, and the open toy's sound beside them.
struct Mixer {
    bells: Box<dyn Synth>,
    room: Arc<Mutex<Option<Box<dyn Synth>>>>,
    /// The toy's level (f32 bits), eased toward on the audio thread.
    want: Arc<AtomicU32>,
    level: f32,
    buf: Vec<f32>,
}

impl Synth for Mixer {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        self.bells.render(out, channels, rate);
        if let Ok(mut room) = self.room.try_lock() {
            if let Some(s) = room.as_mut() {
                self.buf.resize(out.len(), 0.0);
                self.buf.fill(0.0);
                s.render(&mut self.buf, channels, rate);
                let want = f32::from_bits(self.want.load(Ordering::Relaxed));
                let step = (1.0 / (0.4 * rate)) as f32;
                for (i, o) in out.iter_mut().enumerate() {
                    if i % channels == 0 {
                        self.level += (want - self.level).clamp(-step, step);
                    }
                    *o += self.buf[i] * self.level;
                }
            } else {
                self.level = 0.0;
            }
        }
    }
}

/// The crossfade: a texture laid over the target at an alpha.
struct Veil {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    buf: wgpu::Buffer,
    target: Option<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup, [u32; 2])>,
}

const VEIL_WGSL: &str = r#"
struct U { alpha: f32, _a: f32, _b: f32, _c: f32 };
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> u: U;
struct O { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> O {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[i];
  var o: O;
  o.pos = vec4<f32>(p, 0.0, 1.0);
  o.uv = vec2<f32>(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
  return o;
}
@fragment fn fs_main(i: O) -> @location(0) vec4<f32> {
  let c = textureSample(tex, samp, i.uv);
  return vec4<f32>(c.rgb * u.alpha, u.alpha);
}
"#;

impl Veil {
    fn new(ctx: &Ctx) -> Veil {
        let dev = &ctx.gpu.device;
        let layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terra veil"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let shader = dev.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("terra veil"), source: wgpu::ShaderSource::Wgsl(VEIL_WGSL.into()) });
        let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("terra veil"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let pipeline = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terra veil"),
            layout: Some(&pl),
            vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.gpu.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        let sampler = dev.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let buf = dev.create_buffer(&wgpu::BufferDescriptor { label: Some("terra veil"), size: 16, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        Veil { pipeline, layout, sampler, buf, target: None }
    }

    /// The texture a toy draws into while it fades (made at the screen's size; dropped when unused).
    fn view(&mut self, ctx: &Ctx) -> &wgpu::TextureView {
        let size = [ctx.width.max(1), ctx.height.max(1)];
        if self.target.as_ref().map(|t| t.3) != Some(size) {
            let tex = ctx.gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("terra veil"),
                size: wgpu::Extent3d { width: size[0], height: size[1], depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: ctx.gpu.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            let bind = ctx.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("terra veil"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
                    wgpu::BindGroupEntry { binding: 2, resource: self.buf.as_entire_binding() },
                ],
            });
            self.target = Some((tex, view, bind, size));
        }
        &self.target.as_ref().unwrap().1
    }

    fn lay(&self, ctx: &Ctx, target: &wgpu::TextureView, alpha: f32) {
        let Some((_, _, bind, _)) = &self.target else { return };
        ctx.gpu.queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&[alpha, 0.0, 0.0, 0.0]));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terra veil"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, bind, &[]);
            rp.draw(0..3, 0..1);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }
}

pub struct Terra {
    globe: Globe,
    painter: Painter,
    assets: Arc<dyn Assets>,
    life: Life,
    bells: Bells,
    room_sound: Arc<Mutex<Option<Box<dyn Synth>>>>,
    room_level: Arc<AtomicU32>,
    room: Option<Box<dyn Room>>,
    /// The light whose toy is open (or opening, or closing).
    door: usize,
    state: State,
    veil: Veil,
    t: f64,
    last_touch: f64,
    /// Fingers in a globe toy, and a two-finger pinch's start (their spread, the toy's zoom): pinched out
    /// past the whole Earth, the toy goes back down into Terra's (the dive reversed).
    inside_fingers: std::collections::HashMap<u64, [f64; 2]>,
    pinch: Option<(f64, f64)>,
    /// Fingers down on the Earth: where and when, and whether they moved (a tap is still and short).
    downs: std::collections::HashMap<u64, ([f64; 2], f64, bool)>,
    /// A finger on the Earth mark.
    earth_finger: Option<u64>,
    reduce_motion: bool,
    /// A passage played by itself (`TERRA_PASSAGE=<toy id>`, set by the Simulator's check: sim.env):
    /// into that toy a few seconds after launch, and out again; with the first frame's time.
    auto: Option<(usize, f64, u8)>,
}

/// The month now (0 = January), from the clock (a fixed month without one).
fn this_month() -> u32 {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    // civil from days (Howard Hinnant's algorithm)
    let z = (secs / 86_400) as i64 + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (m - 1) as u32
}

fn ease(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// A camera on the way from `a` to `b` (`u` 0..1, eased): the centre along the great circle, the zoom
/// eased in logarithm and lifted mid-way when the way is long (so the Earth is seen whole as it turns).
fn fly(a: Camera, b: Camera, u: f64) -> Camera {
    let e = ease(u);
    let pa = LonLat::new(a.lon, a.lat).to_xyz();
    let pb = LonLat::new(b.lon, b.lat).to_xyz();
    let d = (pa[0] * pb[0] + pa[1] * pb[1] + pa[2] * pb[2]).clamp(-1.0, 1.0);
    let w = d.acos();
    let p = if w < 1e-6 {
        pb
    } else {
        let (sa, sb) = (((1.0 - e) * w).sin() / w.sin(), (e * w).sin() / w.sin());
        [pa[0] * sa + pb[0] * sb, pa[1] * sa + pb[1] * sb, pa[2] * sa + pb[2] * sb]
    };
    let ll = LonLat::from_xyz(p);
    let lift = 1.0 - 0.45 * (w / std::f64::consts::PI).min(1.0) * (std::f64::consts::PI * e).sin();
    let zoom = (a.zoom.ln() * (1.0 - e) + b.zoom.ln() * e).exp() * lift;
    let mut dr = (b.roll - a.roll).rem_euclid(360.0);
    if dr > 180.0 {
        dr -= 360.0;
    }
    Camera { lon: ll.lon, lat: ll.lat, zoom: zoom.max(0.6), roll: a.roll + dr * e }
}

impl Terra {
    fn points(ctx: &Ctx) -> [f64; 2] {
        ctx.points()
    }

    fn inside(&self) -> bool {
        matches!(self.state, State::Opening { .. } | State::Inside | State::Closing { .. })
    }

    fn strike(&self, degree: i32, amp: f32, pan: f32) {
        self.bells.strike(Note { freq: pentatonic(392.0, degree), amp, decay: 2.2, pan, bright: 0.9 });
    }

    /// A light was touched: it stirs and rings, its toy is made, and the camera flies to where it opens.
    fn enter(&mut self, ctx: &mut Ctx, i: usize, t: f64) {
        self.life.stir(i, t);
        let pan = ((self.life.spot(&self.globe, i).0[0] / Self::points(ctx)[0]) * 2.0 - 1.0) as f32;
        self.strike(self.life.doors[i].degree, 0.32, pan * 0.6);
        self.strike(self.life.doors[i].degree + 5, 0.12, pan * 0.6);
        ctx.haptic();
        let open = self.life.doors[i].open;
        let room = match open(ctx, &self.assets) {
            Ok(r) => r,
            Err(e) => {
                toy_kit::log(&format!("terra: {} did not open: {e:#}", self.life.doors[i].id));
                return;
            }
        };
        let at = self.life.doors[i].at;
        // a globe toy: down close over the light, where the toy will open (the whales' own framing of the
        // pair, which is already close); a scene of its own: as close over its place as the Earth goes
        let to = match room.camera() {
            // the toys whose first sight is a whole sea or the whole Earth (the whales' pair, the bell's
            // ring round the world, the swell's crossing an ocean): down to that, the toy's own view
            Some(own) if matches!(self.life.doors[i].id, "whales" | "bell" | "swell") => own,
            Some(own) => Camera::new(at[0], at[1], (own.zoom * 2.0).max(3.0)),
            None => Camera::new(at[0], at[1], 9.0),
        };
        let mut room = room;
        // a toy that takes passages: carried in without a cut, its own eye doing the last of the way
        if !self.reduce_motion {
            let rpz = self.globe.disc().r / self.globe.camera.zoom;
            let s_hand = toy_kit::model::EARTH_RADIUS_M / (rpz * PASS_ZOOM);
            if let Some(bearing) = room.arrival(Some(Arrival { k: 0.0, scale: s_hand })) {
                let from = self.globe.camera;
                let near = Camera { roll: self.roll_for(at, bearing), ..Camera::new(at[0], at[1], PASS_ZOOM) };
                let w = {
                    let (a, b) = (LonLat::new(from.lon, from.lat).to_xyz(), LonLat::new(at[0], at[1]).to_xyz());
                    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).acos()
                };
                *self.room_sound.lock().unwrap() = room.synth();
                room.settings(ctx.night, ctx.reduce_motion);
                self.room = Some(room);
                self.door = i;
                self.state = State::Passing(Passage {
                    inn: true,
                    t0: t,
                    fall: 3.0 + 0.5 * w,
                    swing: PASS_SWING,
                    far: from,
                    near,
                    s_far: toy_kit::model::EARTH_RADIUS_M / (rpz * from.zoom),
                    s_hand,
                    s_low: PASS_LOW,
                });
                return;
            }
        }
        self.room = Some(room);
        self.door = i;
        let from = self.globe.camera;
        let w = {
            let (a, b) = (LonLat::new(from.lon, from.lat).to_xyz(), LonLat::new(to.lon, to.lat).to_xyz());
            (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).acos()
        };
        let dur = if self.reduce_motion { 0.01 } else { 1.1 + 0.5 * w + 0.25 * (to.zoom / from.zoom).ln().abs() };
        self.state = State::Diving { from, to, t0: t, dur };
    }

    /// Back to the Earth: the toy goes down into it, at the toy's own camera.
    fn leave(&mut self, ctx: &mut Ctx, t: f64) {
        if !self.reduce_motion {
            let at = self.life.doors[self.door].at;
            let rpz = self.globe.disc().r / self.globe.camera.zoom;
            let s_hand = toy_kit::model::EARTH_RADIUS_M / (rpz * PASS_ZOOM);
            if let Some(bearing) = self.room.as_mut().and_then(|r| r.arrival(Some(Arrival { k: 1.0, scale: PASS_LOW }))) {
                let far = Camera::new(at[0], at[1], 1.4);
                let near = Camera { roll: self.roll_for(at, bearing), ..Camera::new(at[0], at[1], PASS_ZOOM) };
                self.strike(self.life.doors[self.door].degree - 5, 0.18, 0.0);
                ctx.haptic();
                self.inside_fingers.clear();
                self.pinch = None;
                self.state = State::Passing(Passage {
                    inn: false,
                    t0: t,
                    fall: 3.2,
                    swing: PASS_SWING * 0.8,
                    far,
                    near,
                    s_far: toy_kit::model::EARTH_RADIUS_M / (rpz * far.zoom),
                    s_hand,
                    s_low: PASS_LOW,
                });
                return;
            }
        }
        if let Some(c) = self.room.as_ref().and_then(|r| r.camera()) {
            self.globe.camera = c;
        }
        self.room_level.store(0f32.to_bits(), Ordering::Relaxed);
        self.strike(self.life.doors[self.door].degree - 5, 0.18, 0.0);
        ctx.haptic();
        self.inside_fingers.clear();
        self.pinch = None;
        self.state = State::Closing { t0: t };
    }

    /// The touch that opened a light goes on into its toy: the toy is touched where the light was (the
    /// bell rings from Chile, the swell drops its wave, a whale is followed, the mold fed, the fireflies
    /// stirred). The scenes, whose touch is a hold or a drawn line, are left to the reader.
    fn touch_through(&mut self, t: f64) {
        let d = &self.life.doors[self.door];
        if !matches!(d.id, "bell" | "swell" | "whales" | "mold" | "fireflies") {
            return;
        }
        let Some(r) = self.room.as_mut() else { return };
        // (Terra's Earth is at the toy's camera now, so it says where the light is for a toy that doesn't)
        let at = if d.id == "whales" { r.anchors().first().map(|a| a.1) } else { r.project(d.at).or_else(|| self.globe.point(d.at)) };
        if let Some(p) = at {
            let id = 9_000_000;
            r.pointer(PointerEvent { id, phase: Phase::Down, x: p[0], y: p[1], t });
            r.pointer(PointerEvent { id, phase: Phase::Up, x: p[0], y: p[1], t: t + 0.06 });
        }
    }

    /// Whether this finger ends a pinch that took a globe toy out past its whole Earth (as the kit's
    /// hand's own over-pinch would fly it Home): pinched to below the whole Earth (zoom 0.8), it goes Home to Terra).
    fn pinched_out(&mut self, e: &PointerEvent) -> bool {
        let spread = |f: &std::collections::HashMap<u64, [f64; 2]>| -> Option<f64> {
            let v: Vec<&[f64; 2]> = f.values().collect();
            (v.len() == 2).then(|| (v[0][0] - v[1][0]).hypot(v[0][1] - v[1][1]).max(1.0))
        };
        match e.phase {
            Phase::Down => {
                self.inside_fingers.insert(e.id, [e.x, e.y]);
                self.pinch = match (spread(&self.inside_fingers), self.room.as_ref().and_then(|r| r.camera())) {
                    (Some(d), Some(cam)) => Some((d, cam.zoom)),
                    _ => None,
                };
                false
            }
            Phase::Move => {
                if let Some(f) = self.inside_fingers.get_mut(&e.id) {
                    *f = [e.x, e.y];
                }
                false
            }
            Phase::Up | Phase::Cancel => {
                let out = match (self.pinch, spread(&self.inside_fingers)) {
                    (Some((d0, z0)), Some(d)) => e.phase == Phase::Up && z0 * d / d0 < 0.8,
                    _ => false,
                };
                self.inside_fingers.remove(&e.id);
                self.pinch = None;
                out
            }
        }
    }

    fn atlas_pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        self.last_touch = e.t;
        self.life.beckon = None;
        match e.phase {
            Phase::Down => {
                self.downs.insert(e.id, ([e.x, e.y], e.t, false));
            }
            Phase::Move => {
                if let Some(d) = self.downs.get_mut(&e.id) {
                    if (e.x - d.0[0]).hypot(e.y - d.0[1]) > 10.0 {
                        d.2 = true;
                    }
                }
            }
            Phase::Up | Phase::Cancel => {}
        }
        let many = self.downs.len() > 1;
        self.globe.pointer(e);
        if matches!(e.phase, Phase::Up | Phase::Cancel) {
            if let Some((at, t0, moved)) = self.downs.remove(&e.id) {
                if e.phase == Phase::Up && !moved && !many && e.t - t0 < 0.5 {
                    self.tap(ctx, at, e.t);
                }
            }
        }
    }

    fn tap(&mut self, ctx: &mut Ctx, at: [f64; 2], t: f64) {
        if let Some(i) = self.life.hit(&self.globe, at) {
            self.enter(ctx, i, t);
            return;
        }
        // something happens anywhere: a soft ring where the finger was, and the nearest light answers
        if let Some(ll) = self.globe.unproject(at[0], at[1]) {
            self.life.ring(ll, t);
            let pan = ((at[0] / Self::points(ctx)[0]) * 2.0 - 1.0) as f32;
            self.strike(-3, 0.16, pan * 0.5);
            if let Some(n) = self.life.nearest(&self.globe, at) {
                self.life.glint(n, t + 0.25);
            }
        }
    }

    fn draw_atlas(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView, fade: f64) {
        self.draw_atlas_but(ctx, target, fade, None);
    }

    /// The Earth with its lights, `skip` left out (a passage draws it itself, over the toy).
    fn draw_atlas_but(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView, fade: f64, skip: Option<usize>) {
        self.globe.upload(&ctx.gpu);
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terra"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.globe.draw(&mut rp);
        }
        self.life.paint(&mut self.painter, &self.globe, self.t, ctx.ppp, fade, self.reduce_motion, None, skip);
        self.painter.render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    /// A passage's frame: the globe while it is the picture, the toy's own eye over it from the hand-off,
    /// and the light's likeness over both, growing with the fall until the toy's own subject is as large
    /// and takes its place.
    fn draw_passage(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView, p: Passage) {
        let c = p.clock(self.t);
        let a = p.toy_alpha(c);
        if a < 1.0 {
            let others = 1.0 - (c / p.hand_clock()).clamp(0.0, 1.0);
            self.draw_atlas_but(ctx, target, others, Some(self.door));
        }
        if a > 0.0 {
            if let Some(r) = self.room.as_mut() {
                if a >= 1.0 {
                    r.draw(target);
                } else {
                    let view = self.veil.view(ctx);
                    r.draw(view);
                    self.veil.lay(ctx, target, a as f32);
                }
            }
        }
        // the likeness: the light's size until the subject's own (its half-width at this scale) is larger
        let light = Life::size(&self.globe);
        let own = PASS_SUBJECT / p.scale(c);
        let size = light.max(own * 0.9);
        let gone = ease((own / light - 0.55) / 0.6);
        let fade = 1.0 - gone;
        if fade > 0.0 {
            self.life.paint(&mut self.painter, &self.globe, self.t, ctx.ppp, fade, self.reduce_motion, Some((self.door, size, 1.0 - a)), None);
        }
        let mark = p.swing_at(c);
        if mark > 0.0 {
            self.draw_earth_mark(ctx, target, (mark * mark) as f32);
        } else {
            let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
            self.painter.render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
            ctx.gpu.queue.submit([enc.finish()]);
        }
    }

    /// The camera's roll that puts `bearing` (degrees clockwise from north) at the top of the glass over `at`.
    fn roll_for(&self, at: [f64; 2], bearing: f64) -> f64 {
        let b = bearing.to_radians();
        let d = 0.05;
        let ahead = LonLat::new(at[0] + d * b.sin() / at[1].to_radians().cos().max(0.05), at[1] + d * b.cos());
        let mut best = (f64::MAX, 0.0);
        for k in 0..72 {
            let roll = k as f64 * 5.0;
            let cam = Camera { roll, ..Camera::new(at[0], at[1], 4.0) };
            let g = self.globe.disc();
            let disc = toy_kit::Disc { r: g.r / self.globe.camera.zoom * cam.zoom, ..g };
            let (q, _) = disc.project(&cam, ahead);
            // straight up from the middle: x on the centre, y above it
            let ang = (q[0] - disc.cx).atan2(disc.cy - q[1]).abs();
            if ang < best.0 {
                best = (ang, roll);
            }
        }
        // refine to a degree
        let base = best.1;
        for k in -5..=5 {
            let roll = base + k as f64;
            let cam = Camera { roll, ..Camera::new(at[0], at[1], 4.0) };
            let g = self.globe.disc();
            let disc = toy_kit::Disc { r: g.r / self.globe.camera.zoom * cam.zoom, ..g };
            let (q, _) = disc.project(&cam, ahead);
            let ang = (q[0] - disc.cx).atan2(disc.cy - q[1]).abs();
            if ang < best.0 {
                best = (ang, roll);
            }
        }
        let r = best.1.rem_euclid(360.0);
        if r > 180.0 { r - 360.0 } else { r }
    }

    /// The Earth mark (a small globe: its rim, a meridian, the equator), on the info marks' dark ground.
    fn draw_earth_mark(&mut self, ctx: &Ctx, target: &wgpu::TextureView, alpha: f32) {
        let k = ctx.ppp as f32;
        let s = earth_spot(Self::points(ctx));
        let c = [s[0] as f32 * k, s[1] as f32 * k];
        let lit = 0.62 * alpha;
        let p = &mut self.painter;
        p.disc(c, 9.5 * k, 3.0 * k, [0.0, 0.0, 0.0, 0.42 * alpha]);
        p.ring(c, 7.5 * k, 1.1 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        p.line([c[0] - 6.9 * k, c[1]], [c[0] + 6.9 * k, c[1]], 0.5 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        // the meridian: an ellipse's two halves, drawn as short strokes
        let n = 14;
        let mut prev: Option<[f32; 2]> = None;
        for i in 0..=n {
            let a = std::f32::consts::PI * (i as f32 / n as f32 - 0.5);
            let q = [c[0] + a.cos() * 3.4 * k, c[1] + a.sin() * 7.2 * k];
            if let Some(pp) = prev {
                p.line(pp, q, 0.5 * k, 0.7, [1.0, 1.0, 1.0, lit]);
            }
            prev = Some(q);
        }
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.painter.render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
        ctx.gpu.queue.submit([enc.finish()]);
    }
}

impl Toy for Terra {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        // the data shared with every toy Terra opens
        let assets: Arc<dyn Assets> = Arc::from(std::mem::replace(&mut ctx.assets, Box::new(MemoryAssets(Default::default()))));
        ctx.assets = Box::new(Shared(assets.clone()));
        let month = this_month();
        let mut globe = Globe::new(&ctx.gpu, assets.as_ref(), ctx.width, ctx.height, ctx.ppp, month)?;
        // over the Atlantic's west: the Americas' lights and Europe's in view, the Pacific's coming round
        globe.camera = Camera::new(-66.0, 20.0, 1.4);
        globe.set_home();
        let mut life = Life::new(assets.as_ref());
        let [w, h] = ctx.points();
        life.beckon = life.nearest(&globe, [w / 2.0, h / 2.0]);
        Ok(Terra {
            globe,
            painter: Painter::new(&ctx.gpu),
            assets,
            life,
            bells: Bells::new(),
            room_sound: Arc::new(Mutex::new(None)),
            room_level: Arc::new(AtomicU32::new(0f32.to_bits())),
            room: None,
            door: 0,
            state: State::Atlas,
            veil: Veil::new(ctx),
            t: 0.0,
            last_touch: -100.0,
            downs: Default::default(),
            earth_finger: None,
            inside_fingers: Default::default(),
            pinch: None,
            reduce_motion: ctx.reduce_motion,
            auto: std::env::var("TERRA_PASSAGE").ok().and_then(|id| life_index(&id)).map(|i| (i, f64::NAN, 0)),
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
        if let Some(r) = self.room.as_mut() {
            r.resize(ctx.width, ctx.height, ctx.ppp);
        }
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        if self.inside() {
            // the Earth mark is Terra's; everything else is the toy's
            let s = earth_spot(Self::points(ctx));
            if e.phase == Phase::Down && self.earth_finger.is_none() && (e.x - s[0]).hypot(e.y - s[1]) < EARTH_REACH && matches!(self.state, State::Inside | State::Opening { .. }) {
                self.earth_finger = Some(e.id);
                return;
            }
            if self.earth_finger == Some(e.id) {
                if matches!(e.phase, Phase::Up | Phase::Cancel) {
                    self.earth_finger = None;
                    if e.phase == Phase::Up && (e.x - s[0]).hypot(e.y - s[1]) < EARTH_REACH * 1.5 && matches!(self.state, State::Inside | State::Opening { .. }) {
                        self.leave(ctx, e.t);
                    }
                }
                return;
            }
            if matches!(self.state, State::Inside) {
                let out = self.pinched_out(&e);
                if let Some(r) = self.room.as_mut() {
                    r.pointer(e);
                }
                if out {
                    self.leave(ctx, e.t);
                }
            }
            return;
        }
        if matches!(self.state, State::Atlas) {
            self.atlas_pointer(ctx, e);
        } else {
            self.last_touch = e.t;
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        if let Some((i, t0, step)) = self.auto {
            let t0 = if t0.is_nan() { t } else { t0 };
            self.auto = Some((i, t0, step));
            if step == 0 && t - t0 > 4.0 && matches!(self.state, State::Atlas) {
                self.enter(ctx, i, t);
                self.auto = Some((i, t0, 1));
            } else if step == 1 && t - t0 > 45.0 && matches!(self.state, State::Inside) {
                self.leave(ctx, t);
                self.auto = Some((i, t0, 2));
            }
        }
        let held = self.globe.camera;
        self.globe.update(&ctx.gpu, self.assets.as_ref(), t, dt);
        if !matches!(self.state, State::Atlas) {
            self.globe.camera = held;
        }
        self.life.update(t, dt);
        match self.state {
            State::Atlas => {
                // left alone, the Earth turns slowly under the reader (west to east, as it does)
                if !self.reduce_motion && !self.globe.hand.busy() && self.downs.is_empty() {
                    let k = ease((t - self.last_touch - 4.0) / 3.0);
                    self.globe.camera.lon -= 1.6 * k * dt;
                }
            }
            State::Diving { from, to, t0, dur } => {
                let u = (t - t0) / dur;
                self.globe.camera = fly(from, to, u.min(1.0));
                if self.life.doors[self.door].night {
                    self.globe.night = ease((u - 0.55) / 0.45) as f32;
                }
                if u >= 1.0 {
                    if let Some(r) = self.room.as_mut() {
                        // the toy opens where the dive arrived: one picture becoming another
                        r.set_camera(Camera { roll: self.globe.camera.roll, ..to });
                        r.settings(ctx.night, ctx.reduce_motion);
                        *self.room_sound.lock().unwrap() = r.synth();
                        r.update(t);
                    }
                    self.room_level.store(1f32.to_bits(), Ordering::Relaxed);
                    self.touch_through(t);
                    self.state = State::Opening { t0: t };
                }
            }
            State::Opening { t0 } => {
                if let Some(r) = self.room.as_mut() {
                    r.update(t);
                }
                if t - t0 >= FADE {
                    self.state = State::Inside;
                    // the Earth's lights sleep while a toy is open
                    self.veil.target = None;
                }
            }
            State::Inside => {
                if let Some(r) = self.room.as_mut() {
                    r.update(t);
                }
            }
            State::Closing { t0 } => {
                // the toy held still as it goes down (its own hand would fly it Home after a pinch out),
                // so its picture and the Earth's under it stay one
                if t - t0 >= FADE {
                    self.room = None;
                    *self.room_sound.lock().unwrap() = None;
                    self.veil.target = None;
                    let from = self.globe.camera;
                    let to = Camera { zoom: 1.4, roll: 0.0, ..from };
                    let dur = if self.reduce_motion { 0.01 } else { 1.2 + 0.3 * (from.zoom / to.zoom).ln().abs() };
                    self.state = State::Rising { from, to, t0: t, dur };
                }
            }
            State::Passing(p) => {
                let c = p.clock(t);
                let rpz = self.globe.disc().r / self.globe.camera.zoom;
                self.globe.camera = p.camera(c, rpz);
                // the dusk comes down with the globe's last fall (the toy's ground is dim)
                let h = p.hand_clock();
                self.globe.night = (PASS_DUSK * ease((c - 0.4 * h) / (0.6 * h + PASS_FADE))) as f32;
                let a = p.toy_alpha(c);
                self.room_level.store((a as f32).to_bits(), Ordering::Relaxed);
                if let Some(r) = self.room.as_mut() {
                    r.arrival(Some(Arrival { k: ease(p.swing_at(c)), scale: p.scale(c) }));
                    r.update(t);
                }
                if p.inn && c >= p.total() {
                    if let Some(r) = self.room.as_mut() {
                        r.arrival(None);
                    }
                    self.veil.target = None;
                    self.state = State::Inside;
                } else if !p.inn && c <= 0.0 {
                    self.room = None;
                    *self.room_sound.lock().unwrap() = None;
                    self.veil.target = None;
                    self.globe.night = 0.0;
                    self.globe.camera = Camera { roll: 0.0, ..self.globe.camera };
                    self.last_touch = t;
                    self.state = State::Atlas;
                }
            }
            State::Rising { from, to, t0, dur } => {
                let u = (t - t0) / dur;
                self.globe.camera = fly(from, to, u.min(1.0));
                if self.globe.night > 0.0 {
                    self.globe.night = (1.0 - ease(u * 2.2)) as f32;
                }
                if u >= 1.0 {
                    self.globe.night = 0.0;
                    self.last_touch = t;
                    self.state = State::Atlas;
                }
            }
        }
        if let Some(r) = self.room.as_mut() {
            if r.take_haptic() {
                ctx.haptic();
            }
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        match self.state {
            State::Atlas | State::Rising { .. } => self.draw_atlas(ctx, target, 1.0),
            State::Diving { t0, dur, .. } => {
                // the lights give way to the one being entered
                let u = ((self.t - t0) / dur).clamp(0.0, 1.0);
                self.draw_atlas(ctx, target, 1.0 - 0.6 * u);
            }
            State::Opening { t0 } | State::Closing { t0 } => {
                let u = ease((self.t - t0) / FADE);
                let a = if matches!(self.state, State::Opening { .. }) { u } else { 1.0 - u };
                self.draw_atlas(ctx, target, 0.4);
                if let Some(r) = self.room.as_mut() {
                    let view = self.veil.view(ctx);
                    r.draw(view);
                    self.veil.lay(ctx, target, a as f32);
                }
                self.draw_earth_mark(ctx, target, a as f32);
            }
            State::Inside => {
                if let Some(r) = self.room.as_mut() {
                    r.draw(target);
                }
                self.draw_earth_mark(ctx, target, 1.0);
            }
            State::Passing(p) => self.draw_passage(ctx, target, p),
        }
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(Box::new(Mixer { bells: self.bells.synth(), room: self.room_sound.clone(), want: self.room_level.clone(), level: 0.0, buf: Vec::new() }))
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
        self.globe.hand.reduce_motion = ctx.reduce_motion;
        if let Some(r) = self.room.as_mut() {
            r.settings(ctx.night, ctx.reduce_motion);
        }
    }

    fn save(&self) -> Option<String> {
        // the Earth as it was left (a toy open is closed on the way out: Terra reopens on the Earth)
        let camera = if matches!(self.state, State::Atlas) { self.globe.camera } else { self.globe.hand.home.unwrap_or(self.globe.camera) };
        serde_json::to_string(&Saved { camera: Camera { zoom: camera.zoom.min(2.5), ..camera } }).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            if matches!(self.state, State::Atlas) {
                self.globe.camera = s.camera;
            }
        }
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        if self.inside() {
            let mut a = self.room.as_ref().map(|r| r.anchors()).unwrap_or_default();
            let pts = [self.globe_points()[0], self.globe_points()[1]];
            a.push(("earth".into(), earth_spot(pts)));
            return a;
        }
        self.visible_doors().into_iter().map(|(i, s)| (self.life.doors[i].id.to_string(), s)).collect()
    }

    fn project(&self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        if self.inside() {
            return self.room.as_ref().and_then(|r| r.project(lonlat));
        }
        self.globe.point(lonlat)
    }

    fn demo(&self, ctx: &Ctx) -> Option<Demo> {
        if self.inside() {
            return self.room.as_ref().and_then(|r| r.demo());
        }
        // a fingertip touches the light nearest the middle, and in it goes
        let [w, h] = ctx.points();
        let at = self.life.nearest(&self.globe, [w / 2.0, h / 2.0]).map(|i| self.life.spot(&self.globe, i).0).unwrap_or([w / 2.0, h / 2.0]);
        Some(Demo::new("Each light is a toy. Touch one to go in.", vec![Stroke::tap(0.6, at)]))
    }

    fn buttons(&self, ctx: &Ctx) -> Vec<(String, [f64; 2])> {
        if self.inside() {
            return vec![("Back to the Earth".into(), earth_spot(ctx.points()))];
        }
        if !matches!(self.state, State::Atlas) {
            return Vec::new();
        }
        self.visible_doors().into_iter().map(|(i, p)| (self.life.doors[i].name.to_string(), p)).collect()
    }

    fn press(&mut self, ctx: &mut Ctx, i: usize, t: f64) {
        if self.inside() {
            if matches!(self.state, State::Inside | State::Opening { .. }) {
                self.leave(ctx, t);
            }
            return;
        }
        if matches!(self.state, State::Atlas) {
            if let Some(&(d, _)) = self.visible_doors().get(i) {
                self.enter(ctx, d, t);
            }
        }
    }

    fn origins(&self) -> Option<Rules> {
        // the open toy's, while it is open and settled (none on the Earth)
        if matches!(self.state, State::Inside) {
            return self.room.as_ref().and_then(|r| r.origins());
        }
        None
    }

    fn card_at_top(&self, _ctx: &Ctx) -> bool {
        self.inside() && self.room.as_ref().is_some_and(|r| r.card_at_top())
    }

    fn rules(&self) -> Option<Rules> {
        if self.inside() {
            return self.room.as_ref().and_then(|r| r.rules());
        }
        Some(Rules::new(
            &[
                "Every light is a toy, at the place where it is real.",
                "Touch a light to fall into its toy. Pinch out past the whole Earth, or touch the Earth mark bottom left, to come back.",
            ],
            "Real: the Earth this month as NASA's satellites saw it, and each toy's place. Drawn: the lights.",
        ))
    }
}

impl Terra {
    /// The lights facing the reader now, with where they are (points).
    fn visible_doors(&self) -> Vec<(usize, [f64; 2])> {
        (0..self.life.doors.len())
            .filter_map(|i| {
                let (s, f) = self.life.spot(&self.globe, i);
                (f > 0.35).then_some((i, s))
            })
            .collect()
    }

    fn globe_points(&self) -> [f64; 2] {
        let d = self.globe.view();
        [d.width as f64 / d.ppp, d.height as f64 / d.ppp]
    }
}

/// The light whose toy has this id.
fn life_index(id: &str) -> Option<usize> {
    doors::doors().iter().position(|d| d.id == id)
}

toy_kit::ios_shell!(host Terra);
