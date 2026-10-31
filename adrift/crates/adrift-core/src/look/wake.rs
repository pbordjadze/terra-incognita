//! The wake: one bead a day (`wake.wgsl`).
use super::{pipeline, shader, Instances};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BeadInstance {
    pub g: [f32; 3],
    /// 0 the newest bead .. 1 the oldest shown
    pub age: f32,
    pub night: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<BeadInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32],
};

pub struct Wake {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<BeadInstance>,
}

impl Wake {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, view: &wgpu::BindGroupLayout) -> Self {
        let module = shader(device, "wake", include_str!("wake.wgsl"));
        Self { pipeline: pipeline(device, "wake", &module, format, LAYOUT, view, None), instances: Instances::new(device, "wake", 256) }
    }

    /// The beads of every float's wake: each wake's positions oldest first, one a day.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, beads: &[BeadInstance]) {
        self.instances.write(device, queue, "wake", beads);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 { return; }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        pass.draw(0..6, 0..self.instances.count);
    }
}

/// A wake's beads from its daily positions, oldest first.
pub fn beads(path: &[[f32; 3]], night: bool) -> Vec<BeadInstance> {
    let n = path.len().saturating_sub(1).max(1) as f32;
    path.iter().enumerate().map(|(i, g)| BeadInstance { g: *g, age: 1.0 - i as f32 / n, night: night as u8 as f32 }).collect()
}
