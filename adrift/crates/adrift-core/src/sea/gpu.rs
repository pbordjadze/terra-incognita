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
    /// The sea's: 0 free, 1 coast, 2 ice. The wind's (`Aloft::code`, R21): 0 flying, 1 down on
    /// land, 2 down on the sea. Which a cloud's codes are is its leg's medium.
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
    /// The cloud's first path in the buffer (R6: many clouds share one buffer, each its own range;
    /// k is still 1..=count within the range).
    pub base: u32,
    /// The law the cloud's leg was made under (R13, `Leg::law`).
    pub law: u32,
    pub _pad: u32,
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
            base: 0,
            law: super::rules::LAW,
            _pad: 0,
        }
    }

    /// The same under the law of the cloud's leg (R13).
    pub fn under(mut self, law: u32) -> Params {
        self.law = law;
        self
    }

    /// The same for the cloud whose paths begin at `base`.
    pub fn at_base(mut self, base: u32) -> Params {
        self.base = base;
        self
    }
}

/// A checkpoint of the paths buffer: a copy on the GPU (no readback), to restore and step on
/// from, so a rewind of a shown cloud is a copy and a few steps instead of a run from the put.
/// The app records the day it was kept at.
pub struct Keep {
    buffer: wgpu::Buffer,
    pub count: u32,
    /// The first path it was kept from (and is restored to).
    pub base: u32,
}

/// A checkpoint of `count` paths of `paths` from `base` (a copy on the GPU). Shared by the sea's
/// `Maybes` and the wind's `AirMaybes` (R21: both step clouds in the one buffer).
pub(crate) fn keep_from(gpu: &Gpu, paths: &wgpu::Buffer, base: u32, count: u32) -> Keep {
    let size = count as u64 * std::mem::size_of::<GpuPath>() as u64;
    let from = base as u64 * std::mem::size_of::<GpuPath>() as u64;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("maybes keep"),
        size: size.max(4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(paths, from, &buffer, 0, size);
    gpu.queue.submit([enc.finish()]);
    Keep { buffer, count, base }
}

/// A checkpoint's paths copied back into `paths` where they were kept from.
pub(crate) fn restore_into(gpu: &Gpu, paths: &wgpu::Buffer, keep: &Keep) {
    let size = keep.count as u64 * std::mem::size_of::<GpuPath>() as u64;
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(&keep.buffer, 0, paths, keep.base as u64 * std::mem::size_of::<GpuPath>() as u64, size);
    gpu.queue.submit([enc.finish()]);
}

/// Reads `count` paths of `paths` from `base` back through `readback` (waits for the GPU).
pub(crate) fn read_from(gpu: &Gpu, paths: &wgpu::Buffer, readback: &wgpu::Buffer, base: u32, count: u32) -> Vec<GpuPath> {
    let size = count as u64 * std::mem::size_of::<GpuPath>() as u64;
    let mut enc = gpu.device.create_command_encoder(&Default::default());
    enc.copy_buffer_to_buffer(paths, base as u64 * std::mem::size_of::<GpuPath>() as u64, readback, 0, size);
    gpu.queue.submit([enc.finish()]);
    let slice = readback.slice(..size);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.device.poll(wgpu::PollType::wait_indefinitely()).expect("poll");
    let out = bytemuck::cast_slice::<u8, GpuPath>(&slice.get_mapped_range().expect("mapped")).to_vec();
    readback.unmap();
    out
}

/// The most steps one `step_many` call encodes (its parameters ride a ring of uniform slots).
pub const MAX_STEPS: u32 = 1024;
pub(crate) const SLOT: u64 = 256;

pub struct Maybes {
    walk: wgpu::ComputePipeline,
    flight: wgpu::ComputePipeline,
    bind: wgpu::BindGroup,
    params: wgpu::Buffer,
    paths: wgpu::Buffer,
    readback: wgpu::Buffer,
    pub capacity: u32,
    /// The wind's maybes (R21, Truth): `wind.wgsl`'s pipeline over this same paths buffer, so a
    /// seed's cloud lies in its slot as a lamp's does and the look reads both alike.
    pub air: super::wind::AirMaybes,
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
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: true, min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Params>() as u64) },
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
            size: SLOT * MAX_STEPS as u64,
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
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding { buffer: &params, offset: 0, size: wgpu::BufferSize::new(std::mem::size_of::<Params>() as u64) }),
                },
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
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
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
        let air = super::wind::AirMaybes::sharing(gpu, assets, textures, &wind.view, paths.clone(), capacity)?;
        Ok(Maybes { walk: make("maybes"), flight: make("maybes_flight"), bind, params, paths, readback, capacity, air, _wind: wind, _eddy: eddy, _eddy_time: eddy_time })
    }

    /// Writes the paths to step (at most `capacity`).
    pub fn seed(&self, gpu: &Gpu, paths: &[GpuPath]) {
        self.seed_at(gpu, 0, paths);
    }

    /// Writes a cloud's paths from `base` (R6).
    pub fn seed_at(&self, gpu: &Gpu, base: u32, paths: &[GpuPath]) {
        assert!(base + paths.len() as u32 <= self.capacity);
        gpu.queue.write_buffer(&self.paths, base as u64 * std::mem::size_of::<GpuPath>() as u64, bytemuck::cast_slice(paths));
    }

    /// One step of every path (k = its index + 1), with memory (`flight`) or without.
    pub fn step(&self, gpu: &Gpu, params: Params, flight: bool) {
        self.step_many(gpu, &[params], flight);
    }

    /// Several steps in one submit, each with its own parameters (at most `MAX_STEPS`): a scrub
    /// that must catch a cloud up by many steps costs one command buffer, not one a step. The
    /// result is the same as the steps one by one.
    pub fn step_many(&self, gpu: &Gpu, steps: &[Params], flight: bool) {
        assert!(steps.len() as u32 <= MAX_STEPS, "at most {MAX_STEPS} steps a call");
        if steps.is_empty() {
            return;
        }
        let mut bytes = vec![0u8; SLOT as usize * steps.len()];
        for (i, p) in steps.iter().enumerate() {
            bytes[i * SLOT as usize..i * SLOT as usize + std::mem::size_of::<Params>()].copy_from_slice(bytemuck::bytes_of(p));
        }
        gpu.queue.write_buffer(&self.params, 0, &bytes);
        let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("maybes") });
        {
            let mut pass = enc.begin_compute_pass(&wgpu::ComputePassDescriptor { label: Some("maybes"), timestamp_writes: None });
            pass.set_pipeline(if flight { &self.flight } else { &self.walk });
            for (i, p) in steps.iter().enumerate() {
                pass.set_bind_group(0, &self.bind, &[(i as u64 * SLOT) as u32]);
                pass.dispatch_workgroups((p.count + 63) / 64, 1, 1);
            }
        }
        gpu.queue.submit([enc.finish()]);
    }

    /// A checkpoint: the paths as they are now, copied on the GPU.
    pub fn keep(&self, gpu: &Gpu, count: u32) -> Keep {
        self.keep_at(gpu, 0, count)
    }

    /// A checkpoint of the cloud whose `count` paths begin at `base` (R6).
    pub fn keep_at(&self, gpu: &Gpu, base: u32, count: u32) -> Keep {
        keep_from(gpu, &self.paths, base, count)
    }

    /// The paths put back as a checkpoint had them.
    pub fn restore(&self, gpu: &Gpu, keep: &Keep) {
        restore_into(gpu, &self.paths, keep)
    }

    /// The paths' buffer, for a drawing pass to read.
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.paths
    }

    /// Reads `count` paths back (waits for the GPU).
    pub fn read(&self, gpu: &Gpu, count: u32) -> Vec<GpuPath> {
        read_from(gpu, &self.paths, &self.readback, 0, count)
    }

    /// Reads the `count` paths from `base` back (waits for the GPU).
    pub fn read_at(&self, gpu: &Gpu, base: u32, count: u32) -> Vec<GpuPath> {
        read_from(gpu, &self.paths, &self.readback, base, count)
    }
}
