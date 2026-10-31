//! The app: the state (floats, the clock, the camera, undo), a frame, a pointer event, and applying
//! intents (Engineering; DECISIONS.md §2). The seams: Truth's `Rules` step the floats and its
//! `Maybes` run a held float's other selves on the GPU; Visual's `Look` draws the marks and the words
//! from a `LookState` filled here; Motion's `Gestures` turn pointer events into intents through the
//! `Probe` the app answers (touch/probe.rs), and its `CameraControl` owns the camera's motion.

use crate::assets::Assets;
use crate::gpu::{Blit, Budget, Gpu, Stage, Textures, View, ViewUniform};
use crate::look::{sea::Ripple, GroundPass, Look, LookBead, LookFloat, LookState, SeaPass};
use crate::model::*;
use crate::sea::{Drift, Drifters, GpuPath, Maybes, Params, Rules};
use crate::touch::{CameraControl, Gestures, Key, Motion, Out};
use serde::{Deserialize, Serialize};

/// What the reader has done, to take back.
#[derive(Clone, Debug)]
enum Undo {
    Put(u32),
    Moved { id: u32, from: Float },
    Removed(Float),
}

/// Everything about the sea's state that a scene file can set (adrift-check) and a frame reads.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scene {
    pub camera: Camera,
    pub day: f64,
    #[serde(default = "yes")]
    pub running: bool,
    #[serde(default)]
    pub floats: Vec<SceneFloat>,
    /// A float held (its maybes shown), by its index in `floats`.
    #[serde(default)]
    pub hold: Option<usize>,
    /// The drawn motion's seconds.
    #[serde(default)]
    pub seconds: f64,
    /// The night palette (the system's dark appearance).
    #[serde(default)]
    pub night: bool,
}
fn yes() -> bool {
    true
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct SceneFloat {
    pub lon: f64,
    pub lat: f64,
    /// Days before the scene's day it was put (its wake is simulated from there).
    #[serde(default)]
    pub days_ago: f64,
}

/// What is kept between launches: nothing is lost (DECISIONS.md §6).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub version: u32,
    pub floats: Vec<Float>,
    pub next_id: u32,
    pub day: f64,
    pub running: bool,
    pub camera: Camera,
}

pub struct App {
    pub gpu: Gpu,
    assets: Box<dyn Assets>,
    textures: Textures,
    pub rules: Rules,
    /// The real drifters' runs (Truth): the nearest rises in silver beside a held float.
    pub drifters: Drifters,
    view_buffer: wgpu::Buffer,
    view_bind: wgpu::BindGroup,
    stage_view_buffer: wgpu::Buffer,
    stage_view_bind: wgpu::BindGroup,
    ground: GroundPass,
    sea: SeaPass,
    /// Visual's marks and words.
    look: Look,
    blit: Blit,
    stage: Option<Stage>,
    /// Truth's sampler: a held float's maybes on the GPU.
    maybes: Maybes,
    /// The float whose maybes are in the buffer, how many, and the day they were run to.
    maybes_of: Option<(u32, u32, f64)>,
    pub budget: Budget,
    /// What the stage last showed: the view uniform's bytes and the months, to know when to redraw.
    stage_shown: Option<(ViewUniform, (u32, u32))>,
    stage_seconds: f64,
    last_dt: f64,
    pub floats: Vec<Float>,
    next_id: u32,
    pub clock: Clock,
    undo: Vec<Undo>,
    redo: Vec<Undo>,
    /// Motion's camera: the truth of where the reader looks.
    pub camera: CameraControl,
    gestures: Gestures,
    /// A finger resting on the sea, the ring thickening: where, and since when.
    pressing: Option<(LonLat, f64)>,
    /// The maybes' bloom, 0..1, and whether it is opening.
    bloom: f32,
    /// The nearest real drifter's track for the held float, as places in order, and its label.
    drifter: Vec<LonLat>,
    pub drifter_label: String,
    /// The index in `drifter` of the point nearest the float.
    pub drifter_nearest: usize,
    ripples: Vec<Ripple>,
    width: u32,
    height: u32,
    ppp: f64,
    pub reading: f64,
    /// The drawn motion's seconds (held while the date is).
    seconds: f64,
    last_t: Option<f64>,
    now: f64,
    /// How far each float has been stepped (`id` → day).
    stepped: std::collections::HashMap<u32, f64>,
    pub reduce_motion: bool,
    /// The system's dark appearance: the look's night (the shells set it).
    pub night: bool,
    /// How far the sea has settled to rest with the date held, 0..1.
    rest: f32,
    /// The intents applied, by name (the checks' `expect.saw`).
    pub seen: Vec<String>,
    /// A Put or a Hold happened since the shell last asked: the beat, felt (a light haptic).
    pub haptic: bool,
    /// The page of credits is open (Visual's D6; opened and closed by `Intent::ToggleCredits`).
    pub credits_open: bool,
}

/// Days a second while the date runs.
pub const RATE: f64 = 2.0;
/// The longest step a float takes at once (a quarter day: the fastest currents cross less than a cell).
const SUBSTEP: f64 = 0.25;
/// Beads kept in a wake.
const WAKE_MAX: usize = 400;
/// Maybes run for a held float, and their step: 512 at a half day tell the shape 8192 at a quarter
/// day do (Truth's budget, SOURCES.md › How many maybes); a whole day is not honest in a jet.
pub const MAYBES: u32 = 512;
const MAYBES_STEP: f64 = 0.5;

impl App {
    pub fn new(gpu: Gpu, assets: Box<dyn Assets>, width: u32, height: u32, ppp: f64) -> anyhow::Result<App> {
        let clock = Clock { day: 0.0, running: true, rate: RATE };
        let textures = Textures::load(&gpu, assets.as_ref(), clock.months().0)?;
        let rules = Rules::load(assets.as_ref())?;
        let drifters = Drifters::parse(&assets.read("drifters.bin")?).map_err(|e| anyhow::anyhow!("drifters.bin: {e}"))?;
        let view_layout = crate::look::view_layout(&gpu);
        let make_view = |label: &str| {
            let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(label),
                size: std::mem::size_of::<ViewUniform>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let bind = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(label),
                layout: &view_layout,
                entries: &[wgpu::BindGroupEntry { binding: 0, resource: buffer.as_entire_binding() }],
            });
            (buffer, bind)
        };
        let (view_buffer, view_bind) = make_view("view");
        let (stage_view_buffer, stage_view_bind) = make_view("stage view");
        let ground = GroundPass::new(&gpu, &view_layout, &textures);
        let sea = SeaPass::new(&gpu, &view_layout, &textures);
        let look = Look::new(&gpu, &view_layout);
        let blit = Blit::new(&gpu);
        let budget = Budget::new(ppp);
        let maybes = Maybes::new(&gpu, assets.as_ref(), &textures, MAYBES)?;
        let reading = Self::reading_for(width, height, ppp);
        Ok(App {
            gpu,
            assets,
            textures,
            rules,
            drifters,
            view_buffer,
            view_bind,
            stage_view_buffer,
            stage_view_bind,
            ground,
            sea,
            look,
            blit,
            stage: None,
            maybes,
            maybes_of: None,
            budget,
            stage_shown: None,
            stage_seconds: -1.0,
            last_dt: 0.0,
            floats: Vec::new(),
            next_id: 1,
            clock,
            undo: Vec::new(),
            redo: Vec::new(),
            camera: CameraControl::new(Camera { lon: -30.0, lat: 25.0, zoom: 1.0 }),
            gestures: Gestures::new(),
            pressing: None,
            bloom: 0.0,
            drifter: Vec::new(),
            drifter_label: String::new(),
            drifter_nearest: 0,
            ripples: Vec::new(),
            width,
            height,
            ppp,
            reading,
            seconds: 0.0,
            last_t: None,
            now: 0.0,
            stepped: Default::default(),
            reduce_motion: false,
            night: false,
            rest: 0.0,
            seen: Vec::new(),
            haptic: false,
            credits_open: false,
        })
    }

    /// The reading scale: 1.0 on a phone, up to 1.2 on a 13-inch iPad (Terra's rule).
    fn reading_for(width: u32, height: u32, ppp: f64) -> f64 {
        let short = (width.min(height) as f64) / ppp;
        (1.0 + 0.2 * ((short - 430.0) / (1032.0 - 430.0)).clamp(0.0, 1.0)).clamp(1.0, 1.2)
    }

    pub fn resize(&mut self, width: u32, height: u32, ppp: f64) {
        self.width = width;
        self.height = height;
        self.ppp = ppp;
        self.reading = Self::reading_for(width, height, ppp);
    }

    pub fn disc(&self) -> Disc {
        View::layout(self.width, self.height, self.ppp, self.camera.camera)
    }

    pub fn view(&self) -> View {
        View {
            width: self.width,
            height: self.height,
            ppp: self.ppp,
            reading: self.reading,
            disc: self.disc(),
            camera: self.camera.camera,
            seconds: self.seconds,
            clock: self.clock,
            rest: if self.clock.running { self.rest * self.rest } else { 1.0 - (1.0 - self.rest) * (1.0 - self.rest) },
            night: if self.night { 1.0 } else { 0.0 },
        }
    }

    fn motion(&self) -> Motion {
        Motion { reduce: self.reduce_motion }
    }

    /// What is under a point of the screen (points): the nearest float within reach and the place
    /// under the finger, the sea, the land, or nothing (the date's band is the probe's: touch/probe.rs).
    pub fn hit(&self, x: f64, y: f64) -> Hit {
        let disc = self.disc();
        let reach = crate::touch::motion::REACH * self.reading;
        let mut best: Option<(f64, u32, LonLat)> = None;
        for f in &self.floats {
            let (p, facing) = disc.project(&self.camera.camera, f.at);
            if facing < 0.05 {
                continue;
            }
            let d = ((p[0] - x).powi(2) + (p[1] - y).powi(2)).sqrt();
            if d < reach && best.map_or(true, |b| d < b.0) {
                best = Some((d, f.id, f.at));
            }
        }
        let place = disc.unproject(&self.camera.camera, x, y);
        if let Some((_, id, at)) = best {
            return Hit::Float(id, place.unwrap_or(at));
        }
        match place {
            Some(p) if self.rules.coast.is_land(p) => Hit::Land(p),
            Some(p) => Hit::Sea(p),
            None => Hit::Off,
        }
    }

    pub fn pointer(&mut self, ev: PointerEvent) {
        let mut gestures = std::mem::take(&mut self.gestures);
        let out = gestures.feed(ev, &*self);
        self.gestures = gestures;
        self.apply(out, ev.t);
    }

    /// A key the shell forwards (the web build, a hardware keyboard).
    pub fn key(&mut self, key: Key) {
        let out = self.gestures.key(key);
        self.apply(out, self.now);
    }

    /// A wheel or trackpad pinch: zoom by `factor` about the point.
    pub fn wheel(&mut self, x: f64, y: f64, factor: f64) {
        let mut gestures = std::mem::take(&mut self.gestures);
        let out = gestures.wheel(x, y, factor, &*self);
        self.gestures = gestures;
        self.apply(out, self.now);
    }

    /// The device was shaken: undo.
    pub fn shaken(&mut self) {
        let out = self.gestures.shaken();
        self.apply(out, self.now);
    }

    fn apply(&mut self, out: Out, now: f64) {
        let disc = self.disc();
        let motion = self.motion();
        for m in out.camera {
            self.camera.apply(&disc, m, now, motion);
        }
        for i in out.intents {
            self.intend(i);
        }
    }

    /// Applies one intent to the state.
    pub fn intend(&mut self, intent: Intent) {
        let name = match intent {
            Intent::Release { at: None, .. } => "ReleaseBack".to_string(),
            Intent::SetRunning { running: false } => "Pause".to_string(),
            Intent::SetRunning { running: true } => "Play".to_string(),
            other => format!("{other:?}").split(|c| c == ' ' || c == '{' || c == '(').next().unwrap_or("").to_string(),
        };
        if self.seen.last() != Some(&name) {
            self.seen.push(name);
        }
        let day = self.clock.day;
        match intent {
            Intent::Ring { at } => self.ripples.push(Ripple { at, when: self.seconds, strength: 1.0 }),
            Intent::Press { at, since } => self.pressing = Some((at, since)),
            Intent::Unpress => self.pressing = None,
            Intent::Put { at } => {
                self.pressing = None;
                self.haptic = true;
                let id = self.put(at);
                self.gestures.float_put(id);
                self.undo.push(Undo::Put(id));
                self.redo.clear();
                self.ripples.push(Ripple { at, when: self.seconds, strength: 0.6 });
            }
            Intent::Hold { float } => {
                self.haptic = true;
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == float) {
                    f.hand = Hand::Held;
                    f.lit_for = 0.0;
                    let from = f.clone();
                    self.undo.push(Undo::Moved { id: float, from });
                }
            }
            Intent::Carry { float, to } => {
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == float) {
                    f.hand = Hand::Carried;
                    f.at = to;
                }
            }
            Intent::Release { float, at } => {
                let Some(i) = self.floats.iter().position(|f| f.id == float) else { return };
                let moved = self.floats[i].hand == Hand::Carried;
                match (at, moved) {
                    (None, true) => {
                        // carried off the disc and let go: taken out in plain sight (undoable)
                        self.seen.push("TakeOut".to_string());
                        self.take_out(float);
                    }
                    (Some(p), true) => {
                        let f = &mut self.floats[i];
                        f.hand = Hand::Free;
                        f.lit_for = 0.0;
                        f.at = p;
                        f.put_day = day;
                        f.wake.clear();
                        f.held = Held::No;
                        let e = self.rules.put(p, day, f.seed, 0).eddy;
                        f.eddy = [e.0, e.1];
                        self.stepped.insert(float, day);
                        self.redo.clear();
                        self.ripples.push(Ripple { at: p, when: self.seconds, strength: 0.6 });
                    }
                    _ => {
                        // let go where it was, or over land, or past the beat without moving: back as
                        // it was lifted; nothing changed, nothing to undo
                        if let Some(Undo::Moved { id, from }) = self.undo.last().cloned() {
                            if id == float {
                                self.undo.pop();
                                let mut back = from;
                                back.hand = Hand::Free;
                                back.lit_for = 0.0;
                                self.floats[i] = back;
                                self.stepped.insert(float, day);
                                return;
                            }
                        }
                        let f = &mut self.floats[i];
                        f.hand = Hand::Free;
                        f.lit_for = 0.0;
                    }
                }
            }
            Intent::Undo => self.undo(),
            Intent::Redo => self.redo(),
            Intent::Ride { float } => {
                if self.camera.riding == Some(float) {
                    self.camera.riding = None;
                } else if self.floats.iter().any(|f| f.id == float) {
                    self.camera.riding = Some(float);
                    if let Some(f) = self.floats.iter_mut().find(|f| f.id == float) {
                        f.lit_for = 0.0;
                    }
                }
            }
            Intent::Scrub { days } => {
                self.clock.day += days;
                let day = self.clock.day;
                for f in &self.floats {
                    let s = self.stepped.entry(f.id).or_insert(day);
                    if *s > day {
                        *s = day;
                    }
                }
            }
            Intent::SetRunning { running } => self.clock.running = running,
            Intent::ToggleCredits => self.credits_open = !self.credits_open,
        }
    }

    fn put(&mut self, at: LonLat) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let seed = (self.clock.day.to_bits() ^ (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).rotate_left(17);
        // the water there is already moving: the first eddy velocity is drawn from the measured
        // variance (Truth's `Rules::put`), deterministic for the seed
        let e = self.rules.put(at, self.clock.day, seed, 0).eddy;
        self.floats.push(Float { id, at, put_day: self.clock.day, seed, held: Held::No, hand: Hand::Held, eddy: [e.0, e.1], measured: 1.0, wake: Vec::new(), lit_for: 0.0 });
        self.stepped.insert(id, self.clock.day);
        id
    }

    /// A float taken out past the rim (Motion's `Release { at: None }` while carried, or its own
    /// intent): undo gives it back as it was before the touch began.
    fn take_out(&mut self, float: u32) {
        let Some(i) = self.floats.iter().position(|f| f.id == float) else { return };
        let mut f = self.floats.remove(i);
        if let Some(Undo::Moved { id, from }) = self.undo.last().cloned() {
            if id == float {
                self.undo.pop();
                f = from;
            }
        }
        f.hand = Hand::Free;
        self.undo.push(Undo::Removed(f));
        self.redo.clear();
    }

    fn undo(&mut self) {
        let day = self.clock.day;
        match self.undo.pop() {
            Some(Undo::Put(id)) => {
                if let Some(i) = self.floats.iter().position(|f| f.id == id) {
                    let f = self.floats.remove(i);
                    self.redo.push(Undo::Removed(f));
                }
            }
            Some(Undo::Moved { id, from }) => {
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == id) {
                    let now = f.clone();
                    let mut back = from;
                    back.hand = Hand::Free;
                    back.lit_for = 0.0;
                    *f = back;
                    self.stepped.insert(id, day);
                    self.redo.push(Undo::Moved { id, from: now });
                }
            }
            Some(Undo::Removed(mut f)) => {
                f.hand = Hand::Free;
                f.lit_for = 0.0;
                self.stepped.insert(f.id, day);
                self.redo.push(Undo::Put(f.id));
                self.floats.push(f);
            }
            None => {}
        }
    }

    fn redo(&mut self) {
        let day = self.clock.day;
        match self.redo.pop() {
            Some(Undo::Removed(mut f)) => {
                f.hand = Hand::Free;
                f.lit_for = 0.0;
                self.stepped.insert(f.id, day);
                self.undo.push(Undo::Put(f.id));
                self.floats.push(f);
            }
            Some(Undo::Moved { id, from }) => {
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == id) {
                    let now = f.clone();
                    let mut to = from;
                    to.hand = Hand::Free;
                    to.lit_for = 0.0;
                    *f = to;
                    self.stepped.insert(id, day);
                    self.undo.push(Undo::Moved { id, from: now });
                }
            }
            Some(Undo::Put(id)) => {
                if let Some(i) = self.floats.iter().position(|f| f.id == id) {
                    let f = self.floats.remove(i);
                    self.undo.push(Undo::Removed(f));
                }
            }
            None => {}
        }
    }

    /// Time passes: `now` in the shell's seconds.
    pub fn advance(&mut self, now: f64) {
        let dt = match self.last_t {
            Some(t) => (now - t).clamp(0.0, 0.25),
            None => 0.0,
        };
        self.last_t = Some(now);
        self.last_dt = dt;
        self.now = now;
        let mut gestures = std::mem::take(&mut self.gestures);
        let out = gestures.tick(now, &*self);
        self.gestures = gestures;
        self.apply(out, now);
        // a float carried off the disc and let go is taken out (the gestures say so with a release
        // without a place while it is carried)
        let gone: Vec<u32> = self.floats.iter().filter(|f| f.hand == Hand::Carried && self.gestures.is_holding() != Some(f.id)).map(|f| f.id).collect();
        for id in gone {
            self.take_out(id);
        }
        if self.clock.running {
            self.clock.day += self.clock.rate * dt;
            if !self.reduce_motion {
                self.seconds += dt;
            }
        }
        self.step_floats();
        for f in &mut self.floats {
            f.lit_for += dt;
        }
        let ride_at = self.camera.riding.and_then(|id| self.floats.iter().find(|f| f.id == id).map(|f| f.at));
        if self.camera.riding.is_some() && ride_at.is_none() {
            self.camera.riding = None;
        }
        let motion = self.motion();
        self.camera.tick(now, dt, ride_at, self.floats.is_empty(), self.gestures.touching(), motion);
        let horizon = self.seconds - 6.0;
        self.ripples.retain(|r| r.when > horizon);
        // the sea settles to rest when the date is held (0.6 s, eased out) and wakes when it runs
        // again (0.3 s, eased in): Visual's D3
        if self.clock.running {
            self.rest = (self.rest - (dt / 0.3) as f32).max(0.0);
        } else {
            self.rest = (self.rest + (dt / 0.6) as f32).min(1.0);
        }
        // the maybes bloom out as a float is held (0.35 s, eased out) and fold back when it is let
        // go (0.25 s, eased in); the silver rises and sinks with them (Visual's numbers)
        let held = self.floats.iter().find(|f| f.hand == Hand::Held).map(|f| (f.id, f.at));
        match held {
            Some((_, at)) => {
                if self.bloom <= 0.0 {
                    self.drifter = match self.drifters.track(at, 500.0) {
                        Some(t) => {
                            // the month and a half each side of where the buoy passed nearest (the
                            // wake's own 45 days, so the two read as one scale), not its whole run
                            let w = t.window(45);
                            self.drifter_label = t.label_window(w.clone());
                            self.drifter_nearest = t.point.saturating_sub(w.start);
                            t.xyz[w].iter().map(|v| LonLat::from_xyz([v[0] as f64, v[1] as f64, v[2] as f64])).collect()
                        }
                        None => Vec::new(),
                    };
                }
                self.bloom = (self.bloom + (dt / 0.35) as f32).min(1.0);
            }
            None => {
                self.bloom = (self.bloom - (dt / 0.25) as f32).max(0.0);
                if self.bloom <= 0.0 {
                    self.drifter.clear();
                }
            }
        }
    }

    /// The bloom as drawn: eased out while opening, eased in while folding.
    fn bloom_eased(&self) -> f32 {
        let b = self.bloom;
        if self.floats.iter().any(|f| f.hand == Hand::Held) { 1.0 - (1.0 - b) * (1.0 - b) } else { b * b }
    }

    /// Every free float catches up with the clock, a quarter day at a time, a bead at each new day:
    /// the flight (Truth's rules), with the eddy memory the float carries.
    fn step_floats(&mut self) {
        let day = self.clock.day;
        for f in &mut self.floats {
            if f.hand == Hand::Carried {
                self.stepped.insert(f.id, day);
                continue;
            }
            let mut t = *self.stepped.get(&f.id).unwrap_or(&day);
            let mut n = 0;
            while t < day - 1e-9 && n < 240 {
                let dt = SUBSTEP.min(day - t);
                let next_day_mark = t.floor() + 1.0;
                let dt = dt.min(next_day_mark - t).max(1e-6);
                let s = self.rules.flight(Drift { at: f.at, eddy: (f.eddy[0], f.eddy[1]) }, t, dt, f.seed, 0);
                f.at = s.drift.at;
                f.eddy = [s.drift.eddy.0, s.drift.eddy.1];
                f.held = s.held;
                f.measured = s.measured;
                t += dt;
                if (t - next_day_mark).abs() < 1e-6 {
                    t = next_day_mark;
                    if f.wake.last().map_or(true, |b| b.day < next_day_mark - 0.5) {
                        f.wake.push(Bead { at: f.at, day: t, measured: f.measured });
                        if f.wake.len() > WAKE_MAX {
                            f.wake.remove(0);
                        }
                    }
                }
                n += 1;
            }
            self.stepped.insert(f.id, t);
        }
    }

    /// The float the camera drifts with, if any.
    pub fn riding(&self) -> Option<u32> {
        self.camera.riding
    }

    /// What the finger is doing (touch/gestures.rs `describe`).
    pub fn gesture_state(&self) -> &'static str {
        self.gestures.describe()
    }

    /// The state to keep, as JSON.
    pub fn snapshot(&self) -> String {
        let mut floats = self.floats.clone();
        for f in &mut floats {
            f.hand = Hand::Free;
            f.lit_for = 10.0;
        }
        serde_json::to_string(&Snapshot { version: 1, floats, next_id: self.next_id, day: self.clock.day, running: self.clock.running, camera: self.camera.camera }).unwrap_or_default()
    }

    /// Takes a kept state back; a state that can't be read is ignored.
    pub fn restore(&mut self, json: &str) -> bool {
        let Ok(s) = serde_json::from_str::<Snapshot>(json) else { return false };
        if s.version != 1 {
            return false;
        }
        self.floats = s.floats;
        self.next_id = s.next_id.max(self.floats.iter().map(|f| f.id + 1).max().unwrap_or(1));
        self.clock.day = s.day;
        self.clock.running = s.running;
        self.camera = CameraControl::new(s.camera);
        self.undo.clear();
        self.redo.clear();
        self.ripples.clear();
        self.stepped.clear();
        for f in &self.floats {
            self.stepped.insert(f.id, self.clock.day);
        }
        true
    }

    /// Sets the whole scene (the checks): the camera, the date, floats with their wakes simulated.
    pub fn set_scene(&mut self, scene: &Scene) {
        self.floats.clear();
        self.undo.clear();
        self.redo.clear();
        self.ripples.clear();
        self.stepped.clear();
        self.camera = CameraControl::new(scene.camera);
        self.gestures = Gestures::new();
        self.clock = Clock { day: scene.day, running: scene.running, rate: RATE };
        self.seconds = scene.seconds;
        self.night = scene.night;
        self.rest = if scene.running { 0.0 } else { 1.0 };
        for (i, sf) in scene.floats.iter().enumerate() {
            self.clock.day = scene.day - sf.days_ago;
            let id = self.put(LonLat::new(sf.lon, sf.lat));
            if let Some(f) = self.floats.iter_mut().find(|f| f.id == id) {
                f.hand = if scene.hold == Some(i) { Hand::Held } else { Hand::Free };
                f.lit_for = 10.0;
            }
        }
        self.clock.day = scene.day;
        self.step_floats();
        self.bloom = if scene.hold.is_some() { 1.0 } else { 0.0 };
        if let Some(f) = self.floats.iter().find(|f| f.hand == Hand::Held) {
            if let Some(t) = self.drifters.track(f.at, 500.0) {
                let w = t.window(45);
                self.drifter_label = t.label_window(w.clone());
                self.drifter_nearest = t.point.saturating_sub(w.start);
                self.drifter = t.xyz[w].iter().map(|v| LonLat::from_xyz([v[0] as f64, v[1] as f64, v[2] as f64])).collect();
            }
        }
    }

    /// What the look draws this frame, from the state (look/README.md › Wiring).
    fn look_state(&self) -> LookState {
        const MONTHS: [&str; 12] = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
        let (month, day) = self.clock.calendar();
        let resting = self.pressing.map(|(at, since)| (at, crate::touch::motion::thicken(self.now - since) as f32));
        LookState {
            floats: self
                .floats
                .iter()
                .map(|f| LookFloat { at: f.at, lit: (1.0 - (f.lit_for / 3.0)).clamp(0.0, 1.0) as f32, held: f.hand == Hand::Held, wake: f.wake.iter().map(|b| LookBead { at: b.at, measured: b.measured }).collect(), measured: f.measured })
                .collect(),
            resting,
            bloom: if self.maybes_of.is_some() || self.bloom > 0.0 { self.bloom_eased() } else { 0.0 },
            drifter: self.drifter.clone(),
            drifter_label: self.drifter_label.clone(),
            drifter_nearest: if self.drifter.is_empty() { None } else { Some(self.drifter_nearest.min(self.drifter.len() - 1)) },
            rise: self.bloom_eased(),
            night: self.night,
            date: format!("{} {}", day, MONTHS[(month - 1) as usize]),
            note: if self.clock.running { "a typical year, by the drifters’ record".to_string() } else { "held".to_string() },
            credits: crate::sea::CREDIT_LINE.to_string(),
            credits_full: crate::sea::CREDITS.iter().map(|s| s.to_string()).collect(),
            credits_open: self.credits_open,
        }
    }

    /// Draws a frame into `target`.
    pub fn render(&mut self, target: &wgpu::TextureView) {
        let view = self.view();
        let (ma, mb, _) = self.clock.months();
        match self.textures.set_months(&self.gpu, self.assets.as_ref(), ma, mb) {
            Ok(true) => self.ground.rebind(&self.gpu, &self.textures),
            Ok(false) => {}
            Err(e) => log::error!("land for month {ma}: {e}"),
        }
        let uniform = view.uniform();
        self.gpu.queue.write_buffer(&self.view_buffer, 0, bytemuck::bytes_of(&uniform));
        // the stage: the ground and the sea, redrawn when the view, the months or the sea's clock
        // moved (the clock at most thirty times a second), at the budget's scale
        let months = self.textures.land_months;
        let moved = match &self.stage_shown {
            Some((shown, m)) => {
                let (mut a, mut b) = (*shown, uniform);
                a.time[0] = 0.0;
                a.tone[2] = 0.0;
                b.time[0] = 0.0;
                b.tone[2] = 0.0;
                bytemuck::bytes_of(&a) != bytemuck::bytes_of(&b) || *m != months
            }
            None => true,
        };
        let sea_ticked = (self.clock.running || !self.ripples.is_empty()) && self.seconds - self.stage_seconds >= 1.0 / 30.0 - 1e-6;
        let wanted = |s: &Stage, scale: f64, w: u32, h: u32| s.scale == scale && s.width == ((w as f64 * scale).round() as u32).max(1) && s.height == ((h as f64 * scale).round() as u32).max(1);
        let rescaled = self.stage.as_ref().map_or(true, |s| !wanted(s, self.budget.scale, self.width, self.height));
        if rescaled {
            self.stage = Some(self.blit.stage(&self.gpu, self.width, self.height, self.budget.scale));
        }
        if moved || sea_ticked || rescaled {
            if moved && self.budget.frame(self.last_dt) {
                self.stage = Some(self.blit.stage(&self.gpu, self.width, self.height, self.budget.scale));
            }
            let stage = self.stage.as_ref().unwrap();
            let mut u = uniform;
            u.frame = [stage.width as f32, stage.height as f32, (stage.width as f64 / self.width as f64 * self.ppp) as f32, self.reading as f32];
            self.gpu.queue.write_buffer(&self.stage_view_buffer, 0, bytemuck::bytes_of(&u));
            self.sea.prepare(&self.gpu, &self.ripples);
            let mut enc = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("stage") });
            {
                let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("stage"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &stage.view,
                        resolve_target: None,
                        ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color { r: 0.96, g: 0.95, b: 0.92, a: 1.0 }), store: wgpu::StoreOp::Store },
                        depth_slice: None,
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                self.ground.draw(&mut rp, &self.stage_view_bind);
                self.sea.draw(&mut rp, &self.stage_view_bind);
            }
            self.gpu.queue.submit([enc.finish()]);
            self.stage_shown = Some((uniform, months));
            self.stage_seconds = self.seconds;
        }
        // a held float's maybes, run on the GPU from its put place and day to now (again as the day
        // moves): every path flies a quarter day at a time with the drifters' memory
        let held = self.floats.iter().find(|f| f.hand == Hand::Held).map(|f| (f.id, f.put_day, f.seed, f.wake.first().map_or(f.at, |b| b.at)));
        match held {
            Some((id, put_day, seed, origin)) => {
                let rerun = self.maybes_of.map_or(true, |(i, _, d)| i != id || (self.clock.day - d).abs() > 0.5);
                if rerun {
                    let n = if self.budget.scale < 0.75 { MAYBES / 2 } else { MAYBES };
                    let paths: Vec<GpuPath> = (1..=n)
                        .map(|k| {
                            let e = self.rules.put(origin, put_day, seed, k).eddy;
                            GpuPath { lon: origin.lon as f32, lat: origin.lat as f32, held: 0, measured: 1.0, eddy_u: e.0, eddy_v: e.1 }
                        })
                        .collect();
                    self.maybes.seed(&self.gpu, &paths);
                    let mut day = put_day;
                    let mut steps = 0;
                    while day < self.clock.day - 1e-6 && steps < 2000 {
                        let dt = MAYBES_STEP.min(self.clock.day - day);
                        self.maybes.step(&self.gpu, Params::new(&self.rules, day, dt, seed, n), true);
                        day += dt;
                        steps += 1;
                    }
                    self.maybes_of = Some((id, n, self.clock.day));
                }
            }
            None => {
                if self.bloom <= 0.0 {
                    self.maybes_of = None;
                }
            }
        }
        let state = self.look_state();
        let d = view.disc;
        self.look.prepare(&self.gpu, &state, (self.width as f64 / self.ppp) as f32, (self.height as f64 / self.ppp) as f32, self.reading as f32, [d.cx as f32, d.cy as f32, d.r as f32]);
        let mut enc = self.gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("frame") });
        {
            let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            self.blit.draw(&mut rp, self.stage.as_ref().unwrap());
            self.look.draw(&mut rp, &self.view_bind, &self.maybes, self.maybes_of.map_or(0, |m| m.1));
        }
        self.gpu.queue.submit([enc.finish()]);
    }

    /// A frame: time passes, then it is drawn.
    pub fn frame(&mut self, target: &wgpu::TextureView, now: f64) {
        self.advance(now);
        self.render(target);
    }

    /// Where the credit line is drawn, if the look has drawn it (x, y, w, h in points): the tap that
    /// opens the credits (Motion's probe).
    pub fn credit_bounds(&self) -> Option<[f32; 4]> {
        self.look.credit_bounds()
    }

    /// Where the date is drawn, if the look has drawn it (x, y, w, h in points): Motion's date band.
    pub fn date_bounds(&self) -> Option<[f32; 4]> {
        self.look.date_bounds()
    }
}
