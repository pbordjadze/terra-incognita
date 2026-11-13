//! A field: a grid of four numbers a cell that a toy writes on the CPU (a height, a charge, a sand
//! state), uploaded when it changes and drawn by a full-screen fragment of the toy's own. The fragment
//! is the backdrop's (`fn backdrop(px, world) -> vec4<f32>`, premultiplied, `u` in scope; `world` is
//! `u.center + (px - u.size / 2) / u.scale`), with the grid beside it: `field` (Rgba16Float, filterable)
//! and its sampler `field_s` (repeating, linear), and `fld(cell)` sampling it at a cell position (cell
//! centres at `i + 0.5`). Written for Catatumbo, Bubblers and Barchans (painting-company row 327).

use crate::backdrop::BackdropParams;
use crate::Gpu;

const PRELUDE: &str = r#"
struct U { size: vec2<f32>, time: f32, scale: f32, center: vec2<f32>, grid: vec2<f32>, p0: vec4<f32>, p1: vec4<f32> };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var field: texture_2d<f32>;
@group(0) @binding(2) var field_s: sampler;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}
fn hash2(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn noise2(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = fract(p); let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash2(i), hash2(i + vec2(1.0, 0.0)), w.x), mix(hash2(i + vec2(0.0, 1.0)), hash2(i + vec2(1.0, 1.0)), w.x), w.y);
}
fn fld(cell: vec2<f32>) -> vec4<f32> { return textureSampleLevel(field, field_s, cell / u.grid, 0.0); }
@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let px = i.uv * u.size;
  let world = u.center + (px - u.size * 0.5) / u.scale;
  return backdrop(px, world);
}
"#;

/// An f32 as IEEE half bits (round to nearest; overflow to infinity, tiny to zero).
pub fn half(x: f32) -> u16 {
    let b = x.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let man = b & 0x7f_ffff;
    if exp >= 31 {
        return sign | 0x7c00;
    }
    if exp <= 0 {
        if exp < -10 {
            return sign;
        }
        let m = (man | 0x80_0000) >> (1 - exp);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    let h = ((exp as u32) << 10) | (man >> 13);
    // round half up on the dropped bits (a carry rolls into the exponent, as it should)
    sign | (h + ((man >> 12) & 1)) as u16
}

pub struct Field {
    pub grid: [u32; 2],
    texture: wgpu::Texture,
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    uniform: wgpu::Buffer,
    halves: Vec<u16>,
}

impl Field {
    /// A field of `grid` cells, drawn by `body` (which defines `backdrop`); `blend` None replaces the
    /// target, `Some(Gpu::over())` lays the field over it.
    pub fn new(
        gpu: &Gpu,
        label: &str,
        grid: [u32; 2],
        body: &str,
        blend: Option<wgpu::BlendState>,
    ) -> Field {
        let src = format!("{body}\n{PRELUDE}");
        let shader = gpu.shader(label, &src);
        let texture = gpu.texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: grid[0],
                height: grid[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let uniform = gpu.buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: 80,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some(label),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let fragment = wgpu::ShaderStages::FRAGMENT;
        let bgl = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some(label),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: fragment,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: fragment,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: fragment,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });
        let view = texture.create_view(&Default::default());
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let pipeline = gpu.fullscreen_pipeline(label, &shader, &[&bgl], blend);
        Field {
            grid,
            texture,
            pipeline,
            bind,
            uniform,
            halves: Vec::new(),
        }
    }

    /// The whole grid, four numbers a cell, row by row.
    pub fn upload(&mut self, gpu: &Gpu, cells: &[[f32; 4]]) {
        let n = (self.grid[0] * self.grid[1]) as usize;
        assert_eq!(cells.len(), n, "a field takes exactly its grid's cells");
        self.halves.clear();
        self.halves
            .extend(cells.iter().flat_map(|c| c.iter().map(|&x| half(x))));
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&self.halves),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(self.grid[0] * 8),
                rows_per_image: Some(self.grid[1]),
            },
            wgpu::Extent3d {
                width: self.grid[0],
                height: self.grid[1],
                depth_or_array_layers: 1,
            },
        );
    }

    /// Draws the field into `view` of `size` pixels (`clear` clears it first).
    pub fn render(
        &self,
        gpu: &Gpu,
        enc: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        size: [u32; 2],
        p: &BackdropParams,
        clear: bool,
    ) {
        let data: [f32; 20] = [
            size[0] as f32,
            size[1] as f32,
            p.time,
            p.scale,
            p.center[0],
            p.center[1],
            self.grid[0] as f32,
            self.grid[1] as f32,
            p.p[0],
            p.p[1],
            p.p[2],
            p.p[3],
            p.p[4],
            p.p[5],
            p.p[6],
            p.p[7],
            0.0,
            0.0,
            0.0,
            0.0,
        ];
        gpu.queue
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&data));
        let load = if clear {
            wgpu::LoadOp::Clear(wgpu::Color::BLACK)
        } else {
            wgpu::LoadOp::Load
        };
        let mut pass = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("field"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind, &[]);
        pass.draw(0..3, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::half;

    #[test]
    fn halves_are_ieee() {
        assert_eq!(half(0.0), 0);
        assert_eq!(half(1.0), 0x3c00);
        assert_eq!(half(-2.0), 0xc000);
        assert_eq!(half(0.5), 0x3800);
        assert_eq!(half(65504.0), 0x7bff);
        assert_eq!(half(1e6), 0x7c00);
        assert_eq!(half(6.0e-5), 0x03ef);
    }
}
