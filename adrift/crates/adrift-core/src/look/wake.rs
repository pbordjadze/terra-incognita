//! The wake: one bead a day (`wake.wgsl`).
use super::{mark_pipeline, marks_wgsl, Instances, Uniform};
use crate::gpu::Gpu;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct BeadInstance {
    pub g: [f32; 3],
    /// 0 the newest bead .. 1 the oldest shown
    pub age: f32,
    pub night: f32,
    /// how measured the sea was there (0..1); under a half the bead is hollow
    pub measured: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<BeadInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32, 3 => Float32],
};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct WakeUniform {
    /// 0 this draw lays the edges, 1 the beads
    pub layer: f32,
    pub _pad: [f32; 3],
}

pub struct Wake {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<BeadInstance>,
    /// the edges' draw, then the beads'
    uniforms: [Uniform<WakeUniform>; 2],
}

impl Wake {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        let module = gpu.shader("wake", &marks_wgsl(include_str!("wake.wgsl")));
        let uniforms = [Uniform::new(gpu, "wake edges"), Uniform::new(gpu, "wake beads")];
        for (i, u) in uniforms.iter().enumerate() {
            u.write(gpu, &WakeUniform { layer: i as f32, _pad: [0.0; 3] });
        }
        Self { pipeline: mark_pipeline(gpu, "wake", &module, LAYOUT, view, Some(&uniforms[0].layout)), instances: Instances::new(gpu, "wake", 256), uniforms }
    }

    /// The beads of every float's wake: each wake's positions oldest first, one a day.
    pub fn prepare(&mut self, gpu: &Gpu, beads: &[BeadInstance]) {
        self.instances.write(gpu, beads);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 { return; }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        for u in &self.uniforms {
            pass.set_bind_group(1, &u.group, &[]);
            pass.draw(0..6, 0..self.instances.count);
        }
    }
}

/// A wake's beads from its daily places, oldest first.
pub fn beads(path: &[super::LookBead], night: bool) -> Vec<BeadInstance> {
    let n = path.len().saturating_sub(1).max(1) as f32;
    path.iter().enumerate().map(|(i, b)| BeadInstance { g: super::unit(b.at), age: 1.0 - i as f32 / n, night: night as u8 as f32, measured: b.measured }).collect()
}
