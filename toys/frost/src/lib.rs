//! Frost (a working title): ice flowers on a cabin window at Oymyakon, Yakutia, on a winter night at
//! about −50 °C. The warm finger melts what it wipes, and the wipe stays (the glaze: frost creeps back
//! only from its edge, healing over a couple of minutes); a shake clears the window for the frost to
//! come again. Each crystal is grown by Reiter's hexagonal automaton for snow crystals (frost.wgsl;
//! SOURCES.md): its own angle, the wetter air low on the glass
//! growing ferns and the drier air high up plates, and two crystals meeting along a seam.
//!
//! All the simulation is GPU compute in `update`: up to twelve crystals grow at once, each in its own
//! hex grid; the pane keeps who holds each patch of glass (the mask) and what the finished crystals
//! left (the field), and how deep the glaze is (where a melt froze smooth), which `draw` (pane.wgsl) lights. painting-company row 218.

mod sound;

use toy_kit::{info, Ctx, Phase, PointerEvent, Toy};
use wgpu::util::DeviceExt;

/// A grid's side, cells (frost.wgsl's N).
const N: u32 = 512;
const SLOTS: usize = 12;
/// Automaton steps a second: a fern's tip runs about a quarter cell a step.
const STEPS_PER_S: f64 = 230.0;
/// Field texels a point.
const FS: f32 = 2.0;
/// A fingertip's warmth melts this far at once, and as far as this while it rests (over MELT_S).
const MELT: f64 = 30.0;
const MELT_MAX: f64 = 72.0;
const MELT_S: f64 = 1.6;
/// A crystal stops being stepped once its tips have gone no further for this long (blocked by other
/// ice or the frame all round; its grid is then free), or at this age.
const QUIET_S: f64 = 1.5;
const MAX_AGE_S: f64 = 12.0;
/// The window: its frame's width and the crossbar's height (a share of the screen), points.
const FRAME: f64 = 14.0;
const BAR_AT: f64 = 0.36;
/// Left alone, the glass frosts from the frame by itself: a new seed every so many seconds (more
/// often low on the glass, where the air is wetter), until this many have started.
const CREEP_S: f64 = 0.7;
const CREEP_MAX: u32 = 44;
/// A wipe stays: the meltwater freezes smooth, and frost creeps back into it only this deep from its
/// edge (points, on average) at first; then the holes heal, the glaze this much shallower a second,
/// the frost's edge following, so a mark lasts a couple of minutes.
const FRINGE: f64 = 14.0;
const RELAX_PER_S: f64 = 0.55;
/// How often a healing hole's edge is seeded again (seconds), and from what age.
const RENEW_S: f64 = 0.9;
const RENEW_AFTER_S: f64 = 10.0;
/// The stove's warmth (a shake): the clearing's speed, points a second.
const SWEEP_SPEED: f64 = 900.0;

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
    fringe: f32,
    relax: f32,
    _g: [f32; 2],
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
    to: [f32; 2],
    _d: [f32; 2],
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
const OP_PROBE: u32 = 8;
const OP_CLEAR: u32 = 16;
/// The stats buffer: four words a slot, then the probe's (ice samples, samples, the glaze's depth there).
const PROBE_AT: usize = SLOTS * 4;

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

/// A crystal to start, not before a time.
struct Pending {
    at: [f64; 2],
    after: f64,
    /// Its ping as it starts (loudness, 0 for none) and the scale's steps above the height's note.
    chime: f32,
    lift: i32,
    /// Or the scale's step it rings, whatever its height.
    step: Option<i32>,
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    /// Set down this frame: what is under it is being read.
    Undecided,
    /// Read: it melts.
    Melt,
}

/// A melt that is healing: where it was, how wide, since when.
struct Mark {
    /// Its way, each point with the melt's width there.
    path: Vec<[f64; 3]>,
    radius: f64,
    born: f64,
}

/// The finger on the glass: warm. It melts its way, widening where it rests; lifted, the wet glass
/// freezes again from the melt's edges.
struct Finger {
    id: u64,
    at: [f64; 2],
    /// Where the melt has reached along its way.
    melted_to: [f64; 2],
    /// Since when it has rested (within a few points).
    rest_at: [f64; 2],
    rest_since: f64,
    /// Its way, a point every few points, and the widest the melt has been.
    path: Vec<[f64; 2]>,
    radii: Vec<f64>,
    radius: f64,
    lifted: bool,
    mode: Mode,
    /// How much of the glass round it was ice when it was set down.
    ice: f64,
    /// Where it was last frame, for its speed (points a second, smoothed).
    last: [f64; 2],
    speed: f64,
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
    probe: wgpu::ComputePipeline,
    clear: wgpu::ComputePipeline,
    relax: wgpu::ComputePipeline,
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
    finger: Option<Finger>,
    rng: u64,
    crackle: sound::Crackle,
    /// Haptics: freezing owed toward the next tick, and when the last one was.
    crack: f64,
    last_tick: f64,
    reduce_motion: bool,
    /// The frost creeping in from the frame by itself: seeds started, and when the next is due.
    crept: u32,
    next_creep: f64,
    /// When the frost last began creeping in from the frame (the opening, or after a shake).
    creep_from: f64,
    /// The melts healing, and when the next of their edges is seeded.
    marks: Vec<Mark>,
    next_renew: f64,
    /// When the frame or a healing hole last chimed.
    last_chime: f64,
    /// A resting finger's drips: the next one's time, how many so far, one to play.
    next_drip: f64,
    drips: usize,
    drip: Option<(i32, f64)>,
    /// A shake's clearing: since when, from where.
    sweep: Option<(f64, [f64; 2])>,
    sweep_asked: bool,
}

fn storage(binding: u32, read_only: bool, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn uniform(binding: u32, vis: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: vis,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
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
            beta_top: 0.45,
            beta_bottom: 0.74,
            gamma_top: 0.001,
            gamma_bottom: 0.0005,
            eps: 0.3,
            alpha: 1.0,
            frame: FRAME as f32,
            bar: (self.pane[1] * BAR_AT) as f32,
            fringe: FRINGE as f32,
            relax: 0.0,
            _g: [0.0; 2],
            slots,
        }
    }

    /// A crystal at `at` (points): the finger's warmth melts the glass there, then a seed freezes at
    /// its middle, at a lattice angle of its own.
    /// It rings as it starts: `chime` loud (0: silent), `lift` steps above its height's note.
    fn start(&mut self, at: [f64; 2], after: f64, chime: f32, lift: i32) {
        if self.on_glass(at) {
            self.pending.push(Pending {
                at,
                after,
                chime,
                lift,
                step: None,
            });
        }
    }

    /// Where on the pane a point is, as the sound has it: its note and its pan.
    fn voice_at(&self, p: [f64; 2], lift: i32) -> (f32, f32) {
        let f = sound::note(sound::step_at(p[1] / self.pane[1].max(1.0)) + lift);
        (f, (p[0] / self.pane[0].max(1.0) * 2.0 - 1.0) as f32 * 0.7)
    }

    /// A point just inside a melt's edge (`inset` points in), at random: the print's edge wanders with
    /// the angle as frost.wgsl's melt_radius has it, and a point deeper inside another part of the
    /// stroke is not on its edge.
    fn edge_point(&mut self, path: &[[f64; 3]], inset: f64) -> Option<([f64; 2], usize)> {
        let wob = |a: f64, r: f64| {
            r * (1.0
                + 0.16 * (3.0 * a + 1.7).sin()
                + 0.10 * (5.0 * a + 3.4).sin()
                + 0.07 * (8.0 * a + 5.27).sin())
        };
        for _ in 0..10 {
            let k = ((self.rand() * path.len() as f64) as usize).min(path.len() - 1);
            let a = self.rand() * std::f64::consts::TAU - std::f64::consts::PI;
            let c = path[k];
            let d = wob(a, c[2]) - inset;
            if d < 3.0 {
                continue;
            }
            let p = [c[0] + d * a.cos(), c[1] + d * a.sin()];
            let deepest = path
                .iter()
                .map(|q| {
                    wob((p[1] - q[1]).atan2(p[0] - q[0]), q[2]) - (p[0] - q[0]).hypot(p[1] - q[1])
                })
                .fold(f64::MIN, f64::max);
            if deepest <= inset + 1.5 {
                return Some((p, k));
            }
        }
        None
    }

    /// Lifted from ice: seeds along the melt's very edge, a beat later and one after another, each at
    /// its own angle; they creep in from the rim a fringe deep, and the middle stays clear.
    fn regrow(&mut self, f: &Finger, t: f64) {
        let path: Vec<[f64; 3]> = f
            .path
            .iter()
            .zip(&f.radii)
            .map(|(p, r)| [p[0], p[1], *r])
            .collect();
        let len: f64 = f
            .path
            .windows(2)
            .map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]))
            .sum();
        let n = ((len + f.radius * 4.0) / 16.0).clamp(6.0, 18.0) as usize;
        let mut seeds = Vec::new();
        for _ in 0..n * 2 {
            if seeds.len() == n {
                break;
            }
            if let Some(p) = self.edge_point(&path, 1.0) {
                seeds.push(p);
            }
        }
        // in the order the stroke was drawn, so the window answers the gesture as it was made: each
        // seed a ping an octave over the stroke's note there, the phrase fading as it goes
        seeds.sort_by_key(|s| s.1);
        // the answer: the notes the stroke crossed, each once, in the order it crossed them, an octave
        // up; a tap's (one note) rises through the chord from it. Each rings with the seed nearest
        // its place in the phrase; the other seeds grow silently.
        let h = self.pane[1].max(1.0);
        let mut phrase: Vec<i32> = Vec::new();
        for p in &f.path {
            let st = sound::step_at(p[1] / h);
            if phrase.last() != Some(&st) {
                phrase.push(st);
            }
        }
        if phrase.len() > 7 {
            let n = phrase.len();
            phrase = (0..7).map(|i| phrase[i * (n - 1) / 6]).collect();
        }
        if phrase.len() == 1 {
            let s0 = phrase[0];
            phrase = vec![s0, s0 + 2, s0 + 4];
        }
        let beat = if f.ice < 0.05 { 0.05 } else { 0.6 };
        let gap = 0.08;
        let (n, m) = (seeds.len(), phrase.len());
        let mut j = 0;
        for (i, (p, _)) in seeds.iter().enumerate() {
            // (a beat first: the clear wet glass, then the lace creeping in from its rim; on dry
            // glass there is nothing to see in the meantime, so at once)
            let after = t + beat + gap * i as f64;
            if j < m && i * m >= j * n {
                let fade = 1.0 - 0.45 * j as f32 / m as f32;
                self.pending.push(Pending {
                    at: *p,
                    after,
                    chime: 0.11 * fade,
                    lift: 5,
                    step: Some(phrase[j] + 5),
                });
                j += 1;
            } else {
                self.start(*p, after, 0.0, 0);
            }
        }
        self.marks.push(Mark {
            path,
            radius: f.radius,
            born: t,
        });
        if self.marks.len() > 40 {
            self.marks.remove(0);
        }
    }

    /// A healing hole: a seed just inside where its frost may reach by now, so the edge follows.
    fn renew(&mut self, t: f64) {
        self.marks
            .retain(|m| FRINGE + RELAX_PER_S * (t - m.born) < m.radius * 1.2);
        let old: Vec<usize> = (0..self.marks.len())
            .filter(|&i| t - self.marks[i].born > RENEW_AFTER_S)
            .collect();
        if old.is_empty() {
            return;
        }
        let i = old[(self.rand() * old.len() as f64) as usize % old.len()];
        let inset = RELAX_PER_S * (t - self.marks[i].born) + 0.3 * FRINGE;
        let path = std::mem::take(&mut self.marks[i].path);
        let p = self.edge_point(&path, inset);
        self.marks[i].path = path;
        if let Some((p, _)) = p {
            // (a healing hole chimes now and then, faintly)
            self.start(p, t, 0.035, 5);
        }
    }

    fn on_glass(&self, p: [f64; 2]) -> bool {
        let f = FRAME;
        let [w, h] = self.pane;
        // (the help marks' corner stays clear: frost.wgsl's on_glass)
        let tx = ((p[0] - (w - 70.0)) / 44.0).clamp(0.0, 1.0);
        let corner = (p[0] - (w - 70.0 + 44.0 * tx)).hypot(p[1] - (h - 34.0)) <= 32.0;
        p[0] >= f
            && p[1] >= f
            && p[0] < w - f
            && p[1] < h - f
            && (p[1] - h * BAR_AT).abs() > 0.5 * f
            && !corner
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
            let p = if e < 0.20 {
                [f + u * (w - 2.0 * f), h - f]
            } else if e < 0.30 {
                [f + u * (w - 2.0 * f), h * BAR_AT + 0.5 * FRAME + 1.5]
            } else if e < 0.46 {
                [f + u * (w - 2.0 * f), h * BAR_AT - 0.5 * FRAME - 1.5]
            } else if e < 0.62 {
                [f + u * (w - 2.0 * f), f]
            } else {
                // a side, lower a little more often
                [
                    if u < 0.5 { f } else { w - f },
                    f + (1.0 - 0.6 * v * v) * (h - 2.0 * f),
                ]
            };
            if self.on_glass(p) {
                break p;
            }
        };
        let now = self.time;
        // (the frost coming in from the frame chimes softly, low)
        self.pending.push(Pending {
            at,
            after: now,
            chime: 0.05,
            lift: 0,
            step: None,
        });
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
        let grids = [0, 1].map(|i| {
            gpu.buffer(&wgpu::BufferDescriptor {
                label: Some(if i == 0 {
                    "frost grid a"
                } else {
                    "frost grid b"
                }),
                size: grid_bytes,
                usage: st,
                mapped_at_creation: false,
            })
        });
        let mask_len = (mask_side * mask_side) as usize;
        let mask = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frost mask"),
            contents: &vec![0xffu8; mask_len * 4],
            usage: st | wgpu::BufferUsages::COPY_SRC,
        });
        toy_kit::budget::count(mask_len as u64 * 4);
        let field = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("frost field"),
            size: (field_side * field_side) as u64 * 4,
            usage: st,
            mapped_at_creation: false,
        });
        let glaze = dev.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("frost glaze"),
            contents: &vec![0u8; mask_len * 4],
            usage: st,
        });
        toy_kit::budget::count(mask_len as u64 * 4);
        let stats = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("frost stats"),
            size: SLOTS as u64 * 16 + 16,
            usage: st | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("frost readback"),
            size: SLOTS as u64 * 16 + 16,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let ub = |label, size: usize| {
            dev.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: size as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        };
        let globals = ub("frost globals", std::mem::size_of::<Globals>());
        let op = ub("frost op", std::mem::size_of::<Op>());
        let look = ub("frost look", std::mem::size_of::<Look>());

        toy_kit::step("frost: the automaton's pipelines");
        let cs = wgpu::ShaderStages::COMPUTE;
        let layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frost sim"),
            entries: &[
                uniform(0, cs),
                storage(1, true, cs),
                storage(2, false, cs),
                storage(3, false, cs),
                storage(4, false, cs),
                storage(5, false, cs),
                uniform(6, cs),
                storage(7, false, cs),
            ],
        });
        let binds = [0usize, 1].map(|p| {
            dev.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("frost sim"),
                layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: globals.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: grids[p].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: grids[1 - p].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: mask.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: stats.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: field.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: op.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: glaze.as_entire_binding(),
                    },
                ],
            })
        });
        let shader = gpu.shader("frost sim", include_str!("frost.wgsl"));
        let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("frost sim"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
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
        let (grow, claim, seed, melt_cells, melt_pane, paint) = (
            compute("grow"),
            compute("claim"),
            compute("seed"),
            compute("melt_cells"),
            compute("melt_pane"),
            compute("paint"),
        );
        let (probe, clear, relax) = (compute("probe"), compute("clear"), compute("relax"));

        toy_kit::step("frost: the window's pipeline");
        let fs = wgpu::ShaderStages::FRAGMENT;
        let draw_layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("frost draw"),
            entries: &[uniform(0, fs), storage(1, true, fs)],
        });
        let draw_bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frost draw"),
            layout: &draw_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: look.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: field.as_entire_binding(),
                },
            ],
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
            probe,
            clear,
            relax,
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
            creep_from: 0.0,
            marks: Vec::new(),
            next_renew: 0.0,
            last_chime: -10.0,
            next_drip: 0.0,
            drips: 0,
            drip: None,
            sweep: None,
            sweep_asked: false,
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
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                if self.finger.is_none() && self.on_glass(p) {
                    self.finger = Some(Finger {
                        id: e.id,
                        at: p,
                        melted_to: p,
                        rest_at: p,
                        rest_since: e.t,
                        path: vec![p],
                        radii: vec![MELT],
                        radius: MELT,
                        lifted: false,
                        mode: Mode::Undecided,
                        ice: 0.0,
                        last: p,
                        speed: 0.0,
                    });
                }
            }
            Phase::Move => {
                if let Some(f) = &mut self.finger {
                    if f.id == e.id && !f.lifted {
                        f.at = [e.x.clamp(0.0, self.pane[0]), e.y.clamp(0.0, self.pane[1])];
                        if (f.at[0] - f.rest_at[0]).hypot(f.at[1] - f.rest_at[1]) > 6.0 {
                            f.rest_at = f.at;
                            f.rest_since = e.t;
                        }
                        let last = *f.path.last().unwrap();
                        if (f.at[0] - last[0]).hypot(f.at[1] - last[1]) > 8.0 {
                            f.path.push(f.at);
                        }
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                if let Some(f) = &mut self.finger {
                    if f.id == e.id {
                        f.lifted = true;
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.time = t;
        // a shake: the stove's warmth sweeps the window clear from the lamp's side, then the frost
        // creeps in from the frame again, as at the opening
        if self.sweep_asked {
            self.sweep_asked = false;
            if self.sweep.is_none() {
                self.sweep = Some((t, [self.pane[0] * 0.18, self.pane[1] * 1.05]));
                for s in &mut self.slots {
                    s.live = false;
                }
                self.pending.clear();
                self.marks.clear();
                self.finger = None;
                self.crept = CREEP_MAX;
                self.crackle.play(sound::Note::Rush);
                self.crackle.touch(0.0, 0.0, 0.0, 0.0, 0.0);
            }
        }
        let mut sweep_r = None;
        if let Some((t0, c)) = self.sweep {
            let r = (t - t0) * SWEEP_SPEED;
            let far = [0.0, self.pane[0]]
                .iter()
                .flat_map(|&x| [0.0, self.pane[1]].map(|y| (x - c[0]).hypot(y - c[1])))
                .fold(0.0, f64::max);
            sweep_r = Some((c, r));
            if r > far + 70.0 {
                self.sweep = None;
                self.crept = 0;
                self.creep_from = t + 1.2;
            }
        }
        // the opening: frost creeping in from the frame's lower corner, then more along the frame,
        // a seed every few seconds, by itself
        let since = t - self.creep_from;
        if self.crept == 0 && since > 0.6 {
            self.crept = 1;
            self.next_creep = t + CREEP_S;
            self.pending.push(Pending {
                at: [FRAME + 1.5, self.pane[1] - FRAME - 1.5],
                after: t,
                chime: 0.05,
                lift: 0,
                step: None,
            });
        } else if self.crept > 0 && self.crept < CREEP_MAX && t > self.next_creep {
            self.crept += 1;
            let r = self.rand();
            // quick at first (the glass takes about twenty seconds to frost over), then now and then
            let pace = if self.crept < 26 {
                CREEP_S
            } else {
                CREEP_S * 6.0
            };
            self.next_creep = t + pace * (0.6 + 0.8 * r);
            self.creep();
        }
        // the holes heal from their edges
        if t > self.next_renew {
            self.next_renew = t + RENEW_S * (0.6 + 0.8 * self.rand());
            self.renew(t);
        }
        let gpu = &ctx.gpu;
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        let mut op = Op::default();
        // the finger this frame: read what is under it first (how much ice); then its warmth along
        // its way since the last frame, wider the longer it rests
        let mut lifted = None;
        if let Some(f) = &mut self.finger {
            match f.mode {
                Mode::Undecided => {
                    op.kind |= OP_PROBE;
                    op._d = [f.at[0] as f32, f.at[1] as f32];
                }
                Mode::Melt => {
                    let k = ((t - f.rest_since) / MELT_S).clamp(0.0, 1.0);
                    let r = MELT + (MELT_MAX - MELT) * k * k * (3.0 - 2.0 * k);
                    f.radius = f.radius.max(r);
                    f.radii.resize(f.path.len(), r);
                    let last = f.radii.len() - 1;
                    f.radii[last] = f.radii[last].max(r);
                    op.kind |= OP_MELT | OP_PROBE;
                    op._d = [f.at[0] as f32, f.at[1] as f32];
                    op.radius = r as f32;
                }
            }
            if f.mode != Mode::Undecided {
                op.at = [f.melted_to[0] as f32, f.melted_to[1] as f32];
                op.to = [f.at[0] as f32, f.at[1] as f32];
                op._c = 1.7;
                f.melted_to = f.at;
                if f.lifted {
                    lifted = self.finger.take();
                }
            }
        }
        if let Some(f) = lifted {
            if f.mode == Mode::Melt {
                // the melt freezes again smooth; the frost creeps back in from its edge
                self.regrow(&f, t);
            }
        }
        if let Some((c, r)) = sweep_r {
            op.kind = OP_CLEAR;
            op.at = [c[0] as f32, c[1] as f32];
            op.radius = r as f32;
        }
        // one new crystal a frame (each needs its own op)
        if let Some(i) = self.pending.iter().position(|p| p.after <= t) {
            let p = self.pending.remove(i);
            // (the frame's and the healing holes' chimes no closer than a second and a half apart)
            let quiet = p.step.is_none() && t - self.last_chime < 1.5;
            if p.chime > 0.0 && !quiet {
                if p.step.is_none() {
                    self.last_chime = t;
                }
                let (_, pan) = self.voice_at(p.at, p.lift);
                let f = if let Some(st) = p.step {
                    sound::note(st)
                } else {
                    // (round the height's note, now one, now another of its neighbours)
                    let r = self.rand();
                    let st = sound::step_at(p.at[1] / self.pane[1].max(1.0))
                        + p.lift
                        + if r < 0.3 {
                            1
                        } else if r < 0.5 {
                            2
                        } else {
                            0
                        };
                    sound::note(st)
                };
                // (a little louder, a little brighter or not, so no two pings are the same)
                let k = 0.75 + 0.5 * self.rand() as f32;
                self.crackle.play(sound::Note::Ping {
                    f,
                    pan,
                    amp: p.chime * k,
                });
                if std::env::var("FROST_LOG").is_ok() {
                    eprintln!("t {t:.2} ping {f:.0} amp {:.3} at {:?}", p.chime * k, p.at);
                }
            }
            // a free grid, or the one whose tips have stalled longest (it stops where it is)
            let k = (0..SLOTS)
                .find(|&k| !self.slots[k].live)
                .unwrap_or_else(|| {
                    (0..SLOTS)
                        .min_by(|&a, &b| self.slots[a].advanced.total_cmp(&self.slots[b].advanced))
                        .unwrap()
                });
            let angle = self.rand() * std::f64::consts::FRAC_PI_3;
            self.slots[k] = Slot {
                id: self.next_id,
                center: p.at,
                angle,
                live: true,
                born: t,
                grew: t,
                frozen: 0,
                reach: 0,
                advanced: t,
            };
            self.next_id += 1;
            op.kind |= OP_SEED;
            op.slot = k as u32;
        }
        let op = if op.kind != 0 { Some(op) } else { None };
        let mut g = self.globals();
        g.relax = (RELAX_PER_S * dt) as f32;
        gpu.queue
            .write_buffer(&self.globals, 0, bytemuck::bytes_of(&g));
        gpu.queue
            .write_buffer(&self.stats, (PROBE_AT * 4) as u64, &[0u8; 16]);
        let wg = N.div_ceil(8);
        let (fw, fh) = (
            (self.pane[0] * FS as f64).ceil() as u32,
            (self.pane[1] * FS as f64).ceil() as u32,
        );
        {
            let mut cp = enc.begin_compute_pass(&Default::default());
            cp.set_bind_group(0, &self.binds[0], &[]);
            cp.set_pipeline(&self.relax);
            cp.dispatch_workgroups(self.mask_side.div_ceil(8), self.mask_side.div_ceil(8), 1);
        }
        if let Some(o) = op {
            gpu.queue.write_buffer(&self.op, 0, bytemuck::bytes_of(&o));
            let mut cp = enc.begin_compute_pass(&Default::default());
            // the warm fingertip first (every live crystal's cells there, the pane's claim and ice),
            // then the new grid; each writes binding 2, so once a parity
            // (what is under the finger is read before this frame's warmth reaches it)
            if o.kind & OP_PROBE != 0 {
                cp.set_bind_group(0, &self.binds[0], &[]);
                cp.set_pipeline(&self.probe);
                cp.dispatch_workgroups(1, 1, 1);
            }
            for p in 0..2 {
                cp.set_bind_group(0, &self.binds[p], &[]);
                cp.set_pipeline(&self.melt_cells);
                cp.dispatch_workgroups(wg, wg, SLOTS as u32);
                cp.set_pipeline(&self.seed);
                cp.dispatch_workgroups(wg, wg, 1);
            }
            let reach = 2.8 * o.radius + 6.0;
            let span = |a: f32, b: f32| (((a - b).abs() + reach) * FS).ceil() as u32;
            if o.kind & OP_MELT != 0 {
                cp.set_pipeline(&self.melt_pane);
                cp.dispatch_workgroups(
                    span(o.at[0], o.to[0]).div_ceil(8),
                    span(o.at[1], o.to[1]).div_ceil(8),
                    1,
                );
            }
            if o.kind & OP_CLEAR != 0 {
                cp.set_pipeline(&self.clear);
                cp.dispatch_workgroups(fw.div_ceil(8), fh.div_ceil(8), 1);
            }
        }
        // the automaton, at its own pace (slower under Reduce Motion)
        let rate = if self.reduce_motion {
            STEPS_PER_S * 0.4
        } else {
            STEPS_PER_S
        };
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
            cp.dispatch_workgroups(fw.div_ceil(8), fh.div_ceil(8), 1);
        }
        enc.copy_buffer_to_buffer(&self.stats, 0, &self.readback, 0, SLOTS as u64 * 16 + 16);
        gpu.queue.submit([enc.finish()]);

        // what froze this frame (read back at once: a few bytes, and the same on every run)
        let slice = self.readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        let _ = gpu.device.poll(wgpu::PollType::wait_indefinitely());
        let counts: Vec<u32> =
            bytemuck::cast_slice(&slice.get_mapped_range().expect("mapped")).to_vec();
        self.readback.unmap();
        let mut strike = None;
        let mut touch = (0.0, 0.0, 0.0, 0.0, 0.0);
        let [pw, ph] = self.pane;
        if let Some(f) = &mut self.finger {
            let ice = counts[PROBE_AT] as f64 / counts[PROBE_AT + 1].max(1) as f64;
            if f.mode == Mode::Undecided {
                // (on dry glass a lifted print has nothing to show while it waits: its stars come at once)
                let depth = f32::from_bits(counts[PROBE_AT + 2]) as f64;
                f.ice = ice;
                f.mode = Mode::Melt;
                strike = Some((f.at, ice));
                if std::env::var("FROST_LOG").is_ok() {
                    eprintln!("t {t:.2} probe ice {ice:.2} glaze {depth:.1}");
                }
            } else if dt > 0.0 {
                // the finger's way this frame: the ice it melts (ahead of it, as it goes; round it,
                // as a resting melt widens over frost), and the glass singing as it moves
                let v = (f.at[0] - f.last[0]).hypot(f.at[1] - f.last[1]) / dt;
                f.speed += (v - f.speed) * (1.0 - (-dt / 0.08).exp());
                let rest = ((t - f.rest_since) / MELT_S).clamp(0.0, 1.0);
                let moving = (f.speed / 260.0).min(1.0);
                let widening = if rest < 1.0 && f.radius < MELT_MAX {
                    f.ice * 0.5
                } else {
                    0.0
                };
                let sizzle = (ice * (0.35 + 0.65 * moving)).max(widening);
                let snaps = ice * f.speed * 0.12;
                let step = sound::step_at(f.at[1] / ph.max(1.0));
                let mut sing = ((f.speed - 25.0) / 300.0).clamp(0.0, 1.0) * (1.0 - 0.5 * ice);
                // resting, the glass swells to its note as the melt widens, then lets it go (no
                // drone); and the melt drips, slow soft notes rising from it, for as long as it stays
                let held = t - f.rest_since;
                if f.speed < 30.0 {
                    let a = (held / 0.4).clamp(0.0, 1.0);
                    let swell = a * a * (3.0 - 2.0 * a) * (-(held - MELT_S).max(0.0) / 1.5).exp();
                    sing = sing.max(0.75 * swell);
                    if held > 0.35 && t > self.next_drip {
                        const DRIP: [i32; 6] = [2, 4, 5, 4, 7, 5];
                        let k = self.drips % DRIP.len();
                        self.drips += 1;
                        let r = (self.drips as f64 * 0.618).fract();
                        self.next_drip = t + 0.5 + 0.3 * r;
                        self.drip =
                            Some((step + DRIP[k], (f.at[0] / pw.max(1.0) * 2.0 - 1.0) * 0.7));
                    }
                } else {
                    self.drips = 0;
                }
                let pan = (f.at[0] / pw.max(1.0) * 2.0 - 1.0) * 0.7;
                touch = (
                    sizzle as f32,
                    snaps as f32,
                    sound::note(step),
                    sing as f32,
                    pan as f32,
                );
            }
            f.last = f.at;
        }
        if let Some((st, pan)) = self.drip.take() {
            self.crackle.play(sound::Note::Ping {
                f: sound::note(st),
                pan: pan as f32,
                amp: 0.05,
            });
        }
        if let Some((at, ice)) = strike {
            // the glass struck: bare glass rings, frost damps it
            let (f, pan) = self.voice_at(at, 0);
            // (ferns leave gaps: a probe 0.45 iced is a frosted pane)
            let damp = (ice / 0.45).clamp(0.0, 1.0);
            self.crackle.play(sound::Note::Strike {
                f,
                pan,
                amp: 0.22,
                damp: (damp * damp * (3.0 - 2.0 * damp)) as f32,
            });
        }
        self.crackle
            .touch(touch.0, touch.1, touch.2, touch.3, touch.4);
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
            if (t - s.advanced > QUIET_S && t - s.born > 1.0)
                || t - s.born > MAX_AGE_S
                || s.reach as i64 >= (N / 2) as i64 - 6
            {
                s.live = false;
            }
        }
        if std::env::var("FROST_LOG").is_ok() {
            eprintln!("t {t:.2} live {} frozen {frozen_now}", self.live());
        }
        // the crackle: sparse ticks as cells freeze, under the glass's notes (round 1 of row 254: a full
        // bed of it buried them), softened (a fern's run is a rush, a plate's
        // slow spread a scatter)
        let per_s = if dt > 0.0 { frozen_now / dt } else { 0.0 };
        let clicks = (per_s.sqrt() * 0.7).min(60.0);
        let pan = if frozen_now > 0.0 {
            pan_w / frozen_now
        } else {
            0.0
        };
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
        ctx.gpu
            .queue
            .write_buffer(&self.look, 0, bytemuck::bytes_of(&look));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frost"),
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
            rp.set_pipeline(&self.draw);
            rp.set_bind_group(0, &self.draw_bind, &[]);
            rp.draw(0..3, 0..1);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(self.crackle.synth())
    }

    fn shaken(&mut self, _ctx: &mut Ctx) {
        self.sweep_asked = true;
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let at = [self.pane[0] * 0.55, self.pane[1] * 0.45];
        Some(info::Demo::new(
            "Frost on a window at −50 °C. Your finger is warm.",
            vec![info::Stroke::drag(
                0.4,
                1.4,
                at,
                [at[0] - 90.0, at[1] + 140.0],
            )],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "The glass is a honeycomb of cells. A cell that is ice, or touches ice, takes water from the air.",
                "Everywhere else the water spreads to the six neighbours. That alone grows six arms, each crystal at its own angle.",
                "The air is wetter low on the glass: long ferns race there; higher up, slower, stubbier stars.",
                "Two crystals can't share water: they meet in a seam.",
                "Your finger is warm: it melts the frost. The meltwater freezes again smooth, with nothing for frost to catch on: it creeps back only from the edge, slowly.",
            ],
            "Real: Reiter's model of how snow crystals grow. Drawn: the night, the glass, the lamp, the sound.",
        ))
    }
}

toy_kit::ios_shell!(Frost);
