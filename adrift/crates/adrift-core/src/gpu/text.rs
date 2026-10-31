//! Words drawn on the screen: a glyph atlas rasterised once from a bundled font (Libertinus Serif,
//! Terra's type), and quads that read it. The runtime's one text tool: the date, the credits, and
//! whatever a look wants said, laid out by `Text::line` and drawn by `TextPass`.

use super::Gpu;
use ab_glyph::{Font, FontRef, Glyph, PxScale, ScaleFont};
use std::collections::HashMap;

const ATLAS: u32 = 1024;
const CHARS: &str = " !\"'(),-.0123456789:;?ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz–—’·°↗";

#[derive(Clone, Copy, Debug)]
struct Slot {
    /// In the atlas, texels.
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    /// The glyph's bearing from the pen, pixels.
    left: f32,
    top: f32,
    advance: f32,
}

/// One face at one pixel size.
pub struct Face {
    slots: HashMap<char, Slot>,
    /// Pixels (not points): the size the atlas was drawn at.
    pub px: f32,
    pub ascent: f32,
    pub descent: f32,
    kern: HashMap<(char, char), f32>,
}

/// The glyphs of every face in one R8 atlas.
pub struct Text {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub faces: Vec<Face>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct GlyphInstance {
    /// The quad's top left on the screen (points) and its size (points).
    pub at: [f32; 2],
    pub size: [f32; 2],
    /// The atlas rectangle, 0..1.
    pub uv: [f32; 4],
    /// Premultiplied colour with alpha.
    pub color: [f32; 4],
}

impl Text {
    /// Rasterises each (font, pixel size) into the atlas.
    pub fn new(gpu: &Gpu, fonts: &[(&[u8], f32)]) -> anyhow::Result<Text> {
        let mut atlas = vec![0u8; (ATLAS * ATLAS) as usize];
        let (mut cx, mut cy, mut row_h) = (1u32, 1u32, 0u32);
        let mut faces = Vec::new();
        for (bytes, px) in fonts {
            let font = FontRef::try_from_slice(bytes)?;
            let scaled = font.as_scaled(PxScale::from(*px));
            let mut slots = HashMap::new();
            for ch in CHARS.chars() {
                let id = font.glyph_id(ch);
                let glyph: Glyph = id.with_scale_and_position(*px, ab_glyph::point(0.0, 0.0));
                let advance = scaled.h_advance(id);
                let Some(outlined) = font.outline_glyph(glyph) else {
                    slots.insert(ch, Slot { x: 0, y: 0, w: 0, h: 0, left: 0.0, top: 0.0, advance });
                    continue;
                };
                let b = outlined.px_bounds();
                let (w, h) = (b.width().ceil() as u32 + 1, b.height().ceil() as u32 + 1);
                if cx + w + 1 >= ATLAS {
                    cx = 1;
                    cy += row_h + 1;
                    row_h = 0;
                }
                anyhow::ensure!(cy + h + 1 < ATLAS, "the glyph atlas is full");
                outlined.draw(|x, y, c| {
                    let (ax, ay) = (cx + x, cy + y);
                    if ax < ATLAS && ay < ATLAS {
                        let i = (ay * ATLAS + ax) as usize;
                        atlas[i] = atlas[i].max((c * 255.0) as u8);
                    }
                });
                slots.insert(ch, Slot { x: cx, y: cy, w, h, left: b.min.x, top: b.min.y, advance });
                cx += w + 1;
                row_h = row_h.max(h);
            }
            let mut kern = HashMap::new();
            for a in CHARS.chars() {
                for b in CHARS.chars() {
                    let k = scaled.kern(font.glyph_id(a), font.glyph_id(b));
                    if k != 0.0 {
                        kern.insert((a, b), k);
                    }
                }
            }
            faces.push(Face { slots, px: *px, ascent: scaled.ascent(), descent: scaled.descent(), kern });
        }
        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("glyphs"),
            size: wgpu::Extent3d { width: ATLAS, height: ATLAS, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &atlas,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(ATLAS), rows_per_image: None },
            wgpu::Extent3d { width: ATLAS, height: ATLAS, depth_or_array_layers: 1 },
        );
        let view = texture.create_view(&Default::default());
        Ok(Text { texture, view, faces })
    }

    /// The width of a line in points, at `ppp` pixels a point.
    pub fn width(&self, face: usize, s: &str, ppp: f32) -> f32 {
        let f = &self.faces[face];
        let mut x = 0.0;
        let mut prev = None;
        for ch in s.chars() {
            if let Some(p) = prev {
                x += f.kern.get(&(p, ch)).copied().unwrap_or(0.0);
            }
            x += f.slots.get(&ch).map_or(f.px * 0.5, |s| s.advance);
            prev = Some(ch);
        }
        x / ppp
    }

    /// A line of text with its baseline's left end at (x, y) in points; glyphs in points, at `ppp`.
    pub fn line(&self, face: usize, s: &str, x: f32, y: f32, ppp: f32, color: [f32; 4], out: &mut Vec<GlyphInstance>) -> f32 {
        let f = &self.faces[face];
        let mut pen = x * ppp;
        let base = y * ppp;
        let mut prev = None;
        let pm = [color[0] * color[3], color[1] * color[3], color[2] * color[3], color[3]];
        for ch in s.chars() {
            if let Some(p) = prev {
                pen += f.kern.get(&(p, ch)).copied().unwrap_or(0.0);
            }
            let Some(slot) = f.slots.get(&ch) else {
                pen += f.px * 0.5;
                prev = Some(ch);
                continue;
            };
            if slot.w > 0 {
                let gx = (pen + slot.left).round();
                let gy = (base + slot.top).round();
                out.push(GlyphInstance {
                    at: [gx / ppp, gy / ppp],
                    size: [slot.w as f32 / ppp, slot.h as f32 / ppp],
                    uv: [slot.x as f32 / ATLAS as f32, slot.y as f32 / ATLAS as f32, (slot.x + slot.w) as f32 / ATLAS as f32, (slot.y + slot.h) as f32 / ATLAS as f32],
                    color: pm,
                });
            }
            pen += slot.advance;
            prev = Some(ch);
        }
        (pen - x * ppp) / ppp
    }
}

pub const TEXT_WGSL: &str = r#"
struct Glyph {
    @location(0) at: vec2f,
    @location(1) size: vec2f,
    @location(2) uv: vec4f,
    @location(3) color: vec4f,
};
struct TextOut { @builtin(position) position: vec4f, @location(0) uv: vec2f, @location(1) color: vec4f };
@group(1) @binding(0) var glyphs: texture_2d<f32>;
@group(1) @binding(1) var glyphS: sampler;
@vertex fn vs_text(@builtin(vertex_index) vid: u32, g: Glyph) -> TextOut {
    let c = vec2f(f32(vid & 1u), f32((vid >> 1u) & 1u));
    let pt = g.at + c * g.size;
    var out: TextOut;
    out.position = vec4f(pt.x / (U.frame.x / U.frame.z) * 2.0 - 1.0, 1.0 - pt.y / (U.frame.y / U.frame.z) * 2.0, 0.0, 1.0);
    out.uv = mix(g.uv.xy, g.uv.zw, c);
    out.color = g.color;
    return out;
}
@fragment fn fs_text(in: TextOut) -> @location(0) vec4f {
    let a = textureSample(glyphs, glyphS, in.uv).r;
    return in.color * a;
}
"#;

/// Draws glyph instances with the atlas.
pub struct TextPass {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    buffer: wgpu::Buffer,
    capacity: usize,
    count: u32,
}

impl TextPass {
    pub fn new(gpu: &Gpu, view_layout: &wgpu::BindGroupLayout, text: &Text) -> TextPass {
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("glyphs"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor { mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, ..Default::default() });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("glyphs"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&text.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&sampler) },
            ],
        });
        let shader = gpu.shader("text", &format!("{}\n{}", super::view::VIEW_WGSL, TEXT_WGSL).replace("struct View {", "@group(0) @binding(0) var<uniform> U: View;\nstruct View {"));
        let player = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { label: Some("text"), bind_group_layouts: &[view_layout, &layout], push_constant_ranges: &[] });
        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("text"),
            layout: Some(&player),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_text"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphInstance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4, 3 => Float32x4],
                }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState { topology: wgpu::PrimitiveTopology::TriangleStrip, ..Default::default() },
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_text"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        let capacity = 1024;
        let buffer = Self::buffer(gpu, capacity);
        TextPass { pipeline, bind, buffer, capacity, count: 0 }
    }

    fn buffer(gpu: &Gpu, capacity: usize) -> wgpu::Buffer {
        gpu.device.create_buffer(&wgpu::BufferDescriptor { label: Some("glyph quads"), size: (capacity * std::mem::size_of::<GlyphInstance>()) as u64, usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false })
    }

    pub fn prepare(&mut self, gpu: &Gpu, glyphs: &[GlyphInstance]) {
        if glyphs.len() > self.capacity {
            self.capacity = glyphs.len().next_power_of_two();
            self.buffer = Self::buffer(gpu, self.capacity);
        }
        if !glyphs.is_empty() {
            gpu.queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(glyphs));
        }
        self.count = glyphs.len() as u32;
    }

    pub fn draw<'a>(&'a self, rp: &mut wgpu::RenderPass<'a>, view_bind: &'a wgpu::BindGroup) {
        if self.count == 0 {
            return;
        }
        rp.set_pipeline(&self.pipeline);
        rp.set_bind_group(0, view_bind, &[]);
        rp.set_bind_group(1, &self.bind, &[]);
        rp.set_vertex_buffer(0, self.buffer.slice(..));
        rp.draw(0..4, 0..self.count);
    }
}
