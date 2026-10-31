//! The maybes: thousands of pale specks (`maybes.wgsl`) drawn from the GPU buffer Truth's rules
//! fill (`gpu::Maybes`: one `vec4f` a maybe, lon, lat, held, k); the look only draws them.
use super::{mark_pipeline, marks_wgsl, Uniform};
use crate::gpu::Gpu;
use crate::sea::Maybes;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct MaybesUniform {
    /// the held float's place, a unit vector: where the specks bloom from
    pub origin: [f32; 4],
    /// 0..1 as they bloom out of the float and fold back
    pub bloom: f32,
    pub night: f32,
    pub _pad: [f32; 2],
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 24,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32],
};

pub struct MaybesPass {
    pipeline: wgpu::RenderPipeline,
    uniform: Uniform<MaybesUniform>,
}

impl MaybesPass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout) -> MaybesPass {
        let module = gpu.shader("maybes draw", &marks_wgsl(include_str!("maybes.wgsl")));
        let uniform = Uniform::new(gpu, "maybes draw");
        uniform.write(gpu, &MaybesUniform { origin: [0.0, 0.0, 1.0, 0.0], bloom: 1.0, night: 0.0, _pad: [0.0; 2] });
        MaybesPass { pipeline: mark_pipeline(gpu, "maybes draw", &module, LAYOUT, view_layout, Some(&uniform.layout)), uniform }
    }

    /// Where the specks bloom from (the held float's unit vector), how far they have bloomed (0..1) and
    /// whether it is night.
    pub fn prepare(&mut self, gpu: &Gpu, origin: [f32; 3], bloom: f32, night: bool) {
        self.uniform.write(gpu, &MaybesUniform { origin: [origin[0], origin[1], origin[2], 0.0], bloom, night: night as u8 as f32, _pad: [0.0; 2] });
    }

    /// `count` of the paths in Truth's buffer (`sea::Maybes`), as the app last ran them.
    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, maybes: &'a Maybes, count: u32) {
        self.draw_buffer(rp, view_bind, maybes.buffer(), count);
    }

    /// The same from any buffer of `count` paths in `sea::GpuPath`'s layout.
    pub fn draw_buffer<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, buffer: &'a wgpu::Buffer, count: u32) {
        if count == 0 {
            return;
        }
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.uniform.group, &[]);
        rp.set_vertex_buffer(0, buffer.slice(..(count as u64) * 24));
        rp.draw(0..6, 0..count);
    }
}
