//! Nazaré (a working title; painting-company row 217's family, made local): the Atlantic swell over the
//! real floor off Nazaré, Portugal, where a submarine canyon runs almost to the beach. Over the canyon
//! the water is deep and the swell runs on; on the shelf beside it the swell slows; the crests bend round
//! the canyon's head and meet off Praia do Norte, where they stack into the tallest waves surfed (sea.rs).
//! A drag turns the swell (sideways) or lengthens it (up and down), and the peak moves with it.

pub mod sea;
mod sound;

use sea::{Field, Floor};
use serde::Deserialize;
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::gpu::textures::{Pixels, Texture};
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Phase, PointerEvent, Toy};

/// Where the toy is: Nazaré's headland (lon, lat).
/// Sea texels' alpha at the deepest (land's start at 128).
const SEA_A: f64 = 110.0;

pub const PLACE: (f64, f64) = (-9.0850, 39.6040);

/// Seconds of sea a second on the screen.
const RATE: f64 = 6.0;
/// A set: this many waves' periods from one group of big waves to the next.
const GROUP: f64 = 9.0;
/// The swell it opens on: a long winter groundswell from the west-north-west, as on Nazaré's big days.
const PERIOD: f64 = 17.0;
const FROM_DEG: f64 = 290.0;
/// The swells a drag can reach: periods (s) and directions (degrees from north).
const PERIODS: (f64, f64) = (8.0, 22.0);
const FROMS: (f64, f64) = (225.0, 345.0);
/// The height drawn at full brightness (m).
const H_FULL: f32 = 24.0;
/// How much of the floor the screen shows, top to bottom (cells): the canyon's last 15 km and the coast
/// either side of it, the open sea cut at the left.
const SHOWN_ROWS: f32 = 500.0;
/// How long after a finger asks for a new swell the sea takes it up (s): longer than a solve takes.
const SOLVE_S: f64 = 0.35;

#[derive(Deserialize)]
struct Bounds {
    west: f64,
    east: f64,
    south: f64,
    north: f64,
    w: usize,
    h: usize,
    cell_m: f64,
    places: Places,
}

#[derive(Deserialize)]
struct Places {
    headland: Place,
    town: Place,
    praia_do_norte: Place,
}

#[derive(Deserialize)]
struct Place {
    cell: [f32; 2],
}

#[derive(Clone, Copy)]
struct View {
    k: f32,
    off: [f32; 2],
}

impl View {
    /// The floor filling the screen about Praia do Norte: the beach a little right of the middle (the
    /// canyon and the open sea to its left), the land's edge kept off the screen where the floor allows.
    fn of(ctx: &Ctx, size: [usize; 2], focus: [f32; 2]) -> View {
        let (w, h) = (ctx.width as f32, ctx.height as f32);
        let (fw, fh) = (size[0] as f32, size[1] as f32);
        let k = (w / fw).max(h / fh.min(SHOWN_ROWS));
        let x = (0.82 * w - focus[0] * k).clamp(w - k * fw, 0.0);
        let y = (0.45 * h - focus[1] * k).clamp(h - k * fh, 0.0);
        View { k, off: [x, y] }
    }

    fn px(&self, c: [f32; 2]) -> [f32; 2] {
        [self.off[0] + c[0] * self.k, self.off[1] + c[1] * self.k]
    }
}

pub struct Nazare {
    backdrop: Backdrop,
    tex: Texture,
    painter: Painter,
    floor: std::sync::Arc<Floor>,
    bounds: Bounds,
    /// each land cell's light (a hillshade), and the field drawn into the texture
    shade: std::sync::Arc<Vec<u8>>,
    field: Field,
    /// the swell a finger has asked for, and its solve under way (taken up at the time given)
    want: (f64, f64),
    pending: Option<(Solving, f64)>,
    /// where the waves break at Praia do Norte (the tallest of them off the beach), and the crest's
    /// place in its cycle there a frame ago
    peak: Option<usize>,
    last_w: f64,
    /// the spray of the big waves broken there: each its age (s) and how big
    spray: Vec<(f64, f32)>,
    sound: sound::Queue,
    /// a finger's last place (points)
    drag: Option<[f64; 2]>,
    t: f64,
    clock: f64,
    seen: Option<(View, f32)>,
}

/// A swell being solved on its own thread: its field and its texels.
type Solving = std::thread::JoinHandle<(Field, Vec<u8>)>;

const SEA_WGSL: &str = r#"
// u.p0 = [the floor's left, top (px), px a cell, the swell's phase now]; u.p1 = [cells across, down,
// the sets' phase now, 0]. tex: the field's two halves side by side (Nazare::texels).
const TAU: f32 = 6.2831853;
fn backdrop(px: vec2<f32>, world: vec2<f32>) -> vec4<f32> {
  let cell = (px - u.p0.xy) / u.p0.z;
  let raw = cell / u.p1.xy;
  let uv = clamp(raw, vec2(0.0), vec2(1.0));
  let half_texel = 0.5 / (2.0 * u.p1.x);
  let ux = clamp(uv.x * 0.5, half_texel, 0.5 - half_texel);
  let a = textureSampleLevel(tex, samp, vec2(ux, uv.y), 0.0);
  let b = textureSampleLevel(tex, samp, vec2(ux + 0.5, uv.y), 0.0);
  // how many crests a pixel spans: where they crowd past the screen's grain, they fade to their mean
  let dense = length(fwidth(a.rg)) / 3.1416;
  let clear = 1.0 - smoothstep(0.08, 0.2, dense);
  let off = any(raw != uv);
  // the dusk: the sun just down to the west (left), its warmth on the sea and the land's western faces
  let west = clamp(1.0 - px.x / u.size.x, 0.0, 1.0);
  let s = select((a.a - 0.5) * 2.0, 0.0, off);
  let ground = (vec3(0.040, 0.040, 0.036) + vec3(0.13, 0.09, 0.055) * s * s) * (0.92 + 0.16 * noise2(px * 0.08));
  // land and sea meet over a pixel or two, not on the texels' steps
  let landness = select(smoothstep(0.47, 0.53, a.a + 0.6 * fwidth(a.a)), 1.0, off);
  let d = min(a.a, 0.431) / 0.431;
  let deep = clamp(d, 0.0, 1.0);
  // the sea: grey-green over the shelf, darker where it is deep (the canyon shows only as darker
  // water), warmer toward the afterglow
  var base = mix(vec3(0.070, 0.130, 0.150), vec3(0.018, 0.045, 0.080), deep);
  base += vec3(0.07, 0.035, 0.0) * west * west;
  let hn = a.b;
  let focus = b.a * 2.5;
  let ph = atan2(a.g - 0.5, a.r - 0.5) / TAU;
  let w = fract(ph - u.p0.w);
  let q = 0.5 + 0.5 * cos(TAU * w);
  let gph = atan2(b.g - 0.5, b.r - 0.5) / TAU;
  let sets = 0.15 + 0.85 * pow(0.5 + 0.5 * cos(TAU * (gph - u.p1.z)), 3.0);
  // the waves' height against the open sea's (H_FULL / sea::H0 = 4): what shows is its energy, the square
  let rel = hn * 4.0 * sets;
  let e = clamp(rel * rel * 0.16, 0.0, 1.6);
  // each wave's body, then its crest: sharper as the water shoals, lit by the low sun on its back
  let sharp = 6.0 + 14.0 * (1.0 - deep);
  let crest = pow(q, sharp) * clear;
  // the wave's body: its back, toward the afterglow, lit; its front in shadow; more so the taller it is
  let slope = sin(TAU * w);
  let body = 0.85 + clear * (0.30 * slope + 0.12 * q) * min(rel, 3.0);
  let sun = mix(vec3(0.70, 0.80, 0.82), vec3(1.0, 0.78, 0.52), west * 0.7);
  var col = base * body + sun * crest * e;
  // near breaking where the swell is focused, the face stands up into a wall, thick and bright
  let near = smoothstep(0.55, 0.95, b.b);
  let wall = near * smoothstep(1.2, 2.2, focus) * sets;
  col += vec3(0.85, 0.92, 0.92) * pow(q, 2.0) * wall * 0.6 * clear;
  // breaking: white water where the wave is as tall as the water is deep, rolling in behind its crest,
  // as much as the swell is focused there (Praia do Norte white, the sheltered bay barely)
  let breaking = smoothstep(0.80, 1.0, b.b) * (0.6 + 0.4 * noise2(px * 0.05));
  let behind = pow(w, 4.0);
  let lace = noise2(px * 0.07 + vec2(u.p0.w * 3.0, 0.0));
  let churn = 0.5 + 0.5 * noise2(px * 0.13 + vec2(0.0, u.p0.w * 9.0));
  let strength = clamp((0.1 + 0.3 * focus * focus) * (0.5 + 0.5 * sets), 0.05, 1.0);
  let white = clamp((0.9 * behind * churn + 0.55 * churn * lace) * strength * 1.4, 0.0, 1.0);
  col = mix(col, vec3(0.90, 0.93, 0.92), breaking * white);
  return vec4(mix(col, ground, landness), 1.0);
}
"#;

fn load_floor(bytes: &[u8], b: &Bounds) -> anyhow::Result<Floor> {
    anyhow::ensure!(
        bytes.len() == b.w * b.h * 2,
        "floor.bin is not {} x {}",
        b.w,
        b.h
    );
    let ground = bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| i16::from_le_bytes(*c) as f32 / 10.0)
        .collect();
    Ok(Floor {
        w: b.w,
        h: b.h,
        cell_m: b.cell_m,
        ground,
    })
}

/// The land's light: lit from the west-south-west, low (the sun going down over the sea).
fn hillshade(f: &Floor) -> Vec<u8> {
    let (w, h) = (f.w, f.h);
    let mut out = vec![0u8; w * h];
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let i = y * w + x;
            let gx = (f.ground[i + 1] - f.ground[i - 1]) as f64 / (2.0 * f.cell_m);
            let gy = (f.ground[i + w] - f.ground[i - w]) as f64 / (2.0 * f.cell_m);
            // facing west (−x) and a little south (+y) is lit
            let lit = (0.45 - 2.5 * gx + 0.8 * gy).clamp(0.0, 1.0);
            let high = (f.ground[i].max(0.0) as f64 / 250.0).min(1.0);
            out[i] = ((0.75 * lit + 0.25 * high) * 255.0) as u8;
        }
    }
    out
}

/// The field into the texture: the left half the phase (cos, sin of 2πτ/T), the height and the
/// depth or the land's light; the right half the sets' phase and how near the wave is to breaking.
fn texels(floor: &Floor, shade: &[u8], f: &Field) -> Vec<u8> {
    let (w, h) = (floor.w, floor.h);
    let tg = f.period * GROUP;
    let mut data = vec![0u8; 2 * w * h * 4];
    let unit = |v: f64| (127.5 + 127.0 * v) as u8;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let l = (y * 2 * w + x) * 4;
            let r = l + w * 4;
            if !floor.sea(i) {
                data[l + 3] = 128 + shade[i] / 2;
                // land's focus reads as open water's, so the filter bleeds nothing brighter into the shore
                data[r + 3] = (255.0 / sea::REFRACT_MAX) as u8;
                continue;
            }
            let depth = floor.depth(i).min(800.0);
            // well below land's 0.47, so the deepest sea never blends toward ground
            data[l + 3] = (SEA_A * (depth / 800.0).sqrt()) as u8;
            let tau = f.tau[i];
            if tau.is_finite() {
                let a = std::f64::consts::TAU * tau / f.period;
                data[l] = unit(a.cos());
                data[l + 1] = unit(a.sin());
                let hh = f.height[i];
                data[l + 2] = ((hh / H_FULL).min(1.0) * 255.0) as u8;
                let near = hh as f64 / (sea::BREAK * floor.depth(i)).max(0.01);
                data[r + 2] = (near.min(1.0) * 255.0) as u8;
                data[r + 3] = (f.focus[i] / sea::REFRACT_MAX as f32 * 255.0) as u8;
            } else {
                data[l] = 128;
                data[l + 1] = 128;
            }
            let tg_i = f.tau_group[i];
            if tg_i.is_finite() {
                let a = std::f64::consts::TAU * tg_i / tg;
                data[r] = unit(a.cos());
                data[r + 1] = unit(a.sin());
            } else {
                data[r] = 128;
                data[r + 1] = 128;
            }
        }
    }
    data
}

impl Nazare {
    fn view(&self, ctx: &Ctx) -> View {
        View::of(
            ctx,
            [self.floor.w, self.floor.h],
            self.bounds.places.praia_do_norte.cell,
        )
    }

    fn upload(&self, gpu: &toy_kit::Gpu, data: &[u8]) {
        let (w, h) = (self.floor.w, self.floor.h);
        gpu.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.tex.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(2 * w as u32 * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 2 * w as u32,
                height: h as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    /// A new swell asked for: solved on its own thread, taken up a set time later (so a slow solve
    /// never stalls a frame, and the headless check, which waits for it then, sees the same frames
    /// every run).
    fn ask(&mut self) {
        let floor = std::sync::Arc::clone(&self.floor);
        let shade = std::sync::Arc::clone(&self.shade);
        let (period, from) = self.want;
        let job = std::thread::spawn(move || {
            let field = Field::new(&floor, period, from);
            let data = texels(&floor, &shade, &field);
            (field, data)
        });
        self.pending = Some((job, self.clock + SOLVE_S));
    }

    /// The tallest wave off Praia do Norte: within a kilometre or so seaward of the beach.
    fn find_peak(&self) -> Option<usize> {
        let (w, h) = (self.floor.w as i64, self.floor.h as i64);
        let c = self.bounds.places.praia_do_norte.cell;
        let (cx, cy) = (c[0] as i64, c[1] as i64);
        let mut best: Option<(usize, f32)> = None;
        for y in (cy - 16).max(0)..(cy + 16).min(h) {
            for x in (cx - 30).max(0)..(cx + 4).min(w) {
                let i = (y * w + x) as usize;
                let hh = self.field.height[i];
                // the wave the canyon bends there, not merely the shallowest water's cap
                let hh = hh * self.field.focus[i];
                if self.floor.sea(i)
                    && self.field.tau[i].is_finite()
                    && best.is_none_or(|(_, b)| hh > b)
                {
                    best = Some((i, hh));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// The surf's sound: a crash each time a crest reaches the peak, as loud as the wave is tall.
    fn listen(&mut self, dt: f64) {
        for s in &mut self.spray {
            s.0 += dt;
        }
        self.spray.retain(|s| s.0 < 6.0);
        let Some(i) = self.peak else {
            return;
        };
        let f = &self.field;
        let w = (f.tau[i] / f.period - self.t / f.period).rem_euclid(1.0);
        let crest = self.last_w < 0.25 && w > 0.75;
        self.last_w = w;
        let Ok(mut q) = self.sound.lock() else {
            return;
        };
        q.roar = 0.12;
        if crest {
            let tg = f.period * GROUP;
            let g = (f.tau_group[i] / tg - self.t / tg).rem_euclid(1.0);
            let c = 0.5 + 0.5 * (std::f64::consts::TAU * g).cos();
            let sets = 0.15 + 0.85 * c * c * c;
            let rel = f.height[i] as f64 / sea::H0;
            let amp = (rel / 2.5 * sets).clamp(0.05, 1.0);
            q.crashes.push(amp);
            if amp > 0.3 {
                self.spray.push((0.0, amp as f32));
            }
        }
    }

    /// A finger's drag: sideways turns where the swell comes from, up and down its period.
    fn turn(&mut self, d: [f64; 2]) {
        let (period, from) = self.want;
        let from = (from + d[0] * 0.3).clamp(FROMS.0, FROMS.1);
        let period = (period - d[1] * 0.04).clamp(PERIODS.0, PERIODS.1);
        self.want = (period, from);
    }

    /// The spray where a set's wave breaks at the peak: soft white thrown up and drifting landward
    /// on the wind off the sea, lingering a few seconds.
    fn draw_spray(&mut self, v: &View, ppp: f32) {
        let Some(i) = self.peak else {
            return;
        };
        let c = [(i % self.floor.w) as f32, (i / self.floor.w) as f32];
        for (k, &(age, amp)) in self.spray.iter().enumerate() {
            let age = age as f32;
            let grow = 1.0 - (-age / 0.6).exp();
            let fade = (-age / 1.6).exp() * grow;
            for j in 0..14 {
                let h = ((k * 31 + j * 17) % 97) as f32 / 97.0;
                // along the beach northward from the peak, thrown up and blown a little landward
                let along = -(j as f32) * 2.2 * (0.5 + amp) + (h - 0.5) * 3.0;
                let off = [
                    c[0] - 3.0 + age * 1.2 + h * 3.0 + along * -0.25,
                    c[1] + along,
                ];
                let r = (6.0 + 26.0 * grow * amp * (0.5 + 0.5 * h)) * ppp;
                let at = v.px(off);
                let fall = 1.0 - j as f32 / 16.0;
                self.painter.glow(
                    at,
                    at,
                    r * 0.2,
                    r,
                    [0.90, 0.94, 0.95, 0.3 * fade * amp * fall],
                );
            }
        }
    }

    /// The town's and the lighthouse's lights.
    fn draw_lights(&mut self, v: &View, ppp: f32) {
        let town = self.bounds.places.town.cell;
        let head = self.bounds.places.headland.cell;
        let warm = [1.0, 0.72, 0.38];
        let mut s = 0x5eedu64;
        for _ in 0..40 {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            let a = (s % 1000) as f32 / 1000.0 * std::f32::consts::TAU;
            let r = ((s >> 10) % 1000) as f32 / 1000.0;
            let c = [
                town[0] + 4.0 + a.cos() * r * 14.0,
                town[1] + a.sin() * r * 22.0,
            ];
            let i = (c[1].max(0.0) as usize).min(self.floor.h - 1) * self.floor.w
                + (c[0].max(0.0) as usize).min(self.floor.w - 1);
            if self.floor.sea(i) {
                continue;
            }
            let b = 0.35 + 0.5 * ((s >> 20) % 100) as f32 / 100.0;
            self.painter.disc(
                v.px(c),
                0.9 * ppp,
                0.9 * ppp,
                [warm[0], warm[1], warm[2], b],
            );
        }
        // the lighthouse on the fort: a white flash every 5 s (Nazaré's light is three flashes in 15 s)
        let beat = (self.clock % 5.0) as f32;
        let flash = (1.0 - beat / 0.35).max(0.0);
        let at = v.px(head);
        self.painter.glow(
            at,
            at,
            (1.5 + 3.0 * flash) * ppp,
            (3.0 + 16.0 * flash) * ppp,
            [1.0, 0.95, 0.85, 0.35 + 0.65 * flash],
        );
    }
}

impl Toy for Nazare {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let bounds: Bounds = serde_json::from_slice(&ctx.assets.read("floor.json")?)?;
        let floor = load_floor(&ctx.assets.read("floor.bin")?, &bounds)?;
        let shade = hillshade(&floor);
        let field = Field::new(&floor, PERIOD, FROM_DEG);
        let blank = Pixels {
            width: 2 * floor.w as u32,
            height: floor.h as u32,
            channels: 4,
            data: vec![0; 2 * floor.w * floor.h * 4],
        };
        let tex = Texture::flat(&ctx.gpu, "nazare field", &blank);
        let backdrop = Backdrop::textured(&ctx.gpu, "nazare", SEA_WGSL, &tex.view);
        let floor = std::sync::Arc::new(floor);
        let shade = std::sync::Arc::new(shade);
        let data = texels(&floor, &shade, &field);
        let toy = Nazare {
            backdrop,
            tex,
            painter: Painter::new(&ctx.gpu),
            floor,
            bounds,
            shade,
            field,
            want: (PERIOD, FROM_DEG),
            pending: None,
            peak: None,
            last_w: 0.0,
            spray: Vec::new(),
            sound: sound::Queue::default(),
            drag: None,
            t: 0.0,
            clock: 0.0,
            seen: None,
        };
        toy.upload(&ctx.gpu, &data);
        let mut toy = toy;
        toy.peak = toy.find_peak();
        Ok(toy)
    }

    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        match e.phase {
            Phase::Down => self.drag = Some([e.x, e.y]),
            Phase::Move => {
                if let Some(last) = self.drag {
                    self.turn([e.x - last[0], e.y - last[1]]);
                    self.drag = Some([e.x, e.y]);
                }
            }
            _ => self.drag = None,
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        self.seen = Some((self.view(ctx), ctx.ppp as f32));
        let dt = dt.min(0.1);
        self.clock += dt;
        self.t += dt * RATE;
        let asked = (self.want.0 - self.field.period).abs() > 0.05
            || (self.want.1 - self.field.from_deg).abs() > 0.2;
        let due = self
            .pending
            .as_ref()
            .is_some_and(|(_, at)| self.clock >= *at);
        if due {
            if let Some((job, _)) = self.pending.take() {
                if let Ok((field, data)) = job.join() {
                    self.field = field;
                    self.upload(&ctx.gpu, &data);
                    self.peak = self.find_peak();
                    self.last_w = 0.0;
                }
            }
        }
        if asked && self.pending.is_none() {
            self.ask();
        }
        self.listen(dt);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let v = self.view(ctx);
        let ppp = ctx.ppp as f32;
        let size = [ctx.width, ctx.height];
        let period = self.field.period;
        let phase = (self.t / period).fract() as f32;
        let sets = (self.t / (period * GROUP)).fract() as f32;
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        let params = BackdropParams {
            time: self.clock as f32,
            scale: 1.0,
            center: [0.0, 0.0],
            p: [
                v.off[0],
                v.off[1],
                v.k,
                phase,
                self.floor.w as f32,
                self.floor.h as f32,
                sets,
                0.0,
            ],
        };
        self.backdrop
            .render(&ctx.gpu, &mut enc, target, size, &params);
        self.draw_spray(&v, ppp);
        self.draw_lights(&v, ppp);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(Box::new(sound::Surf::new(self.sound.clone())))
    }

    fn demo(&self, ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        let [w, h] = ctx.points();
        Some(toy_kit::info::Demo::new(
            "The swell at Nazaré. Drag to turn it.",
            vec![toy_kit::info::Stroke::drag(
                0.6,
                1.6,
                [0.25 * w, 0.30 * h],
                [0.55 * w, 0.30 * h],
            )],
        ))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::lines(&[
            "A swell slows where the sea is shallow and runs on where it is deep.",
            "Off Nazaré a canyon reaches almost to the beach. The swell bends away from it on both sides, and the two halves meet at Praia do Norte.",
            "There they stack into the tallest waves ever surfed. Drag sideways to turn the swell, up or down to lengthen it.",
        ]))
    }

    fn project(&self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        let (v, ppp) = self.seen.as_ref()?;
        let b = &self.bounds;
        let c = [
            ((lonlat[0] - b.west) / (b.east - b.west) * b.w as f64) as f32,
            ((b.north - lonlat[1]) / (b.north - b.south) * b.h as f64) as f32,
        ];
        let px = v.px(c);
        Some([(px[0] / ppp) as f64, (px[1] / ppp) as f64])
    }
}

toy_kit::ios_shell!(Nazare);
