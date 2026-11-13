//! Terra Incognita: the toy that holds the others (painting-company row 217). The whole Earth, turning
//! slowly, and on it a light for each toy, alive with a small likeness of its rule at the place where it
//! is real (`doors`). Touch a light: it stirs and rings, the camera flies there (the Earth falling into
//! night for a night toy), and the toy itself opens where the camera arrived, already going (`rooms`).
//! The Earth mark, bottom left, brings you back to where you were, and the Earth comes up again.
//!
//! One device, one hand, one ground: every toy's globe is the kit's, so the dive is one picture becoming
//! another. Only one toy is open at a time; closing it frees it.

pub mod doors;
pub mod likeness;
use likeness::SCENES;
pub mod rooms;
mod words;

use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use toy_kit::info::{Demo, Rules};
use toy_kit::paint::Painter;
use toy_kit::sound::{pentatonic, Bells, Note};
use toy_kit::{Assets, Camera, Ctx, Globe, LonLat, MemoryAssets, Phase, PointerEvent, Synth, Toy};

use doors::Life;
use rooms::{Room, Shared};

/// The Earth mark: bottom left, as the info and rules marks are bottom right (toy-kit info.rs).
fn earth_spot(points: [f64; 2]) -> [f64; 2] {
    [26.0, points[1] - 34.0]
}
const EARTH_REACH: f64 = 24.0;
/// The opening's zoom: on a tall phone the Earth a little wider than the screen, so its lights are large
/// (1.4, its height well inside); on a squarer screen (the iPad) the Earth just the screen's width, so no
/// light opens cut by the frame or hidden past it (painting-company row 246: at 1.4 the 13-inch iPad's
/// Earth ran off all four sides). The kit's disc radius is (short side / 2 - 14) x zoom.
pub fn opening_zoom(points: [f64; 2]) -> f64 {
    let (w, h) = (points[0].min(points[1]), points[0].max(points[1]));
    if h / w > 1.8 {
        return 1.4;
    }
    ((w * 0.53) / (w / 2.0 - 14.0).max(40.0)).min(1.4)
}

/// Seconds of the crossfade between the Earth and a toy.
const FADE: f64 = 0.7;
/// How close the Earth comes over a scene before it is the glass (a dive's end): as close as it goes, so the
/// window onto the toy grows at about the ground's own pace and the Earth keeps falling in step with it, its
/// rivers coming up round the window (painting-company row 258: at 9 the window outran the ground, "an iris,
/// not a descent").
const SCENE_ZOOM: f64 = 40.0;
/// Pinched in this close with a scene's light on screen, the light becomes a window onto the toy itself,
/// growing with the zoom until at `SCENE_ZOOM` it is the glass: the fingers carry the descent into the
/// toy (painting-company row 258). Its window is the light's own size here.
const PEEK_ZOOM: f64 = 2.4;
/// Let go with the window this far grown, the descent carries on into the toy.
const PEEK_COMMIT: f64 = 0.5;
/// Pinched out inside a scene and let go with the Earth brought back below this part of the scene's zoom (the
/// fingers' spread down to about half), the Earth rises; else the scene grows back to the glass.
const SQUEEZE_LEAVE: f64 = 0.55;

/// Two fingers pinching a scene back into its light: their spread at the start, and how grown the window
/// is now (1 the glass); `settling` when let go not far enough out (`SQUEEZE_LEAVE`), easing back to the glass.
#[derive(Clone, Copy)]
struct Squeeze {
    a: u64,
    b: u64,
    d0: f64,
    g: f64,
    near: f64,
    settling: bool,
    /// Whether the two fingers are Terra's: only once they have closed in (a pinch out), so two fingers on
    /// a toy that takes several (Meander's pushes, row 253) stay the toy's.
    decided: bool,
}
/// Where in a scene's own frame the window onto it centres while it is small (fractions of the glass):
/// what the toy is, where it is in its first frame (Frost's ice grows from the window's foot).
/// The third number is how much of the frame's width the small window holds (the starlings': the thick of
/// the flock alone, a dark knot in the dusk as the light shows it, widening to the reeds and the sky).
fn window_at(id: &str) -> [f64; 3] {
    match id {
        "frost" => [0.5, 0.8, 1.0],
        "murmuration" => [0.55, 0.36, 0.45],
        _ => [0.5, 0.5, 1.0],
    }
}

enum State {
    /// The Earth and its lights.
    Atlas,
    /// Flying to a light; its toy already made, waiting under the Earth. `near`: the zoom at which a
    /// scene's window is its light's size (where the dive began, or where a pinch made it a window).
    Diving {
        from: Camera,
        to: Camera,
        t0: f64,
        dur: f64,
        near: f64,
    },
    /// The toy coming up through the Earth.
    Opening { t0: f64 },
    /// In the toy.
    Inside,
    /// The toy going down into the Earth, at its camera.
    Closing { t0: f64 },
    /// The Earth coming back up to the whole of it. `far`: the zoom at which a scene's window is the glass.
    Rising {
        from: Camera,
        to: Camera,
        t0: f64,
        dur: f64,
        far: f64,
    },
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
struct U { alpha: f32, rag: f32, _b: f32, _c: f32, iris: vec4<f32>, map: vec4<f32>, aff: vec4<f32>, at: vec4<f32>, edge: vec4<f32> };
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
fn h2(p: vec2<f32>) -> f32 {
  var q = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
  q = q + dot(q, q.yzx + 33.33);
  return fract((q.x + q.y) * q.z);
}
fn vn(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = p - i; let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(h2(i), h2(i + vec2<f32>(1.0, 0.0)), w.x), mix(h2(i + vec2<f32>(0.0, 1.0)), h2(i + vec2<f32>(1.0, 1.0)), w.x), w.y);
}
@fragment fn fs_main(i: O) -> @location(0) vec4<f32> {
  var c = textureSample(tex, samp, i.uv);
  var a = u.alpha;
  if (u.iris.z < 0.0) {
    // lying on the ground (a toy seen from above): the screen's pixel into the frame's by the place's turn and
    // the scale between the Earth's ground and the toy's, about the place's point (at.xy) and the frame's
    // middle (at.zw); its edge dissolving into the same ground over a few percent, unevenly
    let size = vec2<f32>(textureDimensions(tex));
    let d = i.pos.xy - u.at.xy;
    let q = u.at.zw + vec2<f32>(u.aff.x * d.x + u.aff.y * d.y, u.aff.z * d.x + u.aff.w * d.y);
    c = textureSampleLevel(tex, samp, q / size, 0.0);
    let e = min(min(q.x, size.x - q.x), min(q.y, size.y - q.y));
    let n = vn(q / size * 9.0) * 0.6 + vn(q / size * 23.0 + 5.0) * 0.4;
    let soft = max(u.edge.x, 1.0);
    a = a * smoothstep(0.0, soft, e - soft * 0.6 * (n - 0.5) * u.edge.y);
  } else if (u.iris.z > 0.0) {
    // a window (iris: its centre, radius, softness, pixels): the toy's whole frame scaled by map.z about
    // the frame's point map.xy, which lies at the window's centre; sampled at five points when small
    let size = vec2<f32>(textureDimensions(tex));
    let q = u.map.xy + (i.pos.xy - u.iris.xy) / u.map.z;
    let d = 0.35 / u.map.z;
    c = (textureSampleLevel(tex, samp, q / size, 0.0) * 2.0
      + textureSampleLevel(tex, samp, (q + vec2<f32>(d, d)) / size, 0.0)
      + textureSampleLevel(tex, samp, (q + vec2<f32>(-d, d)) / size, 0.0)
      + textureSampleLevel(tex, samp, (q + vec2<f32>(d, -d)) / size, 0.0)
      + textureSampleLevel(tex, samp, (q + vec2<f32>(-d, -d)) / size, 0.0)) / 6.0;
    // its edge an uneven dissolve into the ground, not a drawn circle: wide, broken by a noise that grows
    // with the window (so it keeps its shape as it opens) and shrinks to nothing as it becomes the glass
    let rel = (i.pos.xy - u.iris.xy) / u.iris.z;
    let n = vn(rel * 3.0) * 0.6 + vn(rel * 7.0 + 11.0) * 0.4;
    let ragged = u.iris.z * (0.10 + 0.16 * n) * u.rag;
    a = a * (1.0 - smoothstep(u.iris.z - u.iris.w - ragged, u.iris.z + u.iris.w * 0.3 - ragged * 0.4, distance(i.pos.xy, u.iris.xy)));
    // nothing from beyond the frame's edge (the globe shows there until the light has come to the middle)
    let e = min(min(q.x, size.x - q.x), min(q.y, size.y - q.y));
    a = a * smoothstep(-1.0, 6.0, e * u.map.z);
  }
  return vec4<f32>(c.rgb * a, a);
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
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = dev.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terra veil"),
            source: wgpu::ShaderSource::Wgsl(VEIL_WGSL.into()),
        });
        let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terra veil"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terra veil"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
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
            multiview_mask: None,
            cache: None,
        });
        let sampler = dev.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terra veil"),
            size: 96,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Veil {
            pipeline,
            layout,
            sampler,
            buf,
            target: None,
        }
    }

    /// The texture a toy draws into while it fades (made at the screen's size; dropped when unused).
    fn view(&mut self, ctx: &Ctx) -> &wgpu::TextureView {
        let size = [ctx.width.max(1), ctx.height.max(1)];
        if self.target.as_ref().map(|t| t.3) != Some(size) {
            let tex = ctx.gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("terra veil"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: ctx.gpu.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            let bind = ctx
                .gpu
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("terra veil"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.buf.as_entire_binding(),
                        },
                    ],
                });
            self.target = Some((tex, view, bind, size));
        }
        &self.target.as_ref().unwrap().1
    }

    fn lay(&self, ctx: &Ctx, target: &wgpu::TextureView, alpha: f32) {
        self.lay_window(ctx, target, alpha, [0.0; 4], [0.0, 0.0, 1.0]);
    }

    /// Laid as a window: `iris` its centre, radius and softness (pixels; radius 0: the whole glass), and
    /// `map` the frame's point at its centre and the scale it is shown at.
    fn lay_window(
        &self,
        ctx: &Ctx,
        target: &wgpu::TextureView,
        alpha: f32,
        iris: [f32; 4],
        map: [f32; 3],
    ) {
        self.lay_window_ragged(ctx, target, alpha, iris, map, 0.0);
    }

    /// Laid on the ground: `aff` the screen-to-frame matrix (rows), `at` the place's pixel and the frame's
    /// middle, `edge` the dissolve's width (frame pixels) and how uneven (0..1).
    fn lay_ground(
        &self,
        ctx: &Ctx,
        target: &wgpu::TextureView,
        alpha: f32,
        aff: [f32; 4],
        at: [f32; 4],
        edge: [f32; 2],
    ) {
        if self.target.is_none() {
            return;
        }
        let mut u = [0f32; 24];
        u[0] = alpha;
        u[6] = -1.0;
        u[12..16].copy_from_slice(&aff);
        u[16..20].copy_from_slice(&at);
        u[20] = edge[0];
        u[21] = edge[1];
        ctx.gpu
            .queue
            .write_buffer(&self.buf, 0, bytemuck::cast_slice(&u));
        self.pass(ctx, target);
    }

    /// A window whose edge is `rag` (0..1) an uneven dissolve rather than a soft circle.
    fn lay_window_ragged(
        &self,
        ctx: &Ctx,
        target: &wgpu::TextureView,
        alpha: f32,
        iris: [f32; 4],
        map: [f32; 3],
        rag: f32,
    ) {
        if self.target.is_none() {
            return;
        }
        ctx.gpu.queue.write_buffer(
            &self.buf,
            0,
            bytemuck::cast_slice(&[
                alpha, rag, 0.0, 0.0, iris[0], iris[1], iris[2], iris[3], map[0], map[1], map[2],
                0.0,
            ]),
        );
        self.pass(ctx, target);
    }

    fn pass(&self, ctx: &Ctx, target: &wgpu::TextureView) {
        let Some((_, _, bind, _)) = &self.target else {
            return;
        };
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terra veil"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
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
    seen: std::collections::HashSet<usize>, // toys opened since launch: a first opening shows its demo
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
    /// The open toy is a scene of its own (no globe): it opens out of its light through a widening disc
    /// as the Earth keeps falling toward it, rather than crossfading.
    scene: bool,
    /// A passage played by itself (`TERRA_PASSAGE=<toy id>`, set by the Simulator's check: sim.env):
    /// into that toy a few seconds after launch, and out again; with the first frame's time.
    auto: Option<(usize, f64, u8)>,
    /// The idle turn's pace now, degrees a second (eased toward what the lights in view ask).
    drift: f64,
    /// On the Earth, pinched close to a scene's light: that toy is made and shown live in its light, a
    /// window growing with the zoom (`PEEK_ZOOM`).
    peek: Option<usize>,
    /// Inside a scene, two fingers pinching it back into its light.
    squeeze: Option<Squeeze>,
    /// Inside a scene, a first finger not yet given to it: held a tenth of a second (or until it moves or
    /// lifts) in case a second comes and the two are a pinch out, which the scene should never feel.
    held: Vec<PointerEvent>,
}

/// The month now (0 = January), from the clock (a fixed month without one).
fn this_month() -> u32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // civil from days (Howard Hinnant's algorithm)
    let z = (secs / 86_400) as i64 + 719_468;
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (m - 1) as u32
}

/// Where the Earth comes back to from a toy: the opening's own view, so the lights are where the reader
/// first saw them, turned toward the toy just left only as far as keeps its light in view (55 degrees
/// from the middle at most); painting-company row 246: newcomers came home to a turned Earth, the
/// lights they had seen gone off it.
fn homecoming(home: Camera, at: [f64; 2]) -> Camera {
    let reach = 55f64.to_radians();
    let (a, b) = (
        LonLat::new(home.lon, home.lat).to_xyz(),
        LonLat::new(at[0], at[1]).to_xyz(),
    );
    let w = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
        .clamp(-1.0, 1.0)
        .acos();
    if w <= reach || w > std::f64::consts::PI - 1e-3 {
        return Camera { roll: 0.0, ..home };
    }
    let u = (w - reach) / w;
    let (sa, sb) = (((1.0 - u) * w).sin() / w.sin(), (u * w).sin() / w.sin());
    let ll = LonLat::from_xyz([
        a[0] * sa + b[0] * sb,
        a[1] * sa + b[1] * sb,
        a[2] * sa + b[2] * sb,
    ]);
    Camera {
        lon: ll.lon,
        lat: ll.lat,
        zoom: home.zoom,
        roll: 0.0,
    }
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
        [
            pa[0] * sa + pb[0] * sb,
            pa[1] * sa + pb[1] * sb,
            pa[2] * sa + pb[2] * sb,
        ]
    };
    let ll = LonLat::from_xyz(p);
    let lift = 1.0 - 0.45 * (w / std::f64::consts::PI).min(1.0) * (std::f64::consts::PI * e).sin();
    let zoom = (a.zoom.ln() * (1.0 - e) + b.zoom.ln() * e).exp() * lift;
    let mut dr = (b.roll - a.roll).rem_euclid(360.0);
    if dr > 180.0 {
        dr -= 360.0;
    }
    Camera {
        lon: ll.lon,
        lat: ll.lat,
        zoom: zoom.max(0.6),
        roll: a.roll + dr * e,
    }
}

/// A scene's dive: the Earth turns the light to the middle first (its window there, small), then falls
/// toward it (the window growing about the middle), the two overlapping a little.
fn fly_turn_then_fall(a: Camera, b: Camera, u: f64) -> Camera {
    let turn = fly(a, Camera { zoom: a.zoom, ..b }, (u / 0.5).min(1.0));
    let fall = ((u - 0.35) / 0.65).clamp(0.0, 1.0);
    let fall = fall * fall * (3.0 - 2.0 * fall);
    Camera {
        zoom: (a.zoom.ln() * (1.0 - fall) + b.zoom.ln() * fall).exp(),
        ..turn
    }
}

impl Terra {
    fn points(ctx: &Ctx) -> [f64; 2] {
        ctx.points()
    }

    /// Where the Earth rises to after a toy (from the camera over its place): one place for it, the
    /// opening's own minute (row 246) may choose it.
    fn rise_to(&self, _from: Camera) -> Camera {
        // the opening's own view, turned toward the toy just left only as far as keeps its light in view (row 246)
        let home = self
            .globe
            .hand
            .home
            .unwrap_or(Camera::new(-66.0, 20.0, 1.4));
        homecoming(home, self.life.doors[self.door].at)
    }

    fn inside(&self) -> bool {
        matches!(
            self.state,
            State::Opening { .. } | State::Inside | State::Closing { .. }
        )
    }

    fn strike(&self, degree: i32, amp: f32, pan: f32) {
        self.bells.strike(Note {
            freq: pentatonic(392.0, degree),
            amp,
            decay: 2.2,
            pan,
            bright: 0.9,
        });
    }

    /// A light was touched: it stirs and rings, its toy is made, and the camera flies to where it opens.
    fn enter(&mut self, ctx: &mut Ctx, i: usize, t: f64) {
        self.life.stir(i, t);
        let pan = ((self.life.spot(&self.globe, i).0[0] / Self::points(ctx)[0]) * 2.0 - 1.0) as f32;
        self.strike(self.life.doors[i].degree, 0.32, pan * 0.6);
        self.strike(self.life.doors[i].degree + 5, 0.12, pan * 0.6);
        ctx.haptic();
        // a scene already open in its light (pinched close): the same toy goes on, its window growing on
        let reuse = self.peek == Some(i) && self.room.is_some();
        if !reuse && self.peek.is_some() {
            self.drop_peek();
        }
        self.peek = None;
        let open = self.life.doors[i].open;
        let room = if reuse {
            self.room.take().unwrap()
        } else {
            match open(ctx, &self.assets) {
                Ok(r) => r,
                Err(e) => {
                    toy_kit::log(&format!(
                        "terra: {} did not open: {e:#}",
                        self.life.doors[i].id
                    ));
                    return;
                }
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
            // a toy seen from above: down onto its place at its true scale and turn, where its first frame is
            // what the Earth shows (painting-company row 258)
            None => match room.seen_from_above() {
                Some(a) => Camera {
                    lon: a.centre[0],
                    lat: a.centre[1],
                    zoom: self.fill_zoom(&a),
                    roll: a.turn,
                },
                None => Camera::new(at[0], at[1], window_zooms(self.life.doors[i].id).1),
            },
        };
        self.scene = room.camera().is_none();
        let mut room = room;
        if self.scene && !reuse {
            // a scene lives from the touch: it is seen at once in its light, as a window
            room.settings(ctx.night, ctx.reduce_motion);
            *self.room_sound.lock().unwrap() = room.synth();
            room.update(t);
        }
        self.room = Some(room);
        self.door = i;
        let from = self.globe.camera;
        let w = {
            let (a, b) = (
                LonLat::new(from.lon, from.lat).to_xyz(),
                LonLat::new(to.lon, to.lat).to_xyz(),
            );
            (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
                .clamp(-1.0, 1.0)
                .acos()
        };
        let dur = if self.reduce_motion {
            0.01
        } else {
            1.1 + 0.5 * w
                + 0.25 * (to.zoom / from.zoom).ln().abs()
                + if self.scene { 0.9 } else { 0.0 }
        };
        let near = if reuse { self.peek_zoom() } else { from.zoom };
        self.state = State::Diving {
            from,
            to,
            t0: t,
            dur,
            near,
        };
    }

    /// Back to the Earth: the toy goes down into it, at the toy's own camera.
    fn leave(&mut self, ctx: &mut Ctx, t: f64) {
        self.leave_from(ctx, t, None);
    }

    /// Back to the Earth; a scene pinched part way back into its light (`squeezed`: the Earth's camera as
    /// the fingers left it) rises on from there.
    fn leave_from(&mut self, ctx: &mut Ctx, t: f64, squeezed: Option<Camera>) {
        if let Some(c) = self.room.as_ref().and_then(|r| r.camera()) {
            self.globe.camera = c;
        }
        self.room_level.store(0f32.to_bits(), Ordering::Relaxed);
        self.strike(self.life.doors[self.door].degree - 5, 0.18, 0.0);
        ctx.haptic();
        self.inside_fingers.clear();
        self.pinch = None;
        if self.scene && !self.reduce_motion {
            self.room_level.store(1f32.to_bits(), Ordering::Relaxed);
            let far = if squeezed.is_some() {
                self.scene_zoom()
            } else {
                self.globe.camera.zoom
            };
            let from = squeezed.unwrap_or(self.globe.camera);
            self.squeeze = None;
            // pinched out, the Earth rises straight up over the place the reader went down into (the mark's
            // way home turns to the opening's view: row 246)
            let to = match squeezed {
                Some(c) => Camera {
                    lon: c.lon,
                    lat: c.lat,
                    zoom: self.rise_to(from).zoom,
                    roll: 0.0,
                },
                None => self.rise_to(from),
            };
            let dur = 1.6 + 0.35 * (from.zoom / to.zoom).ln().abs();
            self.state = State::Rising {
                from,
                to,
                t0: t,
                dur,
                far,
            };
            return;
        }
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
        let at = if d.id == "whales" {
            r.anchors().first().map(|a| a.1)
        } else {
            r.project(d.at).or_else(|| self.globe.point(d.at))
        };
        if let Some(p) = at {
            let id = 9_000_000;
            r.pointer(PointerEvent {
                id,
                phase: Phase::Down,
                x: p[0],
                y: p[1],
                t,
            });
            r.pointer(PointerEvent {
                id,
                phase: Phase::Up,
                x: p[0],
                y: p[1],
                t: t + 0.06,
            });
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
                self.pinch = match (
                    spread(&self.inside_fingers),
                    self.room.as_ref().and_then(|r| r.camera()),
                ) {
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
                    // pinched in, ending at (or toward) the toy's whole Earth: its own hand stops a pinch at its
                    // least zoom, so the toy's camera is asked too, not only the fingers' spread
                    (Some((d0, z0)), Some(d)) => {
                        let now = self
                            .room
                            .as_ref()
                            .and_then(|r| r.camera())
                            .map_or(z0, |c| c.zoom);
                        // (or pinched hard against a toy whose globe will go no farther out: it is at its floor)
                        e.phase == Phase::Up
                            && d / d0 < 0.85
                            && (z0 * d / d0 < 0.8
                                || now < 1.15
                                || (d / d0 < 0.65 && now > z0 * 0.85))
                    }
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
                    return;
                }
            }
            // let go with a scene's window well grown: the descent carries on into it
            if self.downs.is_empty() && matches!(self.state, State::Atlas) {
                if let Some(i) = self.peek {
                    let commit = if self.above().is_some() {
                        0.8
                    } else {
                        PEEK_COMMIT
                    };
                    if self.window_growth(self.peek_zoom(), self.scene_zoom()) >= commit {
                        self.enter(ctx, i, e.t);
                    }
                }
            }
        }
    }

    /// How far the Earth has fallen into the open scene's own light as its window grows (`g`): the night for
    /// a night scene (none holds one now), the dusk for the starlings, so the window's sky meets a ground as dark as
    /// itself rather than a disc of evening laid on noon (painting-company row 258); none for the day scenes.
    fn dusk(&self, g: f64) -> f32 {
        let d = &self.life.doors[self.door];
        if d.id == "murmuration" {
            return starlings_dusk(self.globe.camera.zoom);
        }
        // (a night scene's window is dark from its first size: the Earth falls into night soon after it opens,
        // so it is never a black disc on the day)
        let (deep, by) = if d.night {
            (1.0, 0.35)
        } else if d.id == "murmuration" {
            (0.8, 0.75)
        } else {
            (0.0, 1.0)
        };
        (deep * ease(g / by)) as f32
    }

    /// Where the open toy lies, when its picture is the ground seen from above (Meander: `Toy::seen_from_above`).
    fn above(&self) -> Option<toy_kit::Above> {
        self.room.as_ref().and_then(|r| r.seen_from_above())
    }

    /// The globe's zoom at which a toy seen from above is the glass (its ground at its true scale): the kit's
    /// disc is (short side / 2 - 14) x zoom points across the Earth's radius.
    fn fill_zoom(&self, a: &toy_kit::Above) -> f64 {
        let [w, h] = self.globe_points();
        6_371_000.0 / (a.metres_per_point * (w.min(h) / 2.0 - 14.0).max(40.0))
    }

    /// The zoom at which the open scene is the glass: a toy seen from above at its true scale, else a window's.
    fn scene_zoom(&self) -> f64 {
        self.above()
            .map(|a| self.fill_zoom(&a))
            .unwrap_or(window_zooms(self.life.doors[self.door].id).1)
    }

    /// The zoom at which the open scene's light becomes a window onto it (its light's size).
    fn peek_zoom(&self) -> f64 {
        window_zooms(self.life.doors[self.door].id).0
    }

    /// The scene drawn over the Earth: lying on the ground at its place (a toy seen from above), or a window in
    /// its light grown `g` of the way to the glass.
    fn draw_scene(
        &mut self,
        ctx: &mut Ctx,
        target: &wgpu::TextureView,
        g: f64,
        alpha: f64,
        closing: bool,
    ) {
        match self.above() {
            Some(a) => self.draw_on_ground(ctx, target, &a, alpha),
            None => self.draw_window(ctx, target, g, alpha, closing),
        }
    }

    /// A toy seen from above, its live picture lying on the Earth at its true place, scale and turn: from far a
    /// speck in its sheet of ground, growing exactly as the ground does until it is the glass; then the toy is
    /// what the Earth already showed (painting-company row 258).
    fn draw_on_ground(
        &mut self,
        ctx: &mut Ctx,
        target: &wgpu::TextureView,
        a: &toy_kit::Above,
        alpha: f64,
    ) {
        let fill = self.fill_zoom(a);
        let cam = self.globe.camera;
        let scale = cam.zoom / fill;
        let [_, hp] = Self::points(ctx);
        let shown = ease((hp * scale - 14.0) / 40.0) * alpha;
        let (pc, facing) = self.globe.project(LonLat::new(a.centre[0], a.centre[1]));
        if shown <= 0.0 || facing <= 0.0 {
            return;
        }
        let Some(r) = self.room.as_mut() else { return };
        let view = self.veil.view(ctx);
        r.draw(view);
        // the frame's up lies at the bearing `turn`, the screen's up at the camera's roll: a screen vector into
        // the frame turned back by their difference, and divided by the scale
        let phi = (a.turn - cam.roll).to_radians();
        let (sn, cs) = phi.sin_cos();
        let aff = [
            (cs / scale) as f32,
            (sn / scale) as f32,
            (-sn / scale) as f32,
            (cs / scale) as f32,
        ];
        let k = ctx.ppp;
        let at = [
            (pc[0] * k) as f32,
            (pc[1] * k) as f32,
            ctx.width as f32 * 0.5,
            ctx.height as f32 * 0.5,
        ];
        // the dissolve: a few percent of the frame while it is small on the glass, none as it becomes the glass
        let near = 1.0 - ease((scale - 0.6) / 0.4);
        let edge = (ctx.width.min(ctx.height) as f64 * 0.05 * near) as f32;
        self.veil
            .lay_ground(ctx, target, shown as f32, aff, at, [edge, near as f32]);
    }

    /// The held first finger goes to the scene after all (it was not a pinch).
    fn give_held(&mut self) {
        let held = std::mem::take(&mut self.held);
        if let Some(r) = self.room.as_mut() {
            for ev in held {
                r.pointer(ev);
            }
        }
    }

    /// Lets go of a scene shown in its light.
    fn drop_peek(&mut self) {
        if self.peek.take().is_some() {
            self.room = None;
            *self.room_sound.lock().unwrap() = None;
            self.room_level.store(0f32.to_bits(), Ordering::Relaxed);
            self.veil.target = None;
        }
    }

    /// On the Earth: pinched close (`PEEK_ZOOM`) with a scene's light on screen, the scene nearest the
    /// middle is made and lives in its light, its window growing with the zoom; grown to the glass under
    /// the fingers, the descent goes on into it without them.
    fn update_peek(&mut self, ctx: &mut Ctx, t: f64) {
        if self.reduce_motion {
            return;
        }
        let z = self.globe.camera.zoom;
        let [w, h] = Self::points(ctx);
        // (the scene light nearest the middle, and only once it is close enough for its own window: a light
        // whose window opens sooner, off to the side, never opens while the reader comes down onto another)
        let want = (0..self.life.doors.len())
            .filter(|&i| SCENES.contains(&self.life.doors[i].id))
            .filter_map(|i| {
                let (p, f) = self.life.true_spot(&self.globe, i);
                let inside = p[0] > 0.0 && p[0] < w && p[1] > 0.0 && p[1] < h;
                (f > 0.3 && inside).then(|| (i, (p[0] - w / 2.0).hypot(p[1] - h / 2.0)))
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|x| x.0)
            .filter(|&i| z >= window_zooms(self.life.doors[i].id).0 * 0.9);
        // (deep over a toy seen from above, it stays: nothing else is near enough to be in view)
        let want = if self.peek.is_some()
            && z > 30.0
            && (self.above().is_some() || self.scene_zoom() > SCENE_ZOOM)
        {
            self.peek
        } else {
            want
        };
        if want != self.peek {
            self.drop_peek();
            if let Some(i) = want {
                match (self.life.doors[i].open)(ctx, &self.assets) {
                    Ok(mut r) => {
                        r.settings(ctx.night, ctx.reduce_motion);
                        *self.room_sound.lock().unwrap() = r.synth();
                        self.room = Some(r);
                        self.door = i;
                        self.scene = true;
                        self.peek = Some(i);
                    }
                    Err(e) => toy_kit::log(&format!(
                        "terra: {} did not open: {e:#}",
                        self.life.doors[i].id
                    )),
                }
            }
        }
        if let Some(i) = self.peek {
            if let Some(r) = self.room.as_mut() {
                r.update(t);
            }
            let g = self.window_growth(self.peek_zoom(), self.scene_zoom());
            self.room_level.store(
                ((0.6 * g * ease(g / 0.1)) as f32).to_bits(),
                Ordering::Relaxed,
            );
            self.globe.night = self.dusk(g);
            if g >= 0.999 {
                // grown to the glass under the fingers: they are the toy's from here (the Earth's hand let go)
                for (&id, &(p, _, _)) in self.downs.iter() {
                    self.globe.pointer(PointerEvent {
                        id,
                        phase: Phase::Cancel,
                        x: p[0],
                        y: p[1],
                        t,
                    });
                }
                self.downs.clear();
                self.enter(ctx, i, t);
            }
        }
    }

    /// Inside a scene: two fingers pinch it back toward its light (the window shrinking with them, the Earth
    /// under it); let go small enough, the Earth rises on from there; else it grows back to the glass. Says
    /// whether the finger was Terra's.
    fn squeeze_pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) -> bool {
        let spread =
            |f: &std::collections::HashMap<u64, [f64; 2]>, a: u64, b: u64| -> Option<f64> {
                Some(
                    (f.get(&a)?[0] - f.get(&b)?[0])
                        .hypot(f.get(&a)?[1] - f.get(&b)?[1])
                        .max(1.0),
                )
            };
        let ours = |sq: &Option<Squeeze>| {
            sq.is_some_and(|s| s.decided && !s.settling && (s.a == e.id || s.b == e.id))
        };
        let watching =
            |sq: &Option<Squeeze>| sq.is_some_and(|s| !s.decided && (s.a == e.id || s.b == e.id));
        match e.phase {
            Phase::Down => {
                self.inside_fingers.insert(e.id, [e.x, e.y]);
                if self.inside_fingers.len() == 2
                    && self.squeeze.is_none_or(|s| s.settling)
                    && !self.reduce_motion
                {
                    let other = *self.inside_fingers.keys().find(|&&k| k != e.id).unwrap();
                    // pinching again while the last settles: Terra's at once; else the two are watched, and
                    // stay the toy's unless they close in
                    let again = self.squeeze.is_some_and(|s| s.settling);
                    let d0 = spread(&self.inside_fingers, other, e.id).unwrap_or(1.0);
                    let near = self.rise_to(self.globe.camera).zoom;
                    let g = self.squeeze.map_or(1.0, |s| s.g);
                    let far = self.scene_zoom();
                    self.squeeze = Some(Squeeze {
                        a: other,
                        b: e.id,
                        d0: d0 / ((far / near).ln() * (g - 1.0)).exp().max(0.05),
                        g,
                        near,
                        settling: false,
                        decided: again,
                    });
                    return again;
                }
                ours(&self.squeeze)
            }
            Phase::Move => {
                if let Some(f) = self.inside_fingers.get_mut(&e.id) {
                    *f = [e.x, e.y];
                }
                if watching(&self.squeeze) {
                    let mut sq = self.squeeze.unwrap();
                    let closed =
                        spread(&self.inside_fingers, sq.a, sq.b).is_some_and(|d| d / sq.d0 < 0.85);
                    if !closed {
                        return false;
                    }
                    // they close in: a pinch out, Terra's; the toy lets go of both fingers (or never had the
                    // first, held)
                    sq.decided = true;
                    self.squeeze = Some(sq);
                    for id in [sq.a, sq.b] {
                        if self.held.first().map(|h| h.id) == Some(id) {
                            self.held.clear();
                        } else if let (Some(r), Some(p)) =
                            (self.room.as_mut(), self.inside_fingers.get(&id))
                        {
                            r.pointer(PointerEvent {
                                id,
                                phase: Phase::Cancel,
                                x: p[0],
                                y: p[1],
                                t: e.t,
                            });
                        }
                    }
                }
                if !ours(&self.squeeze) {
                    return false;
                }
                let mut sq = self.squeeze.unwrap();
                if let Some(d) = spread(&self.inside_fingers, sq.a, sq.b) {
                    // the window's growth in the logarithm of the fingers' spread, as the Earth's zoom is
                    sq.g = (1.0 + (d / sq.d0).ln() / (self.scene_zoom() / sq.near).ln())
                        .clamp(0.0, 1.0);
                }
                self.squeeze = Some(sq);
                true
            }
            Phase::Up | Phase::Cancel => {
                self.inside_fingers.remove(&e.id);
                if watching(&self.squeeze) {
                    // lifted without closing in: they were the toy's all along
                    self.squeeze = None;
                    return false;
                }
                if !ours(&self.squeeze) {
                    return false;
                }
                let mut sq = self.squeeze.unwrap();
                // (by the zoom the fingers brought it back to, whatever the scene's depth)
                if (self.scene_zoom() / sq.near).powf(sq.g - 1.0) < SQUEEZE_LEAVE {
                    let from = Camera {
                        zoom: sq.near * (self.scene_zoom() / sq.near).powf(sq.g),
                        ..self.globe.camera
                    };
                    self.inside_fingers.clear();
                    self.leave_from(ctx, e.t, Some(from));
                } else {
                    sq.settling = true;
                    self.squeeze = Some(sq);
                }
                true
            }
        }
    }

    fn tap(&mut self, ctx: &mut Ctx, at: [f64; 2], t: f64) {
        // a touch on a scene's window, open in its light: in
        if let Some(i) = self.peek {
            let (p, _) = self.life.true_spot(&self.globe, i);
            let g = self.window_growth(self.peek_zoom(), self.scene_zoom());
            let [w, h] = Self::points(ctx);
            let (r0, r1) = (Life::size(&self.globe) * 1.06, 0.5 * w.hypot(h) * 1.08);
            if g > 0.0 && (at[0] - p[0]).hypot(at[1] - p[1]) < r0 * (r1 / r0).powf(g) + 10.0 {
                self.enter(ctx, i, t);
                return;
            }
        }
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
        // (a toy seen from above lying on its ground close in: its light's likeness gives way to it)
        // (only once its sheet of ground is plainly there, so the place is never lost on the way down)
        let skip = (self.room.is_some() && self.globe.camera.zoom > 50.0 && self.above().is_some())
            .then_some(self.door);
        self.draw_atlas_but(ctx, target, fade, skip);
    }

    /// The Earth with its lights, `skip` left out (a passage draws it itself, over the toy).
    fn draw_atlas_but(
        &mut self,
        ctx: &mut Ctx,
        target: &wgpu::TextureView,
        fade: f64,
        skip: Option<usize>,
    ) {
        self.globe.upload(&ctx.gpu);
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terra"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            self.globe.draw(&mut rp);
        }
        self.life.paint(
            &mut self.painter,
            &self.globe,
            self.t,
            ctx.ppp,
            fade,
            self.reduce_motion,
            None,
            skip,
        );
        self.painter
            .render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    /// How far a scene's window has grown (0 the light's size .. 1 the whole glass), from the camera's
    /// zoom on its way from `near` (the light seen from afar) to `far` (the dive's end): the window grows
    /// with the Earth, in the logarithm of its zoom.
    fn window_growth(&self, near: f64, far: f64) -> f64 {
        let (z0, z1) = (near.max(0.1), far.max(0.1));
        if (z1 / z0).ln().abs() < 1e-3 {
            return 1.0;
        }
        ((self.globe.camera.zoom / z0).ln() / (z1 / z0).ln()).clamp(0.0, 1.0)
    }

    /// The scene in its light: the toy's whole frame scaled down into a round window over the light,
    /// growing (`g`) until the window is the glass and the frame is the toy's own, unscaled.
    fn draw_window(
        &mut self,
        ctx: &mut Ctx,
        target: &wgpu::TextureView,
        g: f64,
        alpha: f64,
        _closing: bool,
    ) {
        let Some(r) = self.room.as_mut() else { return };
        let k = ctx.ppp;
        let (w, h) = (ctx.width as f64, ctx.height as f64);
        // from where the light is shown (eased apart from a crowd) to where its place truly is
        let (shown, facing) = self.life.spot(&self.globe, self.door);
        if facing <= 0.0 {
            return;
        }
        let (truth, _) = self.life.true_spot(&self.globe, self.door);
        let kt = (g * 3.0).min(1.0);
        let spot = [
            shown[0] + (truth[0] - shown[0]) * kt,
            shown[1] + (truth[1] - shown[1]) * kt,
        ];
        let c = [spot[0] * k, spot[1] * k];
        let r0 = Life::size(&self.globe) * 1.06 * k;
        // (grown, it reaches the glass's farthest corner: a window pinched open off the middle covers it too)
        let r1 = c[0]
            .max(w - c[0])
            .hypot(c[1].max(h - c[1]))
            .max(0.5 * w.hypot(h))
            * 1.08;
        let rad = r0 * (r1 / r0).powf(g);
        // the frame's scale: the window shows the frame's whole width until it is as wide as the glass, then
        // the frame unscaled; its middle at the window's centre (where the dive brings the light)
        let at = window_at(self.life.doors[self.door].id);
        let m = (rad * 2.0 / (w * at[2])).min(1.0);
        // (a toy whose subject is off the middle: there while the window is small, sliding to the middle as
        // the window grows past the glass's width, kept inside the frame)
        let full = ((rad * 2.0 / (w * at[2])).ln() / (2.0 * r1 / (w * at[2])).ln()).clamp(0.0, 1.0);
        let k_mid = full * full * (3.0 - 2.0 * full);
        let half = rad / m;
        let ay = (at[1] * h).clamp(half.min(h * 0.5), h - half.min(h * 0.5));
        // (to the window's own centre at the end, so the frame is then the glass exactly)
        let (ax, ay) = (
            c[0] + (at[0] * w - c[0]) * (1.0 - k_mid),
            c[1] + (ay - c[1]) * (1.0 - k_mid),
        );
        let view = self.veil.view(ctx);
        r.draw(view);
        let soft = (rad * 0.04 + 1.5 * k) as f32;
        // (ragged while it is a window; a clean edge as it becomes the glass, so the hand-off is exact)
        let rag = 1.0 - ease((g - 0.55) / 0.4);
        self.veil.lay_window_ragged(
            ctx,
            target,
            alpha as f32,
            [c[0] as f32, c[1] as f32, rad as f32, soft],
            [ax as f32, ay as f32, m as f32],
            rag as f32,
        );
        // a toy whose first frame is bare (Frost's glass before the ice grows): its light's likeness stays
        // over the window while it is small, growing with it and fading as the toy's own picture fills it
        if matches!(self.life.doors[self.door].id, "frost") {
            let fade = 1.0 - ease(g / 0.45);
            if fade > 0.0 {
                let size = rad / k / 1.06;
                self.life.paint(
                    &mut self.painter,
                    &self.globe,
                    self.t,
                    k,
                    fade * alpha,
                    self.reduce_motion,
                    Some((self.door, size, 0.0)),
                    None,
                );
                let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
                self.painter
                    .render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
                ctx.gpu.queue.submit([enc.finish()]);
            }
        }
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
        p.line(
            [c[0] - 6.9 * k, c[1]],
            [c[0] + 6.9 * k, c[1]],
            0.5 * k,
            0.7,
            [1.0, 1.0, 1.0, lit],
        );
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
        self.painter
            .render(&ctx.gpu, &mut enc, target, [ctx.width, ctx.height], None);
        ctx.gpu.queue.submit([enc.finish()]);
    }
}

impl Toy for Terra {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        // the data shared with every toy Terra opens
        let assets: Arc<dyn Assets> = Arc::from(std::mem::replace(
            &mut ctx.assets,
            Box::new(MemoryAssets(Default::default())),
        ));
        ctx.assets = Box::new(Shared(assets.clone()));
        let month = this_month();
        let mut globe = Globe::new(
            &ctx.gpu,
            assets.as_ref(),
            ctx.width,
            ctx.height,
            ctx.ppp,
            month,
        )?;
        // the world's rivers, more of them the closer the Earth comes (painting-company row 258)
        globe.with_rivers(&ctx.gpu, assets.as_ref())?;
        // over the Atlantic's west: the Americas' lights and Europe's in view, the Pacific's coming round
        globe.camera = Camera::new(-66.0, 20.0, opening_zoom(ctx.points()));
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
            seen: Default::default(),
            state: State::Atlas,
            veil: Veil::new(ctx),
            t: 0.0,
            last_touch: -100.0,
            downs: Default::default(),
            earth_finger: None,
            inside_fingers: Default::default(),
            pinch: None,
            reduce_motion: ctx.reduce_motion,
            scene: false,
            auto: std::env::var("TERRA_PASSAGE")
                .ok()
                .and_then(|id| life_index(&id))
                .map(|i| (i, f64::NAN, 0)),
            drift: 1.6,
            peek: None,
            squeeze: None,
            held: Vec::new(),
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
            if e.phase == Phase::Down
                && self.earth_finger.is_none()
                && (e.x - s[0]).hypot(e.y - s[1]) < EARTH_REACH
                && matches!(self.state, State::Inside | State::Opening { .. })
            {
                self.earth_finger = Some(e.id);
                return;
            }
            if self.earth_finger == Some(e.id) {
                if matches!(e.phase, Phase::Up | Phase::Cancel) {
                    self.earth_finger = None;
                    if e.phase == Phase::Up
                        && (e.x - s[0]).hypot(e.y - s[1]) < EARTH_REACH * 1.5
                        && matches!(self.state, State::Inside | State::Opening { .. })
                    {
                        self.leave(ctx, e.t);
                    }
                }
                return;
            }
            if matches!(self.state, State::Inside) && self.scene {
                if self.squeeze_pointer(ctx, e) {
                    return;
                }
                let held_id = self.held.first().map(|h| h.id);
                match (e.phase, held_id) {
                    (Phase::Down, None)
                        if self.inside_fingers.len() == 1 && !self.reduce_motion =>
                    {
                        self.held.push(e)
                    }
                    (_, Some(id)) if id == e.id => {
                        let d = self.held[0];
                        self.held.push(e);
                        if matches!(e.phase, Phase::Up | Phase::Cancel)
                            || (e.x - d.x).hypot(e.y - d.y) > 12.0
                        {
                            self.give_held();
                        }
                    }
                    _ => {
                        self.give_held();
                        if let Some(r) = self.room.as_mut() {
                            r.pointer(e);
                        }
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
                // left alone, the Earth turns slowly under the reader (west to east, as it does), lingering
                // where its lights face the reader and crossing the empty sea between them more briskly, so
                // a minute's look never turns them all away (painting-company row 246: untouched, the
                // opening had turned to bare Pacific by 40 s)
                if !self.reduce_motion && !self.globe.hand.busy() && self.downs.is_empty() {
                    let k = ease((t - self.last_touch - 4.0) / 3.0);
                    let [w, h] = Self::points(ctx);
                    let seen: f64 = self
                        .visible_doors()
                        .iter()
                        .map(|&(_, p)| {
                            // a light counts the more the nearer the middle of the screen it is
                            let dx = (p[0] / w - 0.5).abs() * 2.0;
                            let dy = (p[1] / h - 0.5).abs() * 2.0;
                            (1.0 - dx.max(dy)).clamp(0.0, 1.0).sqrt()
                        })
                        .sum();
                    let want = 1.6 * (2.6 / (1.0 + 1.3 * seen)).clamp(0.3, 2.6);
                    self.drift += (want - self.drift) * (dt / 3.0).min(1.0);
                    self.globe.camera.lon -= self.drift * k * dt;
                }
                self.update_peek(ctx, t);
                if self.peek.is_none() {
                    // no scene in its light: the Earth's day comes back; but coming down toward the starlings'
                    // light, their dusk falls already (the light nearest the middle of the scenes in view)
                    let [w, h] = Self::points(ctx);
                    let toward = (0..self.life.doors.len())
                        .filter(|&i| SCENES.contains(&self.life.doors[i].id))
                        .filter_map(|i| {
                            let (p, f) = self.life.true_spot(&self.globe, i);
                            (f > 0.3 && p[0] > 0.0 && p[0] < w && p[1] > 0.0 && p[1] < h)
                                .then(|| (i, (p[0] - w / 2.0).hypot(p[1] - h / 2.0)))
                        })
                        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                        .map(|x| x.0);
                    let want = match toward {
                        Some(i) if self.life.doors[i].id == "murmuration" => {
                            starlings_dusk(self.globe.camera.zoom)
                        }
                        _ => 0.0,
                    };
                    let step = dt as f32 * 1.2;
                    self.globe.night = if want > self.globe.night {
                        want
                    } else {
                        (self.globe.night - step).max(want)
                    };
                }
            }
            State::Diving {
                from,
                to,
                t0,
                dur,
                near,
            } => {
                let u = (t - t0) / dur;
                self.globe.camera = if self.scene {
                    fly_turn_then_fall(from, to, u.min(1.0))
                } else {
                    fly(from, to, u.min(1.0))
                };
                if self.life.doors[self.door].night && !self.scene {
                    self.globe.night = ease((u - 0.55) / 0.45) as f32;
                }
                if self.scene {
                    if let Some(r) = self.room.as_mut() {
                        r.update(t);
                    }
                    let g = self.window_growth(near, to.zoom);
                    self.globe.night = self.dusk(g);
                    self.room_level
                        .store(((0.25 + 0.75 * g) as f32).to_bits(), Ordering::Relaxed);
                }
                if u >= 1.0 {
                    if let Some(r) = self.room.as_mut() {
                        // the toy opens where the dive arrived: one picture becoming another
                        if !self.scene {
                            r.set_camera(Camera {
                                roll: self.globe.camera.roll,
                                ..to
                            });
                            r.settings(ctx.night, ctx.reduce_motion);
                            *self.room_sound.lock().unwrap() = r.synth();
                        }
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
                    self.settle_inside(ctx);
                }
            }
            State::Inside => {
                if self.held.first().is_some_and(|h| t - h.t > 0.1) {
                    self.give_held();
                }
                if let Some(r) = self.room.as_mut() {
                    r.update(t);
                }
                if let Some(mut sq) = self.squeeze.filter(|s| s.decided) {
                    if sq.settling {
                        sq.g += (1.0 - sq.g) * (dt / 0.12).min(1.0) + 0.3 * dt;
                    }
                    let far = self.scene_zoom();
                    if sq.settling && sq.g >= 0.999 {
                        self.squeeze = None;
                        self.globe.camera.zoom = far;
                        self.room_level.store(1f32.to_bits(), Ordering::Relaxed);
                    } else {
                        self.squeeze = Some(sq);
                        self.globe.camera.zoom = sq.near * (far / sq.near).powf(sq.g);
                        self.globe.night = self.dusk(sq.g);
                        self.room_level
                            .store(((sq.g * sq.g) as f32).to_bits(), Ordering::Relaxed);
                    }
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
                    let to = self.rise_to(from);
                    let w = {
                        let (a, b) = (
                            LonLat::new(from.lon, from.lat).to_xyz(),
                            LonLat::new(to.lon, to.lat).to_xyz(),
                        );
                        (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
                            .clamp(-1.0, 1.0)
                            .acos()
                    };
                    let dur = if self.reduce_motion {
                        0.01
                    } else {
                        1.2 + 0.4 * w + 0.3 * (from.zoom / to.zoom).ln().abs()
                    };
                    self.state = State::Rising {
                        from,
                        to,
                        t0: t,
                        dur,
                        far: from.zoom,
                    };
                }
            }
            State::Rising {
                from,
                to,
                t0,
                dur,
                far,
            } => {
                let u = (t - t0) / dur;
                if self.room.is_some() {
                    // a scene going back into its light as the Earth rises
                    if let Some(r) = self.room.as_mut() {
                        r.update(t);
                    }
                    let g = self.window_growth(to.zoom, far);
                    self.room_level
                        .store(((g * g) as f32).to_bits(), Ordering::Relaxed);
                    self.globe.night = self.dusk(g);
                    if u >= 1.0 {
                        self.room = None;
                        *self.room_sound.lock().unwrap() = None;
                        self.veil.target = None;
                    }
                }
                self.globe.camera = fly(from, to, u.min(1.0));
                if self.globe.night > 0.0 && self.room.is_none() {
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
        // the ground an open toy seen from above lies on, laid at its place (its satellite picture sharpening
        // out of the month's ground as the Earth comes down), coming in as it grows past a speck; and the hand
        // reaches down to that toy's own scale while it is pinched toward
        let above = self.above();
        let mut sheets = if self.room.is_some() && self.scene {
            region(self.life.doors[self.door].id)
        } else {
            Vec::new()
        };
        sheets.extend(above.as_ref().and_then(|a| a.sheet.clone()));
        if let Err(e) = self
            .globe
            .set_sheets(&ctx.gpu, self.assets.as_ref(), &sheets)
        {
            toy_kit::log(&format!("terra: the ground sheets: {e:#}"));
        }
        let km_pt = 6371.0 / self.globe.disc().r.max(1.0);
        // the map's rivers give way as a sheet's own picture ripens to its true colour (the sheet's key: a
        // kilometre a point down to 50 m), so they never stop at its edge while it is still the map's ground
        let ripe = ((km_pt.ln() - 0.05f64.ln()) / (1.0f64.ln() - 0.05f64.ln())).clamp(0.0, 1.0);
        if let Some(r) = self.globe.rivers.as_mut() {
            r.strength = if self.globe.sheets.is_empty() {
                1.0
            } else {
                ease(ripe) as f32
            };
        }
        for sp in self.globe.sheets.iter_mut() {
            let side_pt = sp.sheet.side_m / 1000.0 / km_pt;
            sp.alpha = ease((side_pt - 6.0) / 30.0) as f32;
        }
        self.globe.hand.max_zoom = match self.peek {
            Some(_) => self.scene_zoom().max(40.0),
            None => 40.0,
        };
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        match self.state {
            State::Atlas => {
                self.draw_atlas(ctx, target, 1.0);
                if self.peek.is_some() {
                    let g = self.window_growth(self.peek_zoom(), self.scene_zoom());
                    if g > 0.0 || self.above().is_some() {
                        self.draw_scene(ctx, target, g, ease(g / 0.1), false);
                    }
                }
            }
            State::Rising { to, t0, far, .. } => {
                self.draw_atlas(ctx, target, 1.0);
                if self.room.is_some() {
                    let g = self.window_growth(to.zoom, far);
                    let a = ease(g / 0.12);
                    self.draw_scene(ctx, target, g, a, (self.t - t0) < 0.0);
                    self.draw_earth_mark(ctx, target, (1.0 - ease((self.t - t0) / 0.5)) as f32);
                }
            }
            State::Diving {
                to, t0, dur, near, ..
            } => {
                // the lights give way to the one being entered
                let u = ((self.t - t0) / dur).clamp(0.0, 1.0);
                self.draw_atlas(ctx, target, 1.0 - 0.6 * u);
                if self.scene {
                    // the toy itself in its light, at once, the window growing as the Earth comes closer
                    let g = self.window_growth(near, to.zoom);
                    // (a window already open in its light, pinched close, is at full strength at once)
                    let a = if near == self.peek_zoom() {
                        1.0
                    } else {
                        ease((self.t - t0) / 0.35)
                    };
                    self.draw_scene(ctx, target, g, a, false);
                }
            }
            State::Opening { t0 } if self.scene => {
                if let Some(r) = self.room.as_mut() {
                    r.draw(target);
                }
                self.draw_earth_mark(ctx, target, ease((self.t - t0) / FADE) as f32);
            }
            State::Opening { t0 } | State::Closing { t0 } => {
                let u = ease((self.t - t0) / FADE);
                let a = if matches!(self.state, State::Opening { .. }) {
                    u
                } else {
                    1.0 - u
                };
                self.draw_atlas(ctx, target, 0.4);
                if let Some(r) = self.room.as_mut() {
                    let view = self.veil.view(ctx);
                    r.draw(view);
                    self.veil.lay(ctx, target, a as f32);
                }
                self.draw_earth_mark(ctx, target, a as f32);
            }
            State::Inside => {
                if let Some(sq) = self.squeeze.filter(|s| s.decided) {
                    // pinched back toward its light: the Earth under it, the window shrinking with the fingers
                    self.draw_atlas(ctx, target, 1.0);
                    self.draw_scene(ctx, target, sq.g, 1.0, true);
                } else if let Some(r) = self.room.as_mut() {
                    r.draw(target);
                }
                self.draw_earth_mark(ctx, target, 1.0);
            }
        }
    }

    fn shaken(&mut self, _ctx: &mut Ctx) {
        // a shake is the open toy's (Frost sweeps its window clear); on the Earth, nothing
        if matches!(self.state, State::Inside) {
            if let Some(r) = self.room.as_mut() {
                r.shaken();
            }
        }
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(Box::new(Mixer {
            bells: self.bells.synth(),
            room: self.room_sound.clone(),
            want: self.room_level.clone(),
            level: 0.0,
            buf: Vec::new(),
        }))
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
        let camera = if matches!(self.state, State::Atlas) {
            self.globe.camera
        } else {
            self.globe.hand.home.unwrap_or(self.globe.camera)
        };
        serde_json::to_string(&Saved {
            camera: Camera {
                zoom: camera.zoom.min(2.5),
                ..camera
            },
        })
        .ok()
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
        self.visible_doors()
            .into_iter()
            .map(|(i, s)| (self.life.doors[i].id.to_string(), s))
            .collect()
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
        let at = self
            .life
            .nearest(&self.globe, [w / 2.0, h / 2.0])
            .map(|i| self.life.spot(&self.globe, i).0)
            .unwrap_or([w / 2.0, h / 2.0]);
        Some(words::earth_demo(at))
    }

    fn buttons(&self, ctx: &Ctx) -> Vec<(String, [f64; 2])> {
        if self.inside() {
            return vec![("Back to the Earth".into(), earth_spot(ctx.points()))];
        }
        if !matches!(self.state, State::Atlas) {
            return Vec::new();
        }
        self.visible_doors()
            .into_iter()
            .map(|(i, p)| (self.life.doors[i].name.to_string(), p))
            .collect()
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

    fn info_at_start(&self) -> bool {
        false
    }

    fn card_at_top(&self, _ctx: &Ctx) -> bool {
        self.inside() && self.room.as_ref().is_some_and(|r| r.card_at_top())
    }

    fn rules(&self) -> Option<Rules> {
        if self.inside() {
            return self.room.as_ref().and_then(|r| r.rules());
        }
        Some(words::earth_rules())
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

/// The ground round a toy seen from above, laid under its own sheet so the descent has real ground all the way
/// down (painting-company row 258; `tools/toys/terra/region.py`): Meander's, 200 km of Sentinel-2 round the
/// Rio Beni's floodplain, its middle the toy's own picture, its edge easing into the month's ground.
fn region(id: &str) -> Vec<toy_kit::Sheet> {
    match id {
        "meander" => vec![sheet("beni.jpg", [-67.0475, -12.9631], 200_000.0)],
        // the starlings' reed beds: the region, and a finer 20 km at 10 m under the toy
        "murmuration" => vec![
            sheet("hamwall.jpg", [-2.787, 51.153], 200_000.0),
            sheet("hamwall-near.jpg", [-2.787, 51.153], 20_000.0),
        ],
        _ => Vec::new(),
    }
}

fn sheet(image: &str, centre: [f64; 2], side_m: f64) -> toy_kit::Sheet {
    toy_kit::Sheet {
        image: image.into(),
        centre,
        side_m,
        turn: 0.0,
        lift: [0.0, 1.0, 1.0],
    }
}

/// The dusk over the starlings' Levels by the Earth's zoom: falling from zoom 6 to 60, so it is evening
/// over the fields before the light opens into its window (at 40) and the window's sky meets its own dusk.
fn starlings_dusk(zoom: f64) -> f32 {
    (0.8 * ease((zoom / 6.0).max(1e-3).ln() / 10f64.ln())) as f32
}

/// Where a scene seen side-on opens as a window (the zoom at which it is its light's size) and is the glass.
/// The starlings' at the scale of the Levels' own fields (zoom 40 to 1500), their reed beds' ground under it,
/// so the flock is never drawn the size of a country (painting-company row 258); the others' from a region
/// to the Earth's closest.
fn window_zooms(id: &str) -> (f64, f64) {
    match id {
        "murmuration" => (40.0, 1500.0),
        _ => (PEEK_ZOOM, SCENE_ZOOM),
    }
}

/// The light whose toy has this id.
fn life_index(id: &str) -> Option<usize> {
    doors::doors().iter().position(|d| d.id == id)
}

toy_kit::ios_shell!(host Terra);
