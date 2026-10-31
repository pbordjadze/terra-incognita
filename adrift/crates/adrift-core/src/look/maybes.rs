//! The maybes: thousands of pale specks (`maybes.wgsl`). Their positions come from Truth's sampler
//! (a buffer `rules.wgsl` writes, or a slice from the CPU); the look only draws them.
use super::{mark_pipeline, marks_wgsl, Instances, Uniform};
use crate::gpu::Gpu;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SpeckInstance {
    pub g: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct MaybesUniform {
    /// 0..1 as they bloom out of the float and fold back
    pub bloom: f32,
    pub night: f32,
    pub _pad: [f32; 2],
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<SpeckInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
};

pub struct Maybes {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<SpeckInstance>,
    uniform: Uniform<MaybesUniform>,
}

impl Maybes {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        let module = gpu.shader("maybes", &marks_wgsl(include_str!("maybes.wgsl")));
        let uniform = Uniform::new(gpu, "maybes");
        Self { pipeline: mark_pipeline(gpu, "maybes", &module, LAYOUT, view, Some(&uniform.layout)), instances: Instances::new(gpu, "maybes", 4096), uniform }
    }

    pub fn prepare(&mut self, gpu: &Gpu, specks: &[SpeckInstance], bloom: f32, night: bool) {
        self.instances.write(gpu, specks);
        self.uniform.write(gpu, &MaybesUniform { bloom, night: night as u8 as f32, _pad: [0.0; 2] });
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 { return; }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_bind_group(1, &self.uniform.group, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        pass.draw(0..6, 0..self.instances.count);
    }
}
