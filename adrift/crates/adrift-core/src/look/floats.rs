//! The floats and the ring that thickens into one (`floats.wgsl`).
use super::{mark_pipeline, marks_wgsl, Instances};
use crate::gpu::Gpu;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct FloatInstance {
    pub g: [f32; 3],
    /// 0..1 while the finger rests and the ring thickens; 1 a float
    pub beat: f32,
    /// 1 while it is what you just did, fading in time
    pub lit: f32,
    /// 1 while held (its maybes out)
    pub held: f32,
    /// 1 by night
    pub night: f32,
    /// how measured the sea is where it rides (0..1); under a half the rim is dashed
    pub measured: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<FloatInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32, 3 => Float32, 4 => Float32, 5 => Float32],
};

pub struct Floats {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<FloatInstance>,
}

impl Floats {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        let module = gpu.shader("floats", &marks_wgsl(include_str!("floats.wgsl")));
        Self { pipeline: mark_pipeline(gpu, "floats", &module, LAYOUT, view, None), instances: Instances::new(gpu, "floats", 16) }
    }

    pub fn prepare(&mut self, gpu: &Gpu, floats: &[FloatInstance]) {
        self.instances.write(gpu, floats);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 { return; }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        pass.draw(0..6, 0..self.instances.count);
    }
}
