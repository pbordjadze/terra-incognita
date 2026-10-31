use crate::gpu::{Gpu, Maybes};

/// Draws the maybes from the compute buffer (one instance a maybe).
pub struct MaybesPass {
    pipeline: wgpu::RenderPipeline,
}

impl MaybesPass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout) -> MaybesPass {
        let shader = gpu.shader("maybes draw", &super::wgsl(include_str!("maybes.wgsl")));
        let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("maybes draw"), bind_group_layouts: &[view_layout], push_constant_ranges: &[] });
        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("maybes draw"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_maybe"),
                buffers: &[wgpu::VertexBufferLayout { array_stride: 16, step_mode: wgpu::VertexStepMode::Instance, attributes: &wgpu::vertex_attr_array![0 => Float32x4] }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_maybe"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        MaybesPass { pipeline }
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, maybes: &'a Maybes) {
        if maybes.count == 0 {
            return;
        }
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_vertex_buffer(0, maybes.buffer.slice(..(maybes.count as u64) * 16));
        rp.draw(0..4, 0..maybes.count);
    }
}
