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
    /// a place on the globe (unit vector, w 1) the rect is offset from: a label; w 0 for words on the screen
    pub anchor: [f32; 4],
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
    attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4, 2 => Float32x4, 3 => Float32x4, 4 => Float32x4],
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

    /// A label: a line anchored to a place on the globe, its left edge `dx` points right of the place
    /// and its baseline `dy` points below it (negative above), hidden past the limb.
    pub fn label(&self, line: &Line, place: [f32; 3], dx: f32, dy: f32, halo: [f32; 4]) -> Vec<GlyphInstance> {
        let w = self.width(line);
        let mut out = self.layout(line, dx + w / 2.0, dy, halo);
        for g in &mut out {
            g.anchor = [place[0], place[1], place[2], 1.0];
        }
        out
    }

    /// A paragraph's lines, wrapped by words to `max_width` points at `line.size`.
    pub fn wrap<'a>(&self, line: &Line<'a>, max_width: f32) -> Vec<String> {
        let mut lines = Vec::new();
        let mut current = String::new();
        for word in line.text.split(' ') {
            let trial = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
            let probe = Line { text: &trial, style: line.style, size: line.size, ink: line.ink, tracking: line.tracking };
            if self.width(&probe) > max_width && !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current = word.to_string();
            } else {
                current = trial;
            }
        }
        if !current.is_empty() {
            lines.push(current);
        }
        lines
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
                    anchor: [0.0, 0.0, 0.0, 0.0],
                });
            }
            pen += g.advance * s + line.tracking * line.size;
        }
        out
    }
}

/// One lamp's row in the panel (R6).
#[derive(Clone, Copy, Debug)]
pub struct PanelRow {
    pub id: u32,
    pub origin: crate::model::LonLat,
    pub put_day: f64,
    /// its cloud is out
    pub shown: bool,
    /// chosen as itself, its journey followed (R8)
    pub followed: bool,
    /// the one in front
    pub front: bool,
}

/// The most rows the panel shows (the latest lamps), and where its first row's top lies in reading
/// points (below a phone's status bar and island).
pub const PANEL_ROWS: usize = 10;
pub const PANEL_TOP: f32 = 60.0;
/// The stories' row, reading points up from the screen's foot (painting-company row 199).
pub const STORY_ROW: f32 = 64.0;

/// "8 Jun": a day as `Clock.day` counts it, on the typical year's calendar (as the date is).
pub fn day_words(day: f64) -> String {
    const DAYS: [u32; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let mut d = (day.rem_euclid(365.25) * 365.0 / 365.25).floor() as u32;
    for (i, n) in DAYS.iter().enumerate() {
        if d < *n {
            return format!("{} {}", d + 1, MONTHS[i]);
        }
        d -= n;
    }
    "31 Dec".into()
}

/// "35°N 74°W": a place to the whole degree.
pub fn place_words(at: crate::model::LonLat) -> String {
    let (la, lo) = (at.lat.round() as i32, at.lon.round() as i32);
    let ns = if la < 0 { 'S' } else { 'N' };
    let ew = if lo < 0 { 'W' } else { 'E' };
    format!("{}\u{b0}{} {}\u{b0}{}", la.abs(), ns, lo.abs(), ew)
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
    /// Where a credit line was last laid, if one is drawn (none since the ruling: the credits page is a rest on the date away).
    pub credit_bounds: Option<[f32; 4]>,
    /// The panel's rows (R6), each lamp's id and rect in points; empty while no panel is drawn.
    pub panel_bounds: Vec<(u32, [f32; 4])>,
    /// The stories' marks (painting-company row 199), each story and its touch rect in points.
    pub story_bounds: Vec<(crate::story::Story, [f32; 4])>,
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
        Self { pipeline, instances: Instances::new(gpu, "words", 256), buffers: [b0, b1], groups: [g0, g1], face, date_bounds: None, credit_bounds: None, panel_bounds: Vec::new(), story_bounds: Vec::new() }
    }

    /// The glyphs of every line this frame (`Face::layout`).
    pub fn prepare(&mut self, gpu: &Gpu, glyphs: &[GlyphInstance]) {
        self.instances.write(gpu, glyphs);
        for (i, b) in self.buffers.iter().enumerate() {
            gpu.queue.write_buffer(b, 0, bytemuck::bytes_of(&WordsUniform { spread: self.face.spread, halo_pt: 1.5, layer: i as f32, _pad: 0.0 }));
        }
    }

    /// The page of credits (Truth's D2): the sentences in full, 11 reading points, wrapped to a readable
    /// line, on a page of the paper's tone (the night's page by night) centred on the screen, with a
    /// small italic head; words only, nothing to press but anywhere. The page itself is one glyph of
    /// the atlas's solid block.
    pub fn credits_page(&self, sentences: &[String], width: f32, height: f32, reading: f32, night: bool) -> Vec<GlyphInstance> {
        let (ink, head_ink, page) = if night {
            ([0.894, 0.925, 0.957, 1.0], [0.722, 0.776, 0.831, 1.0], [0.055, 0.063, 0.086, 0.96])
        } else {
            ([0.149, 0.188, 0.227, 1.0], [0.243, 0.298, 0.353, 1.0], [0.957, 0.945, 0.918, 0.96])
        };
        let size = 11.0 * reading;
        let lead = 15.0 * reading;
        let inner = (560.0 * reading).min(width - 32.0 * reading) - 32.0 * reading;
        let mut lines: Vec<(String, Style, f32)> = vec![("Credits".into(), Style::Italic, 13.0 * reading)];
        for s in sentences {
            let probe = Line { text: s, style: Style::Regular, size, ink, tracking: 0.0 };
            for l in self.face.wrap(&probe, inner) {
                lines.push((l, Style::Regular, size));
            }
            lines.push((String::new(), Style::Regular, size));      // a paragraph's gap
        }
        lines.pop();
        let total = lines.len() as f32 * lead + 8.0 * reading;
        let page_w = inner + 32.0 * reading;
        let page_h = total + 32.0 * reading;
        let x0 = width / 2.0 - page_w / 2.0;
        let y0 = (height / 2.0 - page_h / 2.0).max(16.0 * reading);
        let mut out = Vec::new();
        if let Some(b) = self.face.faces.get("regular").and_then(|f| f.glyphs.get("\u{2588}")) {
            out.push(GlyphInstance { rect: [x0, y0, page_w, page_h], uv: [(b.x as f32 + 2.0) / self.face.width, (b.y as f32 + 2.0) / self.face.height, (b.w as f32 - 4.0) / self.face.width, (b.h as f32 - 4.0) / self.face.height], ink: page, halo: [0.0, 0.0, 0.0, 0.0], anchor: [0.0; 4] });
        }
        let mut y = y0 + 16.0 * reading + 13.0 * reading;
        for (text, style, sz) in &lines {
            if !text.is_empty() {
                let line = Line { text, style: *style, size: *sz, ink: if *style == Style::Italic { head_ink } else { ink }, tracking: 0.0 };
                let w = self.face.width(&line);
                out.extend(self.face.layout(&line, x0 + 16.0 * reading + w / 2.0, y, [page[0], page[1], page[2], 0.0]));
            }
            y += lead;
        }
        out
    }

    /// The panel of lamps (R6): a very small column at the screen's top left, off the globe's middle,
    /// one row a lamp out there, oldest at the top, at most `PANEL_ROWS` (the latest). A row is the
    /// lamp's mark in its hue (large and glowing while its cloud is out, small while not) and where
    /// and when it was put, in small italic ("8 Jun · 35°N 74°W"): the one place that says which
    /// lamp is which, since on the globe a lamp whose cloud is out is one maybe among them. The front
    /// one's words in full ink. Each row's rect (the whole row, a fingertip tall) goes to
    /// `panel_bounds`, where Motion's probe answers a touch. Words and marks only, each on a halo of
    /// what is under it; nothing is a control that points or waits.
    pub fn panel(&mut self, rows: &[PanelRow], reading: f32, disc: [f32; 3], night: bool) -> Vec<GlyphInstance> {
        self.panel_bounds.clear();
        let mut out = Vec::new();
        if rows.is_empty() {
            return out;
        }
        let (ink, soft_ink, sea_halo, paper_halo) = if night {
            ([0.894, 0.925, 0.957, 1.0], [0.722, 0.776, 0.831, 1.0], [0.063, 0.094, 0.157, 0.92], [0.055, 0.063, 0.086, 0.92])
        } else {
            ([0.149, 0.188, 0.227, 1.0], [0.243, 0.298, 0.353, 0.85], [0.761, 0.894, 0.957, 0.92], [0.96, 0.95, 0.92, 0.92])
        };
        // (36 points a row: a thumb picks the right lamp; Motion takes each row's rect as its target)
        let row_h = 36.0 * reading;
        let x0 = 12.0 * reading;
        let top = PANEL_TOP * reading;
        let size = 10.0 * reading;
        let start = rows.len().saturating_sub(PANEL_ROWS);
        let mut widest: f32 = 0.0;
        let mut lines = Vec::new();
        for (i, r) in rows[start..].iter().enumerate() {
            let y = top + i as f32 * row_h;
            let mid = y + row_h / 2.0;
            let text = format!("{} \u{b7} {}", day_words(r.put_day), place_words(r.origin));
            let w = self.face.width(&Line { text: &text, style: Style::Italic, size, ink, tracking: 0.0 });
            widest = widest.max(w);
            lines.push((r, y, mid, text, w));
        }
        let text_x = x0 + 16.0 * reading;
        let panel_w = (text_x - x0) + widest + 10.0 * reading;
        // over the globe the panel lies on a quiet card of the page's tone, one for the whole column,
        // so its words read on land and sea alike (a halo per glyph would lie as boxes at this size)
        let n_rows = lines.len() as f32;
        let over_globe = lines.iter().any(|(_, _, mid, _, _)| ((x0 + panel_w / 2.0 - disc[0]).powi(2) + (mid - disc[1]).powi(2)).sqrt() < disc[2] + 20.0 * reading);
        if over_globe {
            if let Some(b) = self.face.faces.get("regular").and_then(|f| f.glyphs.get("\u{2588}")) {
                let page = if night { [0.055, 0.063, 0.086, 0.78] } else { [0.957, 0.945, 0.918, 0.78] };
                let uv = [(b.x + 2.0) / self.face.width, (b.y + 2.0) / self.face.height, (b.w - 4.0) / self.face.width, (b.h - 4.0) / self.face.height];
                out.push(GlyphInstance { rect: [x0 - 8.0 * reading, top + 4.0 * reading, panel_w + 12.0 * reading, n_rows * row_h - 8.0 * reading], uv, ink: page, halo: [0.0; 4], anchor: [0.0; 4] });
            }
        }
        for (r, y, mid, text, w) in lines {
            let inside = ((x0 + panel_w / 2.0 - disc[0]).powi(2) + (mid - disc[1]).powi(2)).sqrt() < disc[2];
            // (over the globe no halo: at these sizes it fills the glyph's cell and would lie as a
            // box on the land; the inks hold their contrast on the sea and the night's ground alone)
            let halo = if inside || over_globe { [sea_halo[0], sea_halo[1], sea_halo[2], 0.0] } else { paper_halo };
            let hue = super::lamp_hue(r.id);
            // every mark a filled lamp in its own hue, unmistakable (it is how a reader finds whose
            // cloud is whose): larger with a soft glow of its colour while its cloud is out, a small
            // lamp while not; the front one the largest
            // the way it is chosen, as quietly as the rest (R8): as its cloud a larger lamp in a soft
            // glow of its colour (a haze, as the cloud is); as itself a small lamp in a ring of its
            // colour (a float, as it is on the sea); not chosen a small lamp alone. The front a size up
            let up = if r.front { 1.0 } else { 0.0 };
            let mark_size = if r.shown { 12.0 + up } else { 8.5 + 0.5 * up } * reading;
            if r.shown {
                let gs = mark_size * 1.7;
                let glow = Line { text: "\u{25cf}", style: Style::Regular, size: gs, ink: [hue[0], hue[1], hue[2], 0.22], tracking: 0.0 };
                out.extend(self.face.layout(&glow, x0 + 6.0 * reading, mid + 0.36 * gs, [0.0; 4]));
            }
            if r.followed {
                let rs = (16.0 + 1.5 * up) * reading;
                let ring = Line { text: "\u{25cb}", style: Style::Regular, size: rs, ink: [hue[0], hue[1], hue[2], 1.0], tracking: 0.0 };
                out.extend(self.face.layout(&ring, x0 + 6.0 * reading, mid + 0.33 * rs, [0.0; 4]));
            }
            let mark = Line { text: "\u{25cf}", style: Style::Regular, size: mark_size, ink: [hue[0], hue[1], hue[2], 1.0], tracking: 0.0 };
            out.extend(self.face.layout(&mark, x0 + 6.0 * reading, mid + 0.36 * mark_size, [0.0; 4]));
            let line = Line { text: &text, style: Style::Italic, size, ink: if r.front { ink } else { soft_ink }, tracking: 0.0 };
            out.extend(self.face.layout(&line, text_x + w / 2.0, mid + 0.33 * size, halo));
            self.panel_bounds.push((r.id, [x0 - 6.0 * reading, y, panel_w + 6.0 * reading, row_h]));
        }
        out
    }

    /// The stories' marks (painting-company row 199): a row of three small drawings under the date,
    /// only while two lamps are out. Each is two threads of beads, one in each lamp's hue, left to
    /// right the story's span, their gap the distance between the lamps: kept together, parted,
    /// lost and found at the end. The story told is drawn in full colour; once its take plays, its
    /// drawing becomes the take's own measured distance, lit up to the date, and a row of small beads
    /// under it says which of the few best takes this is. No words: the drawings are the stories.
    pub fn story_marks(&mut self, st: Option<&super::LookStory>, width: f32, height: f32, reading: f32, disc: [f32; 3], night: bool) -> Vec<GlyphInstance> {
        self.story_bounds.clear();
        let mut out = Vec::new();
        let Some(st) = st else { return out };
        // the row keeps its place whatever the camera does (a finger finds it where it was), at
        // the screen's foot above the home indicator; the date rises above it when the globe is close
        let row = height - STORY_ROW * reading;
        let (mw, mh, gap) = (46.0 * reading, 16.0 * reading, 22.0 * reading);
        let n = crate::story::Story::ALL.len() as f32;
        let x0 = width / 2.0 - (n * mw + (n - 1.0) * gap) / 2.0;
        let soft = if night { 0.55 } else { 0.5 };
        let face = &self.face;
        let dot = |out: &mut Vec<GlyphInstance>, x: f32, y: f32, size: f32, rgb: [f32; 3], a: f32| {
            let line = Line { text: "\u{25cf}", style: Style::Regular, size, ink: [rgb[0], rgb[1], rgb[2], a], tracking: 0.0 };
            out.extend(face.layout(&line, x, y + 0.36 * size, [0.0; 4]));
        };
        // over the globe (the camera close in) the row lies on a quiet card of the page's tone, as
        // the panel does, so the beads read on land and sea alike
        let total = n * mw + (n - 1.0) * gap;
        let over = (0..3).any(|i| ((x0 + mw / 2.0 + i as f32 * (mw + gap) - disc[0]).powi(2) + (row - disc[1]).powi(2)).sqrt() < disc[2] + 30.0 * reading);
        if over {
            if let Some(b) = self.face.faces.get("regular").and_then(|f| f.glyphs.get("\u{2588}")) {
                let page = if night { [0.055, 0.063, 0.086, 0.78] } else { [0.957, 0.945, 0.918, 0.78] };
                let uv = [(b.x + 2.0) / self.face.width, (b.y + 2.0) / self.face.height, (b.w - 4.0) / self.face.width, (b.h - 4.0) / self.face.height];
                out.push(GlyphInstance { rect: [x0 - 16.0 * reading, row - 20.0 * reading, total + 32.0 * reading, 44.0 * reading], uv, ink: page, halo: [0.0; 4], anchor: [0.0; 4] });
            }
        }
        let mut bounds = Vec::new();
        for (i, story) in crate::story::Story::ALL.iter().enumerate() {
            let left = x0 + i as f32 * (mw + gap);
            let cx = left + mw / 2.0;
            let active = st.active == Some(i);
            // the finger is answered: the told mark swells and settles
            let swell = if active { 1.0 + 0.22 * (-st.told * 5.0).exp() } else { 1.0 };
            let (w, h) = (mw * swell, mh * swell);
            let real = active && !st.curve.is_empty();
            let beads = if real { 23 } else { 11 };
            for j in 0..beads {
                let s = j as f32 / (beads - 1) as f32;
                let g = if real {
                    let at = s * (st.curve.len() - 1) as f32;
                    let (a, f) = (at.floor() as usize, at.fract());
                    let b = (a + 1).min(st.curve.len() - 1);
                    0.06 + 0.94 * (st.curve[a] * (1.0 - f) + st.curve[b] * f)
                } else {
                    story.shape(s)
                };
                let x = cx - w / 2.0 + s * w;
                let alpha = if real {
                    if s <= st.playhead + 1e-3 { 1.0 } else { 0.3 }
                } else if active && st.searching >= 0.0 {
                    // the search under way: the beads light as the worlds are searched, breathing
                    if s <= st.searching { 0.6 + 0.4 * (st.told * 6.0 - s * 4.0).sin().abs() } else { 0.25 }
                } else if active {
                    1.0
                } else {
                    soft
                };
                let size = if real { 4.2 } else { 5.0 } * reading * swell;
                dot(&mut out, x, row - g * h / 2.0, size, st.hues[0], alpha);
                dot(&mut out, x, row + g * h / 2.0, size, st.hues[1], alpha);
            }
            if active && st.searching < 0.0 {
                // which of the few best takes plays
                let k = crate::story::TAKES;
                let step = 6.0 * reading;
                let ink = if night { [0.894, 0.925, 0.957] } else { [0.149, 0.188, 0.227] };
                for t in 0..k {
                    let x = cx - step * (k - 1) as f32 / 2.0 + t as f32 * step;
                    dot(&mut out, x, row + mh / 2.0 + 10.0 * reading, if t == st.take { 4.0 } else { 3.0 } * reading, ink, if t == st.take { 0.9 } else { 0.3 });
                }
            }
            // a fingertip's room round each drawing
            bounds.push((*story, [left - gap / 2.0, row - 22.0 * reading, mw + gap, 44.0 * reading]));
        }
        self.story_bounds = bounds;
        out
    }

    /// The words at the foot: the date (17 pt) centred under the globe's disc (`cx, cy, r` in points)
    /// on a `width` x `height` point screen, or over the globe when the disc fills it; the note (13 pt
    /// italic) under the date; the credits (10 pt) along the screen's foot. Each in the palette's
    /// inks on a halo of what is under it: the paper below the disc, the sea over it (palette.json).
    pub fn prepare_foot(&mut self, gpu: &Gpu, date: &str, note: &str, width: f32, height: f32, reading: f32, disc: [f32; 3], night: bool, running: bool, year_day: f32, press: f32, seconds: f32, labels: &[GlyphInstance]) {
        let under = disc[1] + disc[2] + 40.0 * reading;
        // (a fingertip clear of the stories' row at the foot, when it is drawn: painting-company row 199)
        let foot = under.min(height - (STORY_ROW + 36.0) * reading);
        let _bottom = height - 12.0 * reading;
        let (date_ink, note_ink, _credit_ink, sea_halo, paper_halo) = if night {
            ([0.894, 0.925, 0.957, 1.0], [0.722, 0.776, 0.831, 1.0], [0.722, 0.776, 0.831, 1.0], [0.063, 0.094, 0.157, 0.92], [0.08, 0.10, 0.14, 0.92])
        } else {
            ([0.149, 0.188, 0.227, 1.0], [0.243, 0.298, 0.353, 1.0], [0.243, 0.298, 0.353, 1.0], [0.761, 0.894, 0.957, 0.92], [0.96, 0.95, 0.92, 0.92])
        };
        let halo_for = |baseline: f32| if baseline - 14.0 * reading < disc[1] + disc[2] { sea_halo } else { paper_halo };
        let cx = width / 2.0;
        let date_line = Line { text: date, style: Style::Regular, size: 17.0 * reading, ink: date_ink, tracking: 0.0 };
        let dw = (self.face.width(&date_line) + 24.0 * reading).max(150.0 * reading);
        let dh = 17.0 * reading * 1.3 + 12.0 * reading + 10.0 * reading;
        self.date_bounds = Some([cx - dw / 2.0, foot - 17.0 * reading * 0.95 - 6.0 * reading, dw, dh]);
        let mut glyphs = self.face.layout(&date_line, cx, foot, halo_for(foot));
        // the year's rule under the date, a hairline the band's width, and the day's marker on it: a
        // lamp of amber while the date runs, a ring of ink while it rests; the one drawn thing that
        // says it can be moved (a drag along it scrubs, a tap rests or runs it: Motion's), without a
        // control that points or waits
        // the band answers the finger (D8): the frame it lands the rule is full ink and the lamp larger;
        // over the rest's beat the rule thickens, as the ring on the sea does, until the page of credits opens
        let touched = if press > 0.0 { 1.0 } else { 0.0 };
        let rule_y = foot + 8.0 * reading;
        let rule_w = dw - 24.0 * reading;
        let rule_a = if night { 0.55 } else { 0.45 } + 0.5 * touched;
        let rule_ink = if night { [0.722, 0.776, 0.831, rule_a] } else { [0.243, 0.298, 0.353, rule_a] };
        let rule_h = (0.7 + 1.1 * press.clamp(0.0, 1.0)) * reading;
        if let Some(b) = self.face.faces.get("regular").and_then(|f| f.glyphs.get("\u{2588}")) {
            let uv = [(b.x as f32 + 2.0) / self.face.width, (b.y as f32 + 2.0) / self.face.height, (b.w as f32 - 4.0) / self.face.width, (b.h as f32 - 4.0) / self.face.height];
            glyphs.push(GlyphInstance { rect: [cx - rule_w / 2.0, rule_y - rule_h / 2.0, rule_w, rule_h], uv, ink: rule_ink, halo: [0.0; 4], anchor: [0.0; 4] });
        }
        let mx = cx - rule_w / 2.0 + rule_w * year_day.clamp(0.0, 1.0);
        let (mark, mark_ink) = if running { ("\u{25cf}", if night { [0.965, 0.682, 0.227, 1.0] } else { [0.957, 0.647, 0.165, 1.0] }) } else { ("\u{25cb}", if night { [0.894, 0.925, 0.957, 1.0] } else { [0.149, 0.188, 0.227, 1.0] }) };
        // the sign that the year runs: the lamp breathes, a soft glow of its own colour swelling and
        // settling over three seconds; at rest the ring is still and has no glow (gentle, unmistakable)
        if running {
            let breath = 0.5 + 0.5 * (seconds * 2.1).sin();
            let glow = Line { text: "\u{25cf}", style: Style::Regular, size: (14.0 + 6.0 * breath + 3.0 * touched) * reading, ink: [mark_ink[0], mark_ink[1], mark_ink[2], 0.16 + 0.14 * breath], tracking: 0.0 };
            let gs = (14.0 + 6.0 * breath + 3.0 * touched) * reading;
            let ms = (9.0 + 3.0 * touched) * reading;
            glyphs.extend(self.face.layout(&glow, mx, rule_y + 3.2 * reading + 0.35 * (gs - ms), [0.0; 4]));
        }
        let marker = Line { text: mark, style: Style::Regular, size: (9.0 + 3.0 * touched) * reading, ink: mark_ink, tracking: 0.0 };
        glyphs.extend(self.face.layout(&marker, mx, rule_y + 3.2 * reading, [0.0; 4]));   // (a mark, not a word: no halo)
        if !note.is_empty() {
            let y = foot + 24.0 * reading;
            glyphs.extend(self.face.layout(&Line { text: note, style: Style::Italic, size: 13.0 * reading, ink: note_ink, tracking: 0.0 }, cx, y, halo_for(y)));
        }
        // (no credits line: the page of credits is a rest on the date away, DECISIONS.md › Truth underneath)
        self.credit_bounds = None;
        glyphs.extend_from_slice(labels);
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
