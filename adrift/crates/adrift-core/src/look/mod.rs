//! How Adrift looks: every drawing shader and the Rust that owns its pipeline (Visual's module;
//! DECISIONS.md §2). Engineering's R0 ports Terra's ground and sea plainly (`ground`, `sea`); the
//! marks on the sea are Visual's passes (`floats`, `wake`, `maybes`, `silver`, `words`), gathered
//! in [`Look`], which the app prepares and draws after the sea. Draw order (§3): ground, sea, wakes,
//! maybes, drifter, floats, chrome. README.md is the map; PALETTE.md the colours and their contrast.

pub mod floats;
pub mod ground;
pub mod maybes;
pub mod sea;
pub mod silver;
pub mod veil;
pub mod sprites;
pub mod wake;
pub mod words;

use crate::gpu::view::VIEW_WGSL;
use crate::gpu::Gpu;
use crate::model::LonLat;
use bytemuck::Pod;

pub const COMMON_WGSL: &str = include_str!("common.wgsl");
/// What every mark's shader shares beyond `common.wgsl`: placing a unit vector on the screen, a
/// quad's corner, reading points, coverage, the zoom's scaling (`marks.wgsl`).
pub const MARKS_WGSL: &str = include_str!("marks.wgsl");

/// A pass's WGSL: the View struct, the shared helpers, then its own.
pub fn wgsl(own: &str) -> String {
    format!("{VIEW_WGSL}\n{COMMON_WGSL}\n{own}")
}

/// A mark's WGSL: the above and `marks.wgsl` (tools/adrift/wgsl_check.sh validates them so).
pub fn marks_wgsl(own: &str) -> String {
    format!("{VIEW_WGSL}\n{COMMON_WGSL}\n{MARKS_WGSL}\n{own}")
}

pub use ground::GroundPass;
pub use maybes::MaybesPass;
pub use sea::SeaPass;
pub use sprites::{Instance, SpritePass};

/// The view's bind group layout (group 0 of every pass): one uniform buffer, `ViewUniform`.
pub fn view_layout(gpu: &Gpu) -> wgpu::BindGroupLayout {
    gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("view"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
            count: None,
        }],
    })
}

/// A mark's pipeline: instanced quads of six vertices (`vs`, `fs`), one instance layout, the
/// view's group 0 and the pass's own group 1 if it has one, premultiplied over the sea.
pub fn mark_pipeline(gpu: &Gpu, name: &str, module: &wgpu::ShaderModule, instance: wgpu::VertexBufferLayout,
                     view: &wgpu::BindGroupLayout, own: Option<&wgpu::BindGroupLayout>) -> wgpu::RenderPipeline {
    let mut layouts = vec![view];
    if let Some(o) = own {
        layouts.push(o);
    }
    let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some(name), bind_group_layouts: &layouts, push_constant_ranges: &[] });
    gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(name),
        layout: Some(&layout),
        vertex: wgpu::VertexState { module, entry_point: Some("vs"), buffers: &[instance], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
        }),
        primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}

/// An instance buffer that grows as it must and is written each frame.
pub struct Instances<T: Pod> {
    buffer: wgpu::Buffer,
    capacity: usize,
    pub count: u32,
    name: &'static str,
    _t: std::marker::PhantomData<T>,
}

impl<T: Pod> Instances<T> {
    pub fn new(gpu: &Gpu, name: &'static str, capacity: usize) -> Self {
        let capacity = capacity.max(16);
        Self { buffer: Self::make(gpu, name, capacity), capacity, count: 0, name, _t: Default::default() }
    }

    fn make(gpu: &Gpu, name: &str, capacity: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(name),
            size: (capacity * std::mem::size_of::<T>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn write(&mut self, gpu: &Gpu, items: &[T]) {
        if items.len() > self.capacity {
            self.capacity = items.len().next_power_of_two();
            self.buffer = Self::make(gpu, self.name, self.capacity);
        }
        if !items.is_empty() {
            gpu.queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(items));
        }
        self.count = items.len() as u32;
    }

    pub fn slice(&self) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(..)
    }
}

/// A pass's own small uniform (group 1, binding 0).
pub struct Uniform<T: Pod> {
    pub layout: wgpu::BindGroupLayout,
    pub buffer: wgpu::Buffer,
    pub group: wgpu::BindGroup,
    _t: std::marker::PhantomData<T>,
}

impl<T: Pod> Uniform<T> {
    pub fn new(gpu: &Gpu, name: &str) -> Self {
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(name),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(name),
            size: std::mem::size_of::<T>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(name),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
        });
        Self { layout, buffer, group, _t: Default::default() }
    }

    pub fn write(&self, gpu: &Gpu, value: &T) {
        gpu.queue.write_buffer(&self.buffer, 0, bytemuck::bytes_of(value));
    }
}

/// A place's unit vector as the shaders take it.
pub fn unit(at: LonLat) -> [f32; 3] {
    let v = at.to_xyz();
    [v[0] as f32, v[1] as f32, v[2] as f32]
}

// ---- what the look draws, as the app hands it over each frame

/// A float as the look sees it.
#[derive(Clone, Debug)]
pub struct LookFloat {
    /// The app's id (1, 2, 3…): a hue per lamp can hang on it (R5).
    pub id: u32,
    pub at: LonLat,
    /// 1 while it is what the reader just did, fading to 0 in time (the app's `lit_for` against the beat).
    pub lit: f32,
    /// Under a resting finger: its maybes out.
    pub held: bool,
    /// The one whose maybes and drifter are shown now (held, pressed, chosen or ridden; R5): with
    /// several clouds out (R6) the one in front, the last chosen or the one under the finger.
    pub chosen: bool,
    /// Its cloud is out (R6: one of `LookState.clouds`), in front or not.
    pub shown: bool,
    /// Chosen as itself (R8): its journey followed, no cloud (`App.followed`).
    pub followed: bool,
    /// Where it was put (its first leg's place: where it started) and on what day (`Clock.day`) (R6).
    pub origin: LonLat,
    pub put_day: f64,
    /// Its path at every step, `STEPS_A_DAY` a day, oldest first (R10: the app's `Float.trail`; the
    /// last `WAKE_DAYS` days are drawn).
    pub wake: Vec<LookBead>,
    /// How measured the sea is where it rides now (0..1, Truth's rules): under a half the rim is
    /// dashed, so nothing claims the current where no drifter went.
    pub measured: f32,
}

/// One bead of a wake as the look sees it.
#[derive(Clone, Copy, Debug)]
pub struct LookBead {
    pub at: LonLat,
    /// How measured the sea was there (0..1): under a half the bead is hollow.
    pub measured: f32,
}

/// One cloud of maybes out (R6: several at once). Its paths are `count` from `base` in Truth's
/// buffer (`sea::Maybes`), k = 1..=count; its lamp is the float `id` (k = 0, the same rules).
#[derive(Clone, Copy, Debug)]
pub struct LookCloud {
    pub id: u32,
    /// Where its lamp is now, and where it was put (the cloud's leg: where it started).
    pub at: LonLat,
    pub origin: LonLat,
    pub base: u32,
    pub count: u32,
    /// 0..1 as it blooms out (eased) and folds back.
    pub bloom: f32,
    /// The one in front (the last chosen, or the one under the finger): its silver and its veil.
    pub front: bool,
}

/// Everything the marks draw this frame. The app fills it from its state; the look reads nothing else.
#[derive(Clone, Debug, Default)]
pub struct LookState {
    pub floats: Vec<LookFloat>,
    /// A take is being recorded, and for how many seconds (R9).
    pub recording: Option<f32>,
    /// This frame is rendered into an exported clip (R9): the screen's chrome (the date, the panel,
    /// the recording's mark) is the look's to leave out or keep.
    pub exporting: bool,
    /// While exporting (R10): how far through the clip's still opening beat, 0..1 (1 once past it,
    /// and always 1 live).
    pub opening: f32,
    /// While exporting (R10): 0 until the clip's eased last beat, then 0..1 through it (the app fades
    /// the sea's rings with it; the look fades whatever else is touch feedback). Always 0 live.
    pub ending: f32,
    /// The clip's window on the screen being drawn, points x, y, w, h (R10: while exporting, the
    /// whole screen, as the clip is drawn at its own 9:16 size; None live).
    pub clip: Option<[f32; 4]>,
    /// The lamp whose panel mark a finger rests on, and how far its rest has thickened, 0..1 (R10:
    /// the lamp swells while its mark is pressed; from Motion's `Gestures::panel_pressing_since`).
    pub panel_press: Option<(u32, f32)>,
    /// A take is being recorded or replayed, so the hand conducts (R11).
    pub conducting: bool,
    /// The shell has a song that plays from its start when a take begins (R12).
    pub song: bool,
    /// The cues of the last two seconds (R11): the look flashes the worlds a lamp chose among and
    /// folds them into the chosen one.
    pub cues: Vec<LookCue>,
    /// A ring on the water at a lamp (R10: a tap on its mark or on it): the lamp's
    /// id, where, and seconds since. The sea draws every ring under the maybes; these are drawn again
    /// over them, in the lamp's hue, so a ring at a lamp is not lost in its own cloud and says whose it is.
    pub lamp_rings: Vec<(u32, LonLat, f32)>,
    /// How many takes are kept, the newest ready to export (R9).
    pub takes: u32,
    /// The clouds out, in the order chosen (the front one last) (R6).
    pub clouds: Vec<LookCloud>,
    /// The days the clouds are still behind the date, the most of any (0 when they have caught up):
    /// a scrub faster than they can follow shows it here (R6).
    pub behind: f32,
    /// A finger resting on the sea before a float is put: where, and how far through the beat (0..1:
    /// Motion's `thicken(now - since)` from its `Press { at, since }`; `Unpress` clears it).
    pub resting: Option<(LonLat, f32)>,
    /// 0..1 as the front cloud blooms out and folds back (each cloud's own is in `clouds`).
    pub bloom: f32,
    /// The nearest real drifter's track, in order, where it passed the held float.
    pub drifter: Vec<LonLat>,
    /// 0..1 as the drifter rises among the maybes and sinks back.
    pub rise: f32,
    /// The drifter's buoy and its dates ("Drifter 300234061198840, 12 May 2019 to 3 Feb 2020"): a real
    /// buoy and a real year, drawn as a label at the point nearest the held float.
    pub drifter_label: String,
    /// The index in `drifter` of the point nearest the held float; found from the held float if none.
    pub drifter_nearest: Option<usize>,
    /// The night palette.
    pub night: bool,
    /// The date as words ("14 October").
    pub date: String,
    /// A note under it, in italic ("a typical year, by the drifters' record"); empty for none.
    pub note: String,
    /// The date runs (its marker is lit, amber) or rests (the marker a ring of ink).
    pub date_running: bool,
    /// How far through the year the date is, 0..1: the marker's place on the year's rule under the date.
    pub year_day: f32,
    /// A finger on the date's band: 0 none, else how far through the rest's beat (Motion's thicken);
    /// the band brightens the frame it lands and thickens over the beat (DEMANDS.md D8, D9).
    pub date_press: f32,
    /// The shell's seconds, for what breathes on the screen (the date's lamp while the year runs).
    pub seconds: f32,
    /// The truth's marks on the screen (the guess mark on a float and its beads): off by default
    /// (DECISIONS.md › Truth underneath, not on top).
    pub truth_marks: bool,
    /// The credits owed, one line (Truth's SOURCES.md says whose).
    pub credits: String,
    /// The credits in full (`sea::CREDITS`), shown on a page over the globe while `credits_open`.
    pub credits_full: Vec<String>,
    /// The page of credits is open (a tap on the credit line opens it, a tap anywhere closes it).
    pub credits_open: bool,
}

/// A lamp's hue by its number (palette.json › hues): amber first, then the others, each a glass
/// lamp's colour that tells its cloud, wake and silver from another's; each next one far round the
/// wheel from the last, so two lamps put one after the other are told apart at a glance (R6).
/// One conductor's cue, as the look draws it (R11).
#[derive(Clone, Debug)]
pub struct LookCue {
    /// The lamp.
    pub id: u32,
    /// Every world's place at the cue (`conduct::WORLDS`, in world order).
    pub worlds: Vec<LonLat>,
    /// The worlds that did what the hand did (1-based world numbers).
    pub near: Vec<u32>,
    /// The world chosen (1-based) and the lamp's place in it, and where the lamp was before.
    pub chosen: u32,
    pub at: LonLat,
    pub from: LonLat,
    /// The runners-up the chooser names (Truth, R12: up to six, spread round the pick), their places at
    /// the cue: the fan that folds into the chosen.
    pub fan: Vec<LonLat>,
    /// Seconds since the cue (the drawn motion's clock).
    pub age: f32,
}

pub fn lamp_hue(id: u32) -> [f32; 3] {
    const HUES: [[f32; 3]; 8] = [
        [0.957, 0.647, 0.165],   // amber
        [0.345, 0.722, 0.910],   // sky
        [0.910, 0.435, 0.627],   // rose
        [0.310, 0.816, 0.659],   // mint
        [0.941, 0.439, 0.353],   // coral
        [0.706, 0.549, 0.941],   // lilac
        [0.722, 0.839, 0.290],   // lime
        [0.949, 0.812, 0.227],   // gold
    ];
    HUES[((id.max(1) - 1) % 8) as usize]
}

fn smooth01(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn slerpish(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let v = [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// A cue's beats (R11), in seconds since it: the worlds flash in, hold, then fold into the chosen;
/// the lamp glides from where it was to where the chosen world has it (a look-only ease: the state
/// jumped); a stroke of its hue marks the glide and fades; a ring spreads where it lands.
const CUE_FOLD: (f32, f32) = (0.3, 0.85);
const CUE_GLIDE: (f32, f32) = (0.1, 0.75);

/// Where a conducted lamp is drawn: gliding from its old place to its new.
fn glide(c: &LookCue) -> [f32; 3] {
    slerpish(unit(c.from), unit(c.at), smooth01(CUE_GLIDE.0, CUE_GLIDE.1, c.age))
}

/// A thin ring on the globe round `at`, `radius` points across the screen (at the globe's scale).
fn ring_on_globe(out: &mut Vec<wake::StrokeInstance>, at: LonLat, radius: f32, disc: [f32; 3], w: f32, a: f32, hue: [f32; 3], night: bool) {
    let ang = radius / disc[2].max(1.0);
    let c = unit(at);
    let up = if c[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let e1 = { let v = [up[1] * c[2] - up[2] * c[1], up[2] * c[0] - up[0] * c[2], up[0] * c[1] - up[1] * c[0]]; let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt(); [v[0] / l, v[1] / l, v[2] / l] };
    let e2 = [c[1] * e1[2] - c[2] * e1[1], c[2] * e1[0] - c[0] * e1[2], c[0] * e1[1] - c[1] * e1[0]];
    let (ca, sa) = (ang.cos(), ang.sin());
    let ring: Vec<wake::Pt> = (0..48).map(|j| {
        let th = j as f32 / 48.0 * std::f32::consts::TAU;
        let (x, y) = (th.cos() * sa, th.sin() * sa);
        wake::Pt { p: [c[0] * ca + e1[0] * x + e2[0] * y, c[1] * ca + e1[1] * x + e2[1] * y, c[2] * ca + e1[2] * x + e2[2] * y], w, a, hollow: 0.0 }
    }).collect();
    wake::line(out, &ring, hue, false, night, true);
}

/// The few maybes a cue shows (R12): up to `n` of the worlds that did what the hand did (all the
/// worlds if too few did), one in each sector of bearing round the chosen place, the one nearest the
/// sector's middle distance, so the fan is spread and none is a stray.
fn fan(c: &LookCue, n: usize) -> Vec<[f32; 3]> {
    // the chooser's own runners-up when it names them (Truth, R12)
    if !c.fan.is_empty() {
        return c.fan.iter().map(|p| unit(*p)).collect();
    }
    // a tap no world came near shows no fan, only the lamp's own stroke
    if c.near.is_empty() {
        return Vec::new();
    }
    let at = unit(c.at);
    let pool: Vec<usize> = c.near.iter().filter(|&&k| k != c.chosen && k >= 1 && (k as usize) <= c.worlds.len()).map(|&k| k as usize - 1).collect();
    // a tangent frame at the chosen place
    let up = if at[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let e1 = cross(up, at);
    let e2 = cross(at, e1);
    let mut sectors: Vec<Vec<(f32, [f32; 3])>> = vec![Vec::new(); n];
    for i in pool {
        let p = unit(c.worlds[i]);
        let (x, y) = (dot(p, e1), dot(p, e2));
        let d = (x * x + y * y).sqrt();
        if d < 1e-7 {
            continue;
        }
        let s = (((y.atan2(x) / std::f32::consts::TAU + 1.0).fract()) * n as f32) as usize % n;
        sectors[s].push((d, p));
    }
    sectors
        .into_iter()
        .filter(|v| !v.is_empty())
        .map(|mut v| {
            v.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            v[v.len() / 2].1
        })
        .collect()
}

/// A cue drawn (R12: the lamp obeying, not the app answering): a brief fan of the maybes the hand
/// chose among, faint lines of the lamp's hue from where it was to where each would have it, which
/// fold into the one chosen as the lamp glides there along its own brighter stroke. No spray, no
/// ring: the bend in the lamp's line is the answer. Over in about a second.
fn cue(out: &mut Vec<wake::StrokeInstance>, c: &LookCue, night: bool, heavy: f32) {
    let hue = lamp_hue(c.id);
    let at = unit(c.at);
    let from = unit(c.from);
    let flash = smooth01(0.0, 0.1, c.age);
    let fold = smooth01(CUE_FOLD.0, CUE_FOLD.1, c.age);
    if fold < 1.0 {
        let gone = (1.0 - fold).powf(0.7);
        for p in fan(c, 7) {
            let end = slerpish(p, at, fold);
            let pts: Vec<wake::Pt> = (0..=12).map(|j| {
                let t = j as f32 / 12.0;
                wake::Pt { p: slerpish(from, end, t), w: (0.8 + 0.9 * t) * heavy, a: 0.5 * flash * gone * (0.25 + 0.75 * t), hollow: 0.0 }
            }).collect();
            wake::line(out, &pts, hue, false, night, false);
            wake::line(out, &[wake::Pt { p: end, w: 4.0 * heavy, a: 0.55 * flash * gone, hollow: 0.0 }], hue, false, night, false);
        }
    }
    // the chosen: the lamp's own stroke from where it was to where it glides, bright, then fading into
    // its line
    let g = smooth01(CUE_GLIDE.0, CUE_GLIDE.1, c.age);
    let stroke_a = 0.95 * (1.0 - smooth01(0.8, 1.4, c.age));
    if g > 0.0 && stroke_a > 0.0 {
        let pts: Vec<wake::Pt> = (0..=16).map(|j| {
            let t = g * j as f32 / 16.0;
            wake::Pt { p: slerpish(from, at, t), w: (1.4 + 1.8 * (j as f32 / 16.0)) * heavy, a: stroke_a * (0.4 + 0.6 * j as f32 / 16.0), hollow: 0.0 }
        }).collect();
        wake::line(out, &pts, hue, false, night, false);
    }
}

/// Days of wake drawn (palette.json › sizes.bead.days).
pub const WAKE_DAYS: usize = 45;
/// The steps a day in `LookFloat.wake` (the rules step a quarter day).
pub const STEPS_A_DAY: usize = 4;

/// The marks on the sea, in their draw order, behind one `prepare` and one `draw`.
pub struct Look {
    /// Where the maybes bloom from: the float last held or chosen.
    bloom_from: [f32; 3],
    /// That float's hue, carried by its maybes and its silver.
    hue: [f32; 3],
    /// The drifter's point nearest the float last held, kept while the silver sinks.
    nearest: usize,
    pub veil: veil::Veil,
    pub wake: wake::Wake,
    /// The screen's own strokes (R10): the panel's marks, the record control's ring; drawn over the
    /// floats, under the words.
    pub strokes: wake::Wake,
    pub maybes: maybes::MaybesPass,
    pub silver: silver::Silver,
    pub floats: floats::Floats,
    pub words: words::Words,
}

impl Look {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Look {
        Look {
            bloom_from: [0.0, 0.0, 1.0],
            hue: [0.957, 0.647, 0.165],
            nearest: 0,
            veil: veil::Veil::new(gpu, view),
            wake: wake::Wake::new(gpu, view),
            strokes: wake::Wake::new(gpu, view),
            maybes: maybes::MaybesPass::new(gpu, view),
            silver: silver::Silver::new(gpu, view),
            floats: floats::Floats::new(gpu, view),
            words: words::Words::new(gpu, view),
        }
    }

    /// Writes this frame's instances. `width`, `height` in points, `reading` the reading scale,
    /// `disc` the globe's centre and radius in points (where the words sit).
    pub fn prepare(&mut self, gpu: &Gpu, state: &LookState, width: f32, height: f32, reading: f32, disc: [f32; 3]) {
        let n = state.night;
        // in a clip the lines carry it (R12): heavier and fuller than live, so the stochastic line
        // wins over the map
        let heavy = if state.exporting { 2.0 } else { 1.0 };
        let mut lines = Vec::new();
        for f in &state.floats {
            // (a lamp whose cloud is out is one maybe among them: no wake singles it out, R6)
            if f.shown {
                continue;
            }
            // (a lamp chosen as itself draws its whole kept journey, R8; the others their last days)
            let start = if f.followed { 0 } else { f.wake.len().saturating_sub(WAKE_DAYS * STEPS_A_DAY) };
            let mut path = f.wake[start..].to_vec();
            if !state.truth_marks {
                for b in &mut path {
                    b.measured = 1.0;
                }
            }
            // (a lamp just conducted, R11: its rewritten line comes in as the worlds fold into it)
            let fade = state.cues.iter().filter(|c| c.id == f.id).map(|c| 0.35 + 0.65 * smooth01(0.3, 0.9, c.age)).fold(1.0, f32::min);
            wake::wake(&mut lines, &path, f.at, n, lamp_hue(f.id), f.followed, fade, heavy);
        }
        // the fermata (R12): while a resting finger holds the date as the hand conducts, every lamp
        // holds its breath, a thin ring of its hue drawn in close and swelling slowly, so the hold is
        // seen; it lets go as the date runs again
        if state.conducting && !state.date_running {
            let breath = 0.5 + 0.5 * (state.seconds * 1.6).sin();
            for f in state.floats.iter().filter(|f| !f.shown) {
                ring_on_globe(&mut lines, f.at, (10.0 + 3.0 * breath) * reading * heavy.sqrt(), disc, 1.4 * heavy.sqrt(), 0.45 + 0.25 * breath, lamp_hue(f.id), n);
            }
        }
        for c in &state.cues {
            cue(&mut lines, c, n, heavy);
        }
        self.wake.prepare(gpu, &lines);
        // every cloud out blooms from its lamp in its own hue (R6: several at once); the veil and the
        // silver are the front one's (its place and hue kept while it folds)
        let draws: Vec<maybes::CloudDraw> = state.clouds.iter().map(|c| maybes::CloudDraw { base: c.base, count: c.count, origin: unit(c.at), hue: lamp_hue(c.id), bloom: c.bloom, front: c.front }).collect();
        self.maybes.prepare(gpu, &draws, n);
        if let Some(f) = state.floats.iter().find(|f| f.held || f.chosen) {
            self.bloom_from = unit(f.at);
            self.hue = lamp_hue(f.id);
        }
        self.veil.prepare(gpu, self.bloom_from, state.bloom, n);
        let track: Vec<[f32; 3]> = state.drifter.iter().map(|p| unit(*p)).collect();
        self.silver.prepare(gpu, &track, state.rise, n, self.hue);
        // in a clip (R10) what marks a finger is out of its last frame: the held ring and a lamp's
        // brightening (what the reader just did) settle through the last beat, eased, so the clip
        // ends on the lamps alone; a finger resting on the sea (the press ring) is never in a clip
        let settle = if state.exporting { let e = state.ending.clamp(0.0, 1.0); 1.0 - e * e * (3.0 - 2.0 * e) } else { 1.0 };
        // a finger resting on a lamp's panel mark (R10): the lamp answers on the globe, lit and ringed
        // as the rest thickens, so the mark is found on the sea
        let pressed = |id: u32| if state.exporting { 0.0 } else { state.panel_press.filter(|p| p.0 == id).map_or(0.0, |p| 0.35 + 0.65 * p.1.clamp(0.0, 1.0)) };
        let mut fl: Vec<floats::FloatInstance> = state
            .floats
            .iter()
            .map(|f| floats::FloatInstance { g: state.cues.iter().rev().find(|c| c.id == f.id).map_or(unit(f.at), |c| glide(c)), beat: 1.0, lit: settle * if f.followed { f.lit.max(0.85) } else { f.lit }.max(pressed(f.id)), held: settle * ((f.held || f.chosen || f.followed) as u8 as f32).max(pressed(f.id)), night: n as u8 as f32, measured: if state.truth_marks { f.measured } else { 1.0 }, hue: lamp_hue(f.id), speck: state.clouds.iter().find(|c| c.id == f.id).map_or(0.0, |c| c.bloom.clamp(0.0, 1.0)) })
            .collect();
        if let Some((at, beat)) = state.resting.filter(|_| !state.exporting) {
            fl.push(floats::FloatInstance { g: unit(at), beat: beat.clamp(0.0, 0.999), lit: 0.0, held: 0.0, night: n as u8 as f32, measured: 1.0, hue: lamp_hue(state.floats.len() as u32 + 1), speck: 0.0 });
        }
        self.floats.prepare(gpu, &fl);
        // the drifter's label, at its point nearest the held float, rising with the silver
        let mut labels = Vec::new();
        // (the label from zoom 1.5: at the whole Earth it would be a banner across an ocean; shown or not,
        // never half)
        let zoom = disc[2] / ((width.min(height) / 2.0 - 14.0).max(40.0));
        if state.truth_marks && state.rise > 0.0 && !state.drifter.is_empty() && !state.drifter_label.is_empty() && zoom >= 1.5 {
            // (the point nearest the held float; kept while the silver sinks after the float is let go)
            if let Some(i) = state.drifter_nearest {
                self.nearest = i;
            } else if let Some(at) = state.floats.iter().find(|f| f.held).map(|f| f.at) {
                self.nearest = (0..state.drifter.len()).min_by(|&a, &b| state.drifter[a].distance_m(at).partial_cmp(&state.drifter[b].distance_m(at)).unwrap()).unwrap_or(0);
            }
            let at = unit(state.drifter[self.nearest.min(state.drifter.len() - 1)]);
            let (ink, halo) = if n { ([0.863, 0.890, 0.918, state.rise], [0.063, 0.094, 0.157, 0.92 * state.rise]) } else { ([0.243, 0.298, 0.353, state.rise], [0.761, 0.894, 0.957, 0.92 * state.rise]) };
            labels = self.words.face.label(&words::Line { text: &state.drifter_label, style: words::Style::Italic, size: 10.0 * reading, ink, tracking: 0.0 }, at, 12.0 * reading, -8.0 * reading, halo);
        }
        // an exported frame (R9) is the sea and its lamps: no panel, no date band, no credits page, no
        // record control; the date alone as a quiet caption. The live screen keeps its chrome, the
        // record control among it
        let mut strokes = Vec::new();
        if state.exporting {
            let clip = state.clip.unwrap_or([0.0, 0.0, width, height]);
            self.strokes.prepare(gpu, &strokes);
            self.words.prepare_caption(gpu, &state.date, clip, reading, disc, n, state.ending, &labels);
            return;
        }
        labels.extend(self.words.record_control(state.recording, state.seconds, width, reading, n, state.song, &mut strokes));
        // the panel of every lamp out there (R6; R10 a row of marks, each its lamp's journey in its hue)
        let rows: Vec<words::PanelRow> = state.floats.iter().map(|f| words::PanelRow { id: f.id, origin: f.origin, put_day: f.put_day, shown: f.shown, followed: f.followed, front: f.chosen && (f.shown || f.followed), journey: &f.wake, at: f.at, pressed: pressed(f.id) }).collect();
        // (while a take is made the marks step aside: the take is the globe alone, R12)
        let quiet_take = state.recording.is_some() || state.conducting;
        self.words.panel(if quiet_take { &[] } else { &rows }, reading, n, &mut strokes);
        // a ring at a lamp (R10), over the maybes: a thin circle of its hue spreading
        // from the lamp over a second and fading as it goes
        for &(id, at, age) in &state.lamp_rings {
            let t = (age / 1.2).clamp(0.0, 1.0);
            if t >= 1.0 {
                continue;
            }
            let ease = 1.0 - (1.0 - t).powi(3);
            ring_on_globe(&mut strokes, at, (9.0 + 34.0 * ease) * reading, disc, 2.2 - 1.2 * t, 0.85 * (1.0 - t).powf(1.5), lamp_hue(id), n);
        }
        self.strokes.prepare(gpu, &strokes);
        if state.credits_open {
            labels.extend(self.words.credits_page(&state.credits_full, width, height, reading, n));
        }
        self.words.prepare_foot(gpu, &state.date, &state.note, width, height, reading, disc, n, state.date_running, state.year_day, state.date_press, state.seconds, &labels);
    }

    /// Draws the marks over the sea; `maybes` is the held float's buffer (`gpu::Maybes`).
    /// Where the date is drawn, x, y, w, h in points (y down), after a `prepare`: the control Motion's
    /// scrub and pause gestures probe (touch/VOCABULARY.md); the only chrome there is.
    pub fn date_bounds(&self) -> Option<[f32; 4]> {
        self.words.date_bounds
    }

    /// The record control's rect, x, y, w, h in points, if the look draws one (R9; Motion's probe).
    pub fn record_bounds(&self) -> Option<[f32; 4]> {
        self.words.record_bounds
    }

    /// The panel's rows, if the look draws one (R6): each lamp's id and its rect, x, y, w, h in points
    /// (y down), after a `prepare`; Motion's probe answers a touch there. Empty until Visual draws it.
    pub fn panel_bounds(&self) -> Vec<(u32, [f32; 4])> {
        self.words.panel_bounds.clone()
    }

    /// Each panel mark's words for the shell's VoiceOver (R10), by lamp id, in the order of
    /// `panel_bounds`: "Lamp 2, put 8 June at 35°N 74°W, its cloud out". Nothing on the screen says them.
    pub fn panel_words(&self) -> Vec<(u32, String)> {
        self.words.panel_words.clone()
    }

    /// Where the credit line is drawn, x, y, w, h in points (y down), after a `prepare`: a tap on it
    /// opens the page of credits (Truth's D2); the page closes on any tap.
    pub fn credit_bounds(&self) -> Option<[f32; 4]> {
        self.words.credit_bounds
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, maybes: &'a crate::sea::Maybes) {
        self.wake.draw(rp, view_bind);
        self.maybes.draw(rp, view_bind, maybes);
        self.silver.draw(rp, view_bind);
        self.floats.draw(rp, view_bind);
        self.strokes.draw(rp, view_bind);
        self.words.draw(rp, view_bind);
    }
}
