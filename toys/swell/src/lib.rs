//! Swell (a working title): touch the sea and a wave runs out from your finger at the speed the real
//! ocean's depth gives it (sea.rs), slowing over the ridges, bending round the seamounts and islands,
//! gathering where the floor focuses it, and wherever it reaches a coast the coast chimes. The floor is
//! never drawn: it is seen only in how the waves move. Hold a finger still on the sea and it beats there,
//! ring after ring. Left alone, the sea is struck now and then where the great tsunamis began.

mod sea;
mod sound;

use serde::{Deserialize, Serialize};
use toy_kit::{Camera, Ctx, Globe, LonLat, Phase, PointerEvent, Toy};

/// Seconds of sea a second on the screen: a wave crosses the Pacific (a day) in about 24 seconds.
const RATE: f64 = 3600.0;
const DROP_KM: f64 = 260.0;
const THRESHOLD: f32 = 0.06;
/// Where the great tsunamis of the record began (offshore of their earthquakes, USGS / NOAA NCEI).
const SOURCES: [(f64, f64); 5] = [
    (143.2, 38.1),   // Tōhoku, 2011
    (-75.0, -38.5),  // Valdivia, 1960
    (94.2, 3.5),     // Sumatra–Andaman, 2004
    (-147.0, 58.6),  // Alaska, 1964
    (-73.2, -35.8),  // Maule, 2010
];
const PENTA: [f64; 5] = [0.0, 2.0, 4.0, 7.0, 9.0];

#[derive(Serialize, Deserialize)]
struct Saved {
    camera: Camera,
}

struct Region {
    armed: bool,
    /// the coast's usual swell lately: a chime is an arrival above it
    base: f32,
    /// the last chime's wave, fading: an echo rings only if it is taller
    last: f32,
    quiet_until: f64,
    lon: f64,
    lat: f64,
}

pub struct Swell {
    globe: Globe,
    sea: sea::Sea,
    tex: wgpu::Texture,
    pipe: wgpu::RenderPipeline,
    buf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    half: Vec<u16>,
    acc: f64,
    t: f64,
    queue: sound::Queue,
    coast_region: Vec<u16>,
    regions: Vec<Region>,
    peak: Vec<f32>,
    tokens: f64,
    downs: std::collections::HashMap<u64, ([f64; 2], f64, f64)>,
    fingers_seen: usize,
    /// a finger held still on the sea: where, and when it next beats
    source: Option<(u64, LonLat, f64)>,
    last_touch: f64,
    next_idle: f64,
    idle_k: usize,
    soft: bool,
}

/// f32 to IEEE half (round to nearest; the waves are far from its limits).
fn half(f: f32) -> u16 {
    let b = f.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let man = b & 0x7f_ffff;
    if exp <= 0 {
        return sign;
    }
    if exp >= 31 {
        return sign | 0x7bff;
    }
    let m = (man + 0x1000) >> 13;
    if m >= 0x400 {
        return sign | (((exp + 1).min(30) as u16) << 10);
    }
    sign | ((exp as u16) << 10) | m as u16
}

impl Swell {
    fn drop_at(&mut self, at: LonLat, height: f64, loud: f64) -> bool {
        if !self.sea.is_wet(at.lon, at.lat) {
            return false;
        }
        self.sea.raise(at.lon, at.lat, height, DROP_KM);
        let ([x, _], facing) = self.globe.project(at);
        let w = self.globe.disc().cx * 2.0;
        let pan = ((x / w) * 2.0 - 1.0).clamp(-0.8, 0.8);
        if let Ok(mut q) = self.queue.lock() {
            q.push(sound::Note::Plunk { amp: loud * if facing > 0.0 { 1.0 } else { 0.3 }, pan });
        }
        true
    }

    fn listen(&mut self, dt: f64) {
        self.tokens = (self.tokens + dt * 1.5).min(3.0);
        for p in self.peak.iter_mut() {
            *p = 0.0;
        }
        for (c, &k) in self.sea.coast.iter().enumerate() {
            let a = self.sea.eta[k as usize].abs();
            let r = self.coast_region[c] as usize;
            if a > self.peak[r] {
                self.peak[r] = a;
            }
        }
        let w = self.globe.disc().cx * 2.0;
        let k = (dt / 4.0) as f32;
        for (r, reg) in self.regions.iter_mut().enumerate() {
            let a = self.peak[r];
            let base = reg.base;
            reg.base += (a - reg.base) * k;
            reg.last *= (-dt / 8.0).exp() as f32;
            if reg.armed && a > THRESHOLD && a > 2.5 * base && a > 1.15 * reg.last && self.t >= reg.quiet_until && self.tokens >= 1.0 {
                reg.armed = false;
                reg.last = a;
                reg.quiet_until = self.t + 5.0;
                self.tokens -= 1.0;
                let at = LonLat::new(reg.lon, reg.lat);
                let ([x, _], facing) = self.globe.project(at);
                let idx = (((reg.lat + 78.0) / 156.0) * 14.0).round().clamp(0.0, 14.0) as usize;
                let semis = 12.0 * (idx / 5) as f64 + PENTA[idx % 5];
                let freq = 196.0 * 2f64.powf(semis / 12.0);
                let amp = ((a / THRESHOLD) as f64 / 5.0).clamp(0.2, 1.0) * if facing > 0.0 { 0.9 } else { 0.25 };
                let pan = ((x / w) * 2.0 - 1.0).clamp(-0.8, 0.8);
                if std::env::var_os("SWELL_LOG").is_some() {
                    eprintln!("chime t {:5.2} at {:7.1} {:6.1} {:4.0} Hz amp {:.2}", self.t, reg.lon, reg.lat, freq, amp);
                }
                if let Ok(mut q) = self.queue.lock() {
                    q.push(sound::Note::Chime { freq, amp, pan });
                }
            } else if !reg.armed && a < (THRESHOLD * 0.5).max(1.2 * base) {
                reg.armed = true;
            }
        }
    }
}

impl Toy for Swell {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut globe = Globe::new(&ctx.gpu, ctx.assets.as_ref(), ctx.width, ctx.height, ctx.ppp, 2)?;
        globe.camera = Camera::new(-178.0, 20.0, 1.3);
        globe.hand.max_zoom = 8.0;
        let sea = sea::Sea::new(&ctx.assets.read("depth.bin")?);
        // the coast's cells gathered by 4-degree tiles: each tile one voice
        let mut coast_region = Vec::with_capacity(sea.coast.len());
        let mut regions: Vec<Region> = Vec::new();
        let mut index = std::collections::HashMap::new();
        for &k in &sea.coast {
            let (i, j) = (k as usize % sea::W, k as usize / sea::W);
            let tile = (i / 8, j / 8);
            let r = *index.entry(tile).or_insert_with(|| {
                regions.push(Region { armed: true, base: 0.0, last: 0.0, quiet_until: 0.0, lon: sea::lon_of(tile.0 * 8 + 4), lat: sea::lat_of(tile.1 * 8 + 4) });
                regions.len() - 1
            });
            coast_region.push(r as u16);
        }
        let tex = ctx.gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("swell eta"),
            size: wgpu::Extent3d { width: sea::W as u32, height: sea::H as u32, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = tex.create_view(&Default::default());
        let sampler = ctx.gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let buf = ctx.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("swell"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = ctx.gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("swell"),
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
        let bind = ctx.gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("swell"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
                wgpu::BindGroupEntry { binding: 2, resource: buf.as_entire_binding() },
            ],
        });
        let shader = ctx.gpu.shader("swell", &toy_kit::globe::wgsl(include_str!("swell.wgsl")));
        let pipe = ctx.gpu.fullscreen_pipeline("swell", &shader, &[&globe.view_layout, &layout], Some(toy_kit::Gpu::over()));
        let n_regions = regions.len();
        Ok(Swell {
            globe,
            sea,
            tex,
            pipe,
            buf,
            bind,
            half: vec![0; sea::W * sea::H],
            acc: 0.0,
            t: 0.0,
            queue: Default::default(),
            coast_region,
            regions,
            peak: vec![0.0; n_regions],
            tokens: 4.0,
            downs: Default::default(),
            fingers_seen: 0,
            source: None,
            last_touch: -100.0,
            next_idle: 1.0,
            idle_k: 0,
            soft: ctx.reduce_motion,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
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
                if self.fingers_seen > 1 {
                    self.source = None;
                }
            }
            Phase::Move => {
                if let Some(d) = self.downs.get_mut(&e.id) {
                    d.2 = d.2.max((e.x - d.0[0]).hypot(e.y - d.0[1]));
                    if d.2 >= 10.0 && self.source.map(|s| s.0) == Some(e.id) {
                        self.source = None;
                    }
                }
            }
            Phase::Up => {
                if self.source.map(|s| s.0) == Some(e.id) {
                    self.source = None;
                }
                if let Some((p, t0, moved)) = self.downs.remove(&e.id) {
                    if self.fingers_seen == 1 && moved < 10.0 && e.t - t0 < 0.45 {
                        if let Some(at) = self.globe.unproject(p[0], p[1]) {
                            if self.drop_at(at, 1.0, 1.0) {
                                ctx.haptic();
                            }
                        }
                    }
                }
            }
            Phase::Cancel => {
                self.downs.remove(&e.id);
                if self.source.map(|s| s.0) == Some(e.id) {
                    self.source = None;
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        self.globe.update(&ctx.gpu, ctx.assets.as_ref(), t, dt);
        // a finger held still on the sea becomes a source: it beats there while it stays
        if self.source.is_none() && self.fingers_seen == 1 && self.downs.len() == 1 {
            let (&id, &(p, t0, moved)) = self.downs.iter().next().unwrap();
            if moved < 10.0 && t - t0 > 0.45 {
                if let Some(at) = self.globe.unproject(p[0], p[1]) {
                    if self.sea.is_wet(at.lon, at.lat) {
                        self.source = Some((id, at, t));
                    }
                }
            }
        }
        if let Some((id, at, next)) = self.source {
            if t >= next {
                if self.drop_at(at, 0.7, 0.6) {
                    ctx.haptic();
                }
                self.source = Some((id, at, next + 1.1));
            }
        }
        // left alone, the sea rises where the great tsunamis began
        if t >= self.next_idle && t - self.last_touch > 20.0 && self.downs.is_empty() {
            let (lon, lat) = SOURCES[self.idle_k % SOURCES.len()];
            let first = self.idle_k == 0;
            self.idle_k += 1;
            self.drop_at(LonLat::new(lon, lat), if first { 1.2 } else { 0.8 }, if first { 0.8 } else { 0.4 });
            self.next_idle = t + 30.0 + 8.0 * ((self.idle_k as f64 * 1.7).sin() + 1.0);
        }
        self.acc += dt * RATE;
        let mut steps = 0;
        while self.acc >= sea::STEP && steps < 8 {
            self.sea.step();
            self.acc -= sea::STEP;
            steps += 1;
        }
        if steps == 8 {
            self.acc = 0.0;
        }
        self.listen(dt);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        self.globe.upload(&ctx.gpu);
        for (h, &e) in self.half.iter_mut().zip(&self.sea.eta) {
            *h = half(e);
        }
        ctx.gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(&self.half),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(sea::W as u32 * 2), rows_per_image: Some(sea::H as u32) },
            wgpu::Extent3d { width: sea::W as u32, height: sea::H as u32, depth_or_array_layers: 1 },
        );
        let info = [if self.soft { 0.25f32 } else { 0.5 }, 25.0, 1.0 / sea::W as f32, 1.0 / sea::H as f32];
        ctx.gpu.queue.write_buffer(&self.buf, 0, bytemuck::cast_slice(&info));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("swell"),
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
            rp.set_pipeline(&self.pipe);
            rp.set_bind_group(0, self.globe.view_bind(), &[]);
            rp.set_bind_group(1, &self.bind, &[]);
            rp.draw(0..3, 0..1);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(Box::new(sound::Chimes::new(self.queue.clone())))
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

toy_kit::ios_shell!(Swell);
