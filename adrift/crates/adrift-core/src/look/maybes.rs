//! The maybes: thousands of pale specks (`maybes.wgsl`) drawn from the GPU buffer Truth's rules
//! fill (`gpu::Maybes`: one `vec4f` a maybe, lon, lat, held, k); the look only draws them.
use super::{mark_pipeline, marks_wgsl, Uniform};
use crate::gpu::{Gpu, Maybes};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct MaybesUniform {
    /// 0..1 as they bloom out of the float and fold back
    pub bloom: f32,
    pub night: f32,
    pub _pad: [f32; 2],
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 16,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x4],
};

pub struct MaybesPass {
    pipeline: wgpu::RenderPipeline,
    uniform: Uniform<MaybesUniform>,
}

impl MaybesPass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout) -> MaybesPass {
        let module = gpu.shader("maybes draw", &marks_wgsl(include_str!("maybes.wgsl")));
        let uniform = Uniform::new(gpu, "maybes draw");
        uniform.write(gpu, &MaybesUniform { bloom: 1.0, night: 0.0, _pad: [0.0; 2] });
        MaybesPass { pipeline: mark_pipeline(gpu, "maybes draw", &module, LAYOUT, view_layout, Some(&uniform.layout)), uniform }
    }

    /// How far the maybes have bloomed (0..1) and whether it is night; 1 and day until told.
    pub fn prepare(&mut self, gpu: &Gpu, bloom: f32, night: bool) {
        self.uniform.write(gpu, &MaybesUniform { bloom, night: night as u8 as f32, _pad: [0.0; 2] });
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, maybes: &'a Maybes) {
        self.draw_buffer(rp, view_bind, &maybes.buffer, maybes.count);
    }

    /// The same from any buffer of `count` `vec4f`s (the bench's).
    pub fn draw_buffer<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, buffer: &'a wgpu::Buffer, count: u32) {
        if count == 0 {
            return;
        }
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.uniform.group, &[]);
        rp.set_vertex_buffer(0, buffer.slice(..(count as u64) * 16));
        rp.draw(0..6, 0..count);
    }
}
