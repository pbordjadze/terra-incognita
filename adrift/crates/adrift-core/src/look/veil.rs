//! The veil (`veil.wgsl`): the night deepening round the chosen lamp so its cloud is what the eye
//! goes to; a full-screen pass over the stage, before the marks.
use super::{marks_wgsl, Uniform};
use crate::gpu::Gpu;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct VeilUniform {
    pub centre: [f32; 4],
    pub radius: f32,
    pub soft: f32,
    pub strength: f32,
    pub night: f32,
    /// (R22) the panel's ground, x, y, w, h in points, and its strength (0 none)
    pub panel: [f32; 4],
    pub panel_a: f32,
    pub _p: [f32; 3],
}

pub struct Veil {
    pipeline: wgpu::RenderPipeline,
    uniform: Uniform<VeilUniform>,
    on: bool,
    last: VeilUniform,
}

impl Veil {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Veil {
        let module = gpu.shader("veil", &marks_wgsl(include_str!("veil.wgsl")));
        let uniform = Uniform::new(gpu, "veil");
        let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("veil"), bind_group_layouts: &[view, &uniform.layout], push_constant_ranges: &[] });
        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("veil"),
            layout: Some(&layout),
            vertex: wgpu::VertexState { module: &module, entry_point: Some("vs"), buffers: &[], compilation_options: Default::default() },
            fragment: Some(wgpu::FragmentState { module: &module, entry_point: Some("fs"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(Gpu::over()), write_mask: wgpu::ColorWrites::ALL })] }),
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            multiview: None,
            cache: None,
        });
        Veil { pipeline, uniform, on: false, last: VeilUniform::default() }
    }

    /// The window round `centre` (a unit vector), `strength` 0..1 with the cloud's bloom.
    pub fn prepare(&mut self, gpu: &Gpu, centre: [f32; 3], strength: f32, night: bool) {
        self.last = VeilUniform { centre: [centre[0], centre[1], centre[2], 0.0], radius: 120.0, soft: 110.0, strength, night: night as u8 as f32, ..Default::default() };
        self.on = strength > 0.0;
        self.uniform.write(gpu, &self.last);
    }

    /// The panel's own ground (R22) under the marks' rect `r` (x, y, w, h in points), `a` 0..1; after
    /// `prepare`.
    pub fn panel(&mut self, gpu: &Gpu, r: [f32; 4], a: f32) {
        self.last.panel = r;
        self.last.panel_a = a;
        self.on = self.last.strength > 0.0 || a > 0.0;
        self.uniform.write(gpu, &self.last);
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup) {
        if !self.on {
            return;
        }
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.uniform.group, &[]);
        rp.draw(0..6, 0..1);
    }
}
