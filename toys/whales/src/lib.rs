//! Whales (a working title; painting-company row 201): two whales of the eastern North Pacific, a
//! humpback and a blue whale, travelling their populations' real routes through the year, singing as
//! they go: their real songs (a humpback recorded off Monterey, a blue whale by NOAA), heard and seen,
//! each moment of a song a ring spreading over the sea from the singer.
//!
//! The finger: a drag turns the globe, a pinch brings it closer; a tap on a whale follows it (the
//! camera keeps it, its song comes forward); a tap on the sea lets it go. Left alone a while, the
//! camera comes back to hold both. The year runs a day a second: the whales go south in December,
//! winter off Mexico and over the Costa Rica Dome, come north in spring, and sing in their seasons.
//!
//! The data (toys/whales/SOURCES.md): `humpback.song`, `blue.song` (tools/toys/whales/recordings.py),
//! `routes.json` (tools/toys/whales/routes.py). Drawn by the toy, not measured: the rings (how a song
//! looks here), the wake, the whales' shapes and their swimming, the year's pace, and a floor under each
//! whale's season so neither is ever quite silent.

pub mod route;
pub mod sea;
pub mod song;

use route::{Route, YEAR};
use sea::Sea;
use song::{Choir, Mix, Song, MAX_DELAY};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use toy_kit::{Camera, Ctx, Globe, LonLat, Phase, PointerEvent, Synth, Toy};

/// Days of the year a second.
const DAYS_PER_SECOND: f64 = 1.0;
/// The day the toy opens on: November 20th, the humpback singing in Monterey Bay, the blue whale off
/// Baja California going south, singing too.
const FIRST_DAY: f64 = 323.0;
const MAX_RINGS: usize = 64;
const RING_LIFE: f64 = 7.0;
/// How long the camera is left where a finger put it before it comes back to hold both whales.
const IDLE_BACK: f64 = 14.0;
/// How long a hydrophone stays down on the water, and the land's silence lasts.
const EAR_LIFE: f64 = 24.0;
const SILENCE_LIFE: f64 = 4.0;
/// How many hydrophones may be down on the water at once (a further one lifts the oldest).
const MAX_EARS: usize = 4;
/// How long a hydrophone takes to come up when it is lifted early.
const EAR_LIFT: f64 = 1.2;
/// How fast each whale's rings spread, points a second (times the screen's size).
const RING_PTS: [f64; 2] = [46.0, 34.0];

#[derive(Clone, Copy, PartialEq, Debug, serde::Serialize, serde::Deserialize)]
enum Mode {
    Both,
    Follow(usize),
    Free,
}

#[derive(Clone, Copy)]
struct Ring {
    at: [f32; 3],
    born: f64,
    amp: f32,
    pitch: f32,
    kind: u32,
    /// Radians a second it spreads.
    speed: f32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RingsU {
    at: [[f32; 4]; MAX_RINGS],
    look: [[f32; 4]; MAX_RINGS],
    n: [f32; 4],
    /// The hydrophones (unit vectors) and how far down each is, 0..1 (0: none).
    ears: [[f32; 4]; MAX_EARS],
    /// Each whale's place now (the land's shadow is cast from it); w unused.
    whale: [[f32; 4]; 2],
    /// x the hydrophone's reach, radians.
    misc: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct MarkI {
    at: [f32; 4],
    look: [f32; 4],
}

const MAX_MARKS: usize = 512;

struct Singer {
    route: Route,
    song: Arc<Song>,
    /// Seconds into the song as the toy last saw it (the audio thread's, when the sound runs).
    pos: f64,
    last_ring: f64,
    loud_avg: f32,
    /// The voice's gain now (season, follow, place), for the rings too.
    gain: f32,
}

/// A touch on the sea lowers a hydrophone there: the songs are heard as they arrive at it (late,
/// quieter and muffled with distance, hardly at all behind the land), and the rings brighten as they
/// pass it. Several may be down at once: the songs are heard at the newest (or the one last touched),
/// and every one flares as a ring passes it, so a song's arrivals can be watched across the sea. A
/// touch on the land is the land's silence.
struct Ear {
    at: [f64; 3],
    born: f64,
    land: bool,
    /// A ring passing it, fading.
    ping: f32,
    /// Whether the way from each whale is open sea.
    open: [bool; 2],
    /// When it began to come up early (a fifth was lowered, or a whale was followed).
    lift: Option<f64>,
}

impl Ear {
    /// How far down it is, 0..1: lowered quickly, lifted slowly at the end of its life.
    fn strength(&self, t: f64) -> f64 {
        let age = t - self.born;
        let life = if self.land { SILENCE_LIFE } else { EAR_LIFE };
        let lift = if self.land { 0.8 } else { 1.5 };
        let early = self.lift.map(|l| ((l + EAR_LIFT - t) / EAR_LIFT).clamp(0.0, 1.0)).unwrap_or(1.0);
        (age / 0.25).clamp(0.0, 1.0) * ((life - age) / lift).clamp(0.0, 1.0) * early
    }

    fn gone(&self, t: f64) -> bool {
        self.strength(t) <= 0.0 && t - self.born > 1.0
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Saved {
    day: f64,
    mode: Mode,
    camera: Camera,
}

pub struct Whales {
    globe: Globe,
    sea: Sea,
    /// The hydrophones on the water, oldest first; the songs are heard at the last one not lifting.
    ears: Vec<Ear>,
    /// The land's silence, while it lasts.
    silence: Option<Ear>,
    /// How close the camera holds a followed whale (a pinch changes it).
    follow_zoom: f64,
    singers: [Singer; 2],
    mix: Arc<Mix>,
    choir: Option<Choir>,
    heard_frames: u64,
    day: f64,
    t: f64,
    rings: Vec<Ring>,
    mode: Mode,
    last_touch: f64,
    /// A finger that may become a tap: where and when it went down, and whether it moved.
    press: Option<(u64, [f64; 2], f64, bool)>,
    fingers: usize,
    reduce_motion: bool,
    rings_pass: Pass,
    marks_pipeline: wgpu::RenderPipeline,
    marks_buf: wgpu::Buffer,
    marks: Vec<MarkI>,
}

struct Pass {
    pipeline: wgpu::RenderPipeline,
    bind: wgpu::BindGroup,
    buf: wgpu::Buffer,
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(v: [f64; 3]) -> [f64; 3] {
    let n = dot(v, v).sqrt().max(1e-12);
    [v[0] / n, v[1] / n, v[2] / n]
}

fn slerp(a: [f64; 3], b: [f64; 3], k: f64) -> [f64; 3] {
    norm([a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k, a[2] + (b[2] - a[2]) * k])
}

impl Whales {
    fn new_passes(ctx: &Ctx, globe: &Globe) -> (Pass, wgpu::RenderPipeline, wgpu::Buffer) {
        let gpu = &ctx.gpu;
        let layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("rings"),
            entries: &[
                toy_kit::globe::ground::tex2d(0),
                toy_kit::globe::ground::sampler(1),
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer { ty: wgpu::BufferBindingType::Uniform, has_dynamic_offset: false, min_binding_size: None },
                    count: None,
                },
            ],
        });
        let buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("rings"),
            size: std::mem::size_of::<RingsU>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("rings"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&globe.textures.geo.view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&globe.textures.linear_wrap_x) },
                wgpu::BindGroupEntry { binding: 2, resource: buf.as_entire_binding() },
            ],
        });
        let shader = gpu.shader("rings", &toy_kit::globe::wgsl(include_str!("rings.wgsl")));
        let pipeline = gpu.fullscreen_pipeline("rings", &shader, &[&globe.view_layout, &layout], Some(toy_kit::Gpu::over()));

        let mshader = gpu.shader("marks", &toy_kit::globe::wgsl(include_str!("marks.wgsl")));
        let mlayout = gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("marks"),
            bind_group_layouts: &[&globe.view_layout],
            push_constant_ranges: &[],
        });
        let attrs = wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4];
        let marks_pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("marks"),
            layout: Some(&mlayout),
            vertex: wgpu::VertexState {
                module: &mshader,
                entry_point: Some("vs_mark"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<MarkI>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &attrs,
                }],
                compilation_options: Default::default(),
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &mshader,
                entry_point: Some("fs_mark"),
                targets: &[Some(wgpu::ColorTargetState { format: gpu.format, blend: Some(toy_kit::Gpu::over()), write_mask: wgpu::ColorWrites::ALL })],
                compilation_options: Default::default(),
            }),
            multiview: None,
            cache: None,
        });
        let marks_buf = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("marks"),
            size: (std::mem::size_of::<MarkI>() * MAX_MARKS) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        (Pass { pipeline, bind, buf }, marks_pipeline, marks_buf)
    }

    /// The camera that holds both whales: their middle, as close as keeps them both well inside.
    fn both_target(&self) -> ([f64; 3], f64) {
        let a = self.singers[0].route.at(self.day);
        let b = self.singers[1].route.at(self.day);
        let mid = norm([a[0] + b[0], a[1] + b[1], a[2] + b[2]]);
        let ll = LonLat::from_xyz(mid);
        let half = dot(a, b).clamp(-1.0, 1.0).acos() / 2.0;
        let disc1 = toy_kit::gpu::View::layout(self.w(), self.h(), self.ppp(), Camera::new(0.0, 0.0, 1.0));
        let [wp, hp] = [self.w() as f64 / self.ppp(), self.h() as f64 / self.ppp()];
        // which way they lie on the screen (seen from their middle), and the room that way
        let cam = Camera::new(ll.lon, ll.lat, 1.0);
        let (va, vb) = (cam.view(a), cam.view(b));
        let (dx, dy) = (vb[0] - va[0], vb[1] - va[1]);
        let l = dx.hypot(dy).max(1e-9);
        let (ux, uy) = ((dx / l).abs(), (dy / l).abs());
        let reach = disc1.r * (half + 0.05).min(1.4).sin();
        // the pair right of the middle, so the open sea (where their songs go) fills the screen and
        // the coast's lit land keeps to its corner
        let right = 0.14 * wp;
        let zx = (wp / 2.0 - right - 40.0).max(40.0) / (reach * ux).max(1e-9);
        let zy = (hp * 0.40 - 90.0).max(40.0) / (reach * uy).max(1e-9);
        let zoom = zx.min(zy).clamp(1.1, 9.0);
        // the camera's centre that far west of their middle
        let r = disc1.r * zoom;
        let west = (right / r).clamp(0.0, 0.6).asin().to_degrees();
        let centre = LonLat { lon: ll.lon - west / ll.lat.to_radians().cos().max(0.2), lat: ll.lat }.to_xyz();
        (centre, zoom)
    }

    /// The marks and the rings grow a little with the screen (a phone 1, the 13-inch iPad about 1.6).
    fn size_k(&self) -> f64 {
        let short = (self.w().min(self.h()) as f64 / self.ppp()).max(1.0);
        (short / 393.0).sqrt().clamp(1.0, 1.7)
    }

    fn w(&self) -> u32 {
        self.globe.view().width
    }
    fn h(&self) -> u32 {
        self.globe.view().height
    }
    fn ppp(&self) -> f64 {
        self.globe.view().ppp
    }

    fn whale_on_screen(&self, i: usize) -> ([f64; 2], f64) {
        self.globe.project(self.singers[i].route.lonlat(self.day))
    }

    fn steer_camera(&mut self, dt: f64) {
        if self.globe.hand.fingers() > 0 {
            return;
        }
        // left alone, the camera comes back to hold both; soon when neither whale is in sight
        let in_sight = (0..2).any(|i| {
            let ([x, y], f) = self.whale_on_screen(i);
            f > 0.0 && x > 0.0 && y > 0.0 && x < self.w() as f64 / self.ppp() && y < self.h() as f64 / self.ppp()
        });
        let idle = if in_sight { IDLE_BACK } else { 4.0 };
        if self.mode == Mode::Free && self.t - self.last_touch > idle {
            self.mode = Mode::Both;
        }
        let (target, zoom) = match self.mode {
            Mode::Free => return,
            Mode::Both => self.both_target(),
            Mode::Follow(i) => (self.singers[i].route.at(self.day), self.follow_zoom),
        };
        let cam = self.globe.camera;
        let centre = cam.world([0.0, 0.0, 1.0]);
        let k = 1.0 - (-dt * if self.reduce_motion { 0.6 } else { 1.1 }).exp();
        let c = slerp(centre, target, k);
        let ll = LonLat::from_xyz(c);
        let mut next = Camera::new(ll.lon, ll.lat, cam.zoom * (zoom / cam.zoom).powf(k));
        // the roll comes back to north up (the turn from the old camera to the new keeps any roll it had)
        next.roll = cam.roll * (1.0 - k);
        self.globe.camera = next;
    }

    #[allow(clippy::needless_range_loop)]
    fn sing(&mut self, dt: f64) {
        // where each song is: the audio thread's when the sound runs, the toy's own clock when it doesn't
        let frames = self.mix.frames.load(Ordering::Relaxed);
        let sounding = frames != self.heard_frames;
        self.heard_frames = frames;
        let disc_r = self.globe.disc().r;
        let w = self.w() as f64 / self.ppp();
        for i in 0..2 {
            let s = &mut self.singers[i];
            s.pos = if sounding { self.mix.pos(i) } else { (s.pos + dt).rem_euclid(s.song.seconds()) };
        }
        for i in 0..2 {
            let season = self.singers[i].route.sings(self.day).max(0.12);
            let follow = match self.mode {
                Mode::Follow(j) if j == i => 1.0,
                Mode::Follow(_) => 0.3,
                _ => 0.75,
            };
            let ([x, _], facing) = self.whale_on_screen(i);
            let place = if facing > 0.0 { 1.0 } else { 0.55 };
            let gain = season * follow * place;
            let pan = (((x / w) * 2.0 - 1.0) * 0.7).clamp(-0.8, 0.8) as f32;
            // the blue whale's sped-up moan is the loudest thing in its band: it sits under the humpback
            let level = if i == 1 { 0.72 } else { 1.15 };
            let (mut g, mut p, mut delay, mut corner) = (gain * level, pan, 0.0f32, 20_000.0f32);
            if let Some(ear) = self.listening() {
                let k = ear.strength(self.t) as f32;
                {
                    // as it arrives at the hydrophone: when its ring does, as faint as its ring is then,
                    // muffled the farther it came, hardly at all from behind the land
                    let wpos = self.singers[i].route.at(self.day);
                    let ang = dot(wpos, ear.at).clamp(-1.0, 1.0).acos();
                    let speed = RING_PTS[i] * self.size_k() / disc_r;
                    let arrive = (ang / speed).min(MAX_DELAY - 0.2);
                    let reach = (1.0 - arrive / RING_LIFE).max(0.0).powf(1.6);
                    let mut heard = (0.45 + 0.85 * reach) as f32;
                    let mut c = 900.0 + 5000.0 * reach as f32;
                    if !ear.open[i] {
                        heard *= 0.15;
                        c = 300.0;
                    }
                    let ([ex, _], _) = self.globe.project(LonLat::from_xyz(ear.at));
                    let ep = (((x - ex) / (w * 0.5)) * 0.8).clamp(-0.85, 0.85) as f32;
                    g = g + (season * place * level * heard - g) * k;
                    p = p + (ep - p) * k;
                    delay = if k > 0.3 { arrive as f32 } else { 0.0 };
                    corner = 20_000.0 + (c - 20_000.0) * k;
                }
            }
            if let Some(sil) = &self.silence {
                // the land's silence
                g *= 1.0 - 0.97 * sil.strength(self.t) as f32;
            }
            if std::env::var("WHALES_DEBUG").is_ok() && (self.t * 30.0).round() as i64 % 30 == 0 {
                eprintln!("  voice {i}: gain {g:.3} pan {p:.2} delay {delay:.2} corner {corner:.0} open {:?}", self.listening().map(|e| e.open));
            }
            self.mix.set(i, g, p);
            self.mix.place(i, delay, corner);
            self.singers[i].gain = gain;

            // a ring for each rise in the song, and one every so often while it holds loud
            let s = &mut self.singers[i];
            let [loud, pitch] = s.song.env_at(s.pos);
            let rise = loud - s.loud_avg;
            s.loud_avg += (loud - s.loud_avg) * (1.0 - (-dt / 0.35).exp() as f32);
            let since = self.t - s.last_ring;
            let min_gap = if i == 0 { 0.2 } else { 0.35 };
            let ring = (rise > 0.10 && loud > 0.25 && since > min_gap) || (loud > 0.45 && since > if i == 0 { 0.7 } else { 1.1 });
            if ring && gain > 0.02 {
                s.last_ring = self.t;
                let at = s.route.at(self.day);
                // the blue whale's low song spreads slower and wider, the humpback's quicker and finer
                let pts_per_s = RING_PTS[i] * self.size_k();
                let r = Ring {
                    at: [at[0] as f32, at[1] as f32, at[2] as f32],
                    born: self.t,
                    amp: (loud * (0.35 + 0.65 * (gain / 0.75).min(1.0))).min(1.0),
                    pitch,
                    kind: i as u32,
                    speed: (pts_per_s / disc_r) as f32,
                };
                if self.rings.len() >= MAX_RINGS {
                    self.rings.remove(0);
                }
                self.rings.push(r);
            }
        }
        let t = self.t;
        self.rings.retain(|r| t - r.born < RING_LIFE);
        // the hydrophones: which ways are open, a ping as a ring passes over each, lifted at their end
        for ear in &mut self.ears {
            for i in 0..2 {
                ear.open[i] = self.sea.open(self.singers[i].route.at(self.day), ear.at);
            }
            ear.ping *= (-dt / 0.3).exp() as f32;
            for r in &self.rings {
                let c = [r.at[0] as f64, r.at[1] as f64, r.at[2] as f64];
                let d = dot(c, ear.at).clamp(-1.0, 1.0).acos();
                let age = t - r.born;
                let (r0, r1) = (r.speed as f64 * (age - dt).max(0.0), r.speed as f64 * age);
                if d >= r0 && d < r1 && ear.open[r.kind as usize] {
                    let fade = (1.0 - age / RING_LIFE).max(0.0).powf(1.6) as f32;
                    ear.ping = ear.ping.max((r.amp * fade * 1.4).min(1.0));
                }
            }
        }
        self.ears.retain(|e| !e.gone(t));
        if self.silence.as_ref().is_some_and(|e| e.gone(t)) {
            self.silence = None;
        }
    }

    /// The hydrophone the songs are heard at: the last lowered or touched, unless it is coming up.
    fn listening(&self) -> Option<&Ear> {
        self.ears.last().filter(|e| e.lift.is_none())
    }

    fn build_marks(&mut self) {
        self.marks.clear();
        let reading = self.globe.reading();
        let zoom = self.globe.camera.zoom;
        let scale = reading * self.size_k() * (zoom / 3.0).powf(0.7).clamp(0.6, 2.4);
        // the wake: a dot each half day over the last six weeks, fading
        for i in 0..2 {
            let route = &self.singers[i].route;
            let mut k = 1;
            let mut last = self.globe.project(route.lonlat(self.day)).0;
            let gap = 7.0 * reading;
            while k <= 72 && self.marks.len() < MAX_MARKS - 4 {
                let d = self.day - k as f64 * 0.25;
                let ([x, y], f) = self.globe.project(route.lonlat(d));
                // a dot each few points of the way it came, so lingering in a bay draws no knot
                if f > 0.05 && (x - last[0]).hypot(y - last[1]) >= gap {
                    last = [x, y];
                    let fade = (1.0 - k as f64 / 73.0).powf(1.5) * (f / 0.2).min(1.0);
                    self.marks.push(MarkI { at: [x as f32, y as f32, (2.6 * reading) as f32, 0.0], look: [2.0, (0.24 * fade) as f32, 0.0, 0.0] });
                }
                k += 1;
            }
        }
        for i in 0..2 {
            let route = &self.singers[i].route;
            let ([x, y], f) = self.globe.project(route.lonlat(self.day));
            if f <= 0.0 {
                continue;
            }
            let ([xa, ya], _) = self.globe.project(route.lonlat(self.day - 1.5));
            let ([xb, yb], _) = self.globe.project(route.lonlat(self.day + 1.5));
            let heading = (yb - ya).atan2(xb - xa);
            let len = if i == 0 { 20.0 } else { 30.0 } * scale;
            let glow = 0.0;
            let phase = self.t * if i == 0 { 2.6 } else { 1.7 } + i as f64;
            let alpha = (f / 0.15).min(1.0);
            self.marks.push(MarkI { at: [x as f32, y as f32, len as f32, heading as f32], look: [i as f32, alpha as f32, phase as f32, glow] });
        }
        // the hydrophones, the one listened at a little larger and brighter; the land's silence
        let n = self.ears.len();
        for (j, ear) in self.ears.iter().chain(self.silence.iter()).enumerate() {
            let ([x, y], f) = self.globe.project(LonLat::from_xyz(ear.at));
            if f > 0.0 {
                let heard = ear.land || (j + 1 == n && ear.lift.is_none());
                let k = (ear.strength(self.t) * if heard { 1.0 } else { 0.7 }) as f32;
                let size = if heard { 30.0 } else { 25.0 } * reading * self.size_k();
                self.marks.push(MarkI { at: [x as f32, y as f32, size as f32, 0.0], look: [3.0, k, ear.ping, if ear.land { 1.0 } else { 0.0 }] });
            }
        }
    }
}

impl Toy for Whales {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut globe = Globe::new(&ctx.gpu, ctx.assets.as_ref(), ctx.width, ctx.height, ctx.ppp, 10)?;
        // the night's Earth always: a dark sea the songs can glow on, the coast's towns along it
        globe.night = 1.0;
        globe.hand.min_zoom = 0.9;
        globe.hand.max_zoom = 16.0;
        let [hr, br] = route::load(&ctx.assets.read("routes.json")?, ["humpback", "blue"])?;
        let hs = Arc::new(Song::parse(&ctx.assets.read("humpback.song")?)?);
        let bs = Arc::new(Song::parse(&ctx.assets.read("blue.song")?)?);
        let mix = Arc::new(Mix::default());
        let choir = Choir::new([hs.clone(), bs.clone()], mix.clone(), [0.0, 3.0]);
        let (rings_pass, marks_pipeline, marks_buf) = Self::new_passes(ctx, &globe);
        let sea = Sea::new(&ctx.assets.read("geo.png")?)?;
        let singer = |route, song: Arc<Song>, pos| Singer { route, song, pos, last_ring: -10.0, loud_avg: 0.0, gain: 0.0 };
        let mut toy = Whales {
            globe,
            sea,
            ears: Vec::new(),
            silence: None,
            follow_zoom: 11.0,
            singers: [singer(hr, hs, 0.0), singer(br, bs, 3.0)],
            mix,
            choir: Some(choir),
            heard_frames: 0,
            day: FIRST_DAY,
            t: 0.0,
            rings: Vec::new(),
            mode: Mode::Both,
            last_touch: -100.0,
            press: None,
            fingers: 0,
            reduce_motion: ctx.reduce_motion,
            rings_pass,
            marks_pipeline,
            marks_buf,
            marks: Vec::new(),
        };
        // open already holding both
        let (centre, zoom) = toy.both_target();
        let ll = LonLat::from_xyz(centre);
        toy.globe.camera = Camera::new(ll.lon, ll.lat, zoom);
        Ok(toy)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
    }

    /// The whales by name, for a plan's `"tap": "humpback"` (check.rs).
    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        (0..2)
            .filter_map(|i| {
                let (p, f) = self.whale_on_screen(i);
                (f > 0.0).then(|| (["humpback", "blue"][i].to_string(), p))
            })
            .collect()
    }

    fn project(&self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        self.globe.point(lonlat)
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        self.choir.take().map(|c| Box::new(c) as Box<dyn Synth>)
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        self.last_touch = self.t;
        match e.phase {
            Phase::Down => {
                self.fingers += 1;
                self.press = if self.fingers == 1 { Some((e.id, [e.x, e.y], e.t, false)) } else { None };
            }
            Phase::Move => {
                if let Some((id, at, _, moved)) = self.press.as_mut() {
                    if *id == e.id && (e.x - at[0]).hypot(e.y - at[1]) > 10.0 {
                        *moved = true;
                    }
                }
            }
            Phase::Up | Phase::Cancel => {
                self.fingers = self.fingers.saturating_sub(1);
                if let Some((id, at, t0, moved)) = self.press.take() {
                    if id == e.id && !moved && e.t - t0 < 1.5 && e.phase == Phase::Up {
                        self.tap(ctx, at);
                    } else if id == e.id && moved {
                        self.mode = Mode::Free;
                    }
                }
            }
        }
        if self.fingers >= 2 {
            self.mode = match self.mode {
                Mode::Follow(i) => Mode::Follow(i),
                _ => Mode::Free,
            };
        }
        // a drag with a whale followed lets it go; a pinch keeps it, at the pinch's closeness
        let follow = self.mode;
        self.globe.pointer(e);
        if self.globe.hand.fingers() >= 2 {
            self.follow_zoom = self.globe.camera.zoom;
        }
        if let (Mode::Follow(_), Some((_, _, _, true))) = (follow, self.press) {
            self.mode = Mode::Free;
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        self.day = (self.day + dt * DAYS_PER_SECOND).rem_euclid(YEAR);
        // the ground's month follows the year
        self.globe.clock.day = self.day;
        self.globe.update(&ctx.gpu, ctx.assets.as_ref(), t, dt);
        self.steer_camera(dt);
        self.sing(dt);
        self.build_marks();
        if std::env::var("WHALES_DEBUG").is_ok() && (t * 30.0).round() as i64 % 30 == 0 {
            let p: Vec<_> = (0..2).map(|i| self.whale_on_screen(i).0.map(|v| v.round())).collect();
            eprintln!("t {t:.1} day {:.1} humpback {:?} blue {:?} zoom {:.2} rings {}", self.day, p[0], p[1], self.globe.camera.zoom, self.rings.len());
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        self.globe.upload(&ctx.gpu);
        let mut u = RingsU { at: [[0.0; 4]; MAX_RINGS], look: [[0.0; 4]; MAX_RINGS], n: [0.0; 4], ears: [[0.0; 4]; MAX_EARS], whale: [[0.0; 4]; 2], misc: [0.0; 4] };
        for i in 0..2 {
            let w = self.singers[i].route.at(self.day);
            u.whale[i] = [w[0] as f32, w[1] as f32, w[2] as f32, 0.0];
        }
        // the last four on the water (one coming up besides four down is the fifth: the oldest is left out)
        let n = self.ears.len();
        for (slot, j) in (n.saturating_sub(MAX_EARS)..n).enumerate() {
            let ear = &self.ears[j];
            let heard = j + 1 == n && ear.lift.is_none();
            let k = ear.strength(self.t) * if heard { 1.0 } else { 0.6 };
            u.ears[slot] = [ear.at[0] as f32, ear.at[1] as f32, ear.at[2] as f32, k as f32];
        }
        u.misc[0] = (22.0 * self.globe.reading() * self.size_k() / self.globe.disc().r) as f32;
        let disc_r = self.globe.disc().r as f32;
        let reading = self.globe.reading() as f32;
        for (k, r) in self.rings.iter().enumerate().take(MAX_RINGS) {
            let age = (self.t - r.born) as f32;
            let life = age / RING_LIFE as f32;
            let radius = r.speed * age;
            // a ring thins as it spreads (the sound's spreading loss), drawn wider as it goes
            let fade = (1.0 - life).max(0.0).powf(1.6) * (1.0 - (-age * 8.0).exp());
            let width_pts = if r.kind == 0 { 0.9 + 0.7 * age } else { 1.5 + 1.2 * age } * reading * self.size_k() as f32;
            u.at[k] = [r.at[0], r.at[1], r.at[2], radius];
            u.look[k] = [r.amp * fade * 0.9, r.pitch, r.kind as f32, width_pts / disc_r];
            u.n[0] = (k + 1) as f32;
        }
        ctx.gpu.queue.write_buffer(&self.rings_pass.buf, 0, bytemuck::bytes_of(&u));
        ctx.gpu.queue.write_buffer(&self.marks_buf, 0, bytemuck::cast_slice(&self.marks));
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("whales"),
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
            self.globe.draw(&mut rp);
            rp.set_pipeline(&self.rings_pass.pipeline);
            rp.set_bind_group(0, self.globe.view_bind(), &[]);
            rp.set_bind_group(1, &self.rings_pass.bind, &[]);
            rp.draw(0..3, 0..1);
            if !self.marks.is_empty() {
                rp.set_pipeline(&self.marks_pipeline);
                rp.set_bind_group(0, self.globe.view_bind(), &[]);
                rp.set_vertex_buffer(0, self.marks_buf.slice(..));
                rp.draw(0..6, 0..self.marks.len() as u32);
            }
        }
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&Saved { day: self.day, mode: self.mode, camera: self.globe.camera }).ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            self.day = s.day.rem_euclid(YEAR);
            self.mode = s.mode;
            self.globe.camera = s.camera;
        }
    }
}

impl Whales {
    fn tap(&mut self, ctx: &mut Ctx, at: [f64; 2]) {
        // the nearest whale within reach (a fingertip's, and the whale's own size)
        let reach = 34.0 * self.globe.reading();
        let mut best: Option<(usize, f64)> = None;
        for i in 0..2 {
            let ([x, y], f) = self.whale_on_screen(i);
            if f <= 0.0 {
                continue;
            }
            let d = (x - at[0]).hypot(y - at[1]);
            if d < reach && best.is_none_or(|(_, b)| d < b) {
                best = Some((i, d));
            }
        }
        match best {
            Some((i, _)) => {
                self.mode = Mode::Follow(i);
                self.follow_zoom = 11.0;
                self.lift_ears();
                self.silence = None;
                ctx.haptic();
                // it answers: a ring at once, whatever the song is doing
                let s = &mut self.singers[i];
                let p = s.route.at(self.day);
                let [_, pitch] = s.song.env_at(s.pos);
                s.last_ring = self.t;
                let speed = RING_PTS[i] * self.size_k() / self.globe.disc().r;
                if self.rings.len() >= MAX_RINGS {
                    self.rings.remove(0);
                }
                self.rings.push(Ring { at: [p[0] as f32, p[1] as f32, p[2] as f32], born: self.t, amp: 0.8, pitch, kind: i as u32, speed: speed as f32 });
            }
            None => {
                // the water: a hydrophone there (or, on one already down, listening there again);
                // the land: its silence
                let Some(place) = self.globe.unproject(at[0], at[1]) else { return };
                let p = place.to_xyz();
                let t = self.t;
                if !self.sea.is_sea(p) {
                    self.silence = Some(Ear { at: p, born: t, land: true, ping: 0.0, open: [false; 2], lift: None });
                    ctx.haptic();
                    return;
                }
                self.silence = None;
                let reach = 26.0 * self.globe.reading();
                let near = self.ears.iter().position(|e| {
                    let ([x, y], f) = self.globe.project(LonLat::from_xyz(e.at));
                    e.lift.is_none() && f > 0.0 && (x - at[0]).hypot(y - at[1]) < reach
                });
                match near {
                    Some(j) => {
                        let mut ear = self.ears.remove(j);
                        // down again from now, at once
                        ear.born = t - 0.25;
                        self.ears.push(ear);
                    }
                    None => {
                        let open = [0, 1].map(|i| self.sea.open(self.singers[i].route.at(self.day), p));
                        self.ears.push(Ear { at: p, born: t, land: false, ping: 0.0, open, lift: None });
                        let down = self.ears.iter().filter(|e| e.lift.is_none()).count();
                        if down > MAX_EARS {
                            if let Some(oldest) = self.ears.iter_mut().find(|e| e.lift.is_none()) {
                                oldest.lift = Some(t);
                            }
                        }
                    }
                }
                self.mix.moved.fetch_add(1, Ordering::Relaxed);
                ctx.haptic();
            }
        }
    }

    /// Every hydrophone comes up (a whale is followed: its song comes forward instead).
    fn lift_ears(&mut self) {
        let t = self.t;
        for e in self.ears.iter_mut().filter(|e| e.lift.is_none()) {
            e.lift = Some(t);
        }
    }
}

toy_kit::ios_shell!(Whales);

#[cfg(test)]
mod tests {
    use super::*;

    fn data(name: &str) -> Vec<u8> {
        std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(name)).unwrap()
    }

    #[test]
    fn the_songs_decode_whole() {
        for (name, secs) in [("humpback.song", 180.0), ("blue.song", 23.0)] {
            let s = Song::parse(&data(name)).unwrap();
            assert!((s.seconds() - secs).abs() < 1.0, "{name} is {} s", s.seconds());
            let peak = s.pcm.iter().fold(0.0f32, |m, x| m.max(x.abs()));
            assert!(peak > 0.25 && peak <= 1.0, "{name} peaks at {peak}");
            assert!(s.env.iter().any(|e| e[0] > 0.8), "{name}'s envelope never rises");
        }
    }

    #[test]
    fn the_whales_go_south_in_winter_and_sing_in_their_seasons() {
        let [h, b] = route::load(&data("routes.json"), ["humpback", "blue"]).unwrap();
        // humpback: Monterey Bay in November, off Mexico in February
        assert!(h.lonlat(323.0).lat > 35.0 && h.lonlat(40.0).lat < 23.0);
        // blue whale: the Costa Rica Dome in February
        let dome = b.lonlat(45.0);
        assert!((dome.lat - 9.0).abs() < 2.0 && (dome.lon + 89.5).abs() < 3.0, "{dome:?}");
        assert!(h.sings(330.0) > 0.8 && b.sings(280.0) > 0.8 && b.sings(60.0) < 0.2);
    }

    #[test]
    fn the_choir_plays_and_tells_where_it_is() {
        let hs = Arc::new(Song::parse(&data("humpback.song")).unwrap());
        let bs = Arc::new(Song::parse(&data("blue.song")).unwrap());
        let mix = Arc::new(Mix::default());
        let mut c = Choir::new([hs, bs], mix.clone(), [0.0, 0.0]);
        mix.set(0, 1.0, -0.5);
        mix.set(1, 1.0, 0.5);
        let mut out = vec![0.0f32; 48_000 * 2];
        c.render(&mut out, 2, 48_000.0);
        assert!((mix.pos(0) - 1.0).abs() < 1e-3, "{}", mix.pos(0));
        assert!(out.iter().all(|x| x.abs() <= 1.0) && out.iter().any(|x| x.abs() > 0.05));
    }
}

#[cfg(test)]
mod round2 {
    use super::*;

    #[test]
    fn baja_stands_between_the_pacific_and_the_gulf() {
        let geo = std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Globe/geo.png")).unwrap();
        let sea = Sea::new(&geo).unwrap();
        let pacific = LonLat::new(-115.0, 26.0).to_xyz();
        let gulf = LonLat::new(-111.0, 26.5).to_xyz();
        let offshore = LonLat::new(-117.0, 28.0).to_xyz();
        assert!(sea.is_sea(pacific) && sea.is_sea(gulf) && !sea.is_sea(LonLat::new(-112.5, 26.5).to_xyz()));
        assert!(!sea.open(pacific, gulf), "the sound crossed the peninsula");
        assert!(sea.open(pacific, offshore));
    }

    #[test]
    fn a_hydrophone_hears_late_and_muffled() {
        let data = |n: &str| std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("data").join(n)).unwrap();
        let hs = Arc::new(Song::parse(&data("humpback.song")).unwrap());
        let bs = Arc::new(Song::parse(&data("blue.song")).unwrap());
        let mix = Arc::new(Mix::default());
        let mut c = Choir::new([hs, bs], mix.clone(), [0.0, 0.0]);
        // three seconds heard at once, then the hydrophone placed two seconds' travel away
        let mut out = vec![0.0f32; 48_000 * 3 * 2];
        c.render(&mut out, 2, 48_000.0);
        mix.set(0, 1.0, 0.0);
        mix.place(0, 2.0, 20_000.0);
        mix.moved.fetch_add(1, Ordering::Relaxed);
        let mut out = vec![0.0f32; 48_000 * 2];
        c.render(&mut out, 2, 48_000.0);
        // what is heard 3.5 s in is what the humpback sang at 1.5 s
        let s = Song::parse(&data("humpback.song")).unwrap();
        let heard: Vec<f32> = (0..2400).map(|k| out[(24_000 + k) * 2] / std::f32::consts::FRAC_1_SQRT_2).collect();
        let sung: Vec<f32> = (0..2400).map(|k| s.pcm[((1.5 + k as f64 / 48_000.0) * s.rate) as usize]).collect();
        let dot = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
        let r = dot(&heard, &sung) / (dot(&heard, &heard) * dot(&sung, &sung)).sqrt().max(1e-9);
        assert!(r > 0.9, "the song heard is not the song sung two seconds before (r {r})");
    }
}

