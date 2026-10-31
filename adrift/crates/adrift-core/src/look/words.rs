//! The date and the credits as drawn things (`words.wgsl`): Libertinus Serif from the signed-distance
//! atlas in `type/` (built by `tools/adrift/glyphs.py`), each glyph on a halo, laid out here in points.
use super::{mark_pipeline, marks_wgsl, Instances};
use crate::gpu::Gpu;
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
    /// the halo's colour and alpha: what is under the glyph (the sea's tone over the globe, the paper's below)
    pub halo: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable)]
pub struct WordsUniform {
    pub spread: f32,
    pub halo_pt: f32,
    /// 0 this draw lays the halos, 1 the inks
    pub layer: f32,
    pub _pad: f32,
}

const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
    array_stride: std::mem::size_of::<GlyphInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4],
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

/// One face's metrics in the atlas.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct FaceMetrics {
    pub font: String,
    pub ascent: f32,
    pub descent: f32,
    pub glyphs: HashMap<String, Glyph>,
}

/// The atlas's metrics (`type/libertinus.json`): Regular and Italic in one sheet.
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Face {
    pub em: f32,
    pub spread: f32,
    pub width: f32,
    pub height: f32,
    pub faces: HashMap<String, FaceMetrics>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Regular,
    Italic,
}

pub const FACE_JSON: &str = include_str!("type/libertinus.json");
pub const ATLAS_PNG: &[u8] = include_bytes!("type/libertinus.png");

/// A line of words: its text, face, size in reading points, ink, and tracking as a fraction of the size.
pub struct Line<'a> {
    pub text: &'a str,
    pub style: Style,
    pub size: f32,
    pub ink: [f32; 4],
    pub tracking: f32,
}

impl Face {
    pub fn load() -> Self { serde_json::from_str(FACE_JSON).expect("type/libertinus.json") }

    /// The width of a line in points at `size`.
    pub fn width(&self, line: &Line) -> f32 {
        let s = line.size / self.em;
        line.text.chars().map(|c| self.glyph(line.style, c).advance * s + line.tracking * line.size).sum::<f32>() - line.tracking * line.size
    }

    fn glyph(&self, style: Style, c: char) -> &Glyph {
        let f = &self.faces[if style == Style::Italic { "italic" } else { "regular" }];
        f.glyphs.get(&c.to_string()).or_else(|| f.glyphs.get("?")).expect("a glyph")
    }

    /// Glyphs for a line centred at x on the baseline y (points, y down), each on a halo of `halo`.
    pub fn layout(&self, line: &Line, x: f32, baseline: f32, halo: [f32; 4]) -> Vec<GlyphInstance> {
        let s = line.size / self.em;
        let mut pen = x - self.width(line) / 2.0;
        let mut out = Vec::new();
        for c in line.text.chars() {
            let g = self.glyph(line.style, c);
            if g.w > 0.0 {
                out.push(GlyphInstance {
                    rect: [pen + g.left * s, baseline + g.top * s, g.w * s, g.h * s],
                    uv: [g.x / self.width, g.y / self.height, g.w / self.width, g.h / self.height],
                    ink: line.ink,
                    halo,
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
    /// Where the date was last laid: x, y, w, h in points (y down), with the halo's room; the one
    /// drawn thing that is also a control (a drag on it scrubs the date, a tap pauses it: Motion's).
    pub date_bounds: Option<[f32; 4]>,
}

impl Words {
    pub fn new(gpu: &Gpu, view: &wgpu::BindGroupLayout) -> Self {
        let device = &gpu.device;
        let face = Face::load();
        let module = gpu.shader("words", &marks_wgsl(include_str!("words.wgsl")));
        let texture = atlas(gpu);
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
        let pipeline = mark_pipeline(gpu, "words", &module, LAYOUT, view, Some(&layout));
        Self { pipeline, instances: Instances::new(gpu, "words", 256), buffers: [b0, b1], groups: [g0, g1], face, date_bounds: None }
    }

    /// The glyphs of every line this frame (`Face::layout`).
    pub fn prepare(&mut self, gpu: &Gpu, glyphs: &[GlyphInstance]) {
        self.instances.write(gpu, glyphs);
        for (i, b) in self.buffers.iter().enumerate() {
            gpu.queue.write_buffer(b, 0, bytemuck::bytes_of(&WordsUniform { spread: self.face.spread, halo_pt: 1.5, layer: i as f32, _pad: 0.0 }));
        }
    }

    /// The words at the foot: the date (17 pt) centred under the globe's disc (`cx, cy, r` in points)
    /// on a `width` x `height` point screen, or over the globe when the disc fills it; the note (13 pt
    /// italic) under the date; the credits (10 pt) along the screen's foot. Each in the palette's
    /// inks on a halo of what is under it: the paper below the disc, the sea over it (palette.json).
    pub fn prepare_foot(&mut self, gpu: &Gpu, date: &str, note: &str, credits: &str, width: f32, height: f32, reading: f32, disc: [f32; 3], night: bool) {
        let under = disc[1] + disc[2] + 40.0 * reading;
        let foot = under.min(height - 72.0 * reading);
        let bottom = height - 12.0 * reading;
        let (date_ink, note_ink, credit_ink, sea_halo, paper_halo) = if night {
            ([0.894, 0.925, 0.957, 1.0], [0.722, 0.776, 0.831, 1.0], [0.722, 0.776, 0.831, 1.0], [0.063, 0.094, 0.157, 0.92], [0.08, 0.10, 0.14, 0.92])
        } else {
            ([0.149, 0.188, 0.227, 1.0], [0.243, 0.298, 0.353, 1.0], [0.243, 0.298, 0.353, 1.0], [0.761, 0.894, 0.957, 0.92], [0.96, 0.95, 0.92, 0.92])
        };
        let halo_for = |baseline: f32| if baseline - 14.0 * reading < disc[1] + disc[2] { sea_halo } else { paper_halo };
        let cx = width / 2.0;
        let date_line = Line { text: date, style: Style::Regular, size: 17.0 * reading, ink: date_ink, tracking: 0.0 };
        let dw = self.face.width(&date_line) + 24.0 * reading;
        let dh = 17.0 * reading * 1.3 + 12.0 * reading;
        self.date_bounds = Some([cx - dw / 2.0, foot - 17.0 * reading * 0.95 - 6.0 * reading, dw, dh]);
        let mut glyphs = self.face.layout(&date_line, cx, foot, halo_for(foot));
        if !note.is_empty() {
            let y = foot + 19.0 * reading;
            glyphs.extend(self.face.layout(&Line { text: note, style: Style::Italic, size: 13.0 * reading, ink: note_ink, tracking: 0.0 }, cx, y, halo_for(y)));
        }
        glyphs.extend(self.face.layout(&Line { text: credits, style: Style::Regular, size: 10.0 * reading, ink: credit_ink, tracking: 0.0 }, cx, bottom, halo_for(bottom)));
        self.prepare(gpu, &glyphs);
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
fn atlas(gpu: &Gpu) -> wgpu::Texture {
    let img = image::load_from_memory(ATLAS_PNG).expect("type/libertinus.png").to_luma8();
    let (w, h) = img.dimensions();
    let mut levels = vec![img.into_raw()];
    let (mut lw, mut lh) = (w, h);
    while lw > 1 && lh > 1 {
        let (nw, nh) = (lw / 2, lh / 2);
        let prev = levels.last().unwrap();
        let mut next = vec![0u8; (nw * nh) as usize];
        for y in 0..nh {
            for x in 0..nw {
                let i = |xx: u32, yy: u32| prev[(yy * lw + xx) as usize] as u32;
                next[(y * nw + x) as usize] = ((i(2 * x, 2 * y) + i(2 * x + 1, 2 * y) + i(2 * x, 2 * y + 1) + i(2 * x + 1, 2 * y + 1) + 2) / 4) as u8;
            }
        }
        levels.push(next);
        lw = nw;
        lh = nh;
    }
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("words atlas"),
        size: wgpu::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        mip_level_count: levels.len() as u32,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R8Unorm,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let (mut lw, mut lh) = (w, h);
    for (level, data) in levels.iter().enumerate() {
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture, mip_level: level as u32, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            data,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(lw), rows_per_image: Some(lh) },
            wgpu::Extent3d { width: lw, height: lh, depth_or_array_layers: 1 },
        );
        lw = (lw / 2).max(1);
        lh = (lh / 2).max(1);
    }
    texture
}
