use super::ground::{sampler, tex2d as tex2d_entry, tex3d, tex_array as tex_array_entry};
use crate::gpu::{Gpu, Textures};
use crate::model::LonLat;

/// (R24) The half-resolution light's texels: a half float, filterable, ample for a light that is
/// scaled to at most 0.7 of a pixel's colour.
const GLOW_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

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
    /// (R24) the stream function's sheet with each texel's next month beside it (`pair_months`)
    _paired: wgpu::Texture,
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    /// (R24) the light along the streamlines at half the stage's resolution: its pipeline, the layout
    /// the sea reads it through, and the texture (made for the stage's size)
    glow_pipeline: wgpu::RenderPipeline,
    glow_layout: wgpu::BindGroupLayout,
    glow: Option<(u32, u32, wgpu::TextureView, wgpu::BindGroup)>,
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
        let paired = pair_months(gpu, &tx.stream);
        let shader = gpu.shader("sea", &super::wgsl(include_str!("sea.wgsl")));
        let glow_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("sea glow"), entries: &[tex2d_entry(0)] });
        let pipeline = gpu.fullscreen_pipeline("sea", &shader, &[view_layout, &layout, &ulayout, &glow_layout], Some(Gpu::over()));
        let glow_pipeline = {
            let pl = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("sea glow"), bind_group_layouts: &[view_layout, &layout], push_constant_ranges: &[] });
            gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("sea glow"),
                layout: Some(&pl),
                vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_glow"),
                    targets: &[Some(wgpu::ColorTargetState { format: GLOW_FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALL })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            })
        };
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sea textures"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&tx.currents.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&paired.create_view(&Default::default())) },
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
        SeaPass { _paired: paired, pipeline, bind, glow_pipeline, glow_layout, glow: None, uniforms, uniform_bind }
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

    /// (R24) The light along the streamlines for a stage `width` x `height`, at half its resolution,
    /// into the sea's own texture: encoded before the stage's pass (whose `draw` reads it).
    pub fn glow_pass(&mut self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, view_bind: &wgpu::BindGroup, width: u32, height: u32) {
        let (w, h) = (width.div_ceil(2).max(1), height.div_ceil(2).max(1));
        if self.glow.as_ref().map_or(true, |g| (g.0, g.1) != (w, h)) {
            let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("sea glow"),
                size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: GLOW_FORMAT,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("sea glow"),
                layout: &self.glow_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) }],
            });
            self.glow = Some((w, h, view, bind));
        }
        let glow = self.glow.as_ref().unwrap();
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("sea glow"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &glow.2,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rp.set_pipeline(&self.glow_pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.bind, &[]);
        rp.draw(0..3, 0..1);
    }

    /// Draws the sea; `glow_pass` for this stage first.
    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup) {
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(3, &self.glow.as_ref().expect("the sea's glow_pass before its draw").3, &[]);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.bind, &[]);
        rp.set_bind_group(2, &self.uniform_bind, &[]);
        rp.draw(0..3, 0..1);
    }
}

/// (R24) The stream function's sheet (twelve months' tiles, each texel's value in two bytes, r and g)
/// copied once with the next month's two bytes at the same place in its tile put in b and a, so the
/// sea's Catmull-Rom surface reads both months it mixes in sixteen loads, not thirty-two. The bytes
/// are copied exactly (a unorm byte read and written back is the same byte), so the sea is the same.
fn pair_months(gpu: &Gpu, stream: &crate::gpu::Texture) -> wgpu::Texture {
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stream, paired months"),
        size: wgpu::Extent3d { width: stream.width, height: stream.height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let shader = gpu.shader(
        "stream pairing",
        r#"
@group(0) @binding(0) var stream: texture_2d<f32>;
@vertex fn vs_main(@builtin(vertex_index) id: u32) -> @builtin(position) vec4f {
    return vec4f(select(-1.0, 3.0, id == 1u), select(-1.0, 3.0, id == 2u), 0.0, 1.0);
}
@fragment fn fs_main(@builtin(position) p: vec4f) -> @location(0) vec4f {
    let t = vec2i(p.xy);
    let cell = vec2i(t.x / 722, t.y / 362);
    let m = cell.y * 4 + cell.x;
    let n = (m + 1) % 12;
    let next = t - cell * vec2i(722, 362) + vec2i(n % 4, n / 4) * vec2i(722, 362);
    return vec4f(textureLoad(stream, t, 0).rg, textureLoad(stream, next, 0).rg);
}
"#,
    );
    let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor { label: Some("stream pairing"), entries: &[tex2d_entry(0)] });
    let pl = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("stream pairing"), bind_group_layouts: &[&layout], push_constant_ranges: &[] });
    let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("stream pairing"),
        layout: Some(&pl),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), buffers: &[], compilation_options: Default::default() },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState { format, blend: None, write_mask: wgpu::ColorWrites::ALL })],
            compilation_options: Default::default(),
        }),
        multiview: None,
        cache: None,
    });
    let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("stream pairing"),
        layout: &layout,
        entries: &[wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&stream.view) }],
    });
    let view = texture.create_view(&Default::default());
    let mut enc = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("stream pairing") });
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("stream pairing"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                depth_slice: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        rp.set_pipeline(&pipeline);
        rp.set_bind_group(0, &bind, &[]);
        rp.draw(0..3, 0..1);
    }
    gpu.queue.submit([enc.finish()]);
    texture
}
