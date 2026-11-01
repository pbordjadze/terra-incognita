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

/// One lamp's mark in the panel (R6; R10 a mark, no words).
#[derive(Clone, Debug)]
pub struct PanelRow<'a> {
    pub id: u32,
    pub origin: crate::model::LonLat,
    pub put_day: f64,
    /// its cloud is out
    pub shown: bool,
    /// chosen as itself, its journey followed (R8)
    pub followed: bool,
    /// the one in front
    pub front: bool,
    /// its journey so far, one place a day, oldest first (R10: the mark is its shape)
    pub journey: &'a [super::LookBead],
    /// where it is now
    pub at: crate::model::LonLat,
    /// a finger rests on its mark (0 none, else 0.35..1 as the rest thickens): the mark brightens
    pub pressed: f32,
    /// kept for good (R13): a small gold pin under its lamp
    pub kept: bool,
}

/// A long day's words for the shell's VoiceOver label ("8 June").
pub fn day_words_long(day: f64) -> String {
    const LONG: [(&str, &str); 12] = [("Jan", "January"), ("Feb", "February"), ("Mar", "March"), ("Apr", "April"), ("May", "May"), ("Jun", "June"), ("Jul", "July"), ("Aug", "August"), ("Sep", "September"), ("Oct", "October"), ("Nov", "November"), ("Dec", "December")];
    let short = day_words(day);
    let mut parts = short.splitn(2, ' ');
    let (d, m) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let m = LONG.iter().find(|(s, _)| *s == m).map_or(m, |(_, l)| l);
    format!("{d} {m}")
}

/// The most rows the panel shows (the latest lamps), and where its first row's top lies in reading
/// points (below a phone's status bar and island).
pub const PANEL_ROWS: usize = 10;
pub const PANEL_TOP: f32 = 60.0;
/// A mark's cell in the panel, reading points square (R10: the touch target, whatever is drawn in it),
/// and how many to a line before the next line down.
pub const PANEL_CELL: f32 = 44.0;
pub const PANEL_ACROSS: usize = 5;

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
    /// Each mark's words for the shell's VoiceOver, by lamp (R10: none on the screen): "Lamp 2, put 8 June
    /// at 35°N 74°W, its cloud out".
    pub panel_words: Vec<(u32, String)>,
    /// The record control's rect in points (R9), when the look draws one.
    pub record_bounds: Option<[f32; 4]>,
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
        Self { pipeline, instances: Instances::new(gpu, "words", 256), buffers: [b0, b1], groups: [g0, g1], face, date_bounds: None, credit_bounds: None, panel_bounds: Vec::new(), panel_words: Vec::new(), record_bounds: None }
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

    /// The panel of lamps (R6; R10 marks, no words): a tight row of marks at the screen's top left,
    /// opposite the record control, one a lamp out there, oldest first, `PANEL_ACROSS` to a line, at
    /// most `PANEL_ROWS` (the latest). A mark is its lamp's journey drawn small in its hue, the shape
    /// its line makes on the sea, ending in the lamp itself: so a mark is told from another by its
    /// colour and its shape, and is found on the globe by both. Its cloud out, the lamp glows; chosen
    /// as itself, a ring round it (a float, as it is on the sea); the one in front full, the others a
    /// shade quieter. Strokes only (`strokes`), no words: each mark's words go to `panel_words` for
    /// VoiceOver, its cell (`PANEL_CELL` reading points square) to `panel_bounds` for Motion's probe.
    pub fn panel(&mut self, rows: &[PanelRow], reading: f32, night: bool, strokes: &mut Vec<super::wake::StrokeInstance>) {
        use super::wake::{line, Pt};
        self.panel_bounds.clear();
        self.panel_words.clear();
        let cell = PANEL_CELL * reading;
        let x0 = 12.0 * reading;
        let top = PANEL_TOP * reading;
        let start = rows.len().saturating_sub(PANEL_ROWS);
        let any_front = rows.iter().any(|r| r.front);
        // a lamp chosen, followed, its cloud out or a mark pressed brings the marks up whole
        let awake = rows.iter().any(|r| r.front || r.shown || r.followed || r.pressed > 0.0);
        // (R15: at rest nothing at all is drawn: the corner is the globe's; a touch on a mark's cell
        // still answers, presses it into view, and chooses its lamp, and VoiceOver reads every mark)
        let (k_size, k_alpha) = (1.0, 1.0);
        for (i, r) in rows[start..].iter().enumerate() {
            let (cx, cy) = (x0 + ((i % PANEL_ACROSS) as f32 + 0.5) * cell, top + ((i / PANEL_ACROSS) as f32 + 0.5) * cell);
            self.panel_bounds.push((r.id, [cx - cell / 2.0, cy - cell / 2.0, cell, cell]));
            let how = format!("{}{}", if r.followed { ", followed" } else if r.shown { ", its cloud out" } else { "" }, if r.kept { ", kept" } else { "" });
            self.panel_words.push((r.id, format!("Lamp {}, put {} at {}{}", r.id, day_words_long(r.put_day), place_words(r.origin), how)));
            if !awake {
                continue;
            }
            let hue = super::lamp_hue(r.id);
            let quiet = k_alpha * if r.front || !any_front || r.pressed > 0.0 { 1.0 } else { 0.72 };
            // the journey in the cell: its places flattened round their middle (east scaled by the
            // latitude's cosine), smoothed as the wake is, fitted to the cell's middle with the lamp
            // kept off its edge; a short journey stays small (at least 4 degrees fill the mark)
            let mut places: Vec<(f32, f32)> = r.journey.iter().step_by(super::STEPS_A_DAY).map(|b| (b.at.lon as f32, b.at.lat as f32)).collect();
            places.push((r.at.lon as f32, r.at.lat as f32));
            let (lon0, lat0) = places[places.len() - 1];
            let flat: Vec<(f32, f32)> = places.iter().map(|&(lo, la)| (((lo - lon0 + 540.0).rem_euclid(360.0) - 180.0) * lat0.to_radians().cos(), la - lat0)).collect();
            let (mut x_lo, mut x_hi, mut y_lo, mut y_hi) = (f32::MAX, f32::MIN, f32::MAX, f32::MIN);
            for &(x, y) in &flat {
                x_lo = x_lo.min(x); x_hi = x_hi.max(x); y_lo = y_lo.min(y); y_hi = y_hi.max(y);
            }
            let span = (x_hi - x_lo).max(y_hi - y_lo).max(4.0);
            let k = 26.0 * k_size * reading / span;
            let (mx, my) = ((x_lo + x_hi) / 2.0, (y_lo + y_hi) / 2.0);
            let to_screen = |x: f32, y: f32| [cx + (x - mx) * k, cy - (y - my) * k, 0.0];
            let as3: Vec<([f32; 3], f32)> = flat.iter().map(|&(x, y)| ([x, y, 0.0], 1.0)).collect();
            let pts: Vec<Pt> = super::wake::smooth_flat(&as3, 3, 3)
                .into_iter()
                .map(|(p, f)| Pt { p: to_screen(p[0], p[1]), w: (1.1 + 1.3 * f) * k_size.sqrt() * reading, a: quiet * (0.45 + 0.55 * f), hollow: 0.0 })
                .collect();
            if pts.len() > 1 {
                line(strokes, &pts, hue, true, night, false);
            }
            let lamp = pts.last().map_or([cx, cy, 0.0], |p| p.p);
            let up = if r.front { 1.0 } else { 0.0 } + 1.5 * r.pressed;
            // (by night no glow: a glowing dot on the dark read as a town's light, R12; the lamp is glass,
            // a ring of its hue with a small spark)
            if r.shown && !night {
                line(strokes, &[Pt { p: lamp, w: (15.0 + 2.0 * up) * reading, a: 0.24 * quiet, hollow: 0.0 }], hue, true, true, false);
            }
            if r.followed {
                let rr = (6.5 + 0.5 * up) * reading;
                let ring: Vec<Pt> = (0..32).map(|j| {
                    let t = j as f32 / 32.0 * std::f32::consts::TAU;
                    Pt { p: [lamp[0] + rr * t.cos(), lamp[1] + rr * t.sin(), 0.0], w: 1.3 * reading, a: quiet, hollow: 0.0 }
                }).collect();
                line(strokes, &ring, hue, true, night, true);
            }
            // kept for good (R13): a small gold pin under the lamp, a dot on a short stem, no words
            if r.kept {
                let (px, py) = (lamp[0], lamp[1] + 7.5 * reading);
                line(strokes, &[Pt { p: [px, py - 2.0 * reading, 0.0], w: 0.9 * reading, a: 0.9, hollow: 0.0 }, Pt { p: [px, py + 1.5 * reading, 0.0], w: 0.9 * reading, a: 0.9, hollow: 0.0 }], super::KEPT_GOLD, true, true, false);
                line(strokes, &[Pt { p: [px, py + 2.6 * reading, 0.0], w: 2.8 * reading, a: 1.0, hollow: 0.0 }], super::KEPT_GOLD, true, night, false);
            }
            if night {
                let rr = (4.2 + 0.6 * up) * reading;
                let glass: Vec<Pt> = (0..24).map(|j| {
                    let t = j as f32 / 24.0 * std::f32::consts::TAU;
                    Pt { p: [lamp[0] + rr * t.cos(), lamp[1] + rr * t.sin(), 0.0], w: 1.2 * reading, a: quiet * if r.shown { 1.0 } else { 0.8 }, hollow: 0.0 }
                }).collect();
                line(strokes, &glass, hue, true, true, true);
                line(strokes, &[Pt { p: lamp, w: (3.0 + 0.6 * up) * reading, a: quiet, hollow: 0.0 }], hue, true, true, false);
            } else {
                line(strokes, &[Pt { p: lamp, w: (6.5 + 1.5 * up) * k_size * reading, a: quiet, hollow: 0.0 }], hue, true, night, false);
            }
        }
    }

    /// The words at the foot: the date (17 pt) centred under the globe's disc (`cx, cy, r` in points)
    /// on a `width` x `height` point screen, or over the globe when the disc fills it; the note (13 pt
    /// italic) under the date; the credits (10 pt) along the screen's foot. Each in the palette's
    /// inks on a halo of what is under it: the paper below the disc, the sea over it (palette.json).
    pub fn prepare_foot(&mut self, gpu: &Gpu, date: &str, note: &str, width: f32, height: f32, reading: f32, disc: [f32; 3], night: bool, running: bool, year_day: f32, press: f32, seconds: f32, labels: &[GlyphInstance]) {
        let under = disc[1] + disc[2] + 40.0 * reading;
        let foot = under.min(height - 72.0 * reading);
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

    /// The words of an exported frame (R9): the clip is the sea and its lamps, so the chrome stays out
    /// (the panel, the date's band with its rule and marker, the credits, the recording's mark) and only
    /// the date is kept, a quiet caption, 15 points, on the paper under the globe in the clip's window
    /// (`clip`, points x, y, w, h), or not at all when the globe fills the clip (R11). Through the clip's
    /// last beat (`ending`) it grows a touch quieter. `date_bounds` keeps the live screen's.
    pub fn prepare_caption(&mut self, gpu: &Gpu, date: &str, clip: [f32; 4], reading: f32, disc: [f32; 3], night: bool, ending: f32, opening: f32, labels: &[GlyphInstance]) {
        // on the paper under the globe, or not at all (R11): over the globe it lay on the land or the
        // sea as a sticker; a close clip is the sea alone and its story is the lamps
        let foot = disc[1] + disc[2] + 30.0 * reading;
        // (fading in through the clip's opening beat, R15, and a touch quieter through its last)
        let o = opening.clamp(0.0, 1.0);
        let quiet = (1.0 - 0.25 * ending.clamp(0.0, 1.0)) * o * o * (3.0 - 2.0 * o);
        let mut glyphs = Vec::new();
        if foot + 6.0 * reading <= clip[1] + clip[3] - 10.0 * reading {
            let (ink, halo) = if night { ([0.722, 0.776, 0.831, 0.95 * quiet], [0.055, 0.063, 0.086, 0.92 * quiet]) } else { ([0.243, 0.298, 0.353, 0.95 * quiet], [0.96, 0.95, 0.92, 0.92 * quiet]) };
            glyphs = self.face.layout(&Line { text: date, style: Style::Regular, size: 15.0 * reading, ink, tracking: 0.02 }, clip[0] + clip[2] / 2.0, foot, halo);
        }
        glyphs.extend_from_slice(labels);
        self.prepare(gpu, &glyphs);
    }

    /// The record control (R9; Motion's gestures: a tap starts a take, a touch while recording ends
    /// it and exports, a rest exports the newest again), top right opposite the panel, on the live
    /// screen only. At rest a thin ring of soft ink round a small muted vermilion lamp: there, never
    /// shouting. Recording, the lamp lit and breathing, and an arc of vermilion drawing round the ring
    /// from the top toward the take's longest (`studio::TAKE_MAX`, 60 s), a smooth stroke (R10), the
    /// seconds beside it in small italic. Its rect, 44 reading points square whatever is drawn, goes
    /// to `record_bounds` for Motion's probe.
    pub fn record_control(&mut self, recording: Option<f32>, now: f32, width: f32, reading: f32, night: bool, song: bool, strokes: &mut Vec<super::wake::StrokeInstance>) -> Vec<GlyphInstance> {
        use super::wake::{line, Pt};
        let side = 44.0 * reading;
        let (x, y) = (width - 16.0 * reading - side, 52.0);
        self.record_bounds = Some([x, y, side, side]);
        let (cx, cy) = (x + side / 2.0, y + side / 2.0);
        let red = if night { [0.937, 0.427, 0.353] } else { [0.835, 0.322, 0.247] };
        let soft = if night { [0.722, 0.776, 0.831] } else { [0.243, 0.298, 0.353] };
        let mut out = Vec::new();
        let ring_r = 10.0 * reading;
        let arc = |from: f32, to: f32, w: f32, a: f32| -> Vec<Pt> {
            let n = ((to - from) * 64.0).ceil().max(2.0) as usize;
            (0..=n).map(|i| {
                let t = from + (to - from) * i as f32 / n as f32;
                let ang = t * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
                Pt { p: [cx + ring_r * ang.cos(), cy + ring_r * ang.sin(), 0.0], w, a, hollow: 0.0 }
            }).collect()
        };
        let dot = |at: [f32; 2], w: f32, a: f32| [Pt { p: [at[0], at[1], 0.0], w, a, hollow: 0.0 }];
        // a song opened (R12): a second, finer ring just outside the first, a record's groove, at rest
        // and while recording; no words
        if song {
            let mut groove: Vec<Pt> = (0..64).map(|j| {
                let ang = j as f32 / 64.0 * std::f32::consts::TAU;
                Pt { p: [cx + (ring_r + 3.2 * reading) * ang.cos(), cy + (ring_r + 3.2 * reading) * ang.sin(), 0.0], w: 0.8 * reading, a: 0.4, hollow: 0.0 }
            }).collect();
            groove.truncate(64);
            line(strokes, &groove, soft, true, true, true);
        }
        // (the soft ring and the lamp by night as by day: no dark edge, they are ink, not lamps on the sea)
        match recording {
            None => {
                let mut ring = arc(0.0, 1.0, 1.3 * reading, 0.45);
                ring.pop();
                line(strokes, &ring, soft, true, true, true);
                line(strokes, &dot([cx, cy], 8.0 * reading, 0.55), red, true, true, false);
            }
            Some(secs) => {
                let breath = 0.5 + 0.5 * (now * 2.6).sin();
                let full = (secs / crate::studio::TAKE_MAX as f32).clamp(0.0, 1.0);
                let mut ring = arc(0.0, 1.0, 1.3 * reading, 0.25);
                ring.pop();
                line(strokes, &ring, soft, true, true, true);
                if full > 0.0 {
                    line(strokes, &arc(0.0, full.max(0.004), 2.4 * reading, 0.92), red, true, true, false);
                }
                line(strokes, &dot([cx, cy], 15.0 * reading, 0.12 + 0.14 * breath), red, true, true, false);
                line(strokes, &dot([cx, cy], 8.5 * reading, 0.8 + 0.2 * breath), red, true, true, false);
                let s = secs.max(0.0) as u32;
                let text = format!("{}:{:02}", s / 60, s % 60);
                let size = 10.5 * reading;
                let line = Line { text: &text, style: Style::Italic, size, ink: [soft[0], soft[1], soft[2], 0.9], tracking: 0.0 };
                let tw = self.face.width(&line);
                out.extend(self.face.layout(&line, x - 2.0 * reading - tw / 2.0, cy + 0.33 * size, [0.0; 4]));
            }
        }
        out
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
