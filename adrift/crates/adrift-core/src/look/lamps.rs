//! Everyone else's lamps (Together, painting-company row 199): a warm point each on the night sea,
//! breathing like a candle, and when two meet, a glow leaning toward the other (`lamps.wgsl`).
use super::{mark_pipeline, marks_wgsl, unit, Instances};
use crate::gpu::Gpu;
use crate::together::OtherLamp;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct LampInstance {
    pub g: [f32; 3],
    pub alpha: f32,
    /// whom a meeting's glow leans toward
    pub toward: [f32; 3],
    pub glow: f32,
    pub warm: f32,
    pub arrival: f32,
    /// a phase for its breathing, its own
    pub phase: f32,
    pub _pad: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<LampInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32, 2 => Float32x3, 3 => Float32, 4 => Float32, 5 => Float32, 6 => Float32, 7 => Float32],
};

pub struct Lamps {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<LampInstance>,
}

impl Lamps {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        let module = gpu.shader("lamps", &marks_wgsl(include_str!("lamps.wgsl")));
        Self { pipeline: mark_pipeline(gpu, "lamps", &module, LAYOUT, view, None), instances: Instances::new(gpu, "lamps", 4096) }
    }

    pub fn prepare(&mut self, gpu: &Gpu, lamps: &[OtherLamp]) {
        let items: Vec<LampInstance> = lamps
            .iter()
            .enumerate()
            .filter(|(_, l)| l.alpha > 0.0)
            .map(|(i, l)| LampInstance { g: unit(l.at), alpha: l.alpha, toward: unit(l.toward), glow: l.glow, warm: l.warm, arrival: l.arrival, phase: (i as f32 * 2.399).fract() * 6.283, _pad: 0.0 })
            .collect();
        self.instances.write(gpu, &items);
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        pass.draw(0..6, 0..self.instances.count);
    }
}
