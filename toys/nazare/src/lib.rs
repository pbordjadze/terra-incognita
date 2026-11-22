//! Nazaré (a working title; painting-company row 217's family, made local): the Atlantic swell over the
//! real floor off Nazaré, Portugal, where a submarine canyon runs almost to the beach, drawn the way Nelson
//! Max drew *Carla's Island* (max.rs).
//!
//! It opens on the cliff by the lighthouse, looking over Praia do Norte at dusk. A long swell bends over the
//! shelf, meets the canyon's edge and is turned back toward the beach, where the two trains cross and stand
//! up into peaks (sea.rs). When a set's biggest wave is about to stack, the camera drops from the cliff to
//! the channel in front of it, low on the water, and the wave stands up as a wall with a tow surfer on its
//! face; then it climbs back. A swipe toward the coast sends a swell from where the finger came, as long a
//! swell as the swipe is long: a long one stacks and the camera goes down to meet it; a short one crosses
//! the canyon and marches in straight.

mod camera;
pub mod floor;
pub mod max;
pub mod sea;
mod sound;

use max::{Cam, Max, TraceU};
use sea::{Field, Floor};
use serde::Deserialize;
use toy_kit::gpu::textures::{Pixels, Texture};
use toy_kit::paint::Painter;
use toy_kit::{Ctx, Phase, PointerEvent, Toy};

/// Where the toy is: Nazaré's headland (lon, lat).
pub const PLACE: (f64, f64) = (-9.0850, 39.6040);

/// A set: this many waves' periods from one group of big waves to the next.
const GROUP: f64 = 9.0;
/// The swell it opens on: a long winter groundswell from the west-north-west, inside the 275° to 315° that
/// focus best (Hope et al. 2026).
const PERIOD: f64 = 17.0;
const FROM_DEG: f64 = 295.0;
/// The swells a swipe can send: periods (s) and directions (degrees from north): the open Atlantic's.
const PERIODS: (f64, f64) = (5.0, 21.0);
const FROMS: (f64, f64) = (240.0, 345.0);
/// Seconds of sea a second on the screen: from the cliff the crests march; down at the wall it is near real
/// time.
const RATE_FORT: f64 = 2.2;
const RATE_WALL: f64 = 1.0;
/// How long after a finger asks for a new swell the sea takes it up (s): longer than a solve takes.
const SOLVE_S: f64 = 0.8;
/// A swipe shorter than this (points) is a touch, not a swell sent.
const SWIPE_MIN: f64 = 24.0;
/// How long after a swell is sent its set's big wave stacks at the peak (s on the screen), and when the
/// opening's first one does.
const SET_LEAD: f64 = 6.0;
const FIRST_SET: f64 = 4.2;
/// The camera's trip down from the cliff and up again (s on the screen), how long before the big wave it
/// leaves, and how long after its crest passes it stays.
const DROP_S: f64 = 2.2;
const DROP_LEAD: f64 = 2.6;
const STAY_S: f64 = 3.2;
/// Where a peak can stand: the depths (m) where the big waves break, off the beach.
const PEAK_DEPTH: (f64, f64) = (10.0, 40.0);
/// A peak stacks (and the camera goes down for it) once its waves are this near breaking.
const STACKS: f64 = 0.6;
/// The sun: its bearing, and its height from the start (degrees), going down this fast (degrees a minute)
/// until it holds at dusk.
/// The lighthouse's tower on the fort (m).
const LIGHT_TALL_M: f32 = 9.0;
/// Where the fort's view is from: back from the lighthouse up the headland (m east, m south).
const FORT_BACK_M: [f32; 2] = [260.0, 30.0];
/// Where the fort looks, from the lighthouse (m east, m south).
const FORT_AIM_M: [f32; 2] = [-300.0, -200.0];
/// Where the camera waits low in the channel: this far ahead of the peak and this far to its side (m).
const WALL_AHEAD_M: f32 = 75.0;
const WALL_SIDE_M: f32 = 45.0;
const SUN_AZ: f32 = 252.0;
const SUN_START: f32 = 3.5;
const SUN_RATE: f32 = 2.2;
const SUN_DUSK: f32 = -5.0;

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
    praia_do_norte: Place,
}

#[derive(Deserialize)]
struct Place {
    cell: [f32; 2],
}

pub struct Nazare {
    max: Max,
    tex: Texture,
    painter: Painter,
    floor: std::sync::Arc<Floor>,
    bounds: Bounds,
    field: Field,
    /// the swell a finger has asked for, and its solve under way (taken up at the time given)
    want: (f64, f64),
    /// which of `sea::RULES` the sheet has on (the field is solved again when they change)
    on: [bool; 2],
    pending: Option<(Solving, f64)>,
    /// where the waves stack (a cell), if they do, and the crest's place in its cycle there a frame ago
    peak: Option<usize>,
    last_w: f64,
    sound: sound::Queue,
    /// a finger's start (points): the swell is sent when it lifts
    drag: Option<(u64, [f64; 2])>,
    /// the sets' timing, in sets, and whether a set is waiting for its field
    sets_off: f64,
    send: bool,
    /// sea time (s), screen time (s)
    t: f64,
    clock: f64,
    /// how far down the camera is (0 on the cliff, 1 at the wall), and until when (screen s) it stays down
    down: f64,
    stay_until: f64,
    cam: Option<Cam>,
    ppp: f64,
}

/// A swell being solved on its own thread: its field and its texels.
#[cfg(not(target_arch = "wasm32"))]
type Solving = std::thread::JoinHandle<(Field, Vec<u8>)>;
/// A browser's WebAssembly has no threads: the swell is solved at once (a frame's stall) and taken up as
/// late as on the phone.
#[cfg(target_arch = "wasm32")]
type Solving = Solved;
#[cfg(target_arch = "wasm32")]
struct Solved((Field, Vec<u8>));
#[cfg(target_arch = "wasm32")]
impl Solved {
    fn join(self) -> Result<(Field, Vec<u8>), ()> {
        Ok(self.0)
    }
    fn is_finished(&self) -> bool {
        true
    }
}

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
    let light = b.places.headland.cell;
    Ok(floor::build(
        b.w,
        b.h,
        b.cell_m,
        ground,
        [light[0] as f64, light[1] as f64],
    ))
}

/// How much of a train of wavenumber `k` (rad/m) the field's cells can draw: none under two cells a
/// wavelength (its crests would alias, even march backward), all of it over three.
fn drawn_share(k: f64, cell_m: f64) -> f64 {
    let cells = std::f64::consts::TAU / k / cell_m;
    ((cells - 2.0) / 1.0).clamp(0.0, 1.0)
}

/// The short swell the field can't draw (its share of the open sea's height, m) and its wavenumber vector
/// (rad/m, x east, y south): straight crests at the open sea's spacing, the shader's to draw. A short swell
/// barely feels this floor, so it crosses the canyon much as it is.
fn short_train(f: &Field, cell_m: f64) -> (f32, [f32; 2]) {
    let k = sea::wavenumber_of(f.period, 4000.0);
    let h = sea::H0 * (1.0 - drawn_share(k, cell_m));
    let a = (f.from_deg + 180.0).to_radians();
    (h as f32, [(a.sin() * k) as f32, (-a.cos() * k) as f32])
}

/// The field into the texture's three blocks (max.wgsl says what each holds).
fn texels(floor: &Floor, f: &Field) -> Vec<u8> {
    let (w, h) = (floor.w, floor.h);
    let tg = f.period * GROUP;
    let mut data = vec![0u8; 3 * w * h * 4];
    let unit = |v: f64| (127.5 + 127.0 * v) as u8;
    let angle = |tau: f64, period: f64| std::f64::consts::TAU * tau / period;
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            let a = (y * 3 * w + x) * 4;
            let b = a + w * 4;
            let c = b + w * 4;
            if !floor.sea(i) {
                let z = (floor.ground[i] as f64).max(0.0);
                data[a + 3] = (128.0 + 127.0 * (z / 200.0).min(1.0)) as u8;
                data[b + 3] = 0;
                for k in [a, b, c] {
                    data[k] = 128;
                    data[k + 1] = 128;
                }
                continue;
            }
            let depth = floor.depth(i).min(800.0);
            data[a + 3] = (110.0 * (depth / 800.0).sqrt()) as u8;
            data[b + 3] = ((depth / 60.0).min(1.0) * 255.0) as u8;
            // a train whose crests are closer than the field's cells can carry is left to the shader's
            // short train (`short_train`)
            let felt = if f.rules[0] { depth.max(0.5) } else { 4000.0 };
            let resolved = drawn_share(sea::wavenumber_of(f.period, felt), floor.cell_m);
            for (k, tau, hh) in [
                (a, f.tau[i], f.height[i] * resolved as f32),
                (b, f.tau_back[i], f.height_back[i] * resolved as f32),
            ] {
                if tau.is_finite() {
                    let ph = angle(tau, f.period);
                    data[k] = unit(ph.cos());
                    data[k + 1] = unit(ph.sin());
                    data[k + 2] = ((hh / max::H_FULL).min(1.0) * 255.0) as u8;
                } else {
                    data[k] = 128;
                    data[k + 1] = 128;
                }
            }
            let tg_i = f.tau_group[i];
            let g = if tg_i.is_finite() {
                angle(tg_i, tg)
            } else {
                0.0
            };
            data[c] = unit(g.cos());
            data[c + 1] = unit(g.sin());
        }
    }
    data
}

impl Nazare {
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
                bytes_per_row: Some(3 * w as u32 * 4),
                rows_per_image: None,
            },
            wgpu::Extent3d {
                width: 3 * w as u32,
                height: h as u32,
                depth_or_array_layers: 1,
            },
        );
    }

    /// A new swell asked for: solved on its own thread, taken up a set time later (so a slow solve never
    /// stalls a frame, and the headless check, which waits for it then, sees the same frames every run).
    fn ask(&mut self) {
        let floor = std::sync::Arc::clone(&self.floor);
        let (period, from) = self.want;
        let rules = self.on;
        let solve = move || {
            let field = Field::with_rules(&floor, period, from, rules);
            let data = texels(&floor, &field);
            (field, data)
        };
        #[cfg(not(target_arch = "wasm32"))]
        let job = std::thread::spawn(solve);
        #[cfg(target_arch = "wasm32")]
        let job = Solved(solve());
        self.pending = Some((job, self.clock + SOLVE_S));
    }

    fn cell_m(&self) -> f64 {
        self.floor.cell_m
    }

    /// A cell's centre (m, x east, y south).
    fn at(&self, i: usize) -> [f32; 2] {
        let w = self.floor.w;
        let c = self.cell_m() as f32;
        [((i % w) as f32 + 0.5) * c, ((i / w) as f32 + 0.5) * c]
    }

    fn light_m(&self) -> [f32; 2] {
        let c = self.bounds.places.headland.cell;
        let m = self.cell_m() as f32;
        [c[0] * m, c[1] * m]
    }

    /// Where the waves stack: the tallest crossing within 1.6 km of the lighthouse, north of the canyon,
    /// near breaking. None for a swell that never stacks (a short one, or the rules switched off).
    fn find_peak(&self) -> Option<usize> {
        let (w, h) = (self.floor.w as i64, self.floor.h as i64);
        let c = self.bounds.places.headland.cell;
        let (cx, cy) = (c[0] as i64, c[1] as i64);
        let reach = (1600.0 / self.cell_m()) as i64;
        let mut best: Option<(usize, f64)> = None;
        for y in (cy - reach).max(0)..(cy + 2).min(h) {
            for x in (cx - reach).max(0)..(cx + 4).min(w) {
                let i = (y * w + x) as usize;
                let depth = self.floor.depth(i);
                if !self.floor.sea(i)
                    || self.floor.canyon[i]
                    || !(PEAK_DEPTH.0..PEAK_DEPTH.1).contains(&depth)
                {
                    continue;
                }
                let hh = self.field.crossed(i);
                let near = self.field.nearness(&self.floor, i);
                if near >= STACKS
                    && self.field.height_back[i] > 0.5
                    && best.is_none_or(|(_, b)| hh > b)
                {
                    best = Some((i, hh));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// The sets' phase at a cell now (cycles): 0 when the set's biggest waves are there.
    fn set_phase(&self, i: usize) -> f64 {
        let tg = self.field.period * GROUP;
        (self.field.tau_group[i] / tg - self.t / tg - self.sets_off).rem_euclid(1.0)
    }

    /// Sea time until the set's biggest crest is at the peak, the crest nearest the set's middle.
    fn to_big_wave(&self, i: usize) -> f64 {
        let f = &self.field;
        let tg = f.period * GROUP;
        // sea time until the set's middle is at the peak (the sets' phase there comes round to 0)
        let mid = ((self.set_phase(i) + 0.5).rem_euclid(1.0) - 0.5) * tg;
        let mid = if mid < -0.5 * f.period { mid + tg } else { mid };
        // the crest there nearest it
        let at = self.t + mid;
        let crest = f.tau[i] + ((at - f.tau[i]) / f.period).round() * f.period;
        crest - self.t
    }

    /// The sets' timing shifted so the set's biggest wave stacks at the peak `lead` screen seconds from
    /// now, on a crest.
    fn time_set(&mut self, lead: f64) {
        let Some(i) = self.peak else {
            return;
        };
        let f = &self.field;
        let tg = f.period * GROUP;
        let want = self.t + lead * RATE_FORT;
        let crest = f.tau[i] + ((want - f.tau[i]) / f.period).round() * f.period;
        // set so the sets' phase at the peak is 0 at that crest
        self.sets_off = (f.tau_group[i] / tg - crest / tg).rem_euclid(1.0);
    }

    /// A swipe from `a` to `b` (points): the swell comes from where the finger came from (the swipe is its
    /// travel), kept to the open Atlantic's directions; the longer the swipe, the longer the swell. On the
    /// screen from the cliff, the sea is ahead (up) and the coast behind and right: a swipe down or to the
    /// right sends a swell in.
    fn swipe(&mut self, a: [f64; 2], b: [f64; 2], wide: f64) -> bool {
        let d = [b[0] - a[0], b[1] - a[1]];
        let len = d[0].hypot(d[1]);
        if len < SWIPE_MIN {
            return false;
        }
        let Some(cam) = self.cam else {
            return false;
        };
        // the swipe on the screen as a direction on the sea: screen right is the camera's right, screen
        // down is toward the camera
        let (r, f) = (cam.right, cam.fwd);
        let fl = f[0].hypot(f[1]).max(1e-6);
        let fwd = [f[0] / fl, f[1] / fl];
        let travel = [
            r[0] as f64 * d[0] - fwd[0] as f64 * d[1],
            r[1] as f64 * d[0] - fwd[1] as f64 * d[1],
        ];
        // travelling (x east, y south) toward the bearing a + 180°: east −sin a, south cos a
        let from = (-travel[0]).atan2(travel[1]).to_degrees().rem_euclid(360.0);
        let mid = 0.5 * (FROMS.0 + FROMS.1);
        let off = (from - mid + 180.0).rem_euclid(360.0) - 180.0;
        // a swipe out to sea sends nothing: no swell comes from the land
        if off.abs() > 0.5 * (FROMS.1 - FROMS.0) + 50.0 {
            return false;
        }
        let from = (mid + off).clamp(FROMS.0, FROMS.1);
        // (eased, so a short flick sends a swell short enough to cross the canyon)
        let k = ((len - SWIPE_MIN) / (0.6 * wide)).clamp(0.0, 1.0).powf(1.5);
        let period = PERIODS.0 + (PERIODS.1 - PERIODS.0) * k;
        self.want = (period, from);
        true
    }

    /// The surf's sound: a crash each time a crest reaches the peak, as loud as the wave is tall. Whether a
    /// set's big wave broke there this frame (felt as well as heard).
    fn listen(&mut self) -> bool {
        let Some(i) = self.peak else {
            if let Ok(mut q) = self.sound.lock() {
                q.roar = 0.08;
            }
            return false;
        };
        let f = &self.field;
        let w = (f.tau[i] / f.period - self.t / f.period).rem_euclid(1.0);
        let crest = self.last_w < 0.25 && w > 0.75;
        self.last_w = w;
        let sets =
            0.3 + 0.7 * (0.5 + 0.5 * (std::f64::consts::TAU * self.set_phase(i)).cos()).powi(6);
        let Ok(mut q) = self.sound.lock() else {
            return false;
        };
        // nearer, louder
        q.roar = 0.10 + 0.08 * self.down;
        if crest {
            let rel = f.crossed(i) / (2.0 * sea::H0);
            let amp = (rel * sets * (0.6 + 0.4 * self.down)).clamp(0.05, 1.0);
            q.crashes.push(amp);
            return amp > 0.55;
        }
        false
    }

    fn sun(&self) -> (f32, [f32; 3]) {
        let el = (SUN_START - SUN_RATE * self.clock as f32 / 60.0).max(SUN_DUSK);
        let (a, e) = (SUN_AZ.to_radians(), el.to_radians());
        // x east, y south, z up
        (el, [a.sin() * e.cos(), -a.cos() * e.cos(), e.sin()])
    }
}

impl Toy for Nazare {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let bounds: Bounds = serde_json::from_slice(&ctx.assets.read("floor.json")?)?;
        let floor = load_floor(&ctx.assets.read("floor.bin")?, &bounds)?;
        let field = Field::new(&floor, PERIOD, FROM_DEG);
        let blank = Pixels {
            width: 3 * floor.w as u32,
            height: floor.h as u32,
            channels: 4,
            data: vec![0; 3 * floor.w * floor.h * 4],
        };
        let tex = Texture::flat(&ctx.gpu, "nazare field", &blank);
        let max = Max::new(&ctx.gpu, &tex.view);
        let data = texels(&floor, &field);
        let mut toy = Nazare {
            max,
            tex,
            painter: Painter::new(&ctx.gpu),
            floor: std::sync::Arc::new(floor),
            bounds,
            field,
            want: (PERIOD, FROM_DEG),
            on: [true; 2],
            pending: None,
            peak: None,
            last_w: 0.0,
            sound: sound::Queue::default(),
            drag: None,
            sets_off: 0.0,
            send: false,
            t: 0.0,
            clock: 0.0,
            down: 0.0,
            stay_until: 0.0,
            cam: None,
            ppp: ctx.ppp,
        };
        toy.upload(&ctx.gpu, &data);
        toy.peak = toy.find_peak();
        // something happens at once: the opening's first set is already on its way to the peak
        // (the sea's clock starts so a crest reaches the peak just then)
        if let Some(i) = toy.peak {
            toy.t = toy.field.tau[i] - FIRST_SET * RATE_FORT;
        }
        toy.time_set(FIRST_SET);
        Ok(toy)
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let wide = ctx.points()[0];
        match e.phase {
            // the first finger is the swipe; a second is ignored
            Phase::Down if self.drag.is_none() => self.drag = Some((e.id, [e.x, e.y])),
            Phase::Down | Phase::Move => {}
            Phase::Up => {
                if let Some((id, a)) = self.drag {
                    if id == e.id {
                        self.drag = None;
                        self.send = self.swipe(a, [e.x, e.y], wide) || self.send;
                    }
                }
            }
            _ => self.drag = None,
        }
    }

    fn update(&mut self, ctx: &mut Ctx, _t: f64, dt: f64) {
        let dt = dt.min(0.1);
        self.clock += dt;
        self.t += dt * (RATE_FORT + (RATE_WALL - RATE_FORT) * self.down);
        let due = self
            .pending
            .as_ref()
            // taken up once solved, or a moment later at the latest (waiting for it then: a machine
            // drawing flat out can starve the solve's thread)
            .is_some_and(|(job, at)| {
                self.clock >= *at && (job.is_finished() || self.clock >= *at + 1.0)
            })
            // and not while the camera is down by a peak: its view is built on it, so the new sea waits
            // for the camera to be back on the headland
            && self.down < 0.05;
        if due {
            if let Some((job, _)) = self.pending.take() {
                if let Ok((field, data)) = job.join() {
                    self.field = field;
                    self.upload(&ctx.gpu, &data);
                    self.peak = self.find_peak();
                    // no crash for the swell merely changing under the peak
                    self.last_w = 1.0;
                }
            }
        }
        let asked = (self.want.0 - self.field.period).abs() > 0.05
            || (self.want.1 - self.field.from_deg).abs() > 0.2
            || self.on != self.field.rules;
        if asked && self.pending.is_none() {
            self.ask();
        }
        // a set sent goes once its swell is the sea's
        if self.send && !asked && self.pending.is_none() && self.drag.is_none() {
            self.send = false;
            self.time_set(SET_LEAD);
        }
        self.steer(dt);
        if self.listen() {
            ctx.haptic();
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let ppp = ctx.ppp as f32;
        let size = [ctx.width, ctx.height];
        let cam = Cam::new([size[0] as f32, size[1] as f32], self.look());
        self.cam = Some(cam);
        self.ppp = ctx.ppp;
        let period = self.field.period;
        let phase = (self.t / period).rem_euclid(1.0) as f32;
        let sets = (self.t / (period * GROUP) + self.sets_off).rem_euclid(1.0) as f32;
        let (sun_el, sun) = self.sun();
        let l = self.light_m();
        let k_deep = sea::wavenumber_of(period, 4000.0) as f32;
        let short = short_train(&self.field, self.cell_m());
        let u = TraceU {
            eye: [cam.eye[0], cam.eye[1], cam.eye[2], self.clock as f32],
            fwd: [cam.fwd[0], cam.fwd[1], cam.fwd[2], cam.ty],
            right: [cam.right[0], cam.right[1], cam.right[2], cam.tx],
            up: [cam.up[0], cam.up[1], cam.up[2], short.0],
            sun: [sun[0], sun[1], sun[2], 4.0f32.to_radians().cos()],
            phase: [phase, sets, k_deep, 0.0],
            grid: [
                self.floor.w as f32,
                self.floor.h as f32,
                self.cell_m() as f32,
                max::H_FULL,
            ],
            size: [short.1[0], short.1[1], l[0], l[1]],
        };
        let table = max::palette(sun_el);
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.max.render(&ctx.gpu, &mut enc, target, size, u, &table);
        self.draw_lighthouse(&cam, ppp);
        self.draw_surfer(&cam, ppp);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(Box::new(sound::Surf::new(self.sound.clone())))
    }

    fn demo(&self, ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        let [w, h] = ctx.points();
        Some(toy_kit::info::Demo::new(
            "Swipe a long swell toward Nazaré's coast.",
            vec![toy_kit::info::Stroke::drag(
                0.6,
                1.4,
                [0.15 * w, 0.30 * h],
                [0.80 * w, 0.62 * h],
            )],
        ))
    }

    fn style(&self) -> Option<toy_kit::info::Style> {
        Some(toy_kit::info::Style {
            name: "Carla's Island",
            maker: "Nelson Max",
            place: "Lawrence Livermore Laboratory, California",
            year: 1981,
        })
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::lines(&[
            "Shallow water slows a long swell and bends it toward the shallows.",
            "A canyon runs almost to Nazaré's beach. Its edge throws long swell back out across the arriving waves, stacking peaks over 20 m.",
            "A longer swipe sends a longer swell. Short swell is too short to feel the canyon's edge, so nothing stacks.",
        ]))
    }

    fn switches(&self) -> Vec<toy_kit::switches::Switch> {
        toy_kit::switches::Switch::all(&sea::RULES, &self.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(r) = self.on.get_mut(i) {
            *r = on;
        }
    }

    fn project(&self, lonlat: [f64; 2]) -> Option<[f64; 2]> {
        let cam = self.cam?;
        let b = &self.bounds;
        let m = self.cell_m();
        let p = [
            ((lonlat[0] - b.west) / (b.east - b.west) * b.w as f64 * m) as f32,
            ((b.north - lonlat[1]) / (b.north - b.south) * b.h as f64 * m) as f32,
        ];
        let (px, _) = cam.project([p[0], p[1], 0.0])?;
        Some([px[0] as f64 / self.ppp, px[1] as f64 / self.ppp])
    }
}

toy_kit::ios_shell!(Nazare);
