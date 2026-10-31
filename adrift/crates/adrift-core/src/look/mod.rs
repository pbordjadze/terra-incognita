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
    pub at: LonLat,
    /// 1 while it is what the reader just did, fading to 0 in time (the app's `lit_for` against the beat).
    pub lit: f32,
    /// Under a resting finger: its maybes out.
    pub held: bool,
    /// One bead a day, oldest first (the last `WAKE_DAYS` are drawn).
    pub wake: Vec<LonLat>,
}

/// Everything the marks draw this frame. The app fills it from its state; the look reads nothing else.
#[derive(Clone, Debug, Default)]
pub struct LookState {
    pub floats: Vec<LookFloat>,
    /// A finger resting on the sea before a float is put: where, and how far through the beat (0..1:
    /// Motion's `thicken(now - since)` from its `Press { at, since }`; `Unpress` clears it).
    pub resting: Option<(LonLat, f32)>,
    /// 0..1 as the held float's maybes (the GPU buffer `gpu::Maybes`) bloom out and fold back.
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
    /// The credits owed, one line (Truth's SOURCES.md says whose).
    pub credits: String,
}

/// Days of wake drawn (palette.json › sizes.bead.days).
pub const WAKE_DAYS: usize = 45;

/// The marks on the sea, in their draw order, behind one `prepare` and one `draw`.
pub struct Look {
    /// Where the maybes bloom from: the float last held.
    bloom_from: [f32; 3],
    pub wake: wake::Wake,
    pub maybes: maybes::MaybesPass,
    pub silver: silver::Silver,
    pub floats: floats::Floats,
    pub words: words::Words,
}

impl Look {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Look {
        Look {
            bloom_from: [0.0, 0.0, 1.0],
            wake: wake::Wake::new(gpu, view),
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
        let mut beads = Vec::new();
        for f in &state.floats {
            let start = f.wake.len().saturating_sub(WAKE_DAYS);
            beads.extend(wake::beads(&f.wake[start..], n));
        }
        self.wake.prepare(gpu, &beads);
        // the maybes bloom from the held float (the last held place is kept while they fold)
        if let Some(f) = state.floats.iter().find(|f| f.held) {
            self.bloom_from = unit(f.at);
        }
        self.maybes.prepare(gpu, self.bloom_from, state.bloom, n);
        let track: Vec<[f32; 3]> = state.drifter.iter().map(|p| unit(*p)).collect();
        self.silver.prepare(gpu, &track, state.rise, n);
        let mut fl: Vec<floats::FloatInstance> = state
            .floats
            .iter()
            .map(|f| floats::FloatInstance { g: unit(f.at), beat: 1.0, lit: f.lit, held: f.held as u8 as f32, night: n as u8 as f32 })
            .collect();
        if let Some((at, beat)) = state.resting {
            fl.push(floats::FloatInstance { g: unit(at), beat: beat.clamp(0.0, 0.999), lit: 0.0, held: 0.0, night: n as u8 as f32 });
        }
        self.floats.prepare(gpu, &fl);
        // the drifter's label, at its point nearest the held float, rising with the silver
        let mut labels = Vec::new();
        if state.rise > 0.0 && !state.drifter.is_empty() && !state.drifter_label.is_empty() {
            let nearest = state.drifter_nearest.unwrap_or_else(|| {
                let from = state.floats.iter().find(|f| f.held).map(|f| f.at);
                match from {
                    Some(at) => (0..state.drifter.len()).min_by(|&a, &b| state.drifter[a].distance_m(at).partial_cmp(&state.drifter[b].distance_m(at)).unwrap()).unwrap_or(0),
                    None => 0,
                }
            });
            let at = unit(state.drifter[nearest.min(state.drifter.len() - 1)]);
            let (ink, halo) = if n { ([0.863, 0.890, 0.918, state.rise], [0.063, 0.094, 0.157, 0.92 * state.rise]) } else { ([0.243, 0.298, 0.353, state.rise], [0.761, 0.894, 0.957, 0.92 * state.rise]) };
            labels = self.words.face.label(&words::Line { text: &state.drifter_label, style: words::Style::Italic, size: 10.0 * reading, ink, tracking: 0.0 }, at, 12.0 * reading, -8.0 * reading, halo);
        }
        self.words.prepare_foot(gpu, &state.date, &state.note, &state.credits, width, height, reading, disc, n, &labels);
    }

    /// Draws the marks over the sea; `maybes` is the held float's buffer (`gpu::Maybes`).
    /// Where the date is drawn, x, y, w, h in points (y down), after a `prepare`: the control Motion's
    /// scrub and pause gestures probe (touch/VOCABULARY.md); the only chrome there is.
    pub fn date_bounds(&self) -> Option<[f32; 4]> {
        self.words.date_bounds
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, maybes: &'a crate::sea::Maybes, count: u32) {
        self.wake.draw(rp, view_bind);
        self.maybes.draw(rp, view_bind, maybes, count);
        self.silver.draw(rp, view_bind);
        self.floats.draw(rp, view_bind);
        self.words.draw(rp, view_bind);
    }
}
