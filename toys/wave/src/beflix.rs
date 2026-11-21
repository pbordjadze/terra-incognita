//! The Wave drawn as a BEFLIX film: Ken Knowlton's language for computer-animated movies (Bell
//! Labs, 1963; "Bell flicks"). Its pictures were a surface of cells, 252 by 184 at the fine
//! setting, each holding a value 0 to 7; the IBM 7090 wrote the surfaces to tape, and a
//! Stromberg-Carlson SC-4020 drew each cell on its charactron tube as a character chosen for how
//! much light it held, the beam defocused, while a camera shot the tube frame by frame. Drawings
//! were held by shooting them more than once (computer time was dear), and the colour of the
//! VanDerBeek-Knowlton Poem Fields was dyed in afterwards by an optical printer. SOURCES.md has the
//! sources.
//!
//! Here the stadium is drawn as before into a picture of its own; `fs_surface` reads it into the
//! surface (184 cells across a phone's short side, as across the film's), and `fs_main` draws the
//! surface as the plotter did. A new drawing twelve times a second, each shown twice.

/// Cells across the screen's short side: the fine surface's short side (252 by 184).
pub const CELLS: f32 = 184.0;
/// Drawings a second (each drawn twice, at 24 frames a second).
pub const DRAWINGS_HZ: f64 = 12.0;
/// Seconds a shot lasts before the next is dyed another way.
const SHOT_S: f64 = 9.0;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct P {
    size: [f32; 2],
    cell: f32,
    drawing: f32,
    grid: [f32; 2],
    shot: f32,
    weave: f32,
}

pub struct Beflix {
    /// the stadium as drawn, which the surface is read from
    pub scene: wgpu::TextureView,
    surface: wgpu::TextureView,
    surface_pipe: wgpu::RenderPipeline,
    film_pipe: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    ubuf: wgpu::Buffer,
    to_surface: wgpu::BindGroup,
    to_film: wgpu::BindGroup,
    size: [u32; 2],
    grid: [u32; 2],
    /// the drawing last made (None: make one)
    pub drawn: Option<i64>,
}

fn target(
    gpu: &toy_kit::Gpu,
    label: &str,
    size: [u32; 2],
    format: wgpu::TextureFormat,
) -> wgpu::TextureView {
    gpu.texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
    .create_view(&Default::default())
}

fn grid(size: [u32; 2]) -> ([u32; 2], f32) {
    let cell = size[0].min(size[1]) as f32 / CELLS;
    let g = [
        (size[0] as f32 / cell).ceil() as u32,
        (size[1] as f32 / cell).ceil() as u32,
    ];
    (g, cell)
}

impl Beflix {
    pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Beflix {
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("beflix"),
                entries: &[
                    entry(
                        0,
                        wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                    ),
                    entry(
                        1,
                        wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                    ),
                    entry(
                        2,
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    ),
                ],
            });
        let shader = gpu.shader("beflix", include_str!("beflix.wgsl"));
        let surface_pipe = surface_pipeline(gpu, &shader, &layout);
        let film_pipe = gpu.fullscreen_pipeline("beflix-film", &shader, &[&layout], None);
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let ubuf = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("beflix-u"),
            size: std::mem::size_of::<P>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (scene, surface, to_surface, to_film, g) =
            Self::targets(gpu, &layout, &ubuf, &sampler, size);
        Beflix {
            scene,
            surface,
            surface_pipe,
            film_pipe,
            layout,
            sampler,
            ubuf,
            to_surface,
            to_film,
            size,
            grid: g,
            drawn: None,
        }
    }

    #[allow(clippy::type_complexity)] // the five things a size makes, made together
    fn targets(
        gpu: &toy_kit::Gpu,
        layout: &wgpu::BindGroupLayout,
        ubuf: &wgpu::Buffer,
        sampler: &wgpu::Sampler,
        size: [u32; 2],
    ) -> (
        wgpu::TextureView,
        wgpu::TextureView,
        wgpu::BindGroup,
        wgpu::BindGroup,
        [u32; 2],
    ) {
        let (g, _) = grid(size);
        let scene = target(gpu, "beflix-scene", size, gpu.format);
        let surface = target(gpu, "beflix-surface", g, wgpu::TextureFormat::R8Unorm);
        let bind = |view: &wgpu::TextureView| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("beflix"),
                layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: ubuf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(sampler),
                    },
                ],
            })
        };
        let (a, b) = (bind(&scene), bind(&surface));
        (scene, surface, a, b, g)
    }

    pub fn resize(&mut self, gpu: &toy_kit::Gpu, size: [u32; 2]) {
        if size == self.size {
            return;
        }
        let (scene, surface, a, b, g) =
            Self::targets(gpu, &self.layout, &self.ubuf, &self.sampler, size);
        (self.scene, self.surface, self.to_surface, self.to_film) = (scene, surface, a, b);
        self.size = size;
        self.grid = g;
        self.drawn = None;
    }

    /// The drawing due at `t`, if it isn't the one already made.
    pub fn due(&self, t: f64) -> Option<i64> {
        let d = (t * DRAWINGS_HZ).floor() as i64;
        (self.drawn != Some(d)).then_some(d)
    }

    fn uniform(&self, gpu: &toy_kit::Gpu, d: i64, reduce_motion: bool) {
        let (g, cell) = grid(self.size);
        let shot = ((d as f64 / DRAWINGS_HZ / SHOT_S).floor() as i64).rem_euclid(3);
        let u = P {
            size: [self.size[0] as f32, self.size[1] as f32],
            cell,
            drawing: (d % 10_000) as f32,
            grid: [g[0] as f32, g[1] as f32],
            shot: shot as f32,
            weave: if reduce_motion { 0.0 } else { 1.0 },
        };
        gpu.queue
            .write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
    }

    /// Reads the scene (drawn into `scene` this frame) into the surface, as drawing `d`.
    pub fn read(
        &mut self,
        gpu: &toy_kit::Gpu,
        enc: &mut wgpu::CommandEncoder,
        d: i64,
        reduce_motion: bool,
    ) {
        self.uniform(gpu, d, reduce_motion);
        let mut rp = pass(enc, &self.surface, "beflix-surface");
        rp.set_pipeline(&self.surface_pipe);
        rp.set_bind_group(0, &self.to_surface, &[]);
        rp.draw(0..3, 0..1);
        drop(rp);
        self.drawn = Some(d);
    }

    /// Shoots the surface onto `out`.
    pub fn film(&self, enc: &mut wgpu::CommandEncoder, out: &wgpu::TextureView) {
        let mut rp = pass(enc, out, "beflix-film");
        rp.set_pipeline(&self.film_pipe);
        rp.set_bind_group(0, &self.to_film, &[]);
        rp.draw(0..3, 0..1);
    }
}

fn pass<'a>(
    enc: &'a mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    label: &str,
) -> wgpu::RenderPass<'a> {
    enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some(label),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
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
        multiview_mask: None,
    })
}

fn surface_pipeline(
    gpu: &toy_kit::Gpu,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let pl = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("beflix-surface"),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("beflix-surface"),
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: shader,
                entry_point: Some("fs_surface"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        })
}
