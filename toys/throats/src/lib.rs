//! Throats (a working title): side-blotched lizards on a sunlit California hillside, seen from above,
//! each on its rock with an orange, blue or yellow throat. Orange takes blue's rock, blue takes
//! yellow's, yellow takes orange's: that's all, and the colours chase each other round in turning
//! spirals. A finger pours in a stroke of one colour; it breaks into new arms. Pinch out for the whole
//! hillside, in for the lizards. painting-company row 257.
//!
//! The model is `sim.rs` (a stochastic cyclic automaton on the CPU, a few steps a second, each step's
//! flips shown at their own moments); `hill.wgsl` draws everything from one texel a rock.

pub mod sim;

use sim::Hill;
use toy_kit::{info, Ctx, Phase, PointerEvent, Toy};

/// The hillside, rocks (a torus).
const W: usize = 512;
const H: usize = 512;
/// Steps of the rule a second.
const STEP_HZ: f64 = 2.2;
/// A rock's size at the opening, points; and how far a pinch goes.
const OPEN_PT: f64 = 3.0;
/// (the least is also never less than the hill filling the screen: it never shows twice)
const MIN_PT: f64 = 1.1;
const MAX_PT: f64 = 90.0;
/// A pour's radius, points (at least a rock and a half).
const POUR_PT: f64 = 24.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct U {
    size: [f32; 2],
    center: [f32; 2],
    scale: f32,
    time: f32,
    ppp: f32,
    night: f32,
    grid: [f32; 2],
    _p: [f32; 2],
}

struct Finger {
    id: u64,
    at: [f64; 2],
}

pub struct Throats {
    hill: Hill,
    tex: wgpu::Texture,
    ubuf: wgpu::Buffer,
    bind: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    texels: Vec<[f32; 4]>,
    /// world point at the screen's centre (rocks) and points a rock
    center: [f64; 2],
    pt: f64,
    acc: f64,
    t: f64,
    fingers: Vec<Finger>,
    /// a one-finger stroke: its colour and last point; None while pinching
    stroke: Option<(u8, [f64; 2])>,
    pinch: Option<([f64; 2], f64, [f64; 2], f64)>,
    next_colour: u8,
    last_haptic: f64,
    size_pt: [f64; 2],
}

impl Throats {
    fn world(&self, p: [f64; 2]) -> [f64; 2] {
        [self.center[0] + (p[0] - self.size_pt[0] * 0.5) / self.pt, self.center[1] + (p[1] - self.size_pt[1] * 0.5) / self.pt]
    }

    fn pour_along(&mut self, ctx: &mut Ctx, a: [f64; 2], b: [f64; 2], k: u8) {
        let wa = self.world(a);
        let wb = self.world(b);
        let d = ((wb[0] - wa[0]).powi(2) + (wb[1] - wa[1]).powi(2)).sqrt();
        let r = (POUR_PT / self.pt).max(1.5);
        let n = ((d / (r * 0.5)).ceil() as usize).max(1);
        let dir = if d > 1e-6 { [((wb[0] - wa[0]) / d) as f32, ((wb[1] - wa[1]) / d) as f32] } else { [0.0, 0.0] };
        let mut laid = 0;
        for s in 1..=n {
            let f = s as f64 / n as f64;
            let p = [wa[0] + (wb[0] - wa[0]) * f, wa[1] + (wb[1] - wa[1]) * f];
            laid += self.hill.pour(p[0].rem_euclid(W as f64) as f32, p[1].rem_euclid(H as f64) as f32, r as f32, k, dir);
        }
        if laid > 0 && self.t - self.last_haptic > 0.09 {
            self.last_haptic = self.t;
            ctx.haptic();
        }
    }

    /// Where the hill opens: the window of the screen's size with the most rocks held (least grass),
    /// so the first frame is full of lizards and spirals.
    fn opening(hill: &Hill, view: [f64; 2]) -> [f64; 2] {
        let (vw, vh) = (view[0].min(W as f64) as i32, view[1].min(H as f64) as i32);
        let mut best = (usize::MAX, [W as f64 * 0.5, H as f64 * 0.5]);
        for cy in (0..H as i32).step_by(16) {
            for cx in (0..W as i32).step_by(16) {
                let mut bare = 0;
                for y in (cy - vh / 2..cy + vh / 2).step_by(3) {
                    for x in (cx - vw / 2..cx + vw / 2).step_by(3) {
                        let i = y.rem_euclid(H as i32) as usize * W + x.rem_euclid(W as i32) as usize;
                        bare += (hill.kind[i] == sim::BOULDER) as usize;
                    }
                }
                if bare < best.0 {
                    best = (bare, [cx as f64 + 0.5, cy as f64 + 0.5]);
                }
            }
        }
        best.1
    }

    fn clamp_pt(&self, pt: f64) -> f64 {
        let fill = (self.size_pt[0] / W as f64).max(self.size_pt[1] / H as f64);
        pt.clamp(MIN_PT.max(fill), MAX_PT)
    }
}

impl Toy for Throats {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let hill = Hill::new(W, H, 0x7A11_5EED);
        let gpu = &ctx.gpu;
        let tex = gpu.texture(&wgpu::TextureDescriptor {
            label: Some("rocks"),
            size: wgpu::Extent3d { width: W as u32, height: H as u32, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let ubuf = gpu.buffer(&wgpu::BufferDescriptor { label: Some("hill-u"), size: std::mem::size_of::<U>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let bgl = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("hill"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: false }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let relief = gpu.texture(&wgpu::TextureDescriptor {
            label: Some("relief"),
            size: wgpu::Extent3d { width: W as u32, height: H as u32, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &relief, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(&hill.relief()),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(W as u32 * 4), rows_per_image: Some(H as u32) },
            wgpu::Extent3d { width: W as u32, height: H as u32, depth_or_array_layers: 1 },
        );
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("relief"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let relief_view = relief.create_view(&Default::default());
        let view = tex.create_view(&Default::default());
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hill"),
            layout: &bgl,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: ubuf.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&relief_view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let shader = gpu.shader("hill", include_str!("hill.wgsl"));
        let pipeline = gpu.fullscreen_pipeline("hill", &shader, &[&bgl], None);
        let pts = ctx.points();
        let center = Self::opening(&hill, [pts[0] / OPEN_PT, pts[1] / OPEN_PT]);
        Ok(Throats {
            hill,
            tex,
            ubuf,
            bind,
            pipeline,
            texels: vec![[0.0; 4]; W * H],
            center,
            pt: OPEN_PT,
            acc: 0.0,
            t: 0.0,
            fingers: Vec::new(),
            stroke: None,
            pinch: None,
            next_colour: sim::ORANGE,
            last_haptic: -1.0,
            size_pt: pts,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                self.fingers.retain(|f| f.id != e.id);
                self.fingers.push(Finger { id: e.id, at: p });
                if self.fingers.len() == 1 {
                    let k = self.next_colour;
                    self.next_colour = k % 3 + 1;
                    self.stroke = Some((k, p));
                    self.pour_along(ctx, p, p, k);
                } else if self.fingers.len() == 2 {
                    self.stroke = None;
                    let (a, b) = (self.fingers[0].at, self.fingers[1].at);
                    let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
                    let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt().max(1.0);
                    self.pinch = Some((self.world(mid), d, mid, self.pt));
                }
            }
            Phase::Move => {
                if let Some(f) = self.fingers.iter_mut().find(|f| f.id == e.id) {
                    f.at = p;
                }
                if let Some((k, last)) = self.stroke {
                    self.pour_along(ctx, last, p, k);
                    self.stroke = Some((k, p));
                } else if let (Some((anchor, d0, _, pt0)), 2) = (self.pinch, self.fingers.len()) {
                    let (a, b) = (self.fingers[0].at, self.fingers[1].at);
                    let mid = [(a[0] + b[0]) * 0.5, (a[1] + b[1]) * 0.5];
                    let d = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt().max(1.0);
                    self.pt = self.clamp_pt(pt0 * d / d0);
                    // the world point that was under the fingers' middle stays under it
                    self.center = [anchor[0] - (mid[0] - self.size_pt[0] * 0.5) / self.pt, anchor[1] - (mid[1] - self.size_pt[1] * 0.5) / self.pt];
                }
            }
            _ => {
                self.fingers.retain(|f| f.id != e.id);
                if self.fingers.is_empty() {
                    self.stroke = None;
                    self.pinch = None;
                } else if self.fingers.len() == 1 {
                    self.pinch = None;
                }
            }
        }
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        self.acc += dt * STEP_HZ;
        while self.acc >= 1.0 {
            self.acc -= 1.0;
            self.hill.step();
        }
        self.hill.reveal(self.acc as f32, dt as f32);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let h = &self.hill;
        for i in 0..W * H {
            let f = h.from[i];
            let fi = ((f[0].round() as i32).clamp(-4, 4) + 4) + 9 * ((f[1].round() as i32).clamp(-4, 4) + 4);
            self.texels[i] = [(h.shown[i] as u32 + 4 * h.was[i] as u32 + 16 * h.look[i] as u32 + 1024 * h.ground[i] as u32) as f32, h.heading[i], h.since[i], fi as f32];
        }
        let gpu = &ctx.gpu;
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            bytemuck::cast_slice(&self.texels),
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(W as u32 * 16), rows_per_image: Some(H as u32) },
            wgpu::Extent3d { width: W as u32, height: H as u32, depth_or_array_layers: 1 },
        );
        let ppp = ctx.ppp as f32;
        let u = U {
            size: [ctx.width as f32, ctx.height as f32],
            center: [self.center[0] as f32, self.center[1] as f32],
            scale: (self.pt * ctx.ppp) as f32,
            time: self.t as f32,
            ppp,
            night: 0.0,
            grid: [W as f32, H as f32],
            _p: [0.0; 2],
        };
        gpu.queue.write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("throats"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment { view: target, resolve_target: None, depth_slice: None, ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store } })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &self.bind, &[]);
            rp.draw(0..3, 0..1);
        }
        gpu.queue.submit([enc.finish()]);
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Lizards on a California hillside. Draw to pour in a colour.",
            vec![info::Stroke::drag(0.4, 1.4, [s[0] * 0.3, s[1] * 0.4], [s[0] * 0.7, s[1] * 0.55])],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "Every rock is held by one lizard, its throat orange, blue or yellow.",
                "Orange takes blue's rock, blue takes yellow's, yellow takes orange's.",
                "A lizard loses its rock when enough of its neighbours wear the colour that beats its own.",
                "Nothing wins for long: the colours chase each other round in spirals.",
            ],
            "Real: side-blotched lizards in California play this game. Drawn: the hillside, the rule's speed, the spirals.",
        ))
    }
}

toy_kit::ios_shell!(Throats);
