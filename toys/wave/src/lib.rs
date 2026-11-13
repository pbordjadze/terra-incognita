//! The Wave (a working title): Estadio Azteca in Mexico City at night, 87,000 people, every one a
//! phone held up as a light. Draw a finger along the stands and the people under it stand up; if
//! enough do, a wave runs round the bowl at twenty seats a second, a roar following it. Too few, and
//! it fizzles. Two waves that meet wipe each other out.
//!
//! `crowd.rs` is the people (Farkas, Helbing & Vicsek's excitable-medium model), `bowl.rs` the
//! stadium and the camera, `bowl.wgsl` the stands and pitch, `city.wgsl` the city round it,
//! `sound.rs` the crowd.

pub mod bowl;
pub mod crowd;
pub mod sound;

use crowd::{Crowd, U, V};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};
use wgpu::util::DeviceExt;

/// The model's steps a second: so a wave runs about 20 seats a second (the paper's 12 m/s).
const TICK_HZ: f64 = 28.0;
/// How many seats round a finger stand up when it touches.
const REACH: f32 = 2.8;
/// Seats round where a finger is first set down that stand (about one group in ten that size starts a wave).
const TAP_REACH: f32 = 2.1;
/// Seconds between a group somewhere trying to start one on its own (most fail).
const TRY_S: [f32; 2] = [7.0, 16.0];

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    eye: [f32; 3],
    tilt: f32,
    size: [f32; 2],
    f: f32,
    time: f32,
    grid: [f32; 2],
    ppp: f32,
    _p: f32,
}

pub struct Wave {
    crowd: Crowd,
    city: Backdrop,
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    ubuf: wgpu::Buffer,
    tex: wgpu::Texture,
    vbuf: wgpu::Buffer,
    ibuf: wgpu::Buffer,
    count: u32,
    texels: Vec<u8>,
    /// each person's own light: 0 none .. 255 bright, and its colour
    lights: Vec<[u8; 2]>,
    acc: f64,
    t: f64,
    next_try: f64,
    size_pt: [f64; 2],
    ppp: f64,
    /// a finger's last seat, so a drag stands up everyone along its path
    last: Vec<(u64, [f32; 2])>,
    sound: sound::Sound,
}

impl Wave {
    fn camera(&self) -> bowl::Camera {
        bowl::Camera::new([
            (self.size_pt[0] * self.ppp) as f32,
            (self.size_pt[1] * self.ppp) as f32,
        ])
    }

    /// The seat under a point on screen (points), if it is on the stands.
    fn seat_at(&self, q: [f64; 2]) -> Option<[f32; 2]> {
        let cam = self.camera();
        let target = [(q[0] * self.ppp) as f32, (q[1] * self.ppp) as f32];
        let near = |u: f32, v: f32| -> f32 {
            cam.project(bowl::seat(u, v)).map_or(f32::MAX, |(s, _)| {
                (s[0] - target[0]).powi(2) + (s[1] - target[1]).powi(2)
            })
        };
        let mut best = (f32::MAX, [0.0f32; 2]);
        for v in (0..V).step_by(2) {
            for u in (0..U).step_by(6) {
                let d = near(u as f32 + 0.5, v as f32 + 0.5);
                if d < best.0 {
                    best = (d, [u as f32 + 0.5, v as f32 + 0.5]);
                }
            }
        }
        // refine round the best
        let c = best.1;
        for dv in -2..=2 {
            for du in -6..=6 {
                let (u, v) = (
                    c[0] + du as f32,
                    (c[1] + dv as f32).clamp(0.5, V as f32 - 0.5),
                );
                let d = near(u, v);
                if d < best.0 {
                    best = (d, [u.rem_euclid(U as f32), v]);
                }
            }
        }
        // on the stands only: within a few seats' size of the nearest
        let pts_per_seat = 6.0 * self.ppp as f32;
        (best.0.sqrt() < pts_per_seat * 2.0).then_some(best.1)
    }

    fn touch(&mut self, ctx: &mut Ctx, id: u64, q: [f64; 2]) {
        let Some(s) = self.seat_at(q) else {
            return;
        };
        let last = self.last.iter().find(|l| l.0 == id).map(|l| l.1);
        let from = last.unwrap_or(s);
        // a finger set down stands only the few under it (too few, alone, to start one); drawn along
        // the stands, it stands everyone it passes
        let reach = if last.is_some() { REACH } else { TAP_REACH };
        // everyone along the way from the last seat to this one
        let mut du = s[0] - from[0];
        if du > U as f32 * 0.5 {
            du -= U as f32;
        } else if du < -(U as f32) * 0.5 {
            du += U as f32;
        }
        let dv = s[1] - from[1];
        let n = ((du.abs().max(dv.abs()) / (REACH * 0.8)).ceil() as usize).max(1);
        let mut stood = 0;
        for k in 1..=n {
            let f = k as f32 / n as f32;
            stood += self.crowd.stand(from[0] + du * f, from[1] + dv * f, reach);
        }
        if stood > 0 {
            ctx.haptic();
        }
        self.last.retain(|l| l.0 != id);
        self.last.push((id, s));
    }

    /// Where the standing people are, for the sound: left to right, and how near.
    fn heard(&self) -> sound::Mix {
        let cam = self.camera();
        let (mut x, mut y, mut n) = (0.0f32, 0.0f32, 0.0f32);
        for v in (0..V).step_by(6) {
            for u in (0..U).step_by(8) {
                if Crowd::is_active(self.crowd.stage[v * U + u]) {
                    if let Some((p, _)) = cam.project(bowl::seat(u as f32, v as f32)) {
                        x += p[0] / cam.size[0] * 2.0 - 1.0;
                        y += p[1] / cam.size[1];
                        n += 1.0;
                    }
                }
            }
        }
        let standing = self.crowd.active as f32 / (U * V) as f32;
        if n == 0.0 {
            return sound::Mix {
                standing,
                pan: 0.0,
                near: 0.5,
            };
        }
        sound::Mix {
            standing,
            pan: x / n,
            near: (y / n).clamp(0.0, 1.0),
        }
    }
}

impl Toy for Wave {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let gpu = &ctx.gpu;
        let mut crowd = Crowd::new(0x0_1986_A2EC);
        let lights: Vec<[u8; 2]> = (0..U * V)
            .map(|_| {
                let has = crowd.rand();
                let col = crowd.rand();
                // most hold up a light; some don't
                let l = if has < 0.6 {
                    120.0 + 135.0 * crowd.rand()
                } else {
                    6.0
                };
                [l as u8, (col * col * 255.0) as u8]
            })
            .collect();
        // a wave already on its way, coming round toward the near end
        crowd.stand(U as f32 * 0.30, V as f32 * 0.5, 6.0);
        for _ in 0..(TICK_HZ * 6.0) as usize {
            crowd.tick();
        }
        let (verts, idx) = bowl::mesh();
        let vbuf = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bowl"),
                contents: bytemuck::cast_slice(&verts),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let ibuf = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bowl-idx"),
                contents: bytemuck::cast_slice(&idx),
                usage: wgpu::BufferUsages::INDEX,
            });
        let tex = gpu.texture(&wgpu::TextureDescriptor {
            label: Some("seats"),
            size: wgpu::Extent3d {
                width: U as u32,
                height: V as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let ubuf = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("bowl-u"),
            size: std::mem::size_of::<Uniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (pipeline, bind) = pipeline(gpu, &ubuf, &tex);
        Ok(Wave {
            crowd,
            city: Backdrop::new(gpu, "wave-city", include_str!("city.wgsl")),
            pipeline,
            bind,
            ubuf,
            tex,
            vbuf,
            ibuf,
            count: idx.len() as u32,
            texels: vec![0; U * V * 4],
            lights,
            acc: 0.0,
            t: 0.0,
            next_try: 5.0,
            size_pt: ctx.points(),
            ppp: ctx.ppp,
            last: Vec::new(),
            sound: sound::Sound::default(),
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
        self.ppp = ctx.ppp;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down | Phase::Move => self.touch(ctx, e.id, [e.x, e.y]),
            _ => self.last.retain(|l| l.0 != e.id),
        }
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        self.acc += dt * TICK_HZ;
        while self.acc >= 1.0 {
            self.acc -= 1.0;
            self.crowd.tick();
        }
        if t > self.next_try {
            let r = self.crowd.rand();
            self.next_try = t + (TRY_S[0] + (TRY_S[1] - TRY_S[0]) * r) as f64;
            self.crowd.somebody_tries();
        }
        self.sound.set(self.heard());
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let frac = self.acc as f32;
        for (i, (s, l)) in self.crowd.stage.iter().zip(&self.lights).enumerate() {
            let up = Crowd::up(*s, frac);
            self.texels[i * 4..i * 4 + 4].copy_from_slice(&[(up * 255.0) as u8, l[0], l[1], 255]);
        }
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.tex,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &self.texels,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(U as u32 * 4),
                rows_per_image: Some(V as u32),
            },
            wgpu::Extent3d {
                width: U as u32,
                height: V as u32,
                depth_or_array_layers: 1,
            },
        );
        let cam = self.camera();
        let u = Uniform {
            eye: cam.eye,
            tilt: cam.tilt,
            size: cam.size,
            f: cam.f,
            time: self.t as f32,
            grid: [U as f32, V as f32],
            ppp: ctx.ppp as f32,
            _p: 0.0,
        };
        gpu.queue
            .write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        let out = bowl::DEPTH * V as f32 + 12.0;
        self.city.render(
            gpu,
            &mut enc,
            target,
            [ctx.width, ctx.height],
            &BackdropParams {
                time: self.t as f32,
                scale: 1.0,
                center: [0.0, 0.0],
                p: [
                    cam.f,
                    cam.tilt,
                    cam.eye[1],
                    cam.eye[2],
                    bowl::HALF[0] + out,
                    bowl::HALF[1] + out,
                    bowl::CORNER + out,
                    0.0,
                ],
            },
        );
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("bowl"),
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
            rp.set_bind_group(0, &self.bind, &[]);
            rp.set_vertex_buffer(0, self.vbuf.slice(..));
            rp.set_index_buffer(self.ibuf.slice(..), wgpu::IndexFormat::Uint32);
            rp.draw_indexed(0..self.count, 0, 0..1);
        }
        gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.sound.synth())
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let cam = self.camera();
        let mut v = Vec::new();
        for (name, u, row) in [
            ("near", U as f32 * 0.25, 30.0),
            ("far", U as f32 * 0.75, 30.0),
            ("side", U as f32 * 0.06, 30.0),
        ] {
            if let Some((p, _)) = cam.project(bowl::seat(u, row)) {
                v.push((
                    name.to_string(),
                    [p[0] as f64 / self.ppp, p[1] as f64 / self.ppp],
                ));
            }
        }
        v
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let a = self.anchors();
        let near = a
            .iter()
            .find(|x| x.0 == "near")
            .map_or([200.0, 700.0], |x| x.1);
        Some(info::Demo::new(
            "A stadium at night. Swipe along the stands to start a wave.",
            vec![info::Stroke::drag(
                0.4,
                0.9,
                [near[0] - 80.0, near[1] + 4.0],
                [near[0] + 60.0, near[1] - 4.0],
            )],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "Each person stands when enough of the people round them are standing, then sits.",
                "Having just sat down, they won't stand again for a moment.",
                "So a wave runs one way at about 20 seats a second; a few dozen must start it.",
                "Swipe along the stands to stand people up. Too few, and it fizzles.",
            ],
            "",
        ))
    }
}

fn pipeline(
    gpu: &toy_kit::Gpu,
    ubuf: &wgpu::Buffer,
    tex: &wgpu::Texture,
) -> (wgpu::RenderPipeline, wgpu::BindGroup) {
    let bgl = gpu
        .device
        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bowl"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
        });
    let view = tex.create_view(&Default::default());
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("bowl"),
        layout: &bgl,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: ubuf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&view),
            },
        ],
    });
    let shader = gpu.shader("bowl", include_str!("bowl.wgsl"));
    let layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("bowl"),
            bind_group_layouts: &[Some(&bgl)],
            immediate_size: 0,
        });
    let pipeline = gpu
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bowl"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Some(wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<bowl::Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2],
                })],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: gpu.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        });
    (pipeline, bind)
}

toy_kit::ios_shell!(Wave);
