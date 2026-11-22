//! The water mask's texture and its bind group, made again when the screen's size changes.

impl super::Meander {
    /// Makes the mask (half the screen's pixels) if there is none of `msize` yet.
    pub(super) fn make_mask(&mut self, gpu: &toy_kit::Gpu, msize: [u32; 2]) {
        if self.mask.as_ref().is_none_or(|m| m.1 != msize) {
            let tex = gpu.texture(&wgpu::TextureDescriptor {
                label: Some("water"),
                size: wgpu::Extent3d {
                    width: msize[0],
                    height: msize[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("meander"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uni.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&self._plain.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&self.dep_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&self.smp),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                ],
            });
            self.mask = Some((tex, msize, bind));
        }
    }
}
