//! The floats and the ring that thickens into one (`floats.wgsl`).
use super::{pipeline, shader, Instances};
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
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<FloatInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32, 3 => Float32, 4 => Float32],
};

pub struct Floats {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<FloatInstance>,
}

impl Floats {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, view: &wgpu::BindGroupLayout) -> Self {
        let module = shader(device, "floats", include_str!("floats.wgsl"));
        Self { pipeline: pipeline(device, "floats", &module, format, LAYOUT, view, None), instances: Instances::new(device, "floats", 16) }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, floats: &[FloatInstance]) {
        self.instances.write(device, queue, "floats", floats);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 { return; }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        pass.draw(0..6, 0..self.instances.count);
    }
}
