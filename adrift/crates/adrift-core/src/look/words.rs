//! The date and the credits as drawn things (`words.wgsl`): Libertinus Serif from the signed-distance
//! atlas in `type/` (built by `tools/adrift/glyphs.py`), each glyph on a halo, laid out here in points.
use super::{pipeline, shader, Instances};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct GlyphInstance {
    /// x, y, w, h on the screen in points (y down)
    pub rect: [f32; 4],
    /// the glyph's atlas rectangle, 0..1
    pub uv: [f32; 4],
    /// the ink and its alpha
    pub ink: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct WordsUniform {
    pub halo: [f32; 4],
    pub spread: f32,
    pub halo_pt: f32,
    /// 0 this draw lays the halos, 1 the inks
    pub layer: f32,
    pub _pad: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<GlyphInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4],
};

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Glyph {
    pub advance: f32,
    pub left: f32,
    pub top: f32,
    pub w: f32,
    pub h: f32,
    pub x: f32,
    pub y: f32,
}

/// The atlas's metrics (`type/libertinus.json`).
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Face {
    pub em: f32,
    pub spread: f32,
    pub ascent: f32,
    pub descent: f32,
    pub width: f32,
    pub height: f32,
    pub glyphs: HashMap<String, Glyph>,
}

pub const FACE_JSON: &str = include_str!("type/libertinus.json");
pub const ATLAS_PNG: &[u8] = include_bytes!("type/libertinus.png");

/// A line of words: its text, size in reading points, ink, and tracking as a fraction of the size.
pub struct Line<'a> {
    pub text: &'a str,
    pub size: f32,
    pub ink: [f32; 4],
    pub tracking: f32,
}

impl Face {
    pub fn load() -> Self { serde_json::from_str(FACE_JSON).expect("type/libertinus.json") }

    /// The width of a line in points at `size`.
    pub fn width(&self, line: &Line) -> f32 {
        let s = line.size / self.em;
        line.text.chars().map(|c| self.glyph(c).advance * s + line.tracking * line.size).sum::<f32>() - line.tracking * line.size
    }

    fn glyph(&self, c: char) -> &Glyph {
        self.glyphs.get(&c.to_string()).or_else(|| self.glyphs.get("?")).expect("a glyph")
    }

    /// Glyphs for a line centred at x on the baseline y (points, y down).
    pub fn layout(&self, line: &Line, x: f32, baseline: f32) -> Vec<GlyphInstance> {
        let s = line.size / self.em;
        let mut pen = x - self.width(line) / 2.0;
        let mut out = Vec::new();
        for c in line.text.chars() {
            let g = self.glyph(c);
            if g.w > 0.0 {
                out.push(GlyphInstance {
                    rect: [pen + g.left * s, baseline + g.top * s, g.w * s, g.h * s],
                    uv: [g.x / self.width, g.y / self.height, g.w / self.width, g.h / self.height],
                    ink: line.ink,
                });
            }
            pen += g.advance * s + line.tracking * line.size;
        }
        out
    }
}

pub struct Words {
    pipeline: wgpu::RenderPipeline,
    instances: Instances<GlyphInstance>,
    buffers: [wgpu::Buffer; 2],
    groups: [wgpu::BindGroup; 2],
    pub face: Face,
}

impl Words {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, view: &wgpu::BindGroupLayout) -> Self {
        let face = Face::load();
        let module = shader(device, "words", include_str!("words.wgsl"));
        let texture = atlas(device, queue);
        let tv = texture.create_view(&Default::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("words atlas"),
            mag_filter: wgpu::FilterMode::Linear, min_filter: wgpu::FilterMode::Linear, mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("words"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 1, visibility: wgpu::ShaderStages::VERTEX_FRAGMENT, ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false }, count: None },
                wgpu::BindGroupLayoutEntry { binding: 2, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
            ],
        });
        // two draws, the halos then the inks: a uniform and a group each
        let make = |name: &str| {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(name), size: std::mem::size_of::<WordsUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false,
            });
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(name), layout: &layout,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&tv) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::Sampler(&sampler) },
                ],
            });
            (buffer, group)
        };
        let (b0, g0) = make("words halos");
        let (b1, g1) = make("words inks");
        let pipeline = pipeline(device, "words", &module, format, LAYOUT, view, Some(&layout));
        Self { pipeline, instances: Instances::new(device, "words", 256), buffers: [b0, b1], groups: [g0, g1], face }
    }

    /// The glyphs of every line this frame (`Face::layout`), on a halo of the given tone.
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, glyphs: &[GlyphInstance], halo: [f32; 4]) {
        self.instances.write(device, queue, "words", glyphs);
        for (i, b) in self.buffers.iter().enumerate() {
            queue.write_buffer(b, 0, bytemuck::bytes_of(&WordsUniform { halo, spread: self.face.spread, halo_pt: 1.5, layer: i as f32, _pad: 0.0 }));
        }
    }

    pub fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, view: &'a wgpu::BindGroup) {
        if self.instances.count == 0 { return; }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, view, &[]);
        pass.set_vertex_buffer(0, self.instances.slice());
        for g in &self.groups {
            pass.set_bind_group(1, g, &[]);
            pass.draw(0..6, 0..self.instances.count);
        }
    }
}

/// The atlas as an R8 texture with its mipmaps (made here, box-filtered, so small words sample it
/// near their own scale).
fn atlas(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Texture {
    let decoder = png::Decoder::new(ATLAS_PNG);
    let mut reader = decoder.read_info().expect("type/libertinus.png");
    let mut buf = vec![0u8; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).expect("type/libertinus.png");
    let (w, h) = (info.width, info.height);
    let mut levels = vec![buf[..info.buffer_size()].to_vec()];
    let (mut lw, mut lh) = (w, h);
    while lw > 1 && lh > 1 {
        let (nw, nh) = (lw / 2, lh / 2);
        let prev = levels.last().unwrap();
        let mut next = vec![0u8; (nw * nh) as usize];
        for y in 0..nh { for x in 0..nw {
            let i = |xx: u32, yy: u32| prev[(yy * lw + xx) as usize] as u32;
            next[(y * nw + x) as usize] = ((i(2 * x, 2 * y) + i(2 * x + 1, 2 * y) + i(2 * x, 2 * y + 1) + i(2 * x + 1, 2 * y + 1) + 2) / 4) as u8;
        } }
        levels.push(next);
        lw = nw; lh = nh;
    }
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("words atlas"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: levels.len() as u32, sample_count: 1, dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm, usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST, view_formats: &[],
    });
    let (mut lw, mut lh) = (w, h);
    for (level, data) in levels.iter().enumerate() {
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: level as u32, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(lw), rows_per_image: Some(lh) },
            wgpu::Extent3d { width: lw, height: lh, depth_or_array_layers: 1 },
        );
        lw = (lw / 2).max(1); lh = (lh / 2).max(1);
    }
    texture
}
