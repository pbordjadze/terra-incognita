//! The maybes: a held float's other possible selves, worked out on the GPU. One thread a seed runs
//! `rules_step` (sea/rules.wgsl, Truth's mirror of `Rules::step`) from the float's put day to now and
//! writes where that self would be; the sprite pass reads the buffer and draws them (look/maybes.wgsl).

use super::{Gpu, Textures};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MaybesParams {
    /// lon, lat (degrees), the day put, the day now
    pub start: [f32; 4],
    /// n, seed low, seed high, the step in days
    pub run: [u32; 4],
}

const COMPUTE_WGSL: &str = r#"
struct Params { start: vec4f, run: vec4u };
@group(0) @binding(0) var<uniform> P: Params;
@group(0) @binding(1) var<storage, read_write> out: array<vec4f>;
@group(1) @binding(0) var rCurrents: texture_2d<f32>;
@group(1) @binding(1) var rGeo: texture_2d<f32>;
@group(1) @binding(2) var rIce: texture_2d_array<f32>;
@group(1) @binding(3) var rHeld: sampler;
@group(1) @binding(4) var rEastWest: sampler;
RULES
@compute @workgroup_size(64) fn maybes(@builtin(global_invocation_id) gid: vec3u) {
    let k = gid.x;
    if (k >= P.run.x) { return; }
    var at = P.start.xy;
    var day = P.start.z;
    let end = P.start.w;
    let sub = bitcast<f32>(P.run.w);
    var held = 0.0;
    var n = 0u;
    loop {
        if (day >= end - 1e-6 || n >= 4000u) { break; }
        let dt = min(sub, end - day);
        let s = rules_step(at, day, dt, P.run.y, k + 1u);
        at = s.xy;
        held = s.z;
        day += dt;
        n += 1u;
    }
    out[k] = vec4f(at, held, f32(k));
}
"#;

pub struct Maybes {
    pipeline: wgpu::ComputePipeline,
    params: wgpu::Buffer,
    pub buffer: wgpu::Buffer,
    pub capacity: u32,
    bind0: wgpu::BindGroup,
    bind1: wgpu::BindGroup,
    layout0: wgpu::BindGroupLayout,
    /// How many are in the buffer now.
    pub count: u32,
}

impl Maybes {
    pub fn new(gpu: &Gpu, tx: &Textures, capacity: u32) -> Maybes {
        let layout0 = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("maybes"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None }, count: None },
            ],
        });
        let t2 = |b| wgpu::BindGroupLayoutEntry { binding: b, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None };
        let layout1 = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("maybes sheets"),
            entries: &[
                t2(0),
                t2(1),
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2Array, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                wgpu::BindGroupLayoutEntry { binding: 4, visibility: wgpu::ShaderStages::COMPUTE, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let shader = gpu.shader("maybes", &COMPUTE_WGSL.replace("RULES", include_str!("../sea/rules.wgsl")));
        let player = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("maybes"), bind_group_layouts: &[&layout0, &layout1], push_constant_ranges: &[] });
        let pipeline = gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("maybes"),
            layout: Some(&player),
            module: &shader,
            entry_point: Some("maybes"),
            compilation_options: Default::default(),
            cache: None,
        });
        let params = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("maybes params"), size: std::mem::size_of::<MaybesParams>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("maybes"), size: (capacity as u64) * 16, usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_SRC, mapped_at_creation: false });
        let bind0 = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("maybes"),
            layout: &layout0,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: buffer.as_entire_binding() },
            ],
        });
        let bind1 = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("maybes sheets"),
            layout: &layout1,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&tx.currents.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tx.geo.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&tx.ice.view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&tx.linear_clamp) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::Sampler(&tx.linear_wrap_x) },
            ],
        });
        Maybes { pipeline, params, buffer, capacity, bind0, bind1, layout0, count: 0 }
    }

    /// Runs `n` maybes of a float put at `start` on `day0`, to `day1`, in steps of `dt` days.
    pub fn run(&mut self, gpu: &Gpu, start: [f64; 2], day0: f64, day1: f64, seed: u64, n: u32, dt: f64) {
        let n = n.min(self.capacity);
        let p = MaybesParams {
            start: [start[0] as f32, start[1] as f32, day0 as f32, day1 as f32],
            run: [n, seed as u32, (seed >> 32) as u32, (dt as f32).to_bits()],
        };
        gpu.queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&p));
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("maybes") });
        {
            let mut cp = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("maybes"), timestamp_writes: None });
            cp.set_pipeline(&self.pipeline);
            cp.set_bind_group(0, &self.bind0, &[]);
            cp.set_bind_group(1, &self.bind1, &[]);
            cp.dispatch_workgroups((n + 63) / 64, 1, 1);
        }
        gpu.queue.submit([enc.finish()]);
        self.count = n;
        let _ = &self.layout0;
    }
}
