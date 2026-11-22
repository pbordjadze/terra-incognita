//! Drawn the way Nelson Max drew *Carla's Island* (Lawrence Livermore, 1981; "Vectorized procedural models
//! for natural terrain: waves and islands in the sunset", SIGGRAPH '81): the sea an analytic surface, ray
//! traced as a height field; the direct ray reflected at the sea and its reflection tested, roughly, against
//! the land; and each pixel EIGHT BITS: for the sky and for what the sea reflects, the angle of elevation
//! above the horizon; for the land, the angle of its normal. A colour translation table turns the bytes into
//! colour, and changing the table as the waves go on is the sunset (Shoup's colour-table animation).
//!
//! Two passes: `max.wgsl` traces the index into an 8-bit texture; the table pass looks each pixel up in the
//! table (`palette`), rebuilt on the CPU each frame for the sun's height. The index layout is ours (the
//! paper's own allotment of the 256 is not quoted in its abstract):
//!   0..63    the sky, by elevation (0 at the horizon, 63 at 50° up)
//!   64..127  the sky in the sea, by the reflected ray's elevation
//!   128..191 the land: four grounds × 16 shades of its normal toward the sun
//!   192..207 the land in the sea
//!   208..223 white water, by its shade
//!   224..255 the sun and its glitter, by closeness

/// The floor's cell (m), the tallest train drawn (m, the texture's full scale) and the land's.
pub const H_FULL: f32 = 24.0;

/// Where the camera is and looks: its eye (m: x east, y south from the floor's corner; z up from the sea),
/// the bearing (degrees from north), how far up (degrees) and the vertical field (degrees).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Look {
    pub eye: [f32; 3],
    pub bearing: f32,
    pub pitch: f32,
    pub vfov: f32,
}

impl Look {
    /// Looking at a point (m) from an eye, it `below` of the field under the middle.
    pub fn at(eye: [f32; 3], target: [f32; 3], vfov: f32, below: f32) -> Look {
        let d = [target[0] - eye[0], target[1] - eye[1]];
        let dist = d[0].hypot(d[1]).max(1.0);
        let up = ((target[2] - eye[2]) / dist).atan().to_degrees();
        Look {
            eye,
            // east is +x, north is −y
            bearing: d[0].atan2(-d[1]).to_degrees().rem_euclid(360.0),
            pitch: up + below * vfov,
            vfov,
        }
    }

    /// A step of the way to another look, turning the short way round.
    pub fn toward(self, to: Look, k: f32) -> Look {
        let db = (to.bearing - self.bearing + 540.0).rem_euclid(360.0) - 180.0;
        Look {
            eye: std::array::from_fn(|i| self.eye[i] + (to.eye[i] - self.eye[i]) * k),
            bearing: (self.bearing + db * k).rem_euclid(360.0),
            pitch: self.pitch + (to.pitch - self.pitch) * k,
            vfov: self.vfov + (to.vfov - self.vfov) * k,
        }
    }
}

/// The camera: its place and axes (x east, y south, z up), and the screen it draws on (px).
#[derive(Clone, Copy)]
pub struct Cam {
    pub eye: [f32; 3],
    pub fwd: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    /// tan of half the fields, across and down
    pub tx: f32,
    pub ty: f32,
    pub size: [f32; 2],
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

impl Cam {
    pub fn new(size: [f32; 2], look: Look) -> Cam {
        let b = look.bearing.to_radians();
        let p = look.pitch.to_radians();
        // facing the bearing b: east sin b, south −cos b
        let (fx, fy) = (b.sin(), -b.cos());
        let fwd = [fx * p.cos(), fy * p.cos(), p.sin()];
        let right = [b.cos(), b.sin(), 0.0];
        let up = [-fx * p.sin(), -fy * p.sin(), p.cos()];
        let ty = (0.5 * look.vfov.to_radians()).tan();
        Cam {
            eye: look.eye,
            fwd,
            right,
            up,
            tx: ty * size[0] / size[1],
            ty,
            size,
        }
    }

    /// A point (m) on the screen (px), and how far ahead it is (m); None behind the camera.
    pub fn project(&self, c: [f32; 3]) -> Option<([f32; 2], f32)> {
        let d = [c[0] - self.eye[0], c[1] - self.eye[1], c[2] - self.eye[2]];
        let z = dot(d, self.fwd);
        if z < 0.5 {
            return None;
        }
        let x = dot(d, self.right) / z / self.tx;
        let y = dot(d, self.up) / z / self.ty;
        Some((
            [
                (0.5 + 0.5 * x) * self.size[0],
                (0.5 - 0.5 * y) * self.size[1],
            ],
            z,
        ))
    }

    /// Where a point on the screen (px) looks onto the sea's level (m); None at or above the horizon.
    pub fn onto_sea(&self, px: [f32; 2]) -> Option<[f32; 2]> {
        let x = (px[0] / self.size[0] * 2.0 - 1.0) * self.tx;
        let y = (1.0 - px[1] / self.size[1] * 2.0) * self.ty;
        let d: [f32; 3] = std::array::from_fn(|k| self.fwd[k] + x * self.right[k] + y * self.up[k]);
        if d[2] > -1e-4 {
            return None;
        }
        let t = -self.eye[2] / d[2];
        Some([self.eye[0] + d[0] * t, self.eye[1] + d[1] * t])
    }

    /// How many px a metre spans this far ahead (m).
    pub fn px_per_m(&self, dist: f32) -> f32 {
        0.5 * self.size[1] / (self.ty * dist.max(0.5))
    }
}

/// The colour translation table for the sun `sun_el` degrees up (negative: set): 256 colours, linear RGB.
/// Rebuilt as the sun goes down, so the same bytes darken into dusk, as Max's table did over his loop.
pub fn palette(sun_el: f32) -> [[f32; 4]; 256] {
    // 1 an hour before sunset, 0 at the end of civil dusk
    let s = ((sun_el + 6.0) / 16.0).clamp(0.0, 1.0);
    let mix3 = |a: [f32; 3], b: [f32; 3], t: f32| -> [f32; 3] {
        std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
    };
    let rgb = |c: [f32; 3]| -> [f32; 3] { c.map(|v| v / 255.0) };
    let horizon = rgb(mix3([110.0, 40.0, 60.0], [250.0, 150.0, 80.0], s));
    let mid = rgb(mix3([60.0, 30.0, 80.0], [190.0, 120.0, 120.0], s));
    let top = rgb(mix3([12.0, 14.0, 50.0], [60.0, 90.0, 160.0], s));
    let deep = rgb(mix3([10.0, 18.0, 90.0], [30.0, 80.0, 170.0], s));
    let ramp = |stops: &[(f32, [f32; 3])], x: f32| -> [f32; 3] {
        for w in stops.windows(2) {
            let ((x0, c0), (x1, c1)) = (w[0], w[1]);
            if x <= x1 {
                return mix3(c0, c1, ((x - x0) / (x1 - x0).max(1e-6)).clamp(0.0, 1.0));
            }
        }
        stops[stops.len() - 1].1
    };
    let mut p = [[0.0f32, 0.0, 0.0, 1.0]; 256];
    let mut put = |i: usize, c: [f32; 3]| p[i] = [c[0], c[1], c[2], 1.0].map(clamp01);
    for i in 0..64 {
        let x = i as f32 / 63.0;
        put(i, ramp(&[(0.0, horizon), (0.15, mid), (1.0, top)], x));
        // the sea: the sky it reflects, its own blue deepening as the reflection climbs (Fresnel folded in)
        let sea = ramp(
            &[
                (0.0, horizon.map(|v| v * 1.05)),
                (0.06, mid),
                (0.2, deep.map(|v| v * 1.3)),
                (1.0, deep.map(|v| v * 0.55)),
            ],
            x,
        );
        put(64 + i, sea);
    }
    let light = rgb(mix3([90.0, 60.0, 90.0], [255.0, 190.0, 140.0], s));
    let grounds = [
        [205.0, 180.0, 140.0], // sand
        [170.0, 120.0, 80.0],  // the cliffs' limestone
        [70.0, 100.0, 55.0],   // scrub and pine on top
        [225.0, 215.0, 200.0], // the town's white walls
    ];
    for (m, base) in grounds.iter().enumerate() {
        let base = rgb(*base);
        let dark: [f32; 3] =
            std::array::from_fn(|k| base[k] * (0.18 + (0.35 - 0.18) * s) * [1.0, 0.95, 1.3][k]);
        let bright: [f32; 3] = std::array::from_fn(|k| base[k] * light[k]);
        for j in 0..16 {
            put(128 + 16 * m + j, mix3(dark, bright, j as f32 / 15.0));
        }
    }
    let shadow = rgb(mix3([20.0, 16.0, 30.0], [60.0, 50.0, 50.0], s));
    for j in 0..16 {
        put(
            192 + j,
            mix3(rgb([6.0, 6.0, 14.0]), shadow, j as f32 / 15.0),
        );
    }
    let foam_lo = rgb(mix3([60.0, 60.0, 90.0], [150.0, 160.0, 180.0], s));
    let foam_hi = rgb(mix3([200.0, 170.0, 170.0], [255.0, 245.0, 235.0], s));
    for j in 0..16 {
        put(208 + j, mix3(foam_lo, foam_hi, j as f32 / 15.0));
    }
    let glint = rgb(mix3([255.0, 120.0, 60.0], [255.0, 210.0, 140.0], s));
    let water = ramp(&[(0.0, mid), (1.0, deep)], 0.5);
    for j in 0..32 {
        let x = j as f32 / 31.0;
        put(
            224 + j,
            ramp(
                &[
                    (0.0, water),
                    (0.5, glint),
                    (1.0, rgb([255.0, 250.0, 230.0])),
                ],
                x,
            ),
        );
    }
    put(
        255,
        rgb(mix3([255.0, 140.0, 80.0], [255.0, 245.0, 220.0], s)),
    );
    p
}

/// The table holds display (sRGB-encoded) colours; the table pass turns them linear for an sRGB target.
fn clamp01(v: f32) -> f32 {
    v.clamp(0.0, 1.0)
}

/// The trace pass's uniforms: 8 vec4s.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TraceU {
    /// eye (m), the wind chop's clock
    pub eye: [f32; 4],
    /// forward, tan of half the vertical field
    pub fwd: [f32; 4],
    /// right, tan of half the horizontal field
    pub right: [f32; 4],
    pub up: [f32; 4],
    /// toward the sun, and its glitter's width (cos)
    pub sun: [f32; 4],
    /// the swell's phase now (cycles), the sets' phase now (cycles), the open sea's wavenumber, the
    /// angle an index pixel spans (rad; set by `render`)
    pub phase: [f32; 4],
    /// the floor's cells across, down, a cell (m), H_FULL
    pub grid: [f32; 4],
    /// the index texture's size (px), the lighthouse (m)
    pub size: [f32; 4],
}

pub struct Max {
    trace: wgpu::RenderPipeline,
    table: wgpu::RenderPipeline,
    trace_bind: wgpu::BindGroup,
    table_bind: Option<wgpu::BindGroup>,
    table_bgl: wgpu::BindGroupLayout,
    trace_u: wgpu::Buffer,
    table_u: wgpu::Buffer,
    index: Option<(wgpu::Texture, wgpu::TextureView, [u32; 2])>,
    srgb: bool,
}

const TABLE_WGSL: &str = r#"
@group(0) @binding(0) var idx: texture_2d<f32>;
@group(0) @binding(1) var<uniform> table: array<vec4<f32>, 256>;
@group(0) @binding(2) var<uniform> how: vec4<f32>;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}
fn to_linear(c: vec3<f32>) -> vec3<f32> {
  return select(pow((c + 0.055) / 1.055, vec3(2.4)), c / 12.92, c <= vec3(0.04045));
}
@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let dims = vec2<f32>(textureDimensions(idx));
  let q = clamp(vec2<i32>(i.uv * dims), vec2(0), vec2<i32>(dims) - 1);
  let k = u32(round(textureLoad(idx, q, 0).r * 255.0));
  let c = table[k].rgb;
  // how.x: 1 when the target is sRGB (it encodes what it is given, so it is given linear colour)
  return vec4(select(c, to_linear(c), how.x > 0.5), 1.0);
}
"#;

impl Max {
    pub fn new(gpu: &toy_kit::Gpu, field: &wgpu::TextureView) -> Max {
        let dev = &gpu.device;
        let trace_u = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("nazare trace"),
            size: std::mem::size_of::<TraceU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let table_u = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some("nazare table"),
            size: 256 * 16 + 16,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let frag = wgpu::ShaderStages::FRAGMENT;
        let uniform = |binding: u32| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: frag,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let tex = |binding: u32, filterable: bool| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: frag,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let trace_bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nazare trace"),
            entries: &[
                uniform(0),
                tex(1, true),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: frag,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let samp = dev.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("nazare field"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let trace_bind = dev.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("nazare trace"),
            layout: &trace_bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: trace_u.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(field),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&samp),
                },
            ],
        });
        let table_bgl = dev.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("nazare table"),
            entries: &[tex(0, false), uniform(1), uniform(2)],
        });
        let shader = gpu.shader("nazare trace", include_str!("max.wgsl"));
        let trace = pipeline(
            gpu,
            "nazare trace",
            &shader,
            &trace_bgl,
            wgpu::TextureFormat::R8Unorm,
        );
        let tshader = gpu.shader("nazare table", TABLE_WGSL);
        let table = pipeline(gpu, "nazare table", &tshader, &table_bgl, gpu.format);
        Max {
            trace,
            table,
            trace_bind,
            table_bind: None,
            table_bgl,
            trace_u,
            table_u,
            index: None,
            srgb: gpu.format.is_srgb(),
        }
    }

    /// The index texture for a screen: half its pixels each way (Max's frames were his film's, ours a
    /// phone's: half keeps the trace within a frame's time and the bytes' banding as his).
    fn ensure(&mut self, gpu: &toy_kit::Gpu, size: [u32; 2]) -> [u32; 2] {
        let want = [size[0].div_ceil(2).max(1), size[1].div_ceil(2).max(1)];
        if self.index.as_ref().is_none_or(|i| i.2 != want) {
            let tex = gpu.texture(&wgpu::TextureDescriptor {
                label: Some("nazare index"),
                size: wgpu::Extent3d {
                    width: want[0],
                    height: want[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            self.table_bind = Some(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("nazare table"),
                layout: &self.table_bgl,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.table_u,
                            offset: 0,
                            size: std::num::NonZeroU64::new(256 * 16),
                        }),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                            buffer: &self.table_u,
                            offset: 256 * 16,
                            size: std::num::NonZeroU64::new(16),
                        }),
                    },
                ],
            }));
            self.index = Some((tex, view, want));
        }
        want
    }

    /// Traces the index and looks it up in the table onto `target`.
    pub fn render(
        &mut self,
        gpu: &toy_kit::Gpu,
        enc: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: [u32; 2],
        mut u: TraceU,
        table: &[[f32; 4]; 256],
    ) {
        let isize = self.ensure(gpu, size);
        u.size[0] = isize[0] as f32;
        u.size[1] = isize[1] as f32;
        // the angle one index pixel spans (rad), which the shader turns into its footprint on the sea
        u.phase[3] = 2.0 * u.fwd[3] / isize[1] as f32;
        gpu.queue
            .write_buffer(&self.trace_u, 0, bytemuck::bytes_of(&u));
        gpu.queue
            .write_buffer(&self.table_u, 0, bytemuck::cast_slice(table));
        let how = [if self.srgb { 1.0f32 } else { 0.0 }, 0.0, 0.0, 0.0];
        gpu.queue
            .write_buffer(&self.table_u, 256 * 16, bytemuck::cast_slice(&how));
        let Some((_, view, _)) = &self.index else {
            return;
        };
        pass(enc, view, &self.trace, &self.trace_bind);
        if let Some(b) = &self.table_bind {
            pass(enc, target, &self.table, b);
        }
    }
}

fn pass(
    enc: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    pipe: &wgpu::RenderPipeline,
    bind: &wgpu::BindGroup,
) {
    let mut p = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("nazare"),
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
    });
    p.set_pipeline(pipe);
    p.set_bind_group(0, bind, &[]);
    p.draw(0..3, 0..1);
}

fn pipeline(
    gpu: &toy_kit::Gpu,
    label: &str,
    shader: &wgpu::ShaderModule,
    bgl: &wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let layout = gpu
        .device
        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[Some(bgl)],
            immediate_size: 0,
        });
    gpu.device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(&layout),
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
                entry_point: Some("fs_main"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_point_on_the_sea_projects_back_where_it_was_seen() {
        let look = Look::at([1000.0, 1000.0, 40.0], [500.0, 900.0, 0.0], 30.0, -0.1);
        let cam = Cam::new([1179.0, 2556.0], look);
        let px = [400.0, 1800.0];
        let c = cam.onto_sea(px).unwrap();
        let (back, _) = cam.project([c[0], c[1], 0.0]).unwrap();
        assert!((back[0] - px[0]).abs() < 0.5 && (back[1] - px[1]).abs() < 0.5);
        assert!(cam.onto_sea([600.0, 10.0]).is_none());
        let a = Look {
            eye: [0.0; 3],
            bearing: 350.0,
            pitch: 0.0,
            vfov: 20.0,
        };
        let b = Look { bearing: 10.0, ..a };
        assert!((a.toward(b, 0.5).bearing - 0.0).abs() < 1e-3);
    }

    #[test]
    fn the_table_darkens_as_the_sun_goes_down() {
        let sum = |p: &[[f32; 4]; 256]| p.iter().map(|c| c[0] + c[1] + c[2]).sum::<f32>();
        assert!(sum(&palette(4.0)) > sum(&palette(-5.0)));
    }
}
