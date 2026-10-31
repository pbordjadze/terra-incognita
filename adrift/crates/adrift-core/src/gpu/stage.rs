//! The stage: the ground and the sea drawn to a texture of their own, at a scale the frame's budget
//! sets, and only when something they show has changed; each frame blits it under the sprites.
//! (Terra draws its sea and clouds the same way: at a fraction of the screen, at rest ten to thirty
//! times a second.)

use super::Gpu;

pub struct Stage {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    bind: wgpu::BindGroup,
}

pub struct Blit {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
}

const BLIT_WGSL: &str = r#"
@group(0) @binding(0) var stage: texture_2d<f32>;
@group(0) @binding(1) var stageS: sampler;
struct Out { @builtin(position) position: vec4f, @location(0) uv: vec2f };
@vertex fn vs_main(@builtin(vertex_index) id: u32) -> Out {
    let p = vec2f(select(-1.0, 3.0, id == 1u), select(-1.0, 3.0, id == 2u));
    var o: Out;
    o.position = vec4f(p, 0.0, 1.0);
    o.uv = vec2f(p.x * 0.5 + 0.5, 0.5 - p.y * 0.5);
    return o;
}
@fragment fn fs_main(in: Out) -> @location(0) vec4f { return textureSample(stage, stageS, in.uv); }
"#;

impl Blit {
    pub fn new(gpu: &Gpu) -> Blit {
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("stage"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, address_mode_u: wgpu::AddressMode::ClampToEdge, address_mode_v: wgpu::AddressMode::ClampToEdge, ..Default::default() });
        let shader = gpu.shader("blit", BLIT_WGSL);
        let pipeline = gpu.fullscreen_pipeline("blit", &shader, &[&layout], None);
        Blit { pipeline, layout, sampler }
    }

    /// A stage of the target's size times `scale`.
    pub fn stage(&self, gpu: &Gpu, width: u32, height: u32, scale: f64) -> Stage {
        let w = ((width as f64 * scale).round() as u32).max(1);
        let h = ((height as f64 * scale).round() as u32).max(1);
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("stage"),
            size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: gpu.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("stage"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.sampler) },
            ],
        });
        Stage { texture, view, width: w, height: h, scale, bind }
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, stage: &'a Stage) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, &stage.bind, &[]);
        rp.draw(0..3, 0..1);
    }
}

/// The frame's budget: from the time between frames while the camera moves, the scale the stage is
/// drawn at (1 down to 1/2), stepped slowly so it never flickers.
pub struct Budget {
    pub scale: f64,
    pub locked: bool,
    recent: std::collections::VecDeque<f64>,
    since_change: u32,
}

impl Budget {
    pub fn new(ppp: f64) -> Budget {
        // a 3x phone starts a notch down and earns its way up
        Budget { scale: if ppp >= 2.5 { 0.75 } else { 1.0 }, locked: false, recent: Default::default(), since_change: 0 }
    }

    /// A frame of `dt` seconds while the stage was redrawn; returns true when the scale changed.
    pub fn frame(&mut self, dt: f64) -> bool {
        if self.locked || dt <= 0.0 {
            return false;
        }
        self.recent.push_back(dt);
        if self.recent.len() > 12 {
            self.recent.pop_front();
        }
        self.since_change += 1;
        if self.recent.len() < 12 || self.since_change < 30 {
            return false;
        }
        let mean = self.recent.iter().sum::<f64>() / self.recent.len() as f64;
        let before = self.scale;
        if mean > 1.0 / 50.0 {
            self.scale = (self.scale - 0.125).max(0.5);
        } else if mean < 1.0 / 100.0 {
            self.scale = (self.scale + 0.125).min(1.0);
        }
        if self.scale != before {
            self.since_change = 0;
            self.recent.clear();
            true
        } else {
            false
        }
    }
}
