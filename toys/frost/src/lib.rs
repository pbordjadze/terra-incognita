//! Frost (a working title): ice flowers on a cabin window at Oymyakon, Yakutia, on a winter night at
//! about −50 °C. Touch the glass and a crystal starts there, grown by Reiter's hexagonal automaton for
//! snow crystals (frost.wgsl; SOURCES.md): each its own lattice angle, the wetter air low on the glass
//! growing ferns and the drier air high up plates, and two crystals meeting along a seam.
//!
//! All the simulation is GPU compute in `update`: up to eight crystals grow at once, each in its own
//! hex grid; the pane keeps who holds each patch of glass (the mask) and what the finished crystals
//! left (the field), which `draw` (pane.wgsl) lights. painting-company row 218.

mod sound;

use toy_kit::{info, Ctx, Phase, PointerEvent, Toy};
use wgpu::util::DeviceExt;

/// A grid's side, cells (frost.wgsl's N).
const N: u32 = 512;
const SLOTS: usize = 8;
/// Automaton steps a second: a fern's tip runs about a quarter cell a step.
const STEPS_PER_S: f64 = 300.0;
/// Field texels a point.
const FS: f32 = 2.0;
/// A fingertip's warmth melts this far, points.
const MELT: f32 = 34.0;
/// A drag starts another crystal every so many points.
const SPACING: f64 = 56.0;
/// A crystal stops being stepped once its tips have gone no further for this long (blocked by other
/// ice or the frame all round; its grid is then free), or at this age.
const QUIET_S: f64 = 1.5;
const MAX_AGE_S: f64 = 60.0;
/// The window: its frame's width and the crossbar's height (a share of the screen), points.
const FRAME: f64 = 14.0;
const BAR_AT: f64 = 0.36;
/// Left alone, the glass frosts from the frame by itself: a new seed every so many seconds (more
/// often low on the glass, where the air is wetter), until this many have started.
const CREEP_S: f64 = 3.2;
const CREEP_MAX: u32 = 24;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct SlotU {
    center: [f32; 2],
    rot: [f32; 2],
    id: u32,
    live: u32,
    _a: [f32; 2],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Globals {
    pane: [f32; 2],
    cell: f32,
    fs: f32,
    mask_w: u32,
    mask_h: u32,
    field_w: u32,
    field_h: u32,
    beta_top: f32,
    beta_bottom: f32,
    gamma_top: f32,
    gamma_bottom: f32,
    eps: f32,
    alpha: f32,
    frame: f32,
    bar: f32,
    slots: [SlotU; SLOTS],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Op {
    slot: u32,
    kind: u32,
    _a: [u32; 2],
    at: [f32; 2],
    radius: f32,
    _c: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct Look {
    size: [f32; 2],
    ppp: f32,
    time: f32,
    pane: [f32; 2],
    fs: f32,
    field_w: f32,
    field_h: f32,
    lamp: f32,
    frame: f32,
    bar: f32,
}

const OP_SEED: u32 = 1;
const OP_MELT: u32 = 2;

/// One crystal's grid, as the CPU keeps it.
#[derive(Clone, Copy, Default)]
struct Slot {
    id: u32,
    center: [f64; 2],
    angle: f64,
    live: bool,
    born: f64,
    /// When it last froze a cell.
    grew: f64,
    /// Cells frozen in all (the GPU's count at the last readback).
    frozen: u32,
    /// How far its tips have gone (cells), and when they last went further.
    reach: u32,
    advanced: f64,
}

struct Pending {
    at: [f64; 2],
    /// A finger's: its warmth melts the glass first.
    warm: bool,
}

pub struct Frost {
    globals: wgpu::Buffer,
    op: wgpu::Buffer,
    look: wgpu::Buffer,
    stats: wgpu::Buffer,
    readback: wgpu::Buffer,
    binds: [wgpu::BindGroup; 2],
    draw_bind: wgpu::BindGroup,
    grow: wgpu::ComputePipeline,
    claim: wgpu::ComputePipeline,
    seed: wgpu::ComputePipeline,
    melt_cells: wgpu::ComputePipeline,
    melt_pane: wgpu::ComputePipeline,
    paint: wgpu::ComputePipeline,
    draw: wgpu::RenderPipeline,
    parity: usize,
    owed: f64,
    slots: [Slot; SLOTS],
    next_id: u32,
    pending: Vec<Pending>,
    /// The pane in points, the cell's size in points, the allocation's sides (mask texels; field texels).
    pane: [f64; 2],
    cell: f64,
    mask_side: u32,
    field_side: u32,
    time: f64,
    /// A finger down: where it last started a crystal.
    finger: Option<(u64, [f64; 2])>,
    rng: u64,
    crackle: sound::Crackle,
    /// Haptics: freezing owed toward the next tick, and when the last one was.
    crack: f64,
    last_tick: f64,
    reduce_motion: bool,
    /// The frost creeping in from the frame by itself: seeds started, and when the next is due.
    crept: u32,
    next_creep: f64,
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

impl Frost {
    fn rand(&mut self) -> f64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng >> 11) as f64 / (1u64 << 53) as f64
    }

    fn globals(&self) -> Globals {
        let mut slots = [SlotU::default(); SLOTS];
        for (u, s) in slots.iter_mut().zip(&self.slots) {
            *u = SlotU {
                center: [s.center[0] as f32, s.center[1] as f32],
                rot: [s.angle.cos() as f32, s.angle.sin() as f32],
                id: s.id,
                live: s.live as u32,
                _a: [0.0; 2],
            };
        }
        Globals {
            pane: [self.pane[0] as f32, self.pane[1] as f32],
            cell: self.cell as f32,
            fs: FS,
            mask_w: self.mask_side,
            mask_h: self.mask_side,
            field_w: self.field_side,
            field_h: self.field_side,
            // high on the glass the air is drier: compact plates; low, wetter: ferns (Reiter's phase
            // diagram; tools/toys/frost/reiter.py)
            beta_top: 0.46,
            beta_bottom: 0.74,
            gamma_top: 0.0016,
            gamma_bottom: 0.0005,
            eps: 0.3,
            alpha: 1.0,
            frame: FRAME as f32,
            bar: (self.pane[1] * BAR_AT) as f32,
            slots,
        }
    }

    /// A crystal at `at` (points): the finger's warmth melts the glass there, then a seed freezes at
    /// its middle, at a lattice angle of its own.
    fn start(&mut self, at: [f64; 2]) {
        if self.on_glass(at) {
            self.pending.push(Pending { at, warm: true });
        }
    }

    fn on_glass(&self, p: [f64; 2]) -> bool {
        let f = FRAME;
        p[0] >= f && p[1] >= f && p[0] < self.pane[0] - f && p[1] < self.pane[1] - f && (p[1] - self.pane[1] * BAR_AT).abs() > 0.5 * f
    }

    /// A seed where the glass meets the frame (or the crossbar), low more often than high.
    fn creep(&mut self) {
        let [w, h] = self.pane;
        let f = FRAME + 1.5;
        let at = loop {
            let u = self.rand();
            let v = self.rand();
            // which edge: the bottom and the crossbar's underside are the wettest
            let e = self.rand();
            let p = if e < 0.30 {
                [f + u * (w - 2.0 * f), h - f]
            } else if e < 0.45 {
                [f + u * (w - 2.0 * f), h * BAR_AT + 0.5 * FRAME + 1.5]
            } else if e < 0.55 {
                [f + u * (w - 2.0 * f), h * BAR_AT - 0.5 * FRAME - 1.5]
            } else if e < 0.62 {
                [f + u * (w - 2.0 * f), f]
            } else {
                // a side, lower more often
                [if u < 0.5 { f } else { w - f }, f + (1.0 - v * v) * (h - 2.0 * f)]
            };
            if self.on_glass(p) {
                break p;
            }
        };
        self.pending.push(Pending { at, warm: false });
    }

    fn live(&self) -> usize {
        self.slots.iter().filter(|s| s.live).count()
    }
}

impl Toy for Frost {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let gpu = &ctx.gpu;
        let dev = &gpu.device;
        let pts = ctx.points();
        let pane = pts;
        // a cell is a point on a phone, a little more on a larger screen (the crystals keep their
        // share of the glass)
        let cell = (pts[0].min(pts[1]) / 393.0).clamp(1.0, 1.4);
        // room for either orientation
        let long = pts[0].max(pts[1]);
        let mask_side = (long / cell).ceil() as u32 + 2;
        let field_side = (long * FS as f64).ceil() as u32 + 2;

        toy_kit::step("frost: buffers");
        let st = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let grid_bytes = SLOTS as u64 * (N * N) as u64 * 4;
        let grids = [0, 1].map(|i| gpu.buffer(&wgpu::BufferDescriptor { label: Some(if i == 0 { "frost grid a" } else { "frost grid b" }), size: grid_bytes, usage: st, mapped_at_creation: false }));
        let mask_len = (mask_side * mask_side) as usize;
        let mask = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor { label: Some("frost mask"), contents: &vec![0xffu8; mask_len * 4], usage: st });
        toy_kit::budget::count(mask_len as u64 * 4);
        let field = gpu.buffer(&wgpu::BufferDescriptor { label: Some("frost field"), size: (field_side * field_side) as u64 * 4, usage: st, mapped_at_creation: false });
        let stats = gpu.buffer(&wgpu::BufferDescriptor { label: Some("frost stats"), size: SLOTS as u64 * 16, usage: st | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let readback = gpu.buffer(&wgpu::BufferDescriptor { label: Some("frost readback"), size: SLOTS as u64 * 16, usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let ub = |label, size: usize| dev.create_buffer(&wgpu::BufferDescriptor { label: Some(label), size: size as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let globals = ub("frost globals", std::mem::size_of::<Globals>());
        let op = ub("frost op", std::mem::size_of::<Op>());
        let look = ub("frost look", std::mem::size_of::<Look>());

        toy_kit::step("frost: the automaton's pipelines");
        let cs = wgpu::ShaderStages::COMPUTE;
        let layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frost sim"),
            entries: &[uniform(0, cs), storage(1, true, cs), storage(2, false, cs), storage(3, false, cs), storage(4, false, cs), storage(5, false, cs), uniform(6, cs)],
        });
        let binds = [0usize, 1].map(|p| {
            dev.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("frost sim"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: globals.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: grids[p].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 2, resource: grids[1 - p].as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 3, resource: mask.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 4, resource: stats.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 5, resource: field.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 6, resource: op.as_entire_binding() },
                ],
            })
        });
        let shader = gpu.shader("frost sim", include_str!("frost.wgsl"));
        let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("frost sim"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
        let compute = |entry: &str| {
            dev.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pl),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        let (grow, claim, seed, melt_cells, melt_pane, paint) = (compute("grow"), compute("claim"), compute("seed"), compute("melt_cells"), compute("melt_pane"), compute("paint"));

        toy_kit::step("frost: the window's pipeline");
        let fs = wgpu::ShaderStages::FRAGMENT;
        let draw_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("frost draw"), entries: &[uniform(0, fs), storage(1, true, fs)] });
        let draw_bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frost draw"),
            layout: &draw_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: look.as_entire_binding() }, wgpu::BindGroupEntry { binding: 1, resource: field.as_entire_binding() }],
        });
        let draw_shader = gpu.shader("frost pane", include_str!("pane.wgsl"));
        let draw = gpu.fullscreen_pipeline("frost pane", &draw_shader, &[&draw_layout], None);
        toy_kit::step("frost: made");

        Ok(Frost {
            globals,
            op,
            look,
            stats,
            readback,
            binds,
            draw_bind,
            grow,
            claim,
            seed,
            melt_cells,
            melt_pane,
            paint,
            draw,
            parity: 0,
            owed: 0.0,
            slots: [Slot::default(); SLOTS],
            next_id: 1,
            pending: Vec::new(),
            pane,
            cell,
            mask_side,
            field_side,
            time: 0.0,
            finger: None,
            rng: 0x9e37_79b9_7f4a_7c15,
            crackle: sound::Crackle::default(),
            crack: 0.0,
            last_tick: f64::NEG_INFINITY,
            reduce_motion: ctx.reduce_motion,
            crept: 0,
            next_creep: 0.0,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        let p = ctx.points();
        // (the allocation is square, for either orientation; a larger screen than it was made for
        // keeps the glass it has)
        let cap = (self.mask_side - 2) as f64 * self.cell;
        self.pane = [p[0].min(cap), p[1].min(cap)];
    }

    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => {
                if self.finger.is_none() {
                    self.finger = Some((e.id, [e.x, e.y]));
                    self.start([e.x, e.y]);
                }
            }
            Phase::Move => {
                if let Some((id, last)) = self.finger {
                    if id == e.id && (e.x - last[0]).hypot(e.y - last[1]) >= SPACING {
                        self.finger = Some((id, [e.x, e.y]));
                        self.start([e.x, e.y]);
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                if self.finger.is_some_and(|(id, _)| id == e.id) {
                    self.finger = None;
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.time = t;
        // the opening: frost creeping in from the frame's lower corner, then more along the frame,
        // a seed every few seconds, by itself
        if self.crept == 0 && t > 0.6 {
            self.crept = 1;
            self.next_creep = t + CREEP_S;
            self.pending.push(Pending { at: [FRAME + 1.5, self.pane[1] - FRAME - 1.5], warm: false });
        } else if self.crept > 0 && self.crept < CREEP_MAX && t > self.next_creep {
            self.crept += 1;
            let r = self.rand();
            self.next_creep = t + CREEP_S * (0.6 + 0.8 * r);
            self.creep();
        }
        let gpu = &ctx.gpu;
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        // one new crystal a frame (each needs its own op)
        let mut op = None;
        if !self.pending.is_empty() {
            let p = self.pending.remove(0);
            // a free grid, or the one whose tips have stalled longest (it stops where it is)
            let k = (0..SLOTS).find(|&k| !self.slots[k].live).unwrap_or_else(|| (0..SLOTS).min_by(|&a, &b| self.slots[a].advanced.total_cmp(&self.slots[b].advanced)).unwrap());
            let angle = self.rand() * std::f64::consts::FRAC_PI_3;
            let wobble = self.rand() as f32 * 6.28;
            self.slots[k] = Slot { id: self.next_id, center: p.at, angle, live: true, born: t, grew: t, frozen: 0, reach: 0, advanced: t };
            self.next_id += 1;
            let kind = if p.warm { OP_SEED | OP_MELT } else { OP_SEED };
            op = Some(Op { slot: k as u32, kind, _a: [0; 2], at: [p.at[0] as f32, p.at[1] as f32], radius: MELT as f32, _c: wobble });
        }
        gpu.queue.write_buffer(&self.globals, 0, bytemuck::bytes_of(&self.globals()));
        let wg = N.div_ceil(8);
        if let Some(o) = op {
            gpu.queue.write_buffer(&self.op, 0, bytemuck::bytes_of(&o));
            let mut cp = enc.begin_compute_pass(&Default::default());
            // the warm fingertip first (every live crystal's cells there, the pane's claim and ice),
            // then the new grid; each writes binding 2, so once a parity
            for p in 0..2 {
                cp.set_bind_group(0, &self.binds[p], &[]);
                cp.set_pipeline(&self.melt_cells);
                cp.dispatch_workgroups(wg, wg, SLOTS as u32);
                cp.set_pipeline(&self.seed);
                cp.dispatch_workgroups(wg, wg, 1);
            }
            let span = ((2.8 * o.radius + 6.0) * FS).ceil() as u32;
            cp.set_pipeline(&self.melt_pane);
            cp.dispatch_workgroups(span.div_ceil(8), span.div_ceil(8), 1);
        }
        // the automaton, at its own pace (slower under Reduce Motion)
        let rate = if self.reduce_motion { STEPS_PER_S * 0.4 } else { STEPS_PER_S };
        self.owed = (self.owed + dt * rate).min(40.0);
        let n = self.owed.floor() as u32;
        self.owed -= n as f64;
        if self.live() > 0 {
            let mut cp = enc.begin_compute_pass(&Default::default());
            for _ in 0..n {
                cp.set_bind_group(0, &self.binds[self.parity], &[]);
                cp.set_pipeline(&self.grow);
                cp.dispatch_workgroups(wg, wg, SLOTS as u32);
                cp.set_pipeline(&self.claim);
                cp.dispatch_workgroups(wg, wg, SLOTS as u32);
                self.parity = 1 - self.parity;
            }
            // the pane's ice from the latest grids (now binding 1 of this parity)
            cp.set_bind_group(0, &self.binds[self.parity], &[]);
            cp.set_pipeline(&self.paint);
            let (fw, fh) = ((self.pane[0] * FS as f64).ceil() as u32, (self.pane[1] * FS as f64).ceil() as u32);
            cp.dispatch_workgroups(fw.div_ceil(8), fh.div_ceil(8), 1);
        }
        enc.copy_buffer_to_buffer(&self.stats, 0, &self.readback, 0, SLOTS as u64 * 16);
        gpu.queue.submit([enc.finish()]);

        // what froze this frame (read back at once: a few bytes, and the same on every run)
        let slice = self.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = gpu.device.poll(wgpu::PollType::Wait);
        let counts: Vec<u32> = bytemuck::cast_slice(&slice.get_mapped_range()).to_vec();
        self.readback.unmap();
        let mut frozen_now = 0.0;
        let mut pan_w = 0.0;
        for k in 0..SLOTS {
            let s = &mut self.slots[k];
            if !s.live {
                continue;
            }
            let total = counts[k * 4 + 1];
            let new = total.saturating_sub(s.frozen);
            s.frozen = total;
            if new > 0 {
                s.grew = t;
            }
            frozen_now += new as f64;
            pan_w += new as f64 * (s.center[0] / self.pane[0] * 2.0 - 1.0);
            if counts[k * 4] > s.reach {
                s.reach = counts[k * 4];
                s.advanced = t;
            }
            // done: its tips blocked all round (by other ice or the frame) for a while, or old
            if (t - s.advanced > QUIET_S && t - s.born > 1.0) || t - s.born > MAX_AGE_S || s.reach as i64 >= (N / 2) as i64 - 6 {
                s.live = false;
            }
        }
        if std::env::var("FROST_LOG").is_ok() {
            eprintln!("t {t:.2} live {} frozen {frozen_now}", self.live());
        }
        // the crackle: as many clicks as cells freezing, softened (a fern's run is a rush, a plate's
        // slow spread a scatter)
        let per_s = if dt > 0.0 { frozen_now / dt } else { 0.0 };
        let clicks = (per_s.sqrt() * 2.2).min(260.0);
        let pan = if frozen_now > 0.0 { pan_w / frozen_now } else { 0.0 };
        self.crackle.set(clicks as f32, pan as f32);
        // and in the hand: a light tick for every so much new ice, never faster than 14 a second
        self.crack += frozen_now;
        if self.crack > 260.0 && t - self.last_tick > 0.07 {
            ctx.haptic();
            self.crack = 0.0;
            self.last_tick = t;
        }
        self.crack *= 0.5f64.powf(dt / 0.4);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let look = Look {
            size: [ctx.width as f32, ctx.height as f32],
            ppp: ctx.ppp as f32,
            time: self.time as f32,
            pane: [self.pane[0] as f32, self.pane[1] as f32],
            fs: FS,
            field_w: self.field_side as f32,
            field_h: self.field_side as f32,
            lamp: 1.0,
            frame: FRAME as f32,
            bar: (self.pane[1] * BAR_AT) as f32,
        };
        ctx.gpu.queue.write_buffer(&self.look, 0, bytemuck::bytes_of(&look));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frost"),
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
            rp.set_pipeline(&self.draw);
            rp.set_bind_group(0, &self.draw_bind, &[]);
            rp.draw(0..3, 0..1);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(self.crackle.synth())
    }

    fn shaken(&mut self, _ctx: &mut Ctx) {}

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let at = [self.pane[0] * 0.55, self.pane[1] * 0.45];
        Some(info::Demo::new("Frost on a window at −50 °C. Touch the glass.", vec![info::Stroke::tap(0.4, at)]))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "The glass is a honeycomb of cells. A cell that is ice, or touches ice, takes water from the air.",
                "Everywhere else the water spreads to the six neighbours. That alone grows six arms.",
                "The air is wetter low on the glass: long ferns race there; higher up, slower, stubbier stars.",
                "Two crystals can't share water: they meet in a seam. Left alone, frost creeps in from the frame.",
            ],
            "Real: Reiter's model of how snow crystals grow. Drawn: the night, the glass, the lamp, the sound.",
        ))
    }
}

toy_kit::ios_shell!(Frost);
