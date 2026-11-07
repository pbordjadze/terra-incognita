//! The toy: the Rio Beni's floodplain (Sentinel-2, today's river taken out), a river on it that wanders
//! by `sim`, the sand it leaves on the inside of its bends greening over, and the lakes it cuts off.
//! One gesture: a finger draws the river's new course; the old one is left to silt up.

use crate::sim::{dist, River, W};
use crate::{extend, opening_course, seed_wobble, YEARS_A_SECOND};
use toy_kit::gpu::textures::{decode, Texture};
use toy_kit::paint::Painter;
use toy_kit::sound::{Bells, Note};
use toy_kit::{Ctx, PointerEvent, Toy};

/// The sheet: metres a side (plain.json), and the deposit grid's cells a side.
const SIDE: f64 = 22000.0;
const NG: usize = 880;
/// The deposit grid's upload tiles: TILE cells square, TILES a side.
const TILE: usize = 40;
const TILES: usize = NG / TILE;
const NEVER: f32 = -1.0e9;
/// How long a lake lives on screen, years: dark water, then a marsh, then forest.
/// The clock while nothing on screen is about to happen, years a second.
const FAST_YEARS: f64 = 20.0;
const LAKE_LIFE: f64 = 320.0; // a lake silts up and is forest again in under half a minute
/// The screen's longer side, m: a few loops at a time, each big enough to watch.
const VIEW_M: f64 = 13000.0;
/// The finger's brush: points across (a fingertip, a little more), and metres of river gained a tick.
const BRUSH_PT: f64 = 38.0;
const TICK_M: f64 = 450.0;
const COURSE_LIFE: f64 = 28.0; // an abandoned course silts and greens over in under three seconds

const WGSL: &str = r#"
struct U { size: vec2<f32>, centre: vec2<f32>, scale: f32, now: f32, side: f32, ppp: f32, river: f32, p0: f32, p1: f32, p2: f32 };
@group(0) @binding(0) var<uniform> u: U;
@group(0) @binding(1) var plain: texture_2d<f32>;
@group(0) @binding(2) var dep: texture_2d<f32>;
@group(0) @binding(3) var smp: sampler;
@group(0) @binding(4) var mask: texture_2d<f32>;
struct VO { @builtin(position) pos: vec4<f32>, @location(0) uv: vec2<f32> };
@vertex fn vs_main(@builtin(vertex_index) vi: u32) -> VO {
  let p = array<vec2<f32>, 3>(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0))[vi];
  var o: VO; o.pos = vec4<f32>(p, 0.0, 1.0); o.uv = vec2<f32>((p.x + 1.0) * 0.5, (1.0 - p.y) * 0.5); return o;
}
fn hash(p: vec2<f32>) -> f32 { return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453); }
fn vnoise(p: vec2<f32>) -> f32 {
  let i = floor(p); let f = fract(p); let w = f * f * (3.0 - 2.0 * f);
  return mix(mix(hash(i), hash(i + vec2(1.0, 0.0)), w.x), mix(hash(i + vec2(0.0, 1.0)), hash(i + vec2(1.0, 1.0)), w.x), w.y);
}
@fragment fn fs_main(i: VO) -> @location(0) vec4<f32> {
  let px = i.uv * u.size;
  let world = u.centre + (px - u.size * 0.5) / u.scale;
  let st = world / u.side + 0.5;
  var c = textureSampleLevel(plain, smp, st, 0.0).rgb;
  // the photograph a touch clearer: lifted out of the haze, the greens a little richer
  let l = dot(c, vec3(0.3, 0.59, 0.11));
  c = clamp((c - 0.06) * 1.18, vec3(0.0), vec3(1.0));
  c = mix(vec3(l), c, 1.15);
  // where the river has been: bare sand, then cane, then the forest again
  // nine taps a cell apart: the 25 m grid's cells blurred into a soft edge
  let cellw = 25.0 / u.side;
  var dg = vec2<f32>(0.0);
  for (var j = -1; j <= 1; j++) {
    for (var k = -1; k <= 1; k++) {
      let wgt = select(0.5, 1.0, j == 0 && k == 0);
      dg += textureSampleLevel(dep, smp, st + vec2(f32(j), f32(k)) * cellw * 0.9, 0.0).rg * wgt;
    }
  }
  dg /= 5.0;
  let near = smoothstep(0.05, 0.85, dg.y);
  let d = select(-2.0e9, dg.x / max(dg.y, 1e-3), dg.y > 0.02);
  let age = u.now - d;
  if (dg.y > 0.02 && age >= 0.0) {
    // where the river has been, its old banks: a thin pale arc every ten years (lines of equal age,
    // the scroll bars a growing bend leaves on its inside), fading as the forest closes over them;
    // a narrow fringe of bare sand at the newest
    let per = 22.0;
    let ph = d / per;
    let fw = max(fwidth(ph), 1e-4);
    let line = smoothstep(0.0, 3.0 * fw + 0.08, abs(fract(ph) - 0.5) * 2.0 - (1.0 - 4.0 * fw));
    let grain = vnoise(world / 30.0);
    let fade = 1.0 - smoothstep(50.0, 200.0, age + grain * 20.0);
    // where the ridges crowd together (a bend's tip, a neck) they would fan into a mesh: let them go
    let calm = 1.0 - smoothstep(0.06, 0.2, fw);
    let arc = clamp(line, 0.0, 1.0) * fade * near * calm * 0.08;
    c = mix(c, vec3(0.78, 0.80, 0.62), arc);
    // just swept: wet silt, darker than the forest's edge, drying within a few years
    // just left by the water: a point bar of pale sand on the inside of the bend, greening in a decade
    let sand = (1.0 - smoothstep(1.0, 10.0, age + grain * 3.0)) * near;
    c = mix(c, vec3(0.72, 0.66, 0.52), sand * 0.5);
    c = c * (1.0 + 0.06 * near * fade);
  }
  // the water, from the mask the river and its lakes were painted into (r the river's water, g where
  // a bend shoals it, b a lake's darkness): drawn into the photograph rather than laid on it, with
  // the photograph's own grain in it, a wet edge, and the faster water down the middle a shade lighter
  let m = textureSampleLevel(mask, smp, i.uv, 0.0);
  let grain = l - 0.25;
  let o = vec2<f32>(u.river * 0.22) / u.size;
  // how far inside the channel: the mask averaged a third of a width around (smooth, no ridges)
  var mid = 0.0;
  mid += textureSampleLevel(mask, smp, i.uv + vec2(o.x, 0.0), 0.0).r;
  mid += textureSampleLevel(mask, smp, i.uv - vec2(o.x, 0.0), 0.0).r;
  mid += textureSampleLevel(mask, smp, i.uv + vec2(0.0, o.y), 0.0).r;
  mid += textureSampleLevel(mask, smp, i.uv - vec2(0.0, o.y), 0.0).r;
  mid = smoothstep(0.5, 1.0, mid * 0.25);
  c = mix(c, vec3(0.13, 0.17, 0.24) * (1.0 + grain * 0.8), clamp(m.b, 0.0, 1.0) * 0.9);
  c = mix(c, vec3(0.42, 0.39, 0.29), smoothstep(0.0, 0.5, m.r) * 0.3);
  // the deep water down the middle a shade darker, the shoals at the bends paler
  let water = mix(vec3(0.68, 0.56, 0.42), vec3(0.78, 0.67, 0.52), clamp(m.g * 1.4, 0.0, 1.0)) * (1.0 - 0.1 * mid * mid) + vec3(0.5, 0.46, 0.36) * smoothstep(0.62, 1.0, m.g);
  // the banks a shade darker (wet, shadowed by the trees), the water a little clear over them
  let bank = smoothstep(0.1, 0.55, m.r) * (1.0 - mid);
  c = mix(c, water * (1.0 + grain * 0.5) * (1.0 - 0.16 * bank), smoothstep(0.1, 1.0, m.r) * 0.86);
  return vec4<f32>(c, 1.0);
}
"#;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uni {
    size: [f32; 2],
    centre: [f32; 2],
    scale: f32,
    now: f32,
    side: f32,
    ppp: f32,
    river: f32,
    pad: [f32; 3],
}

struct Hand {
    id: u64,
    at: [f64; 2],
    travel: f64,
    gained: f64,
    ticks: i32,
}

struct Ripple {
    at: [f64; 2],
    t: f64,
    strong: bool,
}

pub struct Meander {
    river: River,
    /// The year each cell last had the river in it (NEVER if it never has).
    dep: Vec<f32>,
    /// The same as the texture's half floats (year, 1), and the tiles (TILE cells square) whose half
    /// floats changed since the last upload: the river covers a sliver of each row, and a half float
    /// holds a year to a year or so, so most frames change a few tiles, not the 3 MB grid.
    half: Vec<u16>,
    dirty: Vec<bool>,
    epoch: f64,
    dep_tex: wgpu::Texture,
    _plain: Texture,
    pipeline: wgpu::RenderPipeline,
    /// The water mask (half the screen's pixels), its size and the bind group that reads it; made
    /// again when the screen's size changes.
    mask: Option<(wgpu::Texture, [u32; 2], wgpu::BindGroup)>,
    mask_painter: Painter,
    bgl: wgpu::BindGroupLayout,
    smp: wgpu::Sampler,
    dep_view: wgpu::TextureView,
    uni: wgpu::Buffer,
    painter: Painter,
    /// The fingers in the river, each a push of its own: its id, where it was last (m), how far it has
    /// gone (points), the river's length gained since it touched, and the ticks struck for that.
    hands: Vec<Hand>,
    /// When a finger last pushed the river (s).
    pushed: f64,
    /// When a tick last sounded (s): with several fingers pushing, one note and one tap at a time.
    ticked: f64,
    /// When the river was last laid (it swells from a thread to its width).
    laid: f64,
    ripples: Vec<Ripple>,
    cuts_seen: usize,
    /// The clock now, years a second.
    pace: f64,
    /// Cuts on screen: when, and where the neck broke (the new channel there flashes).
    flashes: Vec<(f64, [f64; 2])>,
    t: f64,
    /// Flecks on the water, as fractions of its length.
    flecks: Vec<(f64, f64)>,
    bells: Bells,
    reduce_motion: bool,
    frame: u64,
}

fn f16(v: f32) -> u16 {
    // f32 to IEEE half (the values here are years, finite, |v| < 65504 or NEVER)
    if v < -60000.0 {
        return 0xfbff; // -65504
    }
    let b = v.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let man = b & 0x7f_ffff;
    if exp <= 0 {
        return sign;
    }
    if exp >= 31 {
        return sign | 0x7bff;
    }
    sign | ((exp as u16) << 10) | ((man >> 13) as u16)
}

impl Meander {
    /// Points a metre, and the sheet's point at the screen's centre.
    fn view(&self, ctx: &Ctx) -> (f64, [f64; 2]) {
        let [wp, hp] = ctx.points();
        let s = wp.max(hp) / VIEW_M;
        let half = [wp / s * 0.5, hp / s * 0.5];
        let lim = |h: f64| (SIDE * 0.5 - h).max(0.0);
        let cx = 1100.0f64.clamp(-lim(half[0]), lim(half[0]));
        (s, [cx, 0.0])
    }

    fn to_world(&self, ctx: &Ctx, p: [f64; 2]) -> [f64; 2] {
        let (s, c) = self.view(ctx);
        let [wp, hp] = ctx.points();
        [(p[0] - wp * 0.5) / s + c[0], (p[1] - hp * 0.5) / s + c[1]]
    }

    fn view_bounds(&self, ctx: &Ctx) -> [f64; 4] {
        let [wp, hp] = ctx.points();
        let a = self.to_world(ctx, [0.0, 0.0]);
        let b = self.to_world(ctx, [wp, hp]);
        [a[0], a[1], b[0], b[1]]
    }

    /// Marks the cells under the channel with this year.
    fn stamp(&mut self) {
        let cell = SIDE / NG as f64;
        let r = W * 0.5 / cell;
        let y = (self.river.year - self.epoch) as f32;
        let line = &self.river.line;
        let mut last: Option<[f64; 2]> = None;
        for &p in line {
            if let Some(q) = last {
                if dist(p, q) < cell * 0.8 {
                    continue;
                }
            }
            last = Some(p);
            let gx = (p[0] + SIDE * 0.5) / cell;
            let gy = (p[1] + SIDE * 0.5) / cell;
            let (x0, x1) = ((gx - r).floor() as i64, (gx + r).ceil() as i64);
            let (y0, y1) = ((gy - r).floor() as i64, (gy + r).ceil() as i64);
            for yy in y0.max(0)..=y1.min(NG as i64 - 1) {
                for xx in x0.max(0)..=x1.min(NG as i64 - 1) {
                    let dx = xx as f64 + 0.5 - gx;
                    let dy = yy as f64 + 0.5 - gy;
                    if dx * dx + dy * dy <= r * r {
                        let c = yy as usize * NG + xx as usize;
                        self.dep[c] = y;
                        let h = f16(y);
                        if self.half[c * 2] != h || self.half[c * 2 + 1] != 0x3c00 {
                            self.half[c * 2] = h;
                            self.half[c * 2 + 1] = 0x3c00;
                            self.dirty[(yy as usize / TILE) * TILES + xx as usize / TILE] = true;
                        }
                    }
                }
            }
        }
    }
}

fn smooth01(e0: f64, e1: f64, x: f64) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    (t * t * (3.0 - 2.0 * t)) as f32
}

// the water's colours are in the shader (the Beni's café-au-lait: white water, full of the Andes;
// the lakes the photograph's own oxbows' tone, sampled)

impl Toy for Meander {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let gpu = &ctx.gpu;
        toy_kit::step("the floodplain");
        let plain = Texture::from_pixels(gpu, "plain", &decode(&ctx.assets.read("plain.jpg")?, 4)?, false);
        let dep_tex = gpu.texture(&wgpu::TextureDescriptor {
            label: Some("deposits"),
            size: wgpu::Extent3d { width: NG as u32, height: NG as u32, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rg16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let uni = gpu.buffer(&wgpu::BufferDescriptor { label: Some("meander"), size: std::mem::size_of::<Uni>() as u64, usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, mapped_at_creation: false });
        let smp = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            ..Default::default()
        });
        let tex = |b: u32| wgpu::BindGroupLayoutEntry {
            binding: b,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture { sample_type: wgpu::TextureSampleType::Float { filterable: true }, view_dimension: wgpu::TextureViewDimension::D2, multisampled: false },
            count: None,
        };
        let bgl = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("meander"),
            entries: &[
                wgpu::BindGroupLayoutEntry { binding: 0, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None }, count: None },
                tex(1),
                tex(2),
                wgpu::BindGroupLayoutEntry { binding: 3, visibility: wgpu::ShaderStages::FRAGMENT, ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering), count: None },
                tex(4),
            ],
        });
        let dep_view = dep_tex.create_view(&Default::default());
        let shader = gpu.shader("meander-plain", WGSL);
        let pipeline = gpu.fullscreen_pipeline("meander-plain", &shader, &[&bgl], None);
        let painter = Painter::new(gpu);
        let mut toy = Meander {
            river: River::new(Vec::new(), 7),
            dep: vec![NEVER; NG * NG],
            half: vec![0; NG * NG * 2],
            dirty: vec![true; TILES * TILES],
            epoch: 0.0,
            dep_tex,
            _plain: plain,
            pipeline,
            mask: None,
            mask_painter: Painter::with_format(gpu, wgpu::TextureFormat::Rgba8Unorm),
            bgl,
            smp,
            dep_view,
            uni,
            painter,
            hands: Vec::new(),
            ticked: -100.0,
            pushed: -100.0,
            laid: -10.0,
            ripples: Vec::new(),
            cuts_seen: 0,
            pace: YEARS_A_SECOND,
            flashes: Vec::new(),
            t: 0.0,
            flecks: Vec::new(),
            bells: Bells::new(),
            reduce_motion: ctx.reduce_motion,
            frame: 0,
        };
        let b = toy.view_bounds(ctx);
        toy.river = River::new(seed_wobble(&extend(&opening_course(7), b), 3), 7);
        let mut s = 0x51u64;
        toy.flecks = (0..90).map(|_| (crate::sim::rand(&mut s), crate::sim::rand(&mut s))).collect();
        // the river has been running a while when the toy opens: its bends are already growing
        for _ in 0..60 {
            toy.river.advance(1.0);
            toy.stamp();
        }
        toy.cuts_seen = toy.river.cuts.len();
        Ok(toy)
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(self.bells.synth())
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let w = self.to_world(ctx, [e.x, e.y]);
        let (s, _) = self.view(ctx);
        match e.phase {
            toy_kit::Phase::Down => {
                // every finger is a push of its own (a stale one with the same id is replaced)
                self.hands.retain(|h| h.id != e.id);
                self.hands.push(Hand { id: e.id, at: w, travel: 0.0, gained: 0.0, ticks: 0 });
            }
            toy_kit::Phase::Move => {
                let Some(k) = self.hands.iter().position(|h| h.id == e.id) else { return };
                let h = &mut self.hands[k];
                let d = [w[0] - h.at[0], w[1] - h.at[1]];
                h.travel += dist(w, h.at) * s;
                h.at = w;
                // the finger is a brush about a fingertip wide: whatever river it passes through is
                // carried along, as a string is through wet sand
                let gained = self.river.push(w, d, BRUSH_PT / s);
                if gained.abs() > 1.0 {
                    self.pushed = self.t;
                }
                let h = &mut self.hands[k];
                h.gained += gained.max(0.0);
                // the river stretching: a rising tick every few hundred metres this finger gains it, heard
                // where the finger is; with several fingers, a note at most every 70 ms, so they don't stutter
                let mut strike = None;
                while h.gained > (h.ticks + 1) as f64 * TICK_M && h.ticks < 24 {
                    h.ticks += 1;
                    strike = Some(h.ticks);
                }
                if let Some(n) = strike {
                    if self.t - self.ticked > 0.07 {
                        self.ticked = self.t;
                        let f = toy_kit::sound::pentatonic(392.0, n - 1);
                        let pan = ((e.x / ctx.points()[0].max(1.0)) * 2.0 - 1.0).clamp(-1.0, 1.0) as f32 * 0.5;
                        self.bells.strike(Note { freq: f, amp: 0.07, decay: 0.35, pan, bright: 0.8 });
                        ctx.haptic();
                    }
                }
            }
            toy_kit::Phase::Up | toy_kit::Phase::Cancel => {
                let Some(k) = self.hands.iter().position(|h| h.id == e.id) else { return };
                let h = self.hands.remove(k);
                if h.travel < 12.0 && e.phase == toy_kit::Phase::Up {
                    // a touch: the nearest bank nudged toward the finger and loosened, so a bend starts there
                    self.river.nudge(w, W * 1.2);
                    self.river.kick(w, 1.2);
                    self.ripples.push(Ripple { at: w, t: self.t, strong: false });
                    if self.river.distance(w) < W * 6.0 {
                        self.bells.strike(Note { freq: 196.0, amp: 0.08, decay: 0.8, pan: 0.0, bright: 0.4 });
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        self.frame += 1;
        // the river doesn't move while a new course is being drawn over it
        // the clock runs fast while bends grow and slows as a neck on screen closes, so each cut is
        // seen: from about twenty years a second down to six
        let b = self.view_bounds(ctx);
        let closing = self
            .river
            .necks()
            .iter()
            .filter(|(at, _)| at[0] > b[0] && at[0] < b[2] && at[1] > b[1] && at[1] < b[3])
            .map(|n| n.1)
            .fold(0.0f64, f64::max);
        let target = FAST_YEARS + (YEARS_A_SECOND - FAST_YEARS) * smooth01(0.0, 0.5, closing) as f64;
        let since_cut = self.flashes.last().map_or(9.0, |f| t - f.0);
        let target = if since_cut < 1.5 { YEARS_A_SECOND } else { target };
        let ease = 1.0 - (-dt * 2.0).exp();
        self.pace += (target - self.pace) * ease;
        // while a push is fresh the rest of the river holds still-ish, so the next cut is the finger's
        self.river.calm = 0.85 * (1.0 - smooth01(5.0, 11.0, t - self.pushed) as f64);
        let years = dt * self.pace;
        self.river.advance(years);
        if self.river.year - self.epoch > 30000.0 {
            let shift = 20000.0;
            self.epoch += shift;
            for (c, d) in self.dep.iter_mut().enumerate() {
                if *d > NEVER {
                    *d = (*d - shift as f32).max(-60000.0);
                    self.half[c * 2] = f16(*d);
                }
            }
            self.dirty.fill(true);
        }
        self.stamp();
        // a neck cut: a flash where it broke, a soft low note, a tap in the hand
        while self.cuts_seen < self.river.cuts.len() {
            let (_, at) = self.river.cuts[self.cuts_seen];
            self.cuts_seen += 1;
            if at[0] > b[0] && at[0] < b[2] && at[1] > b[1] && at[1] < b[3] {
                if std::env::var("MEANDER_CUTS").is_ok() {
                    eprintln!("meander: cut on screen at t={t:.1}s");
                }
                self.flashes.push((t, at));
                ctx.haptic();
                let pan = (((at[0] - b[0]) / (b[2] - b[0])) * 2.0 - 1.0) as f32;
                let mut s = self.frame ^ 0x77;
                let f = [196.0, 220.0, 261.6, 293.7, 329.6][(crate::sim::rand(&mut s) * 5.0) as usize % 5];
                self.bells.strike(Note { freq: f, amp: 0.22, decay: 3.5, pan: pan * 0.6, bright: 0.6 });
                self.bells.strike(Note { freq: f * 0.5, amp: 0.12, decay: 4.0, pan: pan * 0.6, bright: 0.1 });
            }
        }
        self.ripples.retain(|r| t - r.t < 2.0);
        self.flashes.retain(|f| t - f.0 < 0.9);
        // the lakes age out
        let year = self.river.year;
        self.river.oxbows.retain(|o| year - o.born < if o.course { COURSE_LIFE } else { LAKE_LIFE });
        // the flecks ride the current downstream
        let len = self.river.length().max(1.0);
        let speed = if self.reduce_motion { 0.0 } else { 900.0 }; // metres a second, on screen
        for f in self.flecks.iter_mut() {
            f.0 = (f.0 + speed * dt / len) % 1.0;
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let (s, c) = self.view(ctx);
        let ppp = ctx.ppp;
        // the deposits, as half floats: only the rows the river touched since the last frame
        // ((year, 1) where the river has been, (0, 0) where it never has: filtered, r/g is the year
        // nearby and g a soft edge)
        // each run of changed tiles along a band of tiles is one copy, read in place from the grid
        let mut sent = 0usize;
        for ty in 0..TILES {
            let mut tx = 0;
            while tx < TILES {
                if !self.dirty[ty * TILES + tx] {
                    tx += 1;
                    continue;
                }
                let t0 = tx;
                while tx < TILES && self.dirty[ty * TILES + tx] {
                    self.dirty[ty * TILES + tx] = false;
                    tx += 1;
                }
                let (x0, y0, w, h) = (t0 * TILE, ty * TILE, (tx - t0) * TILE, TILE);
                let start = (y0 * NG + x0) * 2;
                let end = ((y0 + h - 1) * NG + x0 + w) * 2;
                gpu.queue.write_texture(
                    wgpu::TexelCopyTextureInfo { texture: &self.dep_tex, mip_level: 0, origin: wgpu::Origin3d { x: x0 as u32, y: y0 as u32, z: 0 }, aspect: wgpu::TextureAspect::All },
                    bytemuck::cast_slice(&self.half[start..end]),
                    wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(NG as u32 * 4), rows_per_image: Some(h as u32) },
                    wgpu::Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
                );
                sent += w * h * 4;
            }
        }
        if sent > 0 && std::env::var("MEANDER_UPLOAD").is_ok() {
            eprintln!("upload {sent} t {:.2}", self.t);
        }
        let wpx = (W * s * ppp) as f32; // the channel's width in pixels
        let u = Uni {
            size: [ctx.width as f32, ctx.height as f32],
            centre: [c[0] as f32, c[1] as f32],
            scale: (s * ppp) as f32,
            now: (self.river.year - self.epoch) as f32,
            side: SIDE as f32,
            ppp: ppp as f32,
            river: wpx,
            pad: [0.0; 3],
        };
        gpu.queue.write_buffer(&self.uni, 0, bytemuck::bytes_of(&u));

        let [wp, hp] = ctx.points();
        let px = move |w: [f64; 2]| -> [f32; 2] { [(((w[0] - c[0]) * s + wp * 0.5) * ppp) as f32, (((w[1] - c[1]) * s + hp * 0.5) * ppp) as f32] };
        let year = self.river.year;
        // the mask, half the screen's pixels
        let msize = [(ctx.width / 2).max(1), (ctx.height / 2).max(1)];
        if self.mask.as_ref().map_or(true, |m| m.1 != msize) {
            let tex = gpu.texture(&wgpu::TextureDescriptor {
                label: Some("water"),
                size: wgpu::Extent3d { width: msize[0], height: msize[1], depth_or_array_layers: 1 },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
            let view = tex.create_view(&Default::default());
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("meander"),
                layout: &self.bgl,
                entries: &[
                    wgpu::BindGroupEntry { binding: 0, resource: self.uni.as_entire_binding() },
                    wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::TextureView(&self._plain.view) },
                    wgpu::BindGroupEntry { binding: 2, resource: wgpu::BindingResource::TextureView(&self.dep_view) },
                    wgpu::BindGroupEntry { binding: 3, resource: wgpu::BindingResource::Sampler(&self.smp) },
                    wgpu::BindGroupEntry { binding: 4, resource: wgpu::BindingResource::TextureView(&view) },
                ],
            });
            self.mask = Some((tex, msize, bind));
        }
        let mk = 0.5f32; // mask pixels a screen pixel
        let mp = move |q: [f32; 2]| [q[0] * mk, q[1] * mk];
        let p = &mut self.mask_painter;
        // the lakes: the loop's water clears to black over a few decades, its ends silt shut, then
        // it narrows to a marsh and the forest closes over it
        for o in &self.river.oxbows {
            let age = year - o.born;
            let life = if o.course { COURSE_LIFE } else { LAKE_LIFE };
            let n = o.line.len();
            if n < 5 {
                continue;
            }
            // the ends silt first (the plugs), then the lake shrinks toward its middle as it fills
            let silt = (0.08 + 0.37 * smooth01(0.0, life, age) as f64).min(0.45);
            let cut = ((n as f64) * silt) as usize;
            let (a, b) = (cut.min(n / 2 - 1), (n - cut).max(n / 2 + 1));
            let clear = smooth01(0.0, if o.course { 6.0 } else { 1.5 }, age);
            let marsh = if o.course { smooth01(4.0, 18.0, age) } else { smooth01(life * 0.45, life * 0.9, age) };
            let fade = 1.0 - if o.course { smooth01(8.0, life, age) } else { smooth01(life * 0.55, life, age) };
            let thin = if o.course { (smooth01(0.0, 14.0, age) * 1.6).min(0.92) } else { 0.15 * smooth01(0.0, 30.0, age) + 0.3 * smooth01(life * 0.3, life, age) };
            let width = wpx * 0.5 * (1.0 - thin);
            let span = (b - a).max(2) as f32;
            for i in a..b - 1 {
                // tapering to points at both ends, like the real crescents
                let e = ((i - a) as f32).min((b - 1 - i) as f32) / (span * 0.22);
                let taper = e.clamp(0.0, 1.0).sqrt();
                // opaque in the mask (no seams where pieces overlap): r the river's water still in
                // it, b the lake's darkness, both fading as it silts
                let f = fade * (1.0 - 0.55 * marsh);
                p.line(mp(px(o.line[i])), mp(px(o.line[i + 1])), (width * taper * mk).max(0.5), 1.6 * ppp as f32 * mk, [(1.0 - clear) * f, 0.0, clear * f, 1.0]);
            }
        }
        // the river: a pale bank of wet sand, then the water; a new course swells from a thread
        let swell = smooth01(0.0, 0.7, self.t - self.laid) * 0.75 + 0.25;
        let r = wpx * 0.5 * swell as f32 * if self.t - self.laid < 0.7 { 1.0 } else { 1.0 };
        let line = &self.river.line;
        let step = ((wpx * 0.25 / (130.0 * s * ppp) as f32).floor() as usize).max(1);
        let pts: Vec<[f32; 2]> = line.iter().step_by(step).map(|&q| px(q)).collect();
        // wider at a bend's apex, where the point bar shoals it, and paler there; a soft wet-sand edge
        let bend: Vec<f32> = (0..pts.len())
            .map(|i| {
                let (a, b, c) = (pts[i.saturating_sub(2)], pts[i], pts[(i + 2).min(pts.len() - 1)]);
                let (u, v) = ([b[0] - a[0], b[1] - a[1]], [c[0] - b[0], c[1] - b[1]]);
                let (lu, lv) = ((u[0] * u[0] + u[1] * u[1]).sqrt(), (v[0] * v[0] + v[1] * v[1]).sqrt());
                if lu < 1e-3 || lv < 1e-3 {
                    return 0.0;
                }
                let turn = ((u[0] * v[1] - u[1] * v[0]) / (lu * lv)).abs().asin().abs();
                // radians over the two steps' length, against a channel width: 0 straight, 1 a tight bend
                (turn / ((lu + lv) * 0.5) * wpx * 1.6).min(1.0)
            })
            .collect();
        let rw = |i: usize| r * (0.85 + 0.5 * bend[i]);
        for (i, w) in pts.windows(2).enumerate() {
            let k = (bend[i] + bend[i + 1]) * 0.5;
            p.line(mp(w[0]), mp(w[1]), (rw(i) + rw(i + 1)) * 0.5 * mk + 2.0 * ppp as f32 * mk, wpx * 0.3 * mk, [1.0, k * 0.55, 0.0, 1.0]);
        }
        let p = &mut self.mask_painter;
        // a cut: the new short channel through the neck flashes as the river takes it
        for &(t0, at) in &self.flashes {
            let k = (1.0 - (self.t - t0) / 0.9).clamp(0.0, 1.0) as f32;
            let k = k * k;
            // a pulse of light running out along the river both ways from the break
            let front = (self.t - t0) * 2600.0;
            for w in line.windows(2) {
                let d = dist(w[0], at);
                let f = (1.0 - ((d - front) / 500.0).powi(2)).max(0.0) as f32 + if d < 300.0 { 0.6 } else { 0.0 };
                if f > 0.0 {
                    // into the mask's g, as light in the water itself (opaque there: no seams)
                    p.line(mp(px(w[0])), mp(px(w[1])), r * 0.5 * mk, wpx * 0.25 * mk, [1.0, 0.6 + 0.4 * k * f.min(1.0), 0.0, 1.0]);
                }
            }
        }
        let p = &mut self.painter;
        // the current: faint light flecks drifting down the middle
        let len = self.river.length();
        if len > 0.0 {
            let mut acc = vec![0.0; line.len()];
            for i in 1..line.len() {
                acc[i] = acc[i - 1] + dist(line[i], line[i - 1]);
            }
            for &(f, k) in &self.flecks {
                let at = f * len;
                let i = acc.partition_point(|&a| a < at).clamp(1, line.len() - 1);
                let t0 = ((at - acc[i - 1]) / (acc[i] - acc[i - 1]).max(1e-6)).clamp(0.0, 1.0);
                let q = [line[i - 1][0] + (line[i][0] - line[i - 1][0]) * t0, line[i - 1][1] + (line[i][1] - line[i - 1][1]) * t0];
                let d = [line[i][0] - line[i - 1][0], line[i][1] - line[i - 1][1]];
                let dl = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-6);
                let off = (k - 0.5) * W * 0.5;
                let q = [q[0] - d[1] / dl * off, q[1] + d[0] / dl * off];
                let tail = [q[0] - d[0] / dl * W * 0.6, q[1] - d[1] / dl * W * 0.6];
                p.glow(px(tail), px(q), 0.6 * ppp as f32, 1.0, [0.22, 0.20, 0.16, 0.0]);
            }
        }
        // rings: where a neck broke (bright, wide), or where a finger touched
        for rp in &self.ripples {
            let a = ((self.t - rp.t) / 2.0).clamp(0.0, 1.0) as f32;
            let c = px(rp.at);
            let (r0, r1, i) = if rp.strong { (0.5, 7.0, 0.7) } else { (0.3, 2.5, 0.45) };
            let rad = wpx * (r0 + (r1 - r0) * (1.0 - (1.0 - a) * (1.0 - a)));
            p.glow_ring(c, rad, (1.5 * ppp) as f32, 2.0 * ppp as f32, [1.0, 0.95, 0.85, i * (1.0 - a)]);
        }

        let mut enc = gpu.device.create_command_encoder(&Default::default());
        let (mtex, msize, _) = self.mask.as_ref().unwrap();
        let mview = mtex.create_view(&Default::default());
        self.mask_painter.render(gpu, &mut enc, &mview, *msize, Some([0.0, 0.0, 0.0, 0.0]));
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("meander"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rp.set_pipeline(&self.pipeline);
            rp.set_bind_group(0, &self.mask.as_ref().unwrap().2, &[]);
            rp.draw(0..3, 0..1);
        }
        self.painter.render(gpu, &mut enc, target, [ctx.width, ctx.height], None);
        gpu.queue.submit([enc.finish()]);
    }

    fn demo(&self, ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        let [wp, hp] = ctx.points();
        let path: Vec<[f64; 2]> = (0..=16)
            .map(|i| {
                let f = i as f64 / 16.0;
                [wp * (0.1 + 0.8 * f), hp * (0.5 + 0.03 * (f * 5.0).sin())]
            })
            .collect();
        Some(toy_kit::info::Demo::new("Push the river with your finger.", vec![toy_kit::info::Stroke { start: 0.4, seconds: 1.2, path }]))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::new(
            &[
                "The river cuts the outside of each bend and leaves sand inside, so bends grow. When a loop's neck closes, the river cuts through and leaves the loop as a lake. A bank you push stays soft for a while, so the bend you made keeps growing.",
                "The dark crescents underneath are the Rio Beni's real old loops (Bolivia, 2024). Time runs about twenty years a second, slowing as a neck closes.",
            ],
            "Real: the floodplain and its lakes (Sentinel-2); the bend law (Howard and Knutson, 1984). Drawn: the water's colours, the pace, the finger's push.",
        ))
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        Vec::new()
    }
}


toy_kit::ios_shell!(Meander);
