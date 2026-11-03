//! The look: the flash field (every firefly a soft disc on an equirectangular sheet, field.wgsl) and
//! the night drawn with it (night.wgsl), so each city's own measured light is what flashes.

use crate::sim::Swarm;
use toy_kit::globe::ground::{sampler, tex2d, tex_array};
use toy_kit::{Globe, Gpu};

pub const FIELD_W: u32 = 2048;
pub const FIELD_H: u32 = 1024;
const FIELD_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

pub struct Look {
    field_view: wgpu::TextureView,
    field_pipe: wgpu::RenderPipeline,
    inst: wgpu::Buffer,
    inst_cap: usize,
    inst_n: u32,
    night_pipe: wgpu::RenderPipeline,
    night_layout: wgpu::BindGroupLayout,
    night_bind: wgpu::BindGroup,
    look_buf: wgpu::Buffer,
    staging: Vec<f32>,
}

impl Look {
    pub fn new(gpu: &Gpu, globe: &Globe, n: usize) -> Look {
        let field = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("flash field"),
            size: wgpu::Extent3d {
                width: FIELD_W,
                height: FIELD_H,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FIELD_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let field_view = field.create_view(&Default::default());
        let fshader = gpu.shader("field", include_str!("field.wgsl"));
        let flayout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("field"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });
        let add = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::One,
            operation: wgpu::BlendOperation::Add,
        };
        let field_pipe = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("field"),
                layout: Some(&flayout),
                vertex: wgpu::VertexState {
                    module: &fshader,
                    entry_point: Some("vs_field"),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: 12,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32],
                    }],
                    compilation_options: Default::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &fshader,
                    entry_point: Some("fs_field"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: FIELD_FORMAT,
                        blend: Some(wgpu::BlendState {
                            color: add,
                            alpha: add,
                        }),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: Default::default(),
                }),
                multiview: None,
                cache: None,
            });
        let inst_cap = n * 2 + 16;
        let inst = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("fireflies"),
            size: (inst_cap * 12) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let uniform = wgpu::BindGroupLayoutEntry {
            binding: 9,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let night_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("night"),
                entries: &[
                    tex2d(0),
                    tex2d(1),
                    tex2d(2),
                    tex_array(3),
                    tex2d(4),
                    tex2d(5),
                    sampler(6),
                    tex2d(7),
                    tex2d(8),
                    uniform,
                ],
            });
        let look_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("look"),
            size: 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let nshader = gpu.shader("night", &toy_kit::globe::wgsl(include_str!("night.wgsl")));
        let night_pipe = gpu.fullscreen_pipeline(
            "night",
            &nshader,
            &[&globe.view_layout, &night_layout],
            None,
        );
        let night_bind = Self::bind(gpu, &night_layout, globe, &field_view, &look_buf);
        Look {
            field_view,
            field_pipe,
            inst,
            inst_cap,
            inst_n: 0,
            night_pipe,
            night_layout,
            night_bind,
            look_buf,
            staging: Vec::new(),
        }
    }

    fn bind(
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        globe: &Globe,
        field: &wgpu::TextureView,
        look: &wgpu::Buffer,
    ) -> wgpu::BindGroup {
        let tx = &globe.textures;
        let tv = wgpu::BindingResource::TextureView;
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("night"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: tv(&tx.geo.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: tv(&tx.terrain.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: tv(&tx.floor.view),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: tv(&tx.ice.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: tv(&tx.land_a.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: tv(&tx.land_b.view),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::Sampler(&tx.linear_wrap_x),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: tv(&tx.lights.view),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: tv(field),
                },
                wgpu::BindGroupEntry {
                    binding: 9,
                    resource: look.as_entire_binding(),
                },
            ],
        })
    }

    /// After the globe's land textures changed months.
    pub fn rebind(&mut self, gpu: &Gpu, globe: &Globe) {
        self.night_bind = Self::bind(
            gpu,
            &self.night_layout,
            globe,
            &self.field_view,
            &self.look_buf,
        );
    }

    /// The fireflies' glows into the instance buffer, and the look's levels.
    pub fn upload(&mut self, gpu: &Gpu, swarm: &Swarm, soft: bool) {
        self.staging.clear();
        let mut n = 0;
        for (i, f) in swarm.flies.iter().enumerate() {
            let g = swarm.glow(i, soft);
            let mut push = |lon: f64| {
                self.staging
                    .extend_from_slice(&[lon as f32, f.lat as f32, g]);
                n += 1;
            };
            push(f.lon);
            // its copy across the antimeridian, where its disc would be cut there
            let reach = 1.2 / f.lat.to_radians().cos().max(0.08);
            if f.lon + reach > 180.0 {
                push(f.lon - 360.0);
            } else if f.lon - reach < -180.0 {
                push(f.lon + 360.0);
            }
            if n + 2 > self.inst_cap {
                break;
            }
        }
        self.inst_n = n as u32;
        gpu.queue
            .write_buffer(&self.inst, 0, bytemuck::cast_slice(&self.staging));
        // the rest level, the flash's gain, the tint, the bloom (Reduce Motion: a gentler swell)
        let a: [f32; 4] = if soft {
            [0.45, 0.8, 0.5, 0.4]
        } else {
            [0.24, 1.6, 0.8, 1.0]
        };
        gpu.queue
            .write_buffer(&self.look_buf, 0, bytemuck::cast_slice(&a));
    }

    /// The flash field, then the night with it, into `target`.
    pub fn draw(&self, gpu: &Gpu, globe: &Globe, target: &wgpu::TextureView) {
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("field"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.field_view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.field_pipe);
            rp.set_vertex_buffer(0, self.inst.slice(..));
            rp.draw(0..4, 0..self.inst_n);
        }
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("night"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.night_pipe);
            rp.set_bind_group(0, globe.view_bind(), &[]);
            rp.set_bind_group(1, &self.night_bind, &[]);
            rp.draw(0..3, 0..1);
        }
        gpu.queue.submit([enc.finish()]);
    }
}
