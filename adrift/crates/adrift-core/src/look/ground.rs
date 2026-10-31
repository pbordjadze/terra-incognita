use crate::gpu::{Gpu, Textures};

pub struct GroundPass {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    bind: wgpu::BindGroup,
}

pub(crate) fn tex2d(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
        count: None,
    }
}
pub(crate) fn tex_array(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2Array, multisampled: false },
        count: None,
    }
}
pub(crate) fn tex3d(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D3, multisampled: false },
        count: None,
    }
}
pub(crate) fn sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry { binding, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None }
}

impl GroundPass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout, tx: &Textures) -> GroundPass {
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ground textures"),
            entries: &[tex2d(0), tex2d(1), tex2d(2), tex_array(3), tex2d(4), tex2d(5), sampler(6)],
        });
        let shader = gpu.shader("ground", &super::wgsl(include_str!("ground.wgsl")));
        let pipeline = gpu.fullscreen_pipeline("ground", &shader, &[view_layout, &layout], None);
        let bind = Self::bind(gpu, &layout, tx);
        GroundPass { pipeline, layout, bind }
    }

    fn bind(gpu: &Gpu, layout: &wgpu::BindGroupLayout, tx: &Textures) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ground textures"),
            layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&tx.geo.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tx.terrain.view) },
                wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&tx.floor.view) },
                wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::TextureView(&tx.ice.view) },
                wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&tx.land_a.view) },
                wgpu::BindGroupEntry { binding: 5, resource: wgpu::BindingResource::TextureView(&tx.land_b.view) },
                wgpu::BindGroupEntry { binding: 6, resource: wgpu::BindingResource::Sampler(&tx.linear_wrap_x) },
            ],
        })
    }

    /// After the land textures changed months.
    pub fn rebind(&mut self, gpu: &Gpu, tx: &Textures) {
        self.bind = Self::bind(gpu, &self.layout, tx);
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.bind, &[]);
        rp.draw(0..3, 0..1);
    }
}
