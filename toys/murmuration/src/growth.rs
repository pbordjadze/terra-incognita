//! Murmuration in the look of Yoichiro Kawaguchi's Growth films (Growth: Mysterious Galaxy, 1983;
//! Growth II: Morphogenesis, 1984), made as his 1986 Ars Electronica paper, "Organic surface
//! generation with the morphogenesis model", describes them being made at Osaka:
//!
//! - every form is meta-balls and meta-ellipsoids (centre, radii, weight; Osaka's density
//!   primitives), melted into one surface where their fields sum past a threshold. Each bird is a
//!   body and two wings; birds that come close melt together, so the flock is one skin;
//! - the reeds are grown by his GROWTH recursion: a root branch, each segment's radius the last
//!   one's times a growth ratio (`scale[k]`), bent by a step, joints swollen by `scale[0]`, leaves
//!   branching from the joints at an angle, a flower (here the seed head) at the tip;
//! - colour from each part's attributes and the reflection of a texture-mapped ball round the
//!   scene, with highlights, on black (growth.wgsl).
//!
//! Real time instead of minutes a frame: the field is summed on the screen (growth_density.wgsl),
//! at half resolution on high-density screens (his coarse "screen step"), and shaded per pixel.

use crate::flock::{addv, cross, dot, norm, scale, sub, Flock, Rng, V3};
use crate::Eye;
use toy_kit::Ctx;

/// The Growth look on (this branch's preview) or the dusk as it was.
pub const ON: bool = true;
const REEDS: usize = 90;
const SEGS: usize = 12;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DensU {
    right: [f32; 4],
    up: [f32; 4],
    fwd: [f32; 4],
    eye: [f32; 4],
}

/// One meta-ellipsoid: centre and minor radius, the major half-axis and weight, the material.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Ball {
    c: [f32; 4],
    ax: [f32; 4],
    m: [f32; 4],
}

const BIRD: f32 = 0.0;
const STALK: f32 = 1.0;
const JOINT: f32 = 2.0;
const HEAD: f32 = 3.0;

/// One reed's GROWTH parameters: where it stands round the watcher, and how it grows.
struct Reed {
    az: f32,
    dist: f32,
    length: f32,
    radius: f32,
    lean: f32,
    lean_az: f32,
    step: f32,
    sway: f32,
    leaves: [f32; 2],
}

pub struct Growth {
    reeds: Vec<Reed>,
    balls: Vec<Ball>,
    buf: wgpu::Buffer,
    inst: wgpu::Buffer,
    cap: usize,
    dens_pipe: wgpu::RenderPipeline,
    dens_bind: wgpu::BindGroup,
    shade_pipe: wgpu::RenderPipeline,
    shade_layout: wgpu::BindGroupLayout,
    shade_bind: wgpu::BindGroup,
    dens: wgpu::TextureView,
    /// Field pixels per screen pixel.
    res: f32,
}

fn grow_reeds() -> Vec<Reed> {
    let mut r = Rng::new(0x6702_7783);
    (0..REEDS)
        .map(|_| Reed {
            az: r.next_f32() * std::f32::consts::TAU,
            dist: 3.5 + 13.0 * r.next_f32() * r.next_f32(),
            length: 1.5 + 1.1 * r.next_f32(),
            radius: 0.012 + 0.006 * r.next_f32(),
            lean: 0.05 + 0.2 * r.next_f32(),
            lean_az: r.next_f32() * std::f32::consts::TAU,
            step: 0.02 + 0.05 * r.next_f32(),
            sway: r.next_f32() * std::f32::consts::TAU,
            leaves: [0.2 + 0.2 * r.next_f32(), 0.45 + 0.2 * r.next_f32()],
        })
        .collect()
}

fn rot(v: V3, k: V3, a: f32) -> V3 {
    let (s, c) = a.sin_cos();
    let kv = cross(k, v);
    addv(
        addv(scale(v, c), scale(kv, s)),
        scale(k, dot(k, v) * (1.0 - c)),
    )
}

fn field(ctx: &Ctx, res: f32) -> wgpu::TextureView {
    let w = ((ctx.width as f32 * res).ceil() as u32).max(1);
    let h = ((ctx.height as f32 * res).ceil() as u32).max(1);
    ctx.gpu
        .device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("growth field"),
            size: wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}

fn shade_bind(ctx: &Ctx, layout: &wgpu::BindGroupLayout, v: &wgpu::TextureView) -> wgpu::BindGroup {
    ctx.gpu
        .device
        .create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("growth shade"),
            layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(v),
            }],
        })
}

impl Growth {
    pub fn new(ctx: &Ctx, sky_layout: &wgpu::BindGroupLayout) -> Growth {
        let gpu = &ctx.gpu;
        let dev = &gpu.device;
        let res = if ctx.ppp >= 1.5 { 0.5 } else { 1.0 };
        let dens_shader = gpu.shader("growth field", include_str!("growth_density.wgsl"));
        let shade_src = format!(
            "{}\n{}",
            include_str!("sky.wgsl"),
            include_str!("growth.wgsl")
        );
        let shade_shader = gpu.shader("growth shade", &shade_src);
        let buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("growth u"),
            size: std::mem::size_of::<DensU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let dens_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("growth field"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let dens_bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("growth field"),
            layout: &dens_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buf.as_entire_binding(),
            }],
        });
        let shade_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("growth shade"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4];
        let pipe = |label: &str,
                    layouts: &[Option<&wgpu::BindGroupLayout>],
                    module: &wgpu::ShaderModule,
                    vs: &str,
                    fs: &str,
                    inst: bool,
                    format: wgpu::TextureFormat,
                    blend: Option<wgpu::BlendState>| {
            let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label),
                bind_group_layouts: layouts,
                immediate_size: 0,
            });
            let vb = [Some(wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Ball>() as u64,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &attrs,
            })];
            dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some(vs),
                    buffers: if inst { &vb } else { &[] },
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(fs),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let dens_pipe = pipe(
            "growth field",
            &[Some(&dens_layout)],
            &dens_shader,
            "vs_main",
            "fs_main",
            true,
            wgpu::TextureFormat::Rgba16Float,
            Some(wgpu::BlendState {
                color: add,
                alpha: add,
            }),
        );
        let shade_pipe = pipe(
            "growth shade",
            &[Some(sky_layout), Some(&shade_layout)],
            &shade_shader,
            "vs_main",
            "fs_growth",
            false,
            gpu.format,
            None,
        );
        let dens = field(ctx, res);
        let shade_bind = shade_bind(ctx, &shade_layout, &dens);
        let cap = 16_384;
        let inst = Self::instances(ctx, cap);
        Growth {
            reeds: grow_reeds(),
            balls: Vec::with_capacity(cap),
            buf,
            inst,
            cap,
            dens_pipe,
            dens_bind,
            shade_pipe,
            shade_layout,
            shade_bind,
            dens,
            res,
        }
    }

    fn instances(ctx: &Ctx, n: usize) -> wgpu::Buffer {
        ctx.gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("growth parts"),
            size: (n * std::mem::size_of::<Ball>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn resize(&mut self, ctx: &Ctx) {
        self.dens = field(ctx, self.res);
        self.shade_bind = shade_bind(ctx, &self.shade_layout, &self.dens);
    }

    fn part(&mut self, c: V3, r: f32, ax: V3, w: f32, m: f32, you: f32) {
        self.balls.push(Ball {
            c: [c[0], c[1], c[2], r],
            ax: [ax[0], ax[1], ax[2], w],
            m: [m, you, 0.0, 0.0],
        });
    }

    /// The birds: a body along the heading and two wings out from the shoulders, raised and
    /// lowered by the beat; a far bird (a few pixels) one meta-ellipsoid across its span.
    fn birds(&mut self, flock: &Flock, e: &Eye, width: f32, height: f32) {
        // Your bird, near the eye, hides the birds behind it: drop them, or their skins melt into yours.
        let y = &flock.birds[flock.you];
        let yr = sub(y.pos, e.pos);
        let yz = dot(yr, e.fwd).max(0.3);
        let ys = [
            dot(yr, e.right) * e.focal / yz,
            dot(yr, e.up) * e.focal / yz,
        ];
        let yspan = 0.5 * 0.38 * y.size * e.focal / yz;
        for (i, b) in flock.birds.iter().enumerate() {
            let rel = sub(b.pos, e.pos);
            let z = dot(rel, e.fwd);
            let me = i == flock.you;
            let span = 0.38 * b.size * e.focal / z.max(0.05);
            // As the dusk's birds: none nearer than 5 m or wider than an eighth of the glass, but yours.
            if z < 0.3 || (!me && (z < 5.0 || span > 0.12 * width)) {
                continue;
            }
            let x = dot(rel, e.right) * e.focal / z;
            let y = dot(rel, e.up) * e.focal / z;
            if x.abs() > 0.6 * width + span || y.abs() > 0.6 * height + span {
                continue;
            }
            let (dx, dy) = (x - ys[0], y - ys[1]);
            if !me && z > yz && (dx * dx + dy * dy).sqrt() < 0.8 * yspan + 0.5 * span {
                continue;
            }
            let fwd = norm(b.vel);
            let side = norm(cross(b.lift, fwd));
            let up = cross(fwd, side);
            let s = b.size;
            let you = if me { 1.0 } else { 0.0 };
            if span < 7.0 {
                self.part(b.pos, 0.03 * s, scale(side, 0.15 * s), 1.0, BIRD, you);
                continue;
            }
            // A starling: body, round head, short tail; each wing an arm and a pointed hand swept
            // back from the wrist, the hand beating more than the arm.
            let beat = b.phase.sin() * b.flap;
            let at = |k: f32| addv(b.pos, scale(fwd, k * s));
            self.part(at(0.01), 0.021 * s, scale(fwd, 0.065 * s), 1.0, BIRD, you);
            self.part(at(0.085), 0.017 * s, [0.0; 3], 1.0, BIRD, you);
            self.part(at(-0.085), 0.011 * s, scale(fwd, 0.03 * s), 1.0, BIRD, you);
            for k in [-1.0f32, 1.0] {
                let out = norm(addv(scale(side, k), scale(up, 0.35 * beat)));
                let c = addv(at(0.01), scale(out, 0.055 * s));
                self.part(c, 0.025 * s, scale(out, 0.05 * s), 1.0, BIRD, you);
                let hand = norm(addv(addv(out, scale(fwd, -0.35)), scale(up, 0.4 * beat)));
                let c = addv(addv(at(0.01), scale(out, 0.11 * s)), scale(hand, 0.045 * s));
                self.part(c, 0.016 * s, scale(hand, 0.05 * s), 1.0, BIRD, you);
            }
        }
    }

    /// The reeds, grown each frame by GROWTH round the watcher and swaying in the wind.
    fn reeds(&mut self, e: &Eye, t: f32, strength: f32) {
        if strength <= 0.001 {
            return;
        }
        let w = strength.min(1.0);
        let up = [0.0, 1.0, 0.0];
        let reeds = std::mem::take(&mut self.reeds);
        for r in &reeds {
            let base = [
                e.pos[0] + r.az.sin() * r.dist,
                0.0,
                e.pos[2] + r.az.cos() * r.dist,
            ];
            let rel = sub(base, e.pos);
            if dot(rel, e.fwd) < -2.0 {
                continue;
            }
            let lean_dir = [r.lean_az.sin(), 0.0, r.lean_az.cos()];
            let bend_axis = norm(cross(up, lean_dir));
            let wind = 0.04 * (t * 0.9 + r.sway).sin() + 0.02 * (t * 2.3 + r.sway * 2.0).sin();
            let mut d = rot(up, bend_axis, r.lean);
            let mut p = base;
            let mut rad = r.radius;
            let seg = r.length / SEGS as f32;
            for i in 0..SEGS {
                let f = i as f32 / SEGS as f32;
                let c = addv(p, scale(d, 0.5 * seg));
                self.part(c, rad, scale(d, 0.62 * seg), w, STALK, 0.0);
                p = addv(p, scale(d, seg));
                // The step: each segment bent a little more than the last, and the wind's.
                d = rot(d, bend_axis, r.step * (1.0 + 2.0 * f) + wind * f);
                // scale[k]: the radius shrinking toward the tip.
                rad *= 0.94;
                for (k, &at) in r.leaves.iter().enumerate() {
                    if (f - at).abs() < 0.5 / SEGS as f32 {
                        // A joint, swollen by scale[0], and a leaf branching from it.
                        self.part(p, rad * 1.9, [0.0; 3], w, JOINT, 0.0);
                        let side = if k == 0 { 1.0 } else { -1.0 };
                        self.leaf(p, d, bend_axis, side, rad, w, wind);
                    }
                }
            }
            // The flower: the seed head, a meta-ellipsoid along the tip.
            let c = addv(p, scale(d, 0.07));
            self.part(c, rad * 2.4, scale(d, 0.1), w, HEAD, 0.0);
        }
        self.reeds = reeds;
    }

    #[allow(clippy::too_many_arguments)] // a branch's state, as GROWTH's recursion passes it
    fn leaf(&mut self, at: V3, d: V3, axis: V3, side: f32, rad: f32, w: f32, wind: f32) {
        let mut d = rot(d, axis, side * 0.75);
        let mut p = at;
        let mut r = rad * 0.9;
        for _ in 0..5 {
            let c = addv(p, scale(d, 0.06));
            self.part(c, r, scale(d, 0.075), w, STALK, 0.0);
            p = addv(p, scale(d, 0.12));
            d = rot(d, axis, side * (0.22 + wind));
            r *= 0.8;
        }
    }

    pub fn draw(
        &mut self,
        ctx: &Ctx,
        target: &wgpu::TextureView,
        sky: &wgpu::BindGroup,
        scene: Scene,
    ) {
        let e = &scene.eye;
        let [w, h] = [ctx.width as f32, ctx.height as f32];
        let min_px = (0.8 * ctx.ppp as f32).max(1.3);
        let u = DensU {
            right: [e.right[0], e.right[1], e.right[2], e.focal],
            up: [e.up[0], e.up[1], e.up[2], w],
            fwd: [e.fwd[0], e.fwd[1], e.fwd[2], h],
            eye: [e.pos[0], e.pos[1], e.pos[2], min_px],
        };
        ctx.gpu
            .queue
            .write_buffer(&self.buf, 0, bytemuck::bytes_of(&u));
        self.balls.clear();
        self.birds(scene.flock, e, w, h);
        self.reeds(e, scene.t, scene.reeds);
        if self.balls.len() > self.cap {
            self.cap = self.balls.len().next_power_of_two();
            self.inst = Self::instances(ctx, self.cap);
        }
        ctx.gpu
            .queue
            .write_buffer(&self.inst, 0, bytemuck::cast_slice(&self.balls));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = pass(&mut enc, &self.dens, "growth field");
            rp.set_pipeline(&self.dens_pipe);
            rp.set_bind_group(0, &self.dens_bind, &[]);
            rp.set_vertex_buffer(0, self.inst.slice(..));
            rp.draw(0..6, 0..self.balls.len() as u32);
        }
        {
            let mut rp = pass(&mut enc, target, "growth shade");
            rp.set_pipeline(&self.shade_pipe);
            rp.set_bind_group(0, sky, &[]);
            rp.set_bind_group(1, &self.shade_bind, &[]);
            rp.draw(0..3, 0..1);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }
}

/// What the Growth look draws this frame.
pub struct Scene<'a> {
    pub flock: &'a Flock,
    pub eye: Eye,
    pub t: f32,
    /// The reeds' strength: 1 from afar, gone as the eye drops in.
    pub reeds: f32,
}

fn pass<'a>(
    enc: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
    label: &str,
) -> wgpu::RenderPass<'a> {
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    })
}
