//! The postcard drawn over the Earth (painting-company row 351): a card of paper with the toy itself in it,
//! live, its whole frame scaled down to cover the card's picture; and the same card growing until its picture
//! is the glass and the frame is the toy's own, unscaled. One full-screen pass over a texture the toy draws
//! into (made at the screen's size; dropped when unused).

use toy_kit::Ctx;

/// Where the card and its picture are, in pixels (left, top, right, bottom), their corners' radii, and which
/// point of the toy's frame lies at the picture's middle at what scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lay {
    pub card: [f32; 4],
    pub pic: [f32; 4],
    pub radius: f32,
    pub pic_radius: f32,
    /// The frame's point (pixels) at the picture's middle, and the scale it is shown at.
    pub focus: [f32; 2],
    pub scale: f32,
    /// How much of the card's paper shows (0 as it becomes the glass) and its shadow's strength.
    pub paper: f32,
    pub alpha: f32,
}

pub struct Veil {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    buf: wgpu::Buffer,
    pub target: Option<(wgpu::Texture, wgpu::TextureView, wgpu::BindGroup, [u32; 2])>,
}

const VEIL_WGSL: &str = r#"
struct U { card: vec4<f32>, pic: vec4<f32>, map: vec4<f32>, misc: vec4<f32> };
@group(0) @binding(0) var tex: texture_2d<f32>;
@group(0) @binding(1) var samp: sampler;
@group(0) @binding(2) var<uniform> u: U;
struct O { @builtin(position) pos: vec4<f32> };
@vertex fn vs_main(@builtin(vertex_index) i: u32) -> O {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[i];
  var o: O;
  o.pos = vec4<f32>(p, 0.0, 1.0);
  return o;
}
// the signed distance to a rounded box (its left, top, right, bottom and its corners' radius)
fn rbox(p: vec2<f32>, b: vec4<f32>, r: f32) -> f32 {
  let c = (b.xy + b.zw) * 0.5;
  let h = (b.zw - b.xy) * 0.5 - vec2<f32>(r, r);
  let q = abs(p - c) - h;
  return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}
@fragment fn fs_main(i: O) -> @location(0) vec4<f32> {
  let p = i.pos.xy;
  let radius = u.misc.x; let pic_radius = u.misc.y; let paper = u.misc.z; let alpha = u.misc.w;
  let dc = rbox(p, u.card, radius);
  let in_card = 1.0 - smoothstep(-0.75, 0.75, dc);
  let dp = rbox(p, u.pic, pic_radius);
  let in_pic = 1.0 - smoothstep(-0.75, 0.75, dp);
  // the toy's frame: the point map.xy at the picture's middle, shown at scale map.z
  let size = vec2<f32>(textureDimensions(tex));
  let mid = (u.pic.xy + u.pic.zw) * 0.5;
  let q = u.map.xy + (p - mid) / u.map.z;
  let toy = textureSampleLevel(tex, samp, q / size, 0.0).rgb;
  // the card in the Growth look: black lacquer, its rim catching the striped ball round the scene
  // in bands (gold above, violet below), the words in light ink (lib.rs: dark: false)
  let h = clamp((p.y - u.card.y) / max(u.card.w - u.card.y, 1.0), 0.0, 1.0);
  let ball = mix(vec3<f32>(1.0, 0.8, 0.36), vec3<f32>(0.55, 0.12, 0.62), h);
  // a glossy tube round the card: lit from above its middle, a hard white line along its crest
  let x = clamp(-dc / (4.0 * u.map.w), 0.0, 1.0);
  let tube = ball * (0.35 + 0.65 * sin(x * 3.14159)) + vec3<f32>(smoothstep(0.82, 0.95, sin(x * 3.14159)) * 0.6 * (1.0 - h));
  let rim = 1.0 - smoothstep(3.5 * u.map.w, 4.5 * u.map.w, -dc);
  let ink = mix(vec3<f32>(0.035, 0.035, 0.05), tube, rim);
  let col = mix(ink, toy, in_pic);
  let a = in_card * mix(paper, 1.0, in_pic);
  // a soft shadow under the card, gone as it becomes the glass
  let blur = 26.0 * u.map.w;
  let shadow = paper * 0.30 * (1.0 - smoothstep(-blur * 0.2, blur, dc - blur * 0.25)) * (1.0 - in_card);
  return vec4<f32>(col * a * alpha, (a + shadow) * alpha);
}
"#;

impl Veil {
    pub fn new(ctx: &Ctx) -> Veil {
        let dev = &ctx.gpu.device;
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("terra veil"),
            entries: &[
                entry(
                    0,
                    wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                ),
                entry(
                    1,
                    wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                ),
                entry(
                    2,
                    wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                ),
            ],
        });
        let shader = dev.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("terra veil"),
            source: wgpu::ShaderSource::Wgsl(VEIL_WGSL.into()),
        });
        let pl = dev.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("terra veil"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = dev.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("terra veil"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: ctx.gpu.format,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = dev.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let buf = dev.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terra veil"),
            size: 64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Veil {
            pipeline,
            layout,
            sampler,
            buf,
            target: None,
        }
    }

    /// The texture the toy draws into while it is a card (made at the screen's size).
    pub fn view(&mut self, ctx: &Ctx) -> &wgpu::TextureView {
        let size = [ctx.width.max(1), ctx.height.max(1)];
        if self.target.as_ref().map(|t| t.3) != Some(size) {
            let tex = ctx.gpu.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("terra veil"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: ctx.gpu.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            let bind = ctx
                .gpu
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("terra veil"),
                    layout: &self.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.buf.as_entire_binding(),
                        },
                    ],
                });
            self.target = Some((tex, view, bind, size));
        }
        &self.target.as_ref().unwrap().1
    }

    /// Lays the card (`Lay`) over the target, the toy's frame in its picture; `ppp` pixels a point.
    pub fn lay(&self, ctx: &Ctx, target: &wgpu::TextureView, l: &Lay) {
        let Some((_, _, bind, _)) = &self.target else {
            return;
        };
        let u: [f32; 16] = [
            l.card[0],
            l.card[1],
            l.card[2],
            l.card[3],
            l.pic[0],
            l.pic[1],
            l.pic[2],
            l.pic[3],
            l.focus[0],
            l.focus[1],
            l.scale,
            ctx.ppp as f32,
            l.radius,
            l.pic_radius,
            l.paper,
            l.alpha,
        ];
        ctx.gpu
            .queue
            .write_buffer(&self.buf, 0, bytemuck::cast_slice(&u));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("terra veil"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, bind, &[]);
            rp.draw(0..3, 0..1);
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }
}
