use crate::gpu::Gpu;

/// One thing on the screen: a float, a bead, a ring, a maybe, a drifter (sprites.wgsl's Instance).
#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instance {
    pub at: [f32; 2],
    pub size: f32,
    pub kind: u32,
    pub color: [f32; 4],
    pub t: f32,
    pub _pad: [f32; 3],
}

pub const KIND_FLOAT: u32 = 0;
pub const KIND_BEAD: u32 = 1;
pub const KIND_RING: u32 = 2;
pub const KIND_MAYBE: u32 = 3;
pub const KIND_DRIFTER: u32 = 4;

pub struct SpritePass {
    pipeline: wgpu::RenderPipeline,
    buffer: wgpu::Buffer,
    capacity: usize,
    count: u32,
}

impl SpritePass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout) -> SpritePass {
        let shader = gpu.shader("sprites", &super::wgsl(include_str!("sprites.wgsl")));
        let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprites"),
            bind_group_layouts: &[view_layout],
            push_constant_ranges: &[],
        });
        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("sprites"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_sprite"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32, 2 => Uint32, 3 => Float32x4, 4 => Float32],
                }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sprite"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        let capacity = 4096;
        let buffer = Self::buffer(gpu, capacity);
        SpritePass { pipeline, buffer, capacity, count: 0 }
    }

    fn buffer(gpu: &Gpu, capacity: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprites"),
            size: (capacity * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn prepare(&mut self, gpu: &Gpu, instances: &[Instance]) {
        if instances.len() > self.capacity {
            self.capacity = instances.len().next_power_of_two();
            self.buffer = Self::buffer(gpu, self.capacity);
        }
        if !instances.is_empty() {
            gpu.queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(instances));
        }
        self.count = instances.len() as u32;
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup) {
        if self.count == 0 {
            return;
        }
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_vertex_buffer(0, self.buffer.slice(..));
        rp.draw(0..4, 0..self.count);
    }
}
