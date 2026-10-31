//! The maybes' sampler on the GPU: `rules.wgsl` run as a compute pass over a buffer of paths,
//! thousands at once, the sheets it reads shared with the look's textures (geo, currents, ice)
//! and its own (wind, eddy, eddy time). The CPU's `Rules::step` and `Rules::flight` are the
//! reference; the test `the_gpu_steps_as_the_cpu_does` holds the two together.

use super::RULES_WGSL;
use crate::assets::Assets;
use crate::gpu::textures::{decode, Texture, Textures};
use crate::gpu::Gpu;
use bytemuck::{Pod, Zeroable};

/// One path in the buffer (`Path` in rules.wgsl): 24 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct GpuPath {
    pub lon: f32,
    pub lat: f32,
    /// 0 free, 1 coast, 2 ice.
    pub held: u32,
    pub measured: f32,
    pub eddy_u: f32,
    pub eddy_v: f32,
}

/// The step's parameters (`Params` in rules.wgsl): 48 bytes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Params {
    pub day: f32,
    pub dt: f32,
    pub leeway: f32,
    pub ice_hold: f32,
    pub wander: f32,
    pub shore: f32,
    pub seed_lo: u32,
    pub seed_hi: u32,
    pub count: u32,
    pub _pad: [u32; 3],
}

impl Params {
    pub fn new(rules: &super::Rules, day: f64, dt_days: f64, seed: u64, count: u32) -> Params {
        Params {
            day: day as f32,
            dt: dt_days as f32,
            leeway: rules.leeway,
            ice_hold: rules.ice_hold,
            wander: rules.wander,
            shore: rules.shore,
            seed_lo: seed as u32,
            seed_hi: (seed >> 32) as u32,
            count,
            _pad: [0; 3],
        }
    }
}

pub struct Maybes {
    walk: wgpu::ComputePipeline,
    flight: wgpu::ComputePipeline,
    bind: wgpu::BindGroup,
    params: wgpu::Buffer,
    paths: wgpu::Buffer,
    readback: wgpu::Buffer,
    pub capacity: u32,
    _wind: Texture,
    _eddy: Texture,
    _eddy_time: Texture,
}

impl Maybes {
    /// Room for `capacity` paths. The sheets the look already holds are shared; wind, eddy and
    /// eddy time are read from the assets here.
    pub fn new(gpu: &Gpu, assets: &dyn Assets, textures: &Textures, capacity: u32) -> anyhow::Result<Maybes> {
        let wind = Texture::flat(gpu, "wind", &decode(&assets.read("wind.png")?, 4)?);
        let eddy = Texture::flat(gpu, "eddy", &decode(&assets.read("eddy.png")?, 4)?);
        let eddy_time = Texture::flat(gpu, "eddy-time", &decode(&assets.read("eddy-time.png")?, 4)?);
        let tex = |binding, dim| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: dim, multisampled: false },
            count: None,
        };
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("maybes"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                tex(1, wgpu::TextureViewDimension::D2),
                tex(2, wgpu::TextureViewDimension::D2),
                tex(3, wgpu::TextureViewDimension::D2),
                tex(4, wgpu::TextureViewDimension::D2Array),
                tex(5, wgpu::TextureViewDimension::D2),
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Storage { read_only: false }, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
                tex(7, wgpu::TextureViewDimension::D2),
            ],
        });
        let params = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("maybes params"),
            size: std::mem::size_of::<Params>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let size = capacity as u64 * std::mem::size_of::<GpuPath>() as u64;
        let paths = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("maybes paths"),
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("maybes readback"),
            size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("maybes"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: params.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&textures.currents.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&wind.view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&eddy.view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&textures.ice.view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&textures.geo.view) },
                wgpu::BindGroupEntry { binding: 6, resource: paths.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::TextureView(&eddy_time.view) },
            ],
        });
        let shader = gpu.shader("rules", RULES_WGSL);
        let pipeline_layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("maybes"),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let make = |entry: &str| {
            gpu.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some(entry),
                compilation_options: Default::default(),
                cache: None,
            })
        };
        Ok(Maybes { walk: make("maybes"), flight: make("maybes_flight"), bind, params, paths, readback, capacity, _wind: wind, _eddy: eddy, _eddy_time: eddy_time })
    }

    /// Writes the paths to step (at most `capacity`).
    pub fn seed(&self, gpu: &Gpu, paths: &[GpuPath]) {
        assert!(paths.len() as u32 <= self.capacity);
        gpu.queue.write_buffer(&self.paths, 0, bytemuck::cast_slice(paths));
    }

    /// One step of every path (k = its index + 1), with memory (`flight`) or without.
    pub fn step(&self, gpu: &Gpu, params: Params, flight: bool) {
        gpu.queue.write_buffer(&self.params, 0, bytemuck::bytes_of(&params));
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("maybes") });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("maybes"), timestamp_writes: None });
            pass.set_pipeline(if flight { &self.flight } else { &self.walk });
            pass.set_bind_group(0, &self.bind, &[]);
            pass.dispatch_workgroups((params.count + 63) / 64, 1, 1);
        }
        gpu.queue.submit([enc.finish()]);
    }

    /// The paths' buffer, for a drawing pass to read.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.paths
    }

    /// Reads `count` paths back (waits for the GPU).
    pub fn read(&self, gpu: &Gpu, count: u32) -> Vec<GpuPath> {
        let size = count as u64 * std::mem::size_of::<GpuPath>() as u64;
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        enc.copy_buffer_to_buffer(&self.paths, 0, &self.readback, 0, size);
        gpu.queue.submit([enc.finish()]);
        let slice = self.readback.slice(..size);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        gpu.device.poll(wgpu::PollType::wait()).expect("poll");
        let out = bytemuck::cast_slice::<u8, GpuPath>(&slice.get_mapped_range()).to_vec();
        self.readback.unmap();
        out
    }
}
