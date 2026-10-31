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
    /// its lamp's hue
    pub hue: [f32; 4],
    /// 0..1 as they bloom out of the float and fold back
    pub bloom: f32,
    pub night: f32,
    /// 512 over the count drawn: the field's alpha a kernel scales so it reads the same at any budget
    pub density: f32,
    pub _pad: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 24,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32],
};

/// One cloud as the pass draws it (R6: several at once, each its own range of the buffer).
#[derive(Clone, Copy, Debug)]
pub struct CloudDraw {
    /// Its first path in the buffer and how many.
    pub base: u32,
    pub count: u32,
    pub origin: [f32; 3],
    pub hue: [f32; 3],
    pub bloom: f32,
}

/// The most clouds drawn at once (the app's `MAX_CLOUDS`).
pub const MAX_DRAWN: usize = 8;

pub struct MaybesPass {
    pipeline: wgpu::RenderPipeline,
    /// One uniform a cloud slot, written in `prepare`.
    uniforms: Vec<Uniform<MaybesUniform>>,
    /// What `draw` draws: (uniform slot, base, count).
    clouds: Vec<(usize, u32, u32)>,
}

impl MaybesPass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout) -> MaybesPass {
        let module = gpu.shader("maybes draw", &marks_wgsl(include_str!("maybes.wgsl")));
        let uniforms: Vec<Uniform<MaybesUniform>> = (0..MAX_DRAWN).map(|_| Uniform::new(gpu, "maybes draw")).collect();
        let pipeline = mark_pipeline(gpu, "maybes draw", &module, LAYOUT, view_layout, Some(&uniforms[0].layout));
        MaybesPass { pipeline, uniforms, clouds: Vec::new() }
    }

    /// The clouds to draw this frame, in order (the last on top), and whether it is night.
    pub fn prepare(&mut self, gpu: &Gpu, clouds: &[CloudDraw], night: bool) {
        self.clouds.clear();
        for (slot, c) in clouds.iter().take(MAX_DRAWN).enumerate() {
            if c.count == 0 || c.bloom <= 0.0 {
                continue;
            }
            // the field's density follows the count drawn: it reads the same at any budget
            let u = MaybesUniform { origin: [c.origin[0], c.origin[1], c.origin[2], 0.0], hue: [c.hue[0], c.hue[1], c.hue[2], 0.0], bloom: c.bloom, night: night as u8 as f32, density: 512.0 / c.count as f32, _pad: 0.0 };
            self.uniforms[slot].write(gpu, &u);
            self.clouds.push((slot, c.base, c.count));
        }
    }

    /// The prepared clouds from Truth's buffer (`sea::Maybes`).
    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, maybes: &'a Maybes) {
        self.draw_buffer(rp, view_bind, maybes.buffer());
    }

    /// The same from any buffer of paths in `sea::GpuPath`'s layout.
    pub fn draw_buffer<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup, buffer: &'a wgpu::Buffer) {
        if self.clouds.is_empty() {
            return;
        }
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        for &(slot, base, count) in &self.clouds {
            rp.set_bind_group(1, &self.uniforms[slot].group, &[]);
            rp.set_vertex_buffer(0, buffer.slice(base as u64 * 24..(base + count) as u64 * 24));
            rp.draw(0..6, 0..count);
        }
    }
}
