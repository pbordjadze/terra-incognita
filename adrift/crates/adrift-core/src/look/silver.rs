//! The nearest real drifter's track in silver (`silver.wgsl`).
use super::{pipeline, shader, Instances, Uniform};
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SegmentInstance {
    pub a: [f32; 3],
    pub b: [f32; 3],
    /// 0..1 along the track
    pub t: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct SilverUniform {
    /// 0..1 as it rises among the maybes and sinks back
    pub rise: f32,
    pub night: f32,
    /// 0 this draw lays the dark lines, 1 the wires
    pub layer: f32,
    pub _pad: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<SegmentInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32],
};

pub struct Silver {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<SegmentInstance>,
    /// the dark lines' draw, then the wires'
    uniforms: [Uniform<SilverUniform>; 2],
}

impl Silver {
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat, view: &wgpu::BindGroupLayout) -> Self {
        let module = shader(device, "silver", include_str!("silver.wgsl"));
        let uniforms = [Uniform::new(device, "silver lines"), Uniform::new(device, "silver wires")];
        Self { pipeline: pipeline(device, "silver", &module, format, LAYOUT, view, Some(&uniforms[0].layout)), instances: Instances::new(device, "silver", 512), uniforms }
    }

    /// The track's positions in order, the last where it was last seen.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, track: &[[f32; 3]], rise: f32, night: bool) {
        let n = track.len().saturating_sub(1).max(1) as f32;
        let segs: Vec<SegmentInstance> = track.windows(2).enumerate()
            .map(|(i, w)| SegmentInstance { a: w[0], b: w[1], t: i as f32 / n }).collect();
        self.instances.write(device, queue, "silver", &segs);
        for (i, u) in self.uniforms.iter().enumerate() {
            u.write(queue, &SilverUniform { rise, night: night as u8 as f32, layer: i as f32, _pad: 0.0 });
        }
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
