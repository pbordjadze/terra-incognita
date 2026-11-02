//! The maybes: thousands of pale specks (`maybes.wgsl`) drawn from the GPU buffer Truth's rules
//! fill (`gpu::Maybes`: one `vec4f` a maybe, lon, lat, held, k); the look only draws them.
use super::{marks_wgsl, Uniform};
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
    /// 0 the cloud in front; the others out with it a little quieter (R6)
    pub quiet: f32,
    /// its partner cloud's hue (R6, the worlds): the cloud whose i-th self met the same sea as this
    /// one's i-th; w 1 if there is one
    pub partner_hue: [f32; 4],
    /// how many selves the partner has (an i past it has no partner)
    pub partner_count: u32,
    /// 1 a cloud of seeds on the wind (R21): held 1 down on land, 2 down on the sea
    pub wind: u32,
    pub _p: [u32; 2],
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 24,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Uint32],
};

/// The same buffer again from the partner cloud's range (R6, the worlds): the i-th instance reads its
/// own self from the first and its partner's i-th, the self that met the same sea, from this one.
const PARTNER: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: 24,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![2 => Float32x2],
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
    /// The one in front (R6): drawn whole; the others with it a shade quieter.
    pub front: bool,
    /// A seed's cloud (R21): its selves come down on land or on the sea are drawn so.
    pub wind: bool,
}

/// The most clouds drawn at once (the app's `MAX_CLOUDS`).
pub const MAX_DRAWN: usize = 8;

pub struct MaybesPass {
    pipeline: wgpu::RenderPipeline,
    /// One uniform a cloud slot, written in `prepare`.
    uniforms: Vec<Uniform<MaybesUniform>>,
    /// What `draw` draws: (uniform slot, base, count, the partner's base).
    clouds: Vec<(usize, u32, u32, u32)>,
}

impl MaybesPass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout) -> MaybesPass {
        let module = gpu.shader("maybes draw", &marks_wgsl(include_str!("maybes.wgsl")));
        let uniforms: Vec<Uniform<MaybesUniform>> = (0..MAX_DRAWN).map(|_| Uniform::new(gpu, "maybes draw")).collect();
        let pipeline = pipeline(gpu, &module, view_layout, &uniforms[0].layout);
        MaybesPass { pipeline, uniforms, clouds: Vec::new() }
    }

    /// The clouds to draw this frame, in order (the last on top), and whether it is night.
    pub fn prepare(&mut self, gpu: &Gpu, clouds: &[CloudDraw], night: bool) {
        self.clouds.clear();
        let clouds = &clouds[..clouds.len().min(MAX_DRAWN)];
        // each cloud's partner (R6, the worlds): the cloud in front for every other one, and for the
        // one in front the one out before it; a lone cloud has none
        let front = clouds.iter().rposition(|c| c.front).unwrap_or(clouds.len().saturating_sub(1));
        for (slot, c) in clouds.iter().enumerate() {
            if c.count == 0 || c.bloom <= 0.0 {
                continue;
            }
            let partner = if clouds.len() < 2 { None } else if slot != front { Some(front) } else { (0..clouds.len()).rev().find(|&i| i != front) };
            let (ph, pbase, pcount) = match partner.map(|i| clouds[i]) {
                Some(p) if p.count > 0 => ([p.hue[0], p.hue[1], p.hue[2], 1.0], p.base, p.count),
                _ => ([0.0; 4], c.base, 0),
            };
            // the field's density follows the count drawn: it reads the same at any budget
            let u = MaybesUniform { origin: [c.origin[0], c.origin[1], c.origin[2], 0.0], hue: [c.hue[0], c.hue[1], c.hue[2], 0.0], bloom: c.bloom, night: night as u8 as f32, density: 512.0 / c.count as f32, quiet: if c.front || clouds.len() == 1 { 0.0 } else { 0.18 }, partner_hue: ph, partner_count: pcount, wind: c.wind as u32, _p: [0; 2] };
            self.uniforms[slot].write(gpu, &u);
            self.clouds.push((slot, c.base, c.count, pbase));
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
        // several clouds out (R6) are laid in turns, a slice of each in turn, so where they overlap
        // neither lies wholly over the other: their hues mix as two sands poured together, and where
        // the currents part them each is its own colour again
        let slices = if self.clouds.len() > 1 { 8u32 } else { 1 };
        let end = buffer.size();
        for s in 0..slices {
            for &(slot, base, count, pbase) in &self.clouds {
                let (a, b) = (count * s / slices, count * (s + 1) / slices);
                if b <= a {
                    continue;
                }
                rp.set_bind_group(1, &self.uniforms[slot].group, &[]);
                // (the instances keep their index i within the cloud, so the partner's i-th is read
                // from its own range in the second slot)
                rp.set_vertex_buffer(0, buffer.slice(base as u64 * 24..end));
                rp.set_vertex_buffer(1, buffer.slice(pbase as u64 * 24..end));
                rp.draw(0..6, a..b);
            }
        }
    }
}

/// The maybes' pipeline: `mark_pipeline`'s, with the partner's slot beside the cloud's own.
fn pipeline(gpu: &Gpu, module: &wgpu::ShaderModule, view: &wgpu::BindGroupLayout, own: &wgpu::BindGroupLayout) -> wgpu::RenderPipeline {
    let layout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("maybes draw"), bind_group_layouts: &[view, own], push_constant_ranges: &[] });
    gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("maybes draw"),
        layout: Some(&layout),
        vertex: wgpu::VertexState { module, entry_point: Some("vs"), buffers: &[LAYOUT, PARTNER], compilation_options: Default::default() },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
        }),
        primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleList, ..Default::default() },
        depth_stencil: None,
        multisample: Default::default(),
        multiview: None,
        cache: None,
    })
}
