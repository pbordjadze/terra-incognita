//! Murmuration (a working title; painting-company row 202): starlings wheeling over a real reed bed at
//! the real dusk, on the rules measured in real murmurations, and you are one of them.
//!
//! The place is Ham Wall on the Somerset Levels, where starlings gather in winter; the sky is today's
//! dusk there, the sun where it really is (sun.rs), its light scattered by the air (sky.rs). The flock
//! is flock.rs. A finger is where you want to go: touch, and the eye drops in behind your bird; it
//! leans toward the finger, its neighbours heed it, and the flock bends round you. Lean hard and you
//! slip out; let go and the seven nearest still pull you home, and the flock heals.

pub mod flock;
pub mod sky;
pub mod sun;

use flock::{addv, cross, dot, length, norm, scale, sub, Flock, V3};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use toy_kit::{Ctx, Phase, PointerEvent, Synth, Toy};

/// Ham Wall RSPB reserve, Somerset: a winter roost of starlings in the reed beds.
pub const PLACE: (f64, f64) = (-2.787, 51.153);
/// The dusk the toy opens on, minutes from sunset, and where it rests.
const DUSK_FROM: f64 = -24.0;
const DUSK_TO: f64 = 28.0;
/// Minutes of dusk a minute.
const DUSK_RATE: f64 = 1.5;
const BIRDS: usize = 6000;
const MSAA: u32 = 4;
/// When the eye first glides in by itself (seconds after opening), how long it rides, and how long
/// untouched before it shows again.
const FIRST_SHOW: f64 = 4.5;
const SHOW_FOR: f64 = 6.0;
const SHOW_AGAIN: f64 = 40.0;
/// How far behind your bird the eye rides while flying, metres.
const EYE_BEHIND: f32 = 2.2;
/// A drag of this many points is a full lean.
const LEAN_PT: f64 = 70.0;
/// How far from the roost the watcher stands, metres.
const WATCHER: f32 = 65.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SkyU {
    right: [f32; 4],
    up: [f32; 4],
    fwd: [f32; 4],
    sun: [f32; 4],
    sun_col: [f32; 4],
    ground: [f32; 4],
    horizon: [f32; 4],
    horizon2: [f32; 4],
    /// The eye's place (x, z) for the ground's pattern, and the toy's time.
    eye: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct BirdU {
    right: [f32; 4],
    up: [f32; 4],
    fwd: [f32; 4],
    eye: [f32; 4],
    ink: [f32; 4],
    haze: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Inst {
    pos: [f32; 4],
    fwd: [f32; 4],
    lift: [f32; 4],
}

/// The eye: where it is and its frame.
#[derive(Clone, Copy)]
struct Eye {
    pos: V3,
    right: V3,
    up: V3,
    fwd: V3,
    focal: f32,
}

/// What the sound hears, from the simulation (levels as f32 bits).
#[derive(Default)]
struct Ears {
    rush: AtomicU32,
    murmur: AtomicU32,
    wings: AtomicU32,
    rate: AtomicU32,
    phase: AtomicU32,
}

fn put(a: &AtomicU32, v: f32) {
    a.store(v.to_bits(), Ordering::Relaxed)
}
fn get(a: &AtomicU32) -> f32 {
    f32::from_bits(a.load(Ordering::Relaxed))
}

pub struct Murmuration {
    flock: Flock,
    sky: sky::Sky,
    // The dusk: the UTC day it is for, minutes from sunset, how fast it runs.
    day: f64,
    minutes: f64,
    rate: f64,
    sun_el: f64,
    sun_az: f64,
    // The eye: the heading it follows (smoothed), how far in it has dropped (0 far .. 1 behind you).
    heading: V3,
    dir3: V3,
    look: V3,
    flock_look: V3,
    near: f32,
    near_to: f32,
    /// How long and hard the finger has leaned out of the flock (0..1): the lean's limit gives with it.
    strain: f32,
    release_t: f64,
    /// The toy's own clock (seconds since it opened), and when the eye next glides in by itself:
    /// a few seconds after opening, and again after a while untouched, the eye drops in behind your
    /// bird, rides a while and rises out, showing without a word what a touch does.
    clock: f64,
    show_next: f64,
    show_until: f64,
    finger: Option<(u64, [f64; 2])>,
    /// Where the finger came down: a lean is measured from there.
    finger_from: [f64; 2],
    eye: Eye,
    t: f64,
    reduce_motion: bool,
    ears: Arc<Ears>,
    // GPU
    sky_pipe: wgpu::RenderPipeline,
    reed_pipe: wgpu::RenderPipeline,
    sky_bind: wgpu::BindGroup,
    sky_buf: wgpu::Buffer,
    sky_tex: wgpu::Texture,
    bird_pipe: wgpu::RenderPipeline,
    bird_bind: wgpu::BindGroup,
    bird_buf: wgpu::Buffer,
    inst: wgpu::Buffer,
    msaa: wgpu::TextureView,
    exposure: f32,
}

/// The heading of the birds between `r0` and `r1` metres of yours: the flock round you that your
/// lean has not yet bent (the whole flock's, if none are there).
fn ring_heading(f: &Flock, r0: f32, r1: f32) -> V3 {
    let p = f.birds[f.you].pos;
    let mut h = [0.0f32; 3];
    let mut n = 0;
    for b in &f.birds {
        let d = sub(b.pos, p);
        let d2 = dot(d, d);
        if d2 > r0 * r0 && d2 < r1 * r1 {
            h = addv(h, b.vel);
            n += 1;
        }
    }
    if n < 20 {
        return norm(f.centre().1);
    }
    norm(h)
}

/// The birds within `r` metres of yours: their mean heading (unit), their centre, how many.
fn local(f: &Flock, r: f32) -> (V3, V3, usize) {
    let p = f.birds[f.you].pos;
    let (mut h, mut c, mut n) = ([0.0f32; 3], [0.0f32; 3], 0usize);
    for b in &f.birds {
        let d = sub(b.pos, p);
        if dot(d, d) < r * r {
            h = addv(h, b.vel);
            c = addv(c, b.pos);
            n += 1;
        }
    }
    if n == 0 {
        return (norm(f.birds[f.you].vel), p, 0);
    }
    (norm(h), scale(c, 1.0 / n as f32), n)
}

/// The tone curve the sky's shader uses, for colours worked out here (linear in, display out).
fn tone(c: [f32; 3], exposure: f32) -> [f32; 3] {
    let x = [c[0] * exposure * 0.84, c[1] * exposure, c[2] * exposure * 1.3];
    let l = 0.2126 * x[0] + 0.7152 * x[1] + 0.0722 * x[2];
    let lt = l * (1.0 + l / 9.0) / (1.0 + l);
    let k = lt / l.max(1e-6);
    [(x[0] * k).min(1.0).powf(1.0 / 2.2), (x[1] * k).min(1.0).powf(1.0 / 2.2), (x[2] * k).min(1.0).powf(1.0 / 2.2)]
}

fn msaa_view(ctx: &Ctx) -> wgpu::TextureView {
    ctx.gpu
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("murmuration msaa"),
            size: wgpu::Extent3d { width: ctx.width, height: ctx.height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: MSAA,
            dimension: wgpu::TextureDimension::D2,
            format: ctx.gpu.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

impl Murmuration {
    fn sun_at(&self, minutes: f64) -> (f64, f64) {
        let set = sun::sunset(PLACE.0, PLACE.1, self.day + 0.5);
        sun::position(PLACE.0, PLACE.1, set + minutes / 1440.0)
    }

    /// Which bird you are, chosen as the eye drops in: of a sample, the one with the most birds
    /// round it, so a drop always lands in the thick of the flock.
    fn pick_you(&mut self) {
        let n = self.flock.birds.len();
        let mut best = (0usize, self.flock.you);
        for k in 0..240 {
            let i = (k * 7919 + (self.clock * 1000.0) as usize) % n;
            let p = self.flock.birds[i].pos;
            let mut c = 0;
            for b in &self.flock.birds {
                let d = sub(b.pos, p);
                if dot(d, d) < 36.0 {
                    c += 1;
                }
            }
            if c > best.0 {
                best = (c, i);
            }
        }
        self.flock.you = best.1;
        let v = self.flock.birds[best.1].vel;
        self.dir3 = norm(v);
        self.heading = norm([v[0], 0.0, v[2]]);
    }

    /// The eye this frame, from the flock, your bird and how far in it has dropped.
    fn place_eye(&mut self, ctx: &Ctx, dt: f32) {
        let you = &self.flock.birds[self.flock.you];
        let (centre, _) = self.flock.centre();
        let v = you.vel;
        let flat = norm([v[0], 0.0, v[2]]);
        let e = self.near * self.near * (3.0 - 2.0 * self.near);
        // The heading follows yours: slowly from afar, closely from behind you.
        let tau = 2.6 - 2.2 * e;
        let k = (dt / tau).min(1.0);
        self.heading = norm(addv(scale(self.heading, 1.0 - k), scale(flat, k)));
        let k3 = (dt / (0.25 + 0.4 * (1.0 - e))).min(1.0);
        self.dir3 = norm(addv(scale(self.dir3, 1.0 - k3), scale(norm(v), k3)));
        let target = addv(scale(you.pos, 0.3), scale(centre, 0.7));
        let kl = (dt / 1.2).min(1.0);
        self.look = addv(scale(self.look, 1.0 - kl), scale(target, kl));
        // From afar, a watcher stands in the reeds on the far side of the roost from the sun, and
        // watches the flock wheel against the sunset.
        let sa = (self.sun_az as f32).to_radians();
        let dh = WATCHER.max((self.look[1] - 2.0) / (24.0f32).to_radians().tan());
        let far = [self.look[0] - sa.sin() * dh, 2.0, self.look[2] - sa.cos() * dh];
        // From behind you, a little below, looking where you go and toward the birds round you,
        // so the flock fills the glass while you fly in it.
        let (_, c_loc, n_loc) = local(&self.flock, 30.0);
        let c_loc = if n_loc > 20 { c_loc } else { centre };
        let kc = (dt / 0.8).min(1.0);
        self.flock_look = addv(scale(self.flock_look, 1.0 - kc), scale(c_loc, kc));
        // Close behind your bird and under it, so it is the one bird that holds still on
        // the glass while the flock streams past: you.
        // Just behind and above your bird, so it is the dark shape low and centred on the glass,
        // the one that holds still while the flock streams past.
        let back = norm([self.dir3[0], 0.0, self.dir3[2]]);
        let mut near = addv(sub(you.pos, scale(back, EYE_BEHIND)), [0.0, -0.65, 0.0]);
        near[1] = near[1].max(3.0);
        let look_far = self.look;
        let ahead = addv(addv(you.pos, scale(self.dir3, 12.0)), [0.0, 2.0, 0.0]);
        let look_near = addv(scale(ahead, 0.92), scale(self.flock_look, 0.08));
        let pos = addv(scale(far, 1.0 - e), scale(near, e));
        let look = addv(scale(look_far, 1.0 - e), scale(look_near, e));
        let roll = if self.reduce_motion { 0.0 } else { 0.3 * e };
        let upv = norm(addv(scale([0.0, 1.0, 0.0], 1.0 - roll), scale(you.lift, roll)));
        let [w, h] = [ctx.width as f32, ctx.height as f32];
        // The lens: the same sense of size on a phone and an iPad (by the screen's mean side).
        let focal = (w * h).sqrt() / (2.0 * (28.0f32).to_radians().tan());
        let focal = focal * (1.15 - 0.03 * e) * (1.0 - 0.07 * self.flock.lean_k * e);
        let mut fwd = norm(sub(look, pos));
        // Never so steeply up that the land leaves the frame (from afar).
        // Flying, never pitched up so far that the dusk leaves the glass: the horizon stays in the
        // lowest part of the frame, whatever the screen's field of view.
        let half_v = (0.5 * h / focal).atan();
        let fly_up = (half_v - (6.0f32).to_radians()).max(0.0);
        let max_up = ((22.0f32).to_radians() * (1.0 - e) + fly_up * e).sin();
        let max_down = -(6.0f32).to_radians().sin() * e;
        if fwd[1] < max_down {
            let h = norm([fwd[0], 0.0, fwd[2]]);
            fwd = addv(scale(h, (1.0 - max_down * max_down).sqrt()), [0.0, max_down, 0.0]);
        }
        if fwd[1] > max_up {
            let h = norm([fwd[0], 0.0, fwd[2]]);
            fwd = addv(scale(h, (1.0 - max_up * max_up).sqrt()), [0.0, max_up, 0.0]);
        }
        // Flying, the eye rides under and behind your bird and looks up at it (25°, or less if the
        // screen's field of view would lose the horizon): your bird, wings spread, just below the
        // middle of the glass against the sky, the dusk's band low, the flock round and above.
        // ...unless the flock round you is below: then the eye looks down with it, never losing it.
        let to_flock = sub(self.flock_look, pos);
        let flock_el = (to_flock[1] / length(to_flock).max(1.0)).clamp(-1.0, 1.0).asin();
        let to_you = sub(you.pos, pos);
        let you_el = (to_you[1] / length(to_you).max(0.1)).clamp(-1.0, 1.0).asin();
        let fly_pitch = (you_el + (6.0f32).to_radians())
            .min((0.5 * h / focal).atan() - (4.0f32).to_radians())
            .min(flock_el + (20.0f32).to_radians())
            .max(0.0);
        let p0 = fwd[1].clamp(-1.0, 1.0).asin();
        let p1 = (p0 + (fly_pitch - p0) * e).max(0.0);
        let hz = norm([fwd[0], 0.0, fwd[2]]);
        fwd = addv(scale(hz, p1.cos()), [0.0, p1.sin(), 0.0]);
        let right = norm(cross(upv, fwd));
        let up = cross(fwd, right);
        self.eye = Eye { pos, right, up, fwd, focal };
    }

    fn upload_sky(&mut self, ctx: &Ctx) {
        let mut data = Vec::with_capacity(sky::AZ * sky::EL * 8);
        for c in &self.sky.table {
            for k in 0..4 {
                data.extend_from_slice(&sky::half(c[k]).to_le_bytes());
            }
        }
        ctx.gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &self.sky_tex, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(sky::AZ as u32 * 8), rows_per_image: None },
            wgpu::Extent3d { width: sky::AZ as u32, height: sky::EL as u32, depth_or_array_layers: 1 },
        );
        // The eye adapts to the sky (not wholly: dusk still darkens).
        let l = self.sky.adaptation().max(1e-9);
        self.exposure = (0.16 / l.powf(0.82) * 0.0022f64.powf(-0.18)) as f32;
        self.sky.changed = false;
    }
}

impl Toy for Murmuration {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let gpu = &ctx.gpu;
        let dev = &gpu.device;
        let sky_shader = gpu.shader("sky", include_str!("sky.wgsl"));
        let bird_shader = gpu.shader("birds", include_str!("birds.wgsl"));
        let uni = |size: u64, label: &str| {
            dev.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
        };
        let sky_buf = uni(std::mem::size_of::<SkyU>() as u64, "sky u");
        let bird_buf = uni(std::mem::size_of::<BirdU>() as u64, "bird u");
        let sky_tex = dev.create_texture(&wgpu::TextureDescriptor {
            label: Some("sky table"),
            size: wgpu::Extent3d { width: sky::AZ as u32, height: sky::EL as u32, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let samp = dev.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let frag = wgpu::ShaderStages::VERTEX_FRAGMENT;
        let sky_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sky"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: frag, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: frag,
                    ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: frag, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let tex_view = sky_tex.create_view(&Default::default());
        let sky_bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sky"),
            layout: &sky_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: sky_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tex_view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&samp) },
            ],
        });
        let ms = wgpu::MultisampleState { count: MSAA, mask: !0, alpha_to_coverage_enabled: false };
        let sky_pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("sky"), bind_group_layouts: &[&sky_layout], push_constant_ranges: &[] });
        let sky_pipe = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sky"),
            layout: Some(&sky_pl),
            vertex: wgpu::VertexState { module: &sky_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: ms,
            fragment: Some(wgpu::FragmentState {
                module: &sky_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        let reed_pipe = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("reeds"),
            layout: Some(&sky_pl),
            vertex: wgpu::VertexState { module: &sky_shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: ms,
            fragment: Some(wgpu::FragmentState {
                module: &sky_shader,
                entry_point: Some("fs_reeds"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(toy_kit::Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        let bird_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("birds"),
            entries: &[wgpu::BindGroupLayoutEntry { binding: 0, visibility: frag, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None }],
        });
        let bird_bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("birds"),
            layout: &bird_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: bird_buf.as_entire_binding() }],
        });
        let bird_pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("birds"), bind_group_layouts: &[&bird_layout], push_constant_ranges: &[] });
        let attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let bird_pipe = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("birds"),
            layout: Some(&bird_pl),
            vertex: wgpu::VertexState {
                module: &bird_shader,
                entry_point: Some("vs_main"),
                buffers: &[wgpu::VertexBufferLayout { array_stride: std::mem::size_of::<Inst>() as u64, step_mode: wgpu::VertexStepMode::Instance, attributes: &attrs }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
            depth_stencil: None,
            multisample: ms,
            fragment: Some(wgpu::FragmentState {
                module: &bird_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(toy_kit::Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        let inst = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("birds"),
            size: (BIRDS * std::mem::size_of::<Inst>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let msaa = msaa_view(ctx);
        let mut flock = Flock::new(BIRDS, 0x5eed_2026);
        for _ in 0..360 {
            flock.step(1.0 / 60.0);
        }
        let heading = norm({
            let v = flock.birds[flock.you].vel;
            [v[0], 0.0, v[2]]
        });
        let dir3 = norm(flock.birds[flock.you].vel);
        let look = flock.centre().0;
        let day = sun::now_days().floor();
        let eye = Eye { pos: [0.0; 3], right: [1.0, 0.0, 0.0], up: [0.0, 1.0, 0.0], fwd: [0.0, 0.0, 1.0], focal: 1000.0 };
        let mut m = Murmuration {
            flock,
            sky: sky::Sky::new(),
            day,
            minutes: DUSK_FROM,
            rate: DUSK_RATE,
            sun_el: 0.0,
            sun_az: 0.0,
            heading,
            dir3,
            look,
            flock_look: look,
            near: 0.0,
            near_to: 0.0,
            strain: 0.0,
            release_t: -10.0,
            clock: 0.0,
            show_next: FIRST_SHOW,
            show_until: -1.0,
            finger: None,
            finger_from: [0.0; 2],
            eye,
            t: 0.0,
            reduce_motion: ctx.reduce_motion,
            ears: Arc::new(Ears::default()),
            sky_pipe,
            reed_pipe,
            sky_bind,
            sky_buf,
            sky_tex,
            bird_pipe,
            bird_bind,
            bird_buf,
            inst,
            msaa,
            exposure: 1.0,
        };
        let (el, az) = m.sun_at(m.minutes);
        (m.sun_el, m.sun_az) = (el, az);
        m.sky.compute_now(el);
        m.upload_sky(ctx);
        m.place_eye(ctx, 1.0);
        Ok(m)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.msaa = msaa_view(ctx);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                if self.finger.is_none() {
                    if self.near < 0.3 {
                        self.pick_you();
                    }
                    self.finger = Some((e.id, [e.x, e.y]));
                    self.finger_from = [e.x, e.y];
                    self.near_to = 1.0;
                    self.show_until = -1.0;
                    self.show_next = self.clock + SHOW_AGAIN;
                    ctx.haptic();
                }
            }
            Phase::Move => {
                if let Some((id, _)) = self.finger {
                    if id == e.id {
                        self.finger = Some((id, [e.x, e.y]));
                    }
                }
            }
            _ => {
                if let Some((id, _)) = self.finger {
                    if id == e.id {
                        self.finger = None;
                        self.release_t = self.clock;
                        self.show_next = self.clock + SHOW_AGAIN;
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        self.clock += dt;
        // The dusk moves on, and the sky's table follows the sun a few rows a frame.
        self.minutes = (self.minutes + dt * self.rate / 60.0).min(DUSK_TO);
        let (el, az) = self.sun_at(self.minutes);
        (self.sun_el, self.sun_az) = (el, az);
        self.sky.step(el, 6);
        // Your lean: a touch rides with the flock; a drag from where the finger came down leans you,
        // right and left (turns) more than up and down (a sheet of birds is a few metres thick),
        // always from the heading of the flock round you, so you stay in it. Only a long, hard lean
        // out past the edge of the reach slowly lets you go.
        if let Some((_, p)) = self.finger {
            let from = self.finger_from;
            let dx = ((p[0] - from[0]) / LEAN_PT) as f32;
            let dy = ((p[1] - from[1]) / LEAN_PT) as f32;
            // The thick of the flock round you: its centre, and how far you are from it.
            let (_, c_near, n_near) = local(&self.flock, 60.0);
            let c = if n_near > 300 { c_near } else { self.flock.centre().0 };
            let you_p = self.flock.birds[self.flock.you].pos;
            let off = length(sub(c, you_p));
            // A lean pulls against the flock but never leaves it: the farther you are from its
            // thick, the less the lean can take you, and the harder the flock draws you back.
            let reach = (1.0 - (off - 8.0) / 12.0).clamp(0.0, 1.0);
            self.strain = 0.0;
            let h = ring_heading(&self.flock, 16.0, 45.0);
            let hh = norm([h[0], 0.0, h[2]]);
            let right = norm(cross([0.0, 1.0, 0.0], hh));
            let yaw = (dx.clamp(-1.0, 1.0) * 45.0 * reach).to_radians();
            // Up and down only within the sheet's own height: a climb allowed while you are below
            // the level of the birds round you, a dive while above it; never out of the top or
            // the bottom of the flock.
            let above = you_p[1] - c[1];
            let up_ok = ((2.0 - above) / 3.0).clamp(0.0, 1.0);
            let down_ok = ((above + 2.0) / 3.0).clamp(0.0, 1.0);
            let lift = -dy.clamp(-1.0, 1.0);
            let lift = if lift > 0.0 { lift * up_ok } else { lift * down_ok };
            let pitch = (lift * 7.0 * reach).to_radians() + h[1].asin();
            let flat = addv(scale(hh, yaw.cos()), scale(right, yaw.sin()));
            let mut r = norm(addv(scale(flat, pitch.cos()), [0.0, pitch.sin(), 0.0]));
            let you = &self.flock.birds[self.flock.you];
            let v = you.vel;
            // The reeds are no place to fly: below 30 m the lean turns up, firmly near the ground.
            let alt = you.pos[1];
            if alt < 30.0 {
                r[1] = r[1].max(-0.6 * ((alt - 12.0) / 18.0).clamp(0.0, 1.0) + 0.4 * (1.0 - (alt - 12.0) / 18.0).clamp(0.0, 1.0));
                r = norm(r);
            }
            // Home: gentle while you are in the thick, strong once you stray, and strongest when
            // the finger comes back toward where it came down.
            let lean_now = (dx * dx + dy * dy).sqrt().min(1.0);
            let k_home = 0.15 + 0.5 * (off / 12.0).min(1.5) + 0.6 * (1.0 - lean_now);
            // Home is the back half of the flock: a few metres behind its centre, so the flock is
            // ahead of you and in view, not trailing behind your bird where the eye can't see it.
            let fh = norm(self.flock.centre().1);
            let home = sub(c, scale(fh, 6.0));
            let to_c = sub(home, you_p);
            let mut pull = scale([to_c[0], to_c[1] * 3.0, to_c[2]], k_home);
            let pl = length(pull);
            if pl > 9.0 {
                pull = scale(pull, 9.0 / pl);
            }
            let want = addv(scale(r, flock::V0), pull);
            let mut a = scale(sub(want, v), 3.0);
            let al = length(a);
            if al > 30.0 {
                a = scale(a, 30.0 / al);
            }
            let lean = (dx * dx + dy * dy).sqrt().min(1.0);
            self.flock.lean = a;
            self.flock.lean_dir = r;
            self.flock.lean_k = 0.3 + 0.4 * lean;
            // The flock round you bends with your lean only as far as you lean.
            self.flock.lean_field = lean;
        } else {
            self.strain = (self.strain - dt as f32 * 0.5).max(0.0);
            self.flock.lean = [0.0; 3];
            self.flock.lean_k = 0.0;
            self.flock.lean_field = 0.0;
            if self.clock >= self.show_next {
                if self.near < 0.3 {
                    self.pick_you();
                }
                self.show_until = self.clock + SHOW_FOR;
                self.show_next = self.clock + SHOW_FOR + SHOW_AGAIN;
            }
            if self.clock < self.show_until {
                self.near_to = 1.0;
            } else if self.clock - self.release_t > 1.6 {
                self.near_to = 0.0;
            }
        }
        // The flock, in steps of a sixtieth or less.
        let n = (dt * 60.0).ceil().max(1.0) as usize;
        if dt > 0.0 {
            for _ in 0..n {
                self.flock.step((dt / n as f64) as f32);
            }
        }
        // The eye drops in quickly and rises out slowly.
        let rate = if self.near_to > self.near { if self.finger.is_some() { 1.0 / 0.6 } else { 1.0 / 1.4 } } else { 1.0 / 2.6 };
        let step = (dt as f32 * rate).min((self.near_to - self.near).abs());
        self.near += step * (self.near_to - self.near).signum();
        self.place_eye(ctx, dt as f32);
        // What the ears hear: the rush of wings near the eye, the murmur of the whole.
        // The hush of the birds round the eye, and the whoosh of each one passing close: by how
        // near it comes and how fast it goes past (the eye moves with your bird while you fly).
        let eye_v = scale(self.flock.birds[self.flock.you].vel, self.near);
        let (mut round, mut pass) = (0.0f32, 0.0f32);
        for b in &self.flock.birds {
            let d = sub(b.pos, self.eye.pos);
            let d2 = dot(d, d);
            if d2 < 400.0 {
                round += 1.0 / (1.0 + d2 * 0.25);
                if d2 < 49.0 {
                    let k = 1.0 - d2.sqrt() / 7.0;
                    pass += k * k * length(sub(b.vel, eye_v)) / 10.0;
                }
            }
        }
        let rush = 0.17 * (1.0 + round).ln() + 0.09 * pass;
        let (c, _) = self.flock.centre();
        let dc = length(sub(c, self.eye.pos));
        let you = &self.flock.birds[self.flock.you];
        put(&self.ears.rush, rush.min(1.6));
        put(&self.ears.murmur, (0.55 * (90.0 / dc.max(1.0)).powf(0.7)).min(1.0) * (1.0 - 0.3 * self.near));
        put(&self.ears.wings, self.near * (0.15 + 0.5 * you.flap));
        put(&self.ears.rate, you.rate);
        put(&self.ears.phase, you.phase);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        if self.sky.changed {
            self.upload_sky(ctx);
        }
        let e = self.eye;
        let [w, h] = [ctx.width as f32, ctx.height as f32];
        let se = (self.sun_el as f32).to_radians();
        let sa = (self.sun_az as f32).to_radians();
        let sun = [sa.sin() * se.cos(), se.sin(), sa.cos() * se.cos()];
        let trans = sky::transmittance([0.0, se.sin() as f64, se.cos() as f64]);
        let zen = self.sky.lookup(90.0, 0.0);
        let h0 = self.sky.lookup(0.4, 0.0);
        let h1 = self.sky.lookup(0.4, 180.0);
        let sin_el = se.sin().max(0.0);
        let mut ground = [0.0f32; 3];
        for c in 0..3 {
            ground[c] = 0.2 * (zen[c] * 2.0 + trans[c] as f32 * sin_el / std::f32::consts::PI);
        }
        let u = SkyU {
            right: [e.right[0], e.right[1], e.right[2], e.focal],
            up: [e.up[0], e.up[1], e.up[2], w],
            fwd: [e.fwd[0], e.fwd[1], e.fwd[2], h],
            sun: [sun[0], sun[1], sun[2], sa],
            sun_col: [trans[0] as f32, trans[1] as f32, trans[2] as f32, self.exposure],
            ground: [ground[0], ground[1], ground[2], (self.t % (std::f64::consts::TAU * 100.0)) as f32],
            horizon: [h0[0], h0[1], h0[2], e.pos[1].max(2.0)],
            horizon2: [h1[0], h1[1], h1[2], 1.0 - self.near.min(1.0) * 2.0],
            eye: [e.pos[0], e.pos[1], e.pos[2], 0.0],
        };
        ctx.gpu.queue.write_buffer(&self.sky_buf, 0, bytemuck::bytes_of(&u));
        // The birds: dark against the sky, a little hazed with distance.
        let hz = [(h0[0] + h1[0]) * 0.5, (h0[1] + h1[1]) * 0.5, (h0[2] + h1[2]) * 0.5];
        let ink = tone([ground[0] * 0.3, ground[1] * 0.3, ground[2] * 0.35], self.exposure);
        let haze = tone(hz, self.exposure);
        let min_px = (3.0 * ctx.ppp as f32 / 3.0).max(2.0);
        let bu = BirdU {
            right: [e.right[0], e.right[1], e.right[2], e.focal],
            up: [e.up[0], e.up[1], e.up[2], w],
            fwd: [e.fwd[0], e.fwd[1], e.fwd[2], h],
            eye: [e.pos[0], e.pos[1], e.pos[2], min_px],
            ink: [ink[0], ink[1], ink[2], self.flock.you as f32],
            haze: [haze[0], haze[1], haze[2], 4000.0],
        };
        ctx.gpu.queue.write_buffer(&self.bird_buf, 0, bytemuck::bytes_of(&bu));
        let inst: Vec<Inst> = self
            .flock
            .birds
            .iter()
            .map(|b| {
                let f = norm(b.vel);
                Inst { pos: [b.pos[0], b.pos[1], b.pos[2], b.phase], fwd: [f[0], f[1], f[2], b.size], lift: [b.lift[0], b.lift[1], b.lift[2], b.flap] }
            })
            .collect();
        ctx.gpu.queue.write_buffer(&self.inst, 0, bytemuck::cast_slice(&inst));
        if std::env::var_os("MURMURATION_COUNT").is_some() {
            // For tools/toys/murmuration/holds.py: how many birds are on the glass, and how many of
            // them against the sky.
            let (mut n, mut sky, mut big) = (0, 0, 0);
            for b in &self.flock.birds {
                let rel = sub(b.pos, e.pos);
                let z = dot(rel, e.fwd);
                let me = std::ptr::eq(b, &self.flock.birds[self.flock.you]);
                // As the shader culls: other birds nearer than 5 m, or wider than an eighth of
                // the glass, are not drawn, so not counted.
                let span_px = 0.38 * b.size * e.focal / z.max(0.05);
                if z < 0.1 || (!me && (z < 5.0 || span_px > 0.12 * w)) {
                    continue;
                }
                let x = dot(rel, e.right) * e.focal / z;
                let y = dot(rel, e.up) * e.focal / z;
                if x.abs() < 0.5 * w && y.abs() < 0.5 * h {
                    n += 1;
                    // Big enough to read as a bird near you: a wingspan of 12 points or more.
                    if 0.5 * e.focal / z >= 12.0 * ctx.ppp as f32 {
                        big += 1;
                    }
                    if b.pos[1] > e.pos[1] || z > 2000.0 {
                        sky += 1;
                    }
                }
            }
            println!("visible {:.2} {} {} near {:.2} big {}", self.t, n, sky, self.near, big);
            if std::env::var_os("MURMURATION_YOU").is_some() {
                let b = &self.flock.birds[self.flock.you];
                let rel = sub(b.pos, e.pos);
                let z = dot(rel, e.fwd);
                println!("you {} z {:.2} x {:.0} y {:.0} dist {:.2} alt {:.1} vel {:.1} {:.1} {:.1} eye {:.1}", self.flock.you, z, dot(rel, e.right) * e.focal / z, dot(rel, e.up) * e.focal / z, length(rel), b.pos[1], b.vel[0], b.vel[1], b.vel[2], e.pos[1]);
                let (_, _, n15) = local(&self.flock, 15.0);
                let c = self.flock.centre().0;
                let to_c = sub(c, e.pos);
                println!("round {} centre_dist {:.1} centre_in_view_z {:.1} cx {:.0} cy {:.0}", n15, length(sub(c, b.pos)), dot(to_c, e.fwd), dot(to_c, e.right) * e.focal / dot(to_c, e.fwd), dot(to_c, e.up) * e.focal / dot(to_c, e.fwd));
            }
        }
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("murmuration"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.msaa,
                    resolve_target: Some(target),
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Discard },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.sky_pipe);
            rp.set_bind_group(0, &self.sky_bind, &[]);
            rp.draw(0..3, 0..1);
            rp.set_pipeline(&self.bird_pipe);
            rp.set_bind_group(0, &self.bird_bind, &[]);
            rp.set_vertex_buffer(0, self.inst.slice(..));
            rp.draw(0..90, 0..inst.len() as u32);
            rp.set_pipeline(&self.reed_pipe);
            rp.set_bind_group(0, &self.sky_bind, &[]);
            rp.draw(0..3, 0..1);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(Box::new(Wind::new(self.ears.clone())))
    }

    fn save(&self) -> Option<String> {
        Some(serde_json::json!({ "day": self.day, "minutes": self.minutes }).to_string())
    }

    fn restore(&mut self, ctx: &mut Ctx, saved: &str) {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(saved) else { return };
        let day = v["day"].as_f64();
        let fixed = v["fixed"].as_bool().unwrap_or(false);
        // A new day's dusk starts at its beginning; the same day's goes on from where it was.
        if fixed || day == Some(self.day) {
            if let Some(d) = day {
                self.day = d;
            }
            if let Some(m) = v["minutes"].as_f64() {
                self.minutes = m.clamp(DUSK_FROM - 30.0, DUSK_TO);
            }
            if let Some(r) = v["rate"].as_f64() {
                self.rate = r;
            }
            let (el, az) = self.sun_at(self.minutes);
            (self.sun_el, self.sun_az) = (el, az);
            self.sky.compute_now(el);
            self.upload_sky(ctx);
        }
    }
}

/// The sound of a murmuration (the word is the sound): the hush of thousands of wings far off, the
/// rush of the near ones passing, and, from behind your own bird, its wingbeats.
struct Wind {
    ears: Arc<Ears>,
    seed: u32,
    pink: [f32; 3],
    bp: [[f32; 2]; 3],
    lv: [f32; 3],
    phase: f32,
}

impl Wind {
    fn new(ears: Arc<Ears>) -> Wind {
        Wind { ears, seed: 0x1234_5678, pink: [0.0; 3], bp: [[0.0; 2]; 3], lv: [0.0; 3], phase: 0.0 }
    }
    fn noise(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// One state-variable band-pass step: returns the band.
fn svf(s: &mut [f32; 2], x: f32, f: f32, q: f32) -> f32 {
    let low = s[0] + f * s[1];
    let high = x - low - q * s[1];
    let band = f * high + s[1];
    s[0] = low;
    s[1] = band;
    band
}

impl Synth for Wind {
    fn render(&mut self, out: &mut [f32], channels: usize, rate: f64) {
        let target = [get(&self.ears.murmur), get(&self.ears.rush), get(&self.ears.wings)];
        let beat = get(&self.ears.rate).max(1.0);
        // Your bird's wingbeat, heard in step with the one seen.
        self.phase = get(&self.ears.phase) / std::f32::consts::TAU;
        let fr = |hz: f32| 2.0 * (std::f32::consts::PI * hz / rate as f32).sin();
        let (f_low, f_mid, f_hi) = (fr(420.0), fr(1500.0), fr(3200.0));
        for frame in out.chunks_mut(channels) {
            for k in 0..3 {
                self.lv[k] += (target[k] - self.lv[k]) * 0.0004;
            }
            let w = self.noise();
            // Pinkish: a few one-pole filters summed.
            self.pink[0] = 0.997 * self.pink[0] + 0.03 * w;
            self.pink[1] = 0.96 * self.pink[1] + 0.12 * w;
            self.pink[2] = 0.6 * self.pink[2] + 0.4 * w;
            let pink = self.pink[0] + self.pink[1] + self.pink[2] * 0.5;
            let murmur = svf(&mut self.bp[0], pink, f_low, 1.2) * self.lv[0] * 0.5;
            let rush = svf(&mut self.bp[1], w, f_mid, 1.4) * self.lv[1] * 0.4;
            self.phase = (self.phase + beat / rate as f32) % 1.0;
            let flutter = (0.5 - 0.5 * (self.phase * std::f32::consts::TAU).cos()).powi(3);
            let wings = svf(&mut self.bp[2], w, f_hi, 1.6) * flutter * self.lv[2] * 0.12;
            let s = (murmur + rush + wings).clamp(-1.0, 1.0);
            for c in frame.iter_mut() {
                *c = s;
            }
        }
    }
}

toy_kit::ios_shell!(Murmuration);
