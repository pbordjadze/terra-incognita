//! A film of a toy's scene: the toy draws its scene into `Film::scene` only when a new drawing is due
//! (`due`), the toy's own `fs_surface` reads it into a grid of cells (`cells` across the screen's short
//! side, one R8 texel each), and its `fs_main` draws the cells once a drawing; every frame shows it. The way an
//! old process made pictures out of a coarse grid of values: BEFLIX's characters (The Wave), a
//! pinscreen's pins (Bats).
//!
//! The toy's WGSL gets this prelude: the uniform `p` (`size` px, `cell` px, `drawing`, `grid` cells,
//! `a`: the toy's own four values), `src` (the scene in `fs_surface`, the cells in `fs_main`), `lin` (a
//! linear sampler), `vs_main` (a full-screen triangle) and `hash`.

const PRELUDE: &str = r#"
struct P {
  size: vec2<f32>,
  cell: f32,
  drawing: f32,
  grid: vec2<f32>,
  _pad: vec2<f32>,
  a: vec4<f32>,
};
@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var src: texture_2d<f32>;
@group(0) @binding(2) var lin: sampler;

@vertex fn vs_main(@builtin(vertex_index) i: u32) -> @builtin(position) vec4<f32> {
  let x = f32((i << 1u) & 2u);
  let y = f32(i & 2u);
  return vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
}

fn hash(q: vec2<f32>) -> f32 { return fract(sin(dot(q, vec2<f32>(127.1, 311.7))) * 43758.5453); }

@fragment fn fs_blit(@builtin(position) fc: vec4<f32>) -> @location(0) vec4<f32> {
  return textureLoad(src, vec2<i32>(fc.xy), 0);
}
"#;

use crate::Gpu;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct P {
    size: [f32; 2],
    cell: f32,
    drawing: f32,
    grid: [f32; 2],
    _pad: [f32; 2],
    a: [f32; 4],
}

pub struct Film {
    /// the scene as drawn, which the cells are read from
    pub scene: wgpu::TextureView,
    surface: wgpu::TextureView,
    surface_pipe: wgpu::RenderPipeline,
    film_pipe: wgpu::RenderPipeline,
    blit_pipe: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    ubuf: wgpu::Buffer,
    to_surface: wgpu::BindGroup,
    to_film: wgpu::BindGroup,
    size: [u32; 2],
    grid: [u32; 2],
    cells: f32,
    hz: f64,
    /// the drawing last made (None: make one)
    pub drawn: Option<i64>,
}

fn target(
    gpu: &Gpu,
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

fn grid(size: [u32; 2], cells: f32) -> ([u32; 2], f32) {
    let cell = size[0].min(size[1]) as f32 / cells;
    let g = [
        (size[0] as f32 / cell).ceil() as u32,
        (size[1] as f32 / cell).ceil() as u32,
    ];
    (g, cell)
}

impl Film {
    /// `wgsl` holds the toy's `fs_surface` and `fs_main` (the prelude above is put before it);
    /// `cells` across the screen's short side; a new drawing `hz` times a second.
    pub fn new(gpu: &Gpu, label: &str, wgsl: &str, cells: f32, hz: f64, size: [u32; 2]) -> Film {
        let entry = |binding, ty| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty,
            count: None,
        };
        let layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
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
        let shader = gpu.shader(label, &format!("{PRELUDE}\n{wgsl}"));
        let surface_pipe = pipeline(
            gpu,
            &shader,
            &layout,
            "fs_surface",
            wgpu::TextureFormat::R8Unorm,
        );
        let blit_pipe = pipeline(gpu, &shader, &layout, "fs_blit", gpu.format);
        let film_pipe = gpu.fullscreen_pipeline(label, &shader, &[&layout], None);
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let ubuf = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: std::mem::size_of::<P>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let (scene, surface, to_surface, to_film, g) =
            Self::targets(gpu, &layout, &ubuf, &sampler, size, cells);
        Film {
            scene,
            surface,
            surface_pipe,
            film_pipe,
            blit_pipe,
            layout,
            sampler,
            ubuf,
            to_surface,
            to_film,
            size,
            grid: g,
            cells,
            hz,
            drawn: None,
        }
    }

    #[allow(clippy::type_complexity)] // the five things a size makes, made together
    fn targets(
        gpu: &Gpu,
        layout: &wgpu::BindGroupLayout,
        ubuf: &wgpu::Buffer,
        sampler: &wgpu::Sampler,
        size: [u32; 2],
        cells: f32,
    ) -> (
        wgpu::TextureView,
        wgpu::TextureView,
        wgpu::BindGroup,
        wgpu::BindGroup,
        [u32; 2],
    ) {
        let (g, _) = grid(size, cells);
        let scene = target(gpu, "film-scene", size, gpu.format);
        let surface = target(gpu, "film-cells", g, wgpu::TextureFormat::R8Unorm);
        let bind = |view: &wgpu::TextureView| {
            gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("film"),
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

    pub fn resize(&mut self, gpu: &Gpu, size: [u32; 2]) {
        if size == self.size {
            return;
        }
        let (scene, surface, a, b, g) = Self::targets(
            gpu,
            &self.layout,
            &self.ubuf,
            &self.sampler,
            size,
            self.cells,
        );
        (self.scene, self.surface, self.to_surface, self.to_film) = (scene, surface, a, b);
        self.size = size;
        self.grid = g;
        self.drawn = None;
    }

    /// The drawing due at `t`; if none is (the last one still shows), shows it on `out` now and says
    /// None, so a toy's draw is `let Some(d) = film.next(gpu, out, t) else { return };`.
    pub fn next(&self, gpu: &Gpu, out: &wgpu::TextureView, t: f64) -> Option<i64> {
        let d = self.due(t);
        if d.is_none() {
            let mut enc = gpu.device.create_command_encoder(&Default::default());
            self.film(&mut enc, out);
            gpu.queue.submit([enc.finish()]);
        }
        d
    }

    /// The drawing due at `t`, if it isn't the one already made.
    pub fn due(&self, t: f64) -> Option<i64> {
        let d = (t * self.hz).floor() as i64;
        (self.drawn != Some(d)).then_some(d)
    }

    fn uniform(&self, gpu: &Gpu, d: i64, a: [f32; 4]) {
        let (g, cell) = grid(self.size, self.cells);
        let u = P {
            size: [self.size[0] as f32, self.size[1] as f32],
            cell,
            drawing: (d % 10_000) as f32,
            grid: [g[0] as f32, g[1] as f32],
            _pad: [0.0; 2],
            a,
        };
        gpu.queue
            .write_buffer(&self.ubuf, 0, bytemuck::bytes_of(&u));
    }

    /// Reads the scene (drawn into `scene` this frame) into the cells, as drawing `d`; `a` is the
    /// toy's own four values, `p.a` in both passes.
    pub fn read(&mut self, gpu: &Gpu, enc: &mut wgpu::CommandEncoder, d: i64, a: [f32; 4]) {
        self.uniform(gpu, d, a);
        let mut rp = pass(enc, &self.surface, "film-cells");
        rp.set_pipeline(&self.surface_pipe);
        rp.set_bind_group(0, &self.to_surface, &[]);
        rp.draw(0..3, 0..1);
        drop(rp);
        // the cells drawn once a drawing, over the scene they were read from (it isn't needed again
        // until the next drawing), and shown from there every frame
        let mut rp = pass(enc, &self.scene, "film");
        rp.set_pipeline(&self.film_pipe);
        rp.set_bind_group(0, &self.to_film, &[]);
        rp.draw(0..3, 0..1);
        drop(rp);
        self.drawn = Some(d);
    }

    /// Shows the last drawing on `out`.
    pub fn film(&self, enc: &mut wgpu::CommandEncoder, out: &wgpu::TextureView) {
        let mut rp = pass(enc, out, "film-show");
        rp.set_pipeline(&self.blit_pipe);
        rp.set_bind_group(0, &self.to_surface, &[]);
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

fn pipeline(
    gpu: &Gpu,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::BindGroupLayout,
    entry: &str,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let pl = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(entry),
            bind_group_layouts: &[Some(layout)],
            immediate_size: 0,
        });
    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(entry),
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
                entry_point: Some(entry),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            multiview_mask: None,
            cache: None,
        })
}
