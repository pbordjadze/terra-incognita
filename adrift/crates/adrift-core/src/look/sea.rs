use super::ground::{sampler, tex2d as tex2d_entry, tex3d, tex_array as tex_array_entry};
use crate::gpu::{Gpu, Textures};
use crate::model::LonLat;

/// A touch on the water: where, when (the sea's seconds), how hard.
#[derive(Clone, Copy, Debug)]
pub struct Ripple {
    pub at: LonLat,
    pub when: f64,
    pub strength: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct SeaUniforms {
    ripple: [[f32; 4]; 8],
}

pub struct SeaPass {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    uniforms: wgpu::Buffer,
    uniform_bind: wgpu::BindGroup,
}

impl SeaPass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout, tx: &Textures) -> SeaPass {
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sea textures"),
            entries: &[tex2d_entry(0), tex2d_entry(1), tex2d_entry(2), tex_array_entry(3), tex3d(4), sampler(5), sampler(6), sampler(7)],
        });
        let ulayout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sea uniforms"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                count: None,
            }],
        });
        let shader = gpu.shader("sea", &super::wgsl(include_str!("sea.wgsl")));
        let pipeline = gpu.fullscreen_pipeline("sea", &shader, &[view_layout, &layout, &ulayout], Some(Gpu::over()));
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sea textures"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&tx.currents.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tx.stream.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&tx.geo.view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&tx.ice.view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&tx.noise.view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::Sampler(&tx.linear_clamp) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(&tx.linear_wrap_x) },
                wgpu::BindGroupEntry { binding: 7, resource: wgpu::BindingResource::Sampler(&tx.linear_repeat) },
            ],
        });
        let uniforms = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sea uniforms"),
            size: std::mem::size_of::<SeaUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform_bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sea uniforms"),
            layout: &ulayout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: uniforms.as_entire_binding() }],
        });
        SeaPass { pipeline, bind, uniforms, uniform_bind }
    }

    /// The newest four touches.
    pub fn prepare(&self, gpu: &Gpu, ripples: &[Ripple]) {
        let mut u = SeaUniforms { ripple: [[0.0; 4]; 8] };
        for (i, r) in ripples.iter().rev().take(4).enumerate() {
            let w = r.at.to_xyz();
            u.ripple[2 * i] = [w[0] as f32, w[1] as f32, w[2] as f32, r.when as f32];
            u.ripple[2 * i + 1] = [r.strength, 0.0, 0.0, 0.0];
        }
        gpu.queue.write_buffer(&self.uniforms, 0, bytemuck::bytes_of(&u));
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.bind, &[]);
        rp.set_bind_group(2, &self.uniform_bind, &[]);
        rp.draw(0..3, 0..1);
    }
}
