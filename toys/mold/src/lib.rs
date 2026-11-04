//! Mold (a working title, painting-company row 203): a glowing slime mold on the night Earth, whose
//! food is the cities' lights. It starts as specks on the brightest cities, reaches out over the land
//! and finds the routes between them, its tubes thickening where the traffic is; a tap feeds it a
//! flake of oats and the network reroutes to it. Real behaviour underneath: the agent model of
//! Jones (2010), whose networks approximate Physarum's, the mold Tero et al. (2010) grew between
//! oat flakes placed as the towns round Tokyo into a network like the region's railways. The food is
//! the measured lights (NASA's Black Marble), never printed on top (SOURCES.md).
//!
//! All the simulation is GPU compute in `update` (sim.wgsl); `draw` lays its field over the kit's
//! night globe (mold.wgsl).

use toy_kit::gpu::textures::decode;
use toy_kit::{Camera, Ctx, Globe, LonLat, Phase, PointerEvent, Toy};
use wgpu::util::DeviceExt;

/// The field's grid (the globe's maps' layout, a cell about 10 km at the equator).
const W: u32 = 4096;
const H: u32 = 2048;
const GLOW: u32 = 4;
/// The agents: enough for a fine network over every continent.
const AGENTS: u32 = 1 << 16;
/// The lit places an agent may bud again at (drawn by brightness).
const SPAWNS: usize = 1 << 16;
const MAX_FEEDS: usize = 32;
const MAX_RIPPLES: usize = 8;
/// The closest the camera rests (the opening is 1.55): closer, the 10 km field and the lights' sheet
/// would show their cells.
const MAX_ZOOM: f64 = 9.0;
/// Where the mold lives thickest, by longitude: the Americas, Europe and Africa, India, East Asia,
/// Australia. A globe left over an empty ocean turns to the nearest.
const LIT_LONGITUDES: [f64; 6] = [-95.0, -50.0, 15.0, 78.0, 118.0, 145.0];
/// A flake's box's half height in cells (sim.wgsl's REACH).
const FEED_REACH: u32 = 96;
/// Steps a second (the mold's speed: a step is about a cell).
const RATE: f64 = 60.0;
/// How much of the body's light a cell keeps a step (its easing in time).
const BODY_EASE: f32 = 0.97;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    grid: [u32; 4],
    mv: [f32; 4],
    tr: [f32; 4],
    misc: [f32; 4],
    ex: [f32; 4],
    feeds: [[f32; 4]; MAX_FEEDS],
}

/// What the drawing passes read besides the params (mold.wgsl's and streaks.wgsl's `Look`).
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Look {
    a: [f32; 4],
    ages: [[f32; 4]; MAX_FEEDS],
    /// lon, lat (radians), seconds since, 1 on land (0 the sea)
    ripples: [[f32; 4]; MAX_RIPPLES],
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct Feed {
    lon: f64,
    lat: f64,
    at: f64,
}

#[derive(Clone, Copy)]
struct Ripple {
    lon: f64,
    lat: f64,
    at: f64,
    land: bool,
}

struct Touch {
    id: u64,
    x: f64,
    y: f64,
    t: f64,
    moved: bool,
}

pub struct Mold {
    globe: Globe,
    night: Night,
    params: wgpu::Buffer,
    look: wgpu::Buffer,
    agents_pipe: wgpu::ComputePipeline,
    diffuse_pipe: wgpu::ComputePipeline,
    feed_pipe: wgpu::ComputePipeline,
    blur_pipe: wgpu::ComputePipeline,
    sim_binds: [wgpu::BindGroup; 2],
    draw_pipe: wgpu::RenderPipeline,
    draw_binds: [wgpu::BindGroup; 2],
    streak_pipe: wgpu::RenderPipeline,
    streak_bind: wgpu::BindGroup,
    /// Which trail buffer holds the latest field.
    parity: usize,
    steps: u64,
    owed: f64,
    seconds: f64,
    /// Seconds the mold has lived since the app opened.
    age: f64,
    feeds: Vec<Feed>,
    touch: Option<Touch>,
    /// The taps' rings (on land and sea).
    ripples: Vec<Ripple>,
    /// When a finger last touched (the shell's seconds).
    touched: f64,
    /// Where the mold may go (ground.png's G), for a tap.
    land: Vec<u64>,
    reduce_motion: bool,
}

fn storage(binding: u32, read_only: bool, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only }, has_dynamic_offset: false, min_binding_size: None },
        count: None,
    }
}
fn uniform(binding: u32, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
        count: None,
    }
}

/// A small deterministic random source (the mold's start is the same every launch).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Mold {
    fn params(&self) -> Params {
        let mut feeds = [[0.0f32; 4]; MAX_FEEDS];
        for (i, f) in self.feeds.iter().rev().take(MAX_FEEDS).enumerate() {
            feeds[i] = [f.lon.to_radians() as f32, f.lat.to_radians() as f32, 3.0, 5.0];
        }
        Params {
            grid: [W, H, AGENTS, self.steps as u32],
            // Jones's sensor angle 45°, offset 9, rotation 45°, step 1
            mv: [0.4, 9.0, 0.785, 1.0],
            // deposit, decay, diffusion, food gain
            tr: [1.0, 0.9, 0.3, 2.0],
            misc: [0.0005, self.feeds.len().min(MAX_FEEDS) as f32, -1.0, BODY_EASE],
            ex: [0.03, 0.0, 0.0, 0.0],
            feeds,
        }
    }
}

impl Toy for Mold {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut globe = Globe::new(&ctx.gpu, ctx.assets.as_ref(), ctx.width, ctx.height, ctx.ppp, 10)?;
        globe.night = 1.0;
        // as close as the mold still reads as tubes and the ground's lights as lights: a pinch past it
        // goes a little further, then eases back when the fingers lift
        globe.hand.max_zoom = MAX_ZOOM * 1.3;
        globe.camera = Camera::new(12.0, 42.0, 1.55);
        globe.set_home();
        let gpu = &ctx.gpu;
        let dev = &gpu.device;

        toy_kit::step("mold: the ground");
        // the ground: food on land (the lights), and the sea
        let gp = decode(&ctx.assets.read("mold-ground.png")?, 4)?;
        anyhow::ensure!(gp.width == W && gp.height == H, "mold-ground.png is {}x{}, not {W}x{H}", gp.width, gp.height);
        // one packed word a cell (sim.wgsl's `cells`): bit 1 land, bits 8..15 the food; bit 0 (an agent
        // holds it) set as the agents are placed. The decoded picture is let go at once.
        let mut cells: Vec<u32> = gp.data.chunks_exact(4).map(|px| if px[1] > 127 { 2 | (px[0] as u32) << 8 } else { 0 }).collect();
        drop(gp);
        let food = |v: u32| -> f32 { if v & 2 == 0 { -1.0 } else { ((v >> 8) & 255) as f32 / 255.0 } };
        // the lit places, the brightest likeliest: where the mold starts, and where an agent buds again
        let mut rng = Rng(0x9E3779B97F4A7C15);
        let mut lit: Vec<(u32, f64)> = Vec::new();
        let mut total = 0.0;
        for (i, &v) in cells.iter().enumerate() {
            let g = food(v);
            if g > 0.2 {
                let lat = (0.5 - ((i as u32 / W) as f64 + 0.5) / H as f64) * std::f64::consts::PI;
                total += (g as f64).powf(2.0) * lat.cos();
                lit.push((i as u32, total));
            }
        }
        let pick = |r: &mut Rng| -> u32 {
            let u = r.next() * total;
            lit[lit.partition_point(|&(_, c)| c < u).min(lit.len() - 1)].0
        };
        let place = |i: u32, r: &mut Rng| -> [f32; 2] {
            let (x, y) = ((i % W) as f64 + r.next(), (i / W) as f64 + r.next());
            [((x / W as f64 - 0.5) * std::f64::consts::TAU) as f32, ((0.5 - y / H as f64) * std::f64::consts::PI) as f32]
        };
        let spawn: Vec<[f32; 2]> = (0..SPAWNS).map(|_| { let i = pick(&mut rng); place(i, &mut rng) }).collect();
        // the start: a mist over the lit land, each agent in a cell of its own (Jones's start, a sparse
        // population that condenses into tubes), a cell taken already passing to one beside it
        let mut agents = Vec::with_capacity(AGENTS as usize);
        while agents.len() < AGENTS as usize {
            let c = pick(&mut rng);
            let (dx, dy) = ((rng.next() * 9.0) as i64 - 4, (rng.next() * 9.0) as i64 - 4);
            let x = ((c % W) as i64 + dx).rem_euclid(W as i64) as u32;
            let y = ((c / W) as i64 + dy).clamp(0, H as i64 - 1) as u32;
            let i = (y * W + x) as usize;
            if cells[i] & 2 == 0 || cells[i] & 1 != 0 {
                continue;
            }
            cells[i] |= 1;
            let p = place(i as u32, &mut rng);
            let n = agents.len() as u32;
            agents.push([p[0].to_bits(), p[1].to_bits(), ((rng.next() * std::f64::consts::TAU) as f32).to_bits(), n ^ 0x5bd1e995, i as u32, 0]);
        }

        toy_kit::step("mold: the buffers");
        let init = |label, contents: &[u8], usage| dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some(label), contents, usage });
        let st = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let agents_buf = init("agents", bytemuck::cast_slice(&agents), st);
        let spawn_buf = init("spawn", bytemuck::cast_slice(&spawn), st);
        // (wgpu zeroes a new buffer: no zeros made on the CPU; the body's start is the first step's, on
        // the GPU)
        let zeroed = |label, n: u32| dev.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: n as u64 * 4, usage: st, mapped_at_creation: false });
        let trails = [zeroed("trail a", W * H), zeroed("trail b", W * H)];
        let body = zeroed("body", W * H);
        let glow = zeroed("glow", (W / GLOW) * (H / GLOW));
        // the land, a bit a cell, for a tap (the rest of the cells live on the GPU only)
        let mut land = vec![0u64; (W * H) as usize / 64];
        for (i, &v) in cells.iter().enumerate() {
            if v & 2 != 0 {
                land[i / 64] |= 1 << (i % 64);
            }
        }
        let cells_buf = init("cells", bytemuck::cast_slice(&cells), st);
        drop(cells);
        let params = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mold params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let look = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mold look"),
            size: std::mem::size_of::<Look>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        toy_kit::step("mold: the simulation's pipelines");
        let cs = wgpu::ShaderStages::COMPUTE;
        let sim_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mold sim"),
            entries: &[
                uniform(0, cs),
                storage(1, false, cs),
                storage(2, true, cs),
                storage(3, false, cs),
                storage(4, false, cs),
                storage(6, true, cs),
                storage(7, false, cs),
                storage(8, false, cs),
            ],
        });
        let sim_binds = [0, 1].map(|p| {
            let (a, b) = (&trails[p], &trails[1 - p]);
            dev.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mold sim"),
                layout: &sim_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: agents_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: a.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: b.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: body.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 6, resource: spawn_buf.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 7, resource: glow.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 8, resource: cells_buf.as_entire_binding() },
                ],
            })
        });
        let sim_shader = gpu.shader("mold sim", include_str!("sim.wgsl"));
        let sim_pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("mold sim"), bind_group_layouts: &[&sim_layout], push_constant_ranges: &[] });
        let compute = |entry: &str| {
            dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&sim_pl),
                module: &sim_shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let (agents_pipe, diffuse_pipe, feed_pipe, blur_pipe) = (compute("step_agents"), compute("diffuse"), compute("feed"), compute("blur"));

        toy_kit::step("mold: the drawing's pipelines");
        let fs = wgpu::ShaderStages::FRAGMENT;
        let draw_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mold draw"),
            entries: &[uniform(0, fs), storage(1, true, fs), storage(2, true, fs), uniform(3, fs)],
        });
        // (one for each parity, the same: the body is one buffer, eased in place)
        let draw_binds = [0, 1].map(|_| {
            dev.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("mold draw"),
                layout: &draw_layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: body.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: glow.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: look.as_entire_binding() },
                ],
            })
        });
        let add = wgpu::BlendState {
            color: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::One, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
            alpha: wgpu::BlendComponent { src_factor: wgpu::BlendFactor::Zero, dst_factor: wgpu::BlendFactor::One, operation: wgpu::BlendOperation::Add },
        };
        let draw_shader = gpu.shader("mold draw", &toy_kit::globe::wgsl(&format!("@group(0) @binding(0) var<uniform> U: View;\n{}", include_str!("mold.wgsl"))));
        let draw_pipe = gpu.fullscreen_pipeline("mold draw", &draw_shader, &[&globe.view_layout, &draw_layout], Some(add));

        // the agents themselves, close in (streaks.wgsl): six vertices an agent, no vertex buffers
        let vf = wgpu::ShaderStages::VERTEX_FRAGMENT;
        let streak_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mold streaks"),
            entries: &[storage(0, true, wgpu::ShaderStages::VERTEX), uniform(1, vf), uniform(2, vf)],
        });
        let streak_bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mold streaks"),
            layout: &streak_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: agents_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: look.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 2, resource: params.as_entire_binding() },
            ],
        });
        let streak_shader = gpu.shader("mold streaks", &toy_kit::globe::wgsl(&format!("@group(0) @binding(0) var<uniform> U: View;\n{}", include_str!("streaks.wgsl"))));
        let streak_pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("mold streaks"), bind_group_layouts: &[&globe.view_layout, &streak_layout], push_constant_ranges: &[] });
        let streak_pipe = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mold streaks"),
            layout: Some(&streak_pl),
            vertex: wgpu::VertexState { module: &streak_shader, entry_point: Some("vs_streak"), buffers: &[], compilation_options: Default::default() },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &streak_shader,
                entry_point: Some("fs_streak"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(add), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });

        let night = Night::new(gpu, &globe);
        toy_kit::step("mold: made");
        Ok(Mold {
            night,
            globe,
            params,
            look,
            agents_pipe,
            diffuse_pipe,
            feed_pipe,
            blur_pipe,
            sim_binds,
            draw_pipe,
            draw_binds,
            streak_pipe,
            streak_bind,
            parity: 0,
            steps: 0,
            // (one step owed at once, so the first frame has the mist's glow)
            owed: 1.0,
            seconds: 0.0,
            age: 0.0,
            feeds: Vec::new(),
            touch: None,
            ripples: Vec::new(),
            touched: f64::NEG_INFINITY,
            land,
            reduce_motion: ctx.reduce_motion,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        self.globe.pointer(e);
        self.touched = self.seconds;
        match e.phase {
            Phase::Down => {
                // a second finger makes it a pinch, never a feed
                self.touch = if self.globe.hand.fingers() == 1 { Some(Touch { id: e.id, x: e.x, y: e.y, t: e.t, moved: false }) } else { None };
            }
            Phase::Move => {
                if let Some(t) = &mut self.touch {
                    if t.id == e.id && (e.x - t.x).hypot(e.y - t.y) > 10.0 {
                        t.moved = true;
                    }
                }
            }
            Phase::Up => {
                if let Some(t) = self.touch.take() {
                    if t.id == e.id && !t.moved && e.t - t.t < 0.5 {
                        if let Some(at) = self.globe.unproject(e.x, e.y) {
                            // on land a flake (the mold may reach it); on the sea only the ring
                            if self.on_land(at) {
                                self.feed(at);
                            }
                            self.ripples.push(Ripple { lon: at.lon, lat: at.lat, at: self.seconds, land: self.on_land(at) });
                            if self.ripples.len() > MAX_RIPPLES {
                                self.ripples.remove(0);
                            }
                            ctx.haptic();
                        }
                    }
                }
            }
            Phase::Cancel => self.touch = None,
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.globe.update(&ctx.gpu, ctx.assets.as_ref(), t, dt);
        self.seconds = t;
        // left alone a while (a fling spent), the globe settles over the mold's latitudes, never resting
        // on the ice or the empty southern ocean (a twist's roll is kept: Home sets it upright)
        if !self.globe.hand.busy() && t - self.touched > 2.5 {
            let k = 1.0 - 0.5f64.powf(dt / 1.2);
            let cam = &mut self.globe.camera;
            let lat = cam.lat.clamp(-30.0, 55.0);
            cam.lat += (lat - cam.lat) * k;
            // and turns toward lit land when it rests over an empty ocean: the nearest of the
            // longitudes where the mold lives, when none is within 30 degrees
            let off = |a: f64| (a - cam.lon + 540.0).rem_euclid(360.0) - 180.0;
            let near = LIT_LONGITUDES.iter().map(|&a| off(a)).min_by(|a, b| a.abs().total_cmp(&b.abs())).unwrap_or(0.0);
            if near.abs() > 30.0 {
                cam.lon += near * k * 0.6;
            }
        }
        // a pinch past the closest view eases back to it
        if self.globe.hand.fingers() == 0 && self.globe.camera.zoom > MAX_ZOOM {
            let k = 1.0 - 0.5f64.powf(dt / 0.25);
            let z = self.globe.camera.zoom;
            self.globe.camera.zoom = z + (MAX_ZOOM - z) * k;
        }
        // the mold wakes slowly: the mist condenses over its first seconds, then it lives at its pace
        self.age += dt;
        let k = (self.age / 7.0).clamp(0.0, 1.0);
        let wake = 0.06 + 0.94 * k * k * (3.0 - 2.0 * k);
        let rate = if self.reduce_motion { RATE * 0.5 } else { RATE } * wake;
        self.owed = (self.owed + dt * rate).min(4.0);
        let n = self.owed.floor() as u32;
        self.owed -= n as f64;
        if n == 0 {
            return;
        }
        let gpu = &ctx.gpu;
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        for _ in 0..n {
            // each step its own params (the step's number seeds the agents' chances)
            let p = self.params();
            let staging = gpu.device.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("mold step"), contents: bytemuck::bytes_of(&p), usage: wgpu::BufferUsages::COPY_SRC });
            enc.copy_buffer_to_buffer(&staging, 0, &self.params, 0, std::mem::size_of::<Params>() as u64);
            let mut cp = enc.begin_compute_pass(&Default::default());
            cp.set_bind_group(0, &self.sim_binds[self.parity], &[]);
            cp.set_pipeline(&self.agents_pipe);
            cp.dispatch_workgroups(AGENTS.div_ceil(256), 1, 1);
            cp.set_pipeline(&self.diffuse_pipe);
            cp.dispatch_workgroups(W.div_ceil(16), H.div_ceil(16), 1);
            let nf = self.feeds.len().min(MAX_FEEDS) as u32;
            if nf > 0 {
                cp.set_pipeline(&self.feed_pipe);
                cp.dispatch_workgroups((4 * FEED_REACH).div_ceil(16), (2 * FEED_REACH).div_ceil(16), nf);
            }
            drop(cp);
            self.parity = 1 - self.parity;
            self.steps += 1;
        }
        let mut cp = enc.begin_compute_pass(&Default::default());
        // the latest field is now trails[parity]: blur it (binding 3 of the bind group that wrote it)
        cp.set_bind_group(0, &self.sim_binds[1 - self.parity], &[]);
        cp.set_pipeline(&self.blur_pipe);
        cp.dispatch_workgroups((W / GLOW).div_ceil(16), (H / GLOW).div_ceil(16), 1);
        drop(cp);
        gpu.queue.submit([enc.finish()]);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        self.globe.upload(&ctx.gpu);
        self.night.follow(&ctx.gpu, &self.globe);
        ctx.gpu.queue.write_buffer(&self.look, 0, bytemuck::bytes_of(&self.look()));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("mold"),
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
            self.night.draw(&mut rp, &self.globe);
            rp.set_pipeline(&self.draw_pipe);
            rp.set_bind_group(0, self.globe.view_bind(), &[]);
            rp.set_bind_group(1, &self.draw_binds[self.parity], &[]);
            rp.draw(0..3, 0..1);
            if self.streak_share() > 0.0 {
                rp.set_pipeline(&self.streak_pipe);
                rp.set_bind_group(1, &self.streak_bind, &[]);
                rp.draw(0..6, 0..AGENTS);
            }
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn demo(&self, _ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        // a tap on a lit city: a flake laid, the mold reaching for it
        let at = self.globe.spot_for(&[[2.35, 48.86], [9.19, 45.46], [-0.13, 51.51], [31.24, 30.04], [3.38, 6.52], [77.21, 28.61], [139.69, 35.69], [-74.0, 40.71], [-99.13, 19.43], [-46.63, -23.55]]);
        Some(toy_kit::info::Demo::new("A slime mold between the cities. Tap to feed it.", vec![toy_kit::info::Stroke::tap(0.5, at)]))
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
        self.globe.hand.reduce_motion = ctx.reduce_motion;
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&(&self.feeds, self.globe.camera)).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok((feeds, cam)) = serde_json::from_str::<(Vec<Feed>, Camera)>(saved) {
            // (laid before: long fed, on this launch's clock)
            self.feeds = feeds.into_iter().map(|f| Feed { at: self.seconds - 600.0, ..f }).collect();
            self.globe.camera = cam;
        }
    }
}

impl Mold {
    /// How far the agents' own streaks have taken over from the field: none at the opening zoom,
    /// all from about two and a half times closer.
    fn streak_share(&self) -> f32 {
        let z = self.globe.camera.zoom;
        let k = ((z - 1.8) / (4.0 - 1.8)).clamp(0.0, 1.0);
        (k * k * (3.0 - 2.0 * k)) as f32
    }

    fn look(&self) -> Look {
        let mut ages = [[0.0f32; 4]; MAX_FEEDS];
        for (i, f) in self.feeds.iter().rev().take(MAX_FEEDS).enumerate() {
            ages[i][0] = (self.seconds - f.at) as f32;
        }
        let mut ripples = [[0.0f32, 0.0, 99.0, 0.0]; MAX_RIPPLES];
        for (i, r) in self.ripples.iter().enumerate() {
            ripples[i] = [r.lon.to_radians() as f32, r.lat.to_radians() as f32, (self.seconds - r.at) as f32, if r.land { 1.0 } else { 0.0 }];
        }
        Look { a: [1.0, 0.125, self.streak_share(), H as f32], ages, ripples }
    }

    fn on_land(&self, at: LonLat) -> bool {
        let x = (((at.lon / 360.0 + 0.5) * W as f64).floor() as i64).rem_euclid(W as i64) as usize;
        let y = (((0.5 - at.lat / 180.0) * H as f64).floor() as i64).clamp(0, H as i64 - 1) as usize;
        let i = y * W as usize + x;
        self.land[i / 64] >> (i % 64) & 1 == 1
    }

    fn feed(&mut self, at: LonLat) {
        self.feeds.push(Feed { lon: at.lon, lat: at.lat, at: self.seconds });
        if self.feeds.len() > MAX_FEEDS {
            self.feeds.remove(0);
        }
    }
}

/// Mold's own night ground (night.wgsl): the kit's ground pass with a darker night, its textures the
/// globe's own.
struct Night {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    bind: wgpu::BindGroup,
    months: (u32, u32),
}

impl Night {
    fn new(gpu: &toy_kit::Gpu, globe: &Globe) -> Night {
        use toy_kit::globe::ground::{sampler, tex2d, tex_array};
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("mold night"),
            entries: &[tex2d(0), tex2d(1), tex2d(2), tex_array(3), tex2d(4), tex2d(5), sampler(6), tex2d(7)],
        });
        let shader = gpu.shader("mold night", &toy_kit::globe::wgsl(include_str!("night.wgsl")));
        let pipeline = gpu.fullscreen_pipeline("mold night", &shader, &[&globe.view_layout, &layout], None);
        let bind = Self::bind(gpu, &layout, globe);
        Night { pipeline, layout, bind, months: globe.textures.land_months }
    }

    fn bind(gpu: &toy_kit::Gpu, layout: &wgpu::BindGroupLayout, globe: &Globe) -> wgpu::BindGroup {
        fn tv(t: &toy_kit::gpu::Texture) -> wgpu::BindingResource<'_> {
            wgpu::BindingResource::TextureView(&t.view)
        }
        let tx = &globe.textures;
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mold night"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: tv(&tx.geo) },
                wgpu::BindGroupEntry { binding: 1, resource: tv(&tx.terrain) },
                wgpu::BindGroupEntry { binding: 2, resource: tv(&tx.floor) },
                wgpu::BindGroupEntry { binding: 3, resource: tv(&tx.ice) },
                wgpu::BindGroupEntry { binding: 4, resource: tv(&tx.land_a) },
                wgpu::BindGroupEntry { binding: 5, resource: tv(&tx.land_b) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(&tx.linear_wrap_x) },
                wgpu::BindGroupEntry { binding: 7, resource: tv(&tx.lights) },
            ],
        })
    }

    /// Keeps the bind group on the globe's land textures (they change when its months do).
    fn follow(&mut self, gpu: &toy_kit::Gpu, globe: &Globe) {
        if self.months != globe.textures.land_months {
            self.bind = Self::bind(gpu, &self.layout, globe);
            self.months = globe.textures.land_months;
        }
    }

    fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, globe: &'a Globe) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, globe.view_bind(), &[]);
        rp.set_bind_group(1, &self.bind, &[]);
        rp.draw(0..3, 0..1);
    }
}

toy_kit::ios_shell!(Mold);
