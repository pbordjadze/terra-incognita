//! Bell (a working title): tap the Earth and it rings, as a great earthquake makes it ring, in its own
//! measured free oscillations (`modes.rs`): the planet's surface swells and sinks in each degree's
//! pattern about the strike, the surface wave runs out round the world and back, and the chord you hear
//! is the one at the place facing you, so turning the ringing globe changes what you hear. Struck
//! twice, the rings add. Left alone, the Earth is struck now and then where great quakes really were.

mod modes;
mod sound;

use modes::MODES;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use toy_kit::{Camera, Ctx, Globe, LonLat, Phase, PointerEvent, Toy};

/// The great earthquakes of the instrumental record, where they struck (USGS): the idle Earth rings
/// from them, faintly. Valdivia first: its ringing in 1960 was the first the instruments heard.
const GREAT: [(f64, f64); 6] = [
    (-73.05, -38.24), // Valdivia, Chile, 1960, M9.5
    (95.85, 3.30),    // Sumatra–Andaman, 2004, M9.1
    (142.37, 38.30),  // Tōhoku, Japan, 2011, M9.1
    (-147.65, 61.02), // Prince William Sound, Alaska, 1964, M9.2
    (159.5, 52.76),   // Kamchatka, 1952, M9.0
    (-72.73, -35.85), // Maule, Chile, 2010, M8.8
];

/// The surface wave's speed (Rayleigh waves, about 3.9 km/s), as the drawn motion runs it: radians a second.
const RING_SPEED: f64 = 3900.0 / 6_371_000.0 * (modes::PITCH / modes::SLOW);
const RING_FADE: f64 = 7.0;
const MAX: usize = sound::MAX_RINGING;

struct Strike {
    at: [f64; 3],
    t0: f64,
    strength: f64,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    camera: Camera,
}

pub struct Bell {
    globe: Globe,
    canvas: (wgpu::Texture, wgpu::TextureView),
    pipe: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    buf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    strikes: Vec<Strike>,
    ear: Arc<Mutex<sound::Ear>>,
    downs: std::collections::HashMap<u64, ([f64; 2], f64, f64)>,
    fingers_seen: usize,
    t: f64,
    last_touch: f64,
    next_idle: f64,
    idle_k: usize,
    soft: bool,
}

fn canvas(ctx: &Ctx) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = ctx.gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("bell canvas"),
        size: wgpu::Extent3d { width: ctx.width.max(1), height: ctx.height.max(1), depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: ctx.gpu.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = tex.create_view(&Default::default());
    (tex, view)
}

impl Bell {
    fn rebind(&mut self, ctx: &Ctx) {
        self.bind = Self::make_bind(ctx, &self.layout, &self.canvas.1, &self.sampler, &self.buf);
    }

    fn make_bind(ctx: &Ctx, layout: &wgpu::BindGroupLayout, view: &wgpu::TextureView, s: &wgpu::Sampler, buf: &wgpu::Buffer) -> wgpu::BindGroup {
        ctx.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bell"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(s) },
                wgpu::BindGroupEntry { binding: 2, resource: buf.as_entire_binding() },
            ],
        })
    }

    fn strike(&mut self, ctx: &mut Ctx, at: LonLat, strength: f64) {
        let p = at.to_xyz();
        if self.strikes.len() >= MAX {
            // the oldest gives way (as the ear lets its quietest go)
            self.strikes.remove(0);
        }
        self.strikes.push(Strike { at: p, t0: self.t, strength });
        if let Ok(mut ear) = self.ear.lock() {
            ear.new.push((p, strength));
        }
        if strength > 0.7 {
            ctx.haptic();
        }
    }

    fn listener(&self) -> [f64; 3] {
        self.globe.camera.world([0.0, 0.0, 1.0])
    }
}

impl Toy for Bell {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut globe = Globe::new(&ctx.gpu, ctx.assets.as_ref(), ctx.width, ctx.height, ctx.ppp, 10)?;
        globe.camera = Camera::new(-62.0, -18.0, 1.0);
        globe.hand.max_zoom = 6.0;
        let layout = ctx.gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bell"),
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
        let shader = ctx.gpu.shader("bell", &toy_kit::globe::wgsl(include_str!("bell.wgsl")));
        let pipe = ctx.gpu.fullscreen_pipeline("bell", &shader, &[&globe.view_layout, &layout], None);
        let sampler = ctx.gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let buf = ctx.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bells"),
            size: 16 * 49,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let canvas = canvas(ctx);
        let bind = Self::make_bind(ctx, &layout, &canvas.1, &sampler, &buf);
        Ok(Bell {
            globe,
            canvas,
            pipe,
            layout,
            sampler,
            buf,
            bind,
            strikes: Vec::new(),
            ear: Arc::new(Mutex::new(sound::Ear::default())),
            downs: Default::default(),
            fingers_seen: 0,
            t: 0.0,
            last_touch: -100.0,
            next_idle: 1.2,
            idle_k: 0,
            soft: ctx.reduce_motion,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
        self.canvas = canvas(ctx);
        self.rebind(ctx);
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        self.globe.pointer(e);
        self.last_touch = self.t;
        match e.phase {
            Phase::Down => {
                if self.downs.is_empty() {
                    self.fingers_seen = 0;
                }
                self.fingers_seen += 1;
                self.downs.insert(e.id, ([e.x, e.y], e.t, 0.0));
            }
            Phase::Move => {
                if let Some(d) = self.downs.get_mut(&e.id) {
                    d.2 = d.2.max((e.x - d.0[0]).hypot(e.y - d.0[1]));
                }
            }
            Phase::Up => {
                if let Some((p, t0, moved)) = self.downs.remove(&e.id) {
                    if self.fingers_seen == 1 && moved < 10.0 && e.t - t0 < 0.5 {
                        if let Some(at) = self.globe.unproject(p[0], p[1]) {
                            self.strike(ctx, at, 1.0);
                        }
                    }
                }
            }
            Phase::Cancel => {
                self.downs.remove(&e.id);
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        self.globe.update(&ctx.gpu, ctx.assets.as_ref(), t, dt);
        // left alone, the Earth is struck now and then from where it really was
        if t >= self.next_idle && t - self.last_touch > 20.0 {
            let (lon, lat) = GREAT[self.idle_k % GREAT.len()];
            let first = self.idle_k == 0;
            self.idle_k += 1;
            self.strike(ctx, LonLat::new(lon, lat), if first { 0.85 } else { 0.5 });
            self.next_idle = t + 28.0 + 9.0 * ((self.idle_k as f64 * 2.39).sin() + 1.0);
        }
        let longest = (0..MODES.len()).map(|i| modes::heard(i).1).fold(0.0, f64::max);
        self.strikes.retain(|s| t - s.t0 < longest * 6.0);
        if let Ok(mut ear) = self.ear.try_lock() {
            ear.listener = self.listener();
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        self.globe.upload(&ctx.gpu);
        let mut u = [[0f32; 4]; 49];
        let n = self.strikes.len().min(MAX);
        u[0] = [n as f32, 0.055, if self.soft { 0.0 } else { 1.0 }, if self.soft { 16.0 } else { 34.0 }];
        for (k, s) in self.strikes.iter().take(MAX).enumerate() {
            let age = (self.t - s.t0).max(0.0);
            let base = 1 + 6 * k;
            u[base] = [s.at[0] as f32, s.at[1] as f32, s.at[2] as f32, 0.0];
            let ring_h = 0.3 * s.strength * (-age / RING_FADE).exp() * (age / 0.15).min(1.0);
            u[base + 1] = [(age * RING_SPEED) as f32, ring_h as f32, 0.16, 0.0];
            for (i, &(l, _, _)) in MODES.iter().enumerate() {
                let (f, tau) = modes::heard(i);
                let fv = f / modes::SLOW;
                let c = modes::weight(i) * s.strength * (-age / tau).exp() * (2.0 * std::f64::consts::PI * fv * age).cos();
                let l = l as usize;
                u[base + 2 + l / 4][l % 4] = c as f32;
            }
        }
        ctx.gpu.queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&u));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bell ground"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.canvas.1,
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
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bell"),
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
            rp.set_pipeline(&self.pipe);
            rp.set_bind_group(0, self.globe.view_bind(), &[]);
            rp.set_bind_group(1, &self.bind, &[]);
            rp.draw(0..3, 0..1);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(Box::new(sound::Voice::new(self.ear.clone())))
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.soft = ctx.reduce_motion;
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&Saved { camera: self.globe.camera }).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            self.globe.camera = s.camera;
        }
    }
}

toy_kit::ios_shell!(Bell);
