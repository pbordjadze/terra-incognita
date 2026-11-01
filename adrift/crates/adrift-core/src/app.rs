//! The app: the state (floats, the clock, the camera, undo), a frame, a pointer event, and applying
//! intents (Engineering; DECISIONS.md §2). The seams: Truth's `Rules` step the floats and its
//! `Maybes` run a held float's other selves on the GPU; Visual's `Look` draws the marks and the words
//! from a `LookState` filled here; Motion's `Gestures` turn pointer events into intents through the
//! `Probe` the app answers (touch/probe.rs), and its `CameraControl` owns the camera's motion.

use crate::assets::Assets;
use crate::gpu::{Blit, Budget, Gpu, Stage, Textures, View, ViewUniform};
use crate::look::{sea::Ripple, GroundPass, Look, LookBead, LookCloud, LookFloat, LookState, SeaPass};
use crate::model::*;
use crate::sea::{Drift, Drifters, GpuPath, Maybes, Params, Rules};
use crate::touch::{CameraControl, Gestures, Key, Motion, Out};
use serde::{Deserialize, Serialize};

/// What an export puts aside of the live app, to put back (R9).
pub struct Stash {
    pub(crate) start: crate::studio::TakeStart,
    undo: Vec<Undo>,
    redo: Vec<Undo>,
    budget: (bool, f64),
    size: (u32, u32, f64),
    recording: Option<crate::studio::Take>,
    now: f64,
    /// How many intents had been seen (the replay's own are dropped after: Motion's R9 note).
    seen: usize,
}

/// What the reader has done, to take back.
#[derive(Clone, Debug)]
enum Undo {
    Put(u32),
    Moved { id: u32, from: Float },
    Removed(Float),
    /// Every float taken out at once (`Intent::ClearAll`); on the redo stack, the clearing itself.
    Cleared(Vec<Float>),
    /// A conductor's cue (R11): the lamps as they were before it.
    Conducted(Vec<Float>),
}

/// The opening view: the whole Earth, the Atlantic facing (`Intent::Home` flies back to it).
/// A ridden float shows its maybes and its nearest drifter as a held one does (R4).
pub const SHOW_RIDDEN: bool = true;

pub const HOME: Camera = Camera::new(-30.0, 25.0, 1.0);

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
    /// Floats chosen, their clouds out, by index in `floats`, in order (the last in front) (R6).
    #[serde(default)]
    pub choose: Vec<usize>,
    /// Floats chosen as themselves, their journeys followed, by index in `floats` (R8).
    #[serde(default)]
    pub follow: Vec<usize>,
    /// The drawn motion's seconds.
    #[serde(default)]
    pub seconds: f64,
    /// The page of credits open (R4: a scene can photograph it without a touch).
    #[serde(default)]
    pub credits_open: bool,
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
    /// Truth's sampler: the shown floats' maybes on the GPU, one range of the buffer a cloud (R6).
    maybes: Maybes,
    /// The clouds out (R6: several at once), in the order shown, the front one last.
    clouds: Vec<Cloud>,
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
    /// A float under a resting finger, from when (`Intent::PressFloat`): its maybes show from the
    /// landing; `Unpress` clears it.
    pressing_float: Option<(u32, f64)>,
    /// The chosen floats (R5; several since R6, the last in front): their maybes stay shown whatever
    /// the camera does, until the sea is tapped (`Unchoose`), one is let go (`ChooseAlso` again) or
    /// taken out. The front one's nearest drifter shows.
    pub chosen: Vec<u32>,
    /// Floats chosen as themselves (R8), the front last: their journeys followed, no cloud. A float
    /// is in `chosen` or `followed`, never both.
    pub followed: Vec<u32>,
    /// A few of each chosen cloud's maybes stepped on the CPU (k = 1..=`FRAME_SAMPLES`), so the
    /// camera can hold every chosen cloud in view (`frame_points`) without reading the GPU back.
    samples: std::collections::HashMap<(u32, u32), Path>,
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
    /// Every leg's path as computed so far (R5): a float's place at any date is read here.
    paths: std::collections::HashMap<(u32, usize), Path>,
    pub reduce_motion: bool,
    /// The system's dark appearance: the look's night (the shells set it).
    pub night: bool,
    /// How far the sea has settled to rest with the date held, 0..1.
    rest: f32,
    /// The intents applied, by name (the checks' `expect.saw`).
    pub seen: Vec<String>,
    /// A Put or a Hold happened since the shell last asked: the beat, felt (a light haptic).
    pub haptic: bool,
    /// The frames since the shell last asked for the stats: their count, the sum and the worst of
    /// their intervals, and how many redrew the stage.
    frames: (u32, f64, f64, u32),
    /// The page of credits is open (Visual's D6; opened and closed by `Intent::ToggleCredits`).
    pub credits_open: bool,
    /// The studio (R9): the take being recorded, the takes kept, an export under way.
    pub studio: crate::studio::Studio,
    /// The frame being drawn goes into an exported clip (R9; the look's `exporting`).
    exporting: bool,
    /// While exporting, how far through the clip's still opening beat (0..1, 1 once past it) and its
    /// eased last one (0 until it, then 0..1) (R10; the look's `opening` and `ending`).
    clip_phase: (f32, f32),
    /// Each lamp's worlds while the hand conducts (R11, `conduct.rs`), by lamp id.
    conduct: std::collections::HashMap<u32, crate::conduct::Worlds>,
    /// The song the shell has, by name (R12): it plays from its start while a take records.
    song: Option<String>,
    /// The cues of the last seconds, for the look (R11).
    cues: Vec<(crate::look::LookCue, f64)>,
    /// The panel's rows as a check holds them, whatever the look draws (Motion's D11, R6): when set,
    /// `panel_bounds()` returns these. Nothing in the app sets it.
    pub panel_stand_in: Option<Vec<(u32, [f32; 4])>>,
}

/// Days a second while the date runs.
pub const RATE: f64 = 2.0;
/// The longest step a float takes at once (a quarter day: the fastest currents cross less than a cell).
const SUBSTEP: f64 = 0.25;
/// Beads kept in a wake.
const WAKE_MAX: usize = 400;
/// Maybes run for a shown float, and their step: 512 at a half day already tell the shape 8192 at a
/// quarter day do (Truth's budget, SOURCES.md › How many maybes), and more are more honest, never
/// less: 2048 so the cloud reads as a field at the whole Earth (R5); a whole day is not honest in a jet.
pub const MAYBES: u32 = 2048;
const MAYBES_STEP: f64 = 0.5;
/// The most flight steps the floats take in one frame (the rest wait for the next): a scrub of a
/// hundred days with thirty floats catches up over a few frames instead of stalling one (R5).
const FLIGHT_BUDGET: usize = 1500;
/// The most steps the shown float's maybes take in one frame (half-day steps: twelve days a frame).
const MAYBES_BUDGET: usize = 24;
/// A keep of the maybes' paths every so many days, so a rewind is a restore and a few steps.
const KEEP_EVERY: f64 = 10.0;
const KEEPS_MAX: usize = 40;
/// The most clouds out at once (R6): each its own `MAYBES` range of one buffer; past this the
/// earliest chosen folds away. Eight at 2048 is 16 384 maybes, 24 steps a cloud a frame.
pub const MAX_CLOUDS: usize = 8;
/// Maybes a chosen cloud steps on the CPU for the camera's framing (R8).
pub const FRAME_SAMPLES: u32 = 16;

/// One cloud of maybes out (R6): the float's, seeded from its leg, in buffer slot `slot`.
struct Cloud {
    id: u32,
    /// The leg it was seeded from (its put day and seed) and its place.
    leg: Leg,
    slot: u32,
    /// How many maybes, and the day they have reached.
    n: u32,
    reached: f64,
    /// Keeps of its paths, oldest first, and the day the next is due.
    keeps: Vec<Kept>,
    next_keep: f64,
    /// 0..1 as it blooms out and folds back.
    bloom: f32,
    /// Shown this frame (else folding away).
    wanted: bool,
}

/// One sample of a leg's path on the quarter-day grid.
#[derive(Clone, Copy, Debug)]
struct Sample {
    day: f64,
    at: LonLat,
    eddy: [f32; 2],
    measured: f32,
    held: Held,
}

/// A leg's path as computed so far (`App.paths`, keyed by float id and leg index; a leg that
/// changed under its key is recomputed).
struct Path {
    leg: Leg,
    samples: Vec<Sample>,
}

/// A keep of the maybes' paths at a day (R5; Truth's `Maybes::keep`).
struct Kept {
    day: f64,
    keep: crate::sea::Keep,
}

/// The next quarter-day mark after `t` (t itself on the grid: the one after it).
pub(crate) fn grid_next(t: f64) -> f64 {
    let q = (t / SUBSTEP).floor() * SUBSTEP + SUBSTEP;
    if q - t < 1e-9 {
        q + SUBSTEP
    } else {
        q
    }
}

/// A place between two samples, the longitude the short way round.
pub(crate) fn lerp_place(a: LonLat, b: LonLat, f: f64) -> LonLat {
    let mut dlon = b.lon - a.lon;
    if dlon > 180.0 {
        dlon -= 360.0;
    } else if dlon < -180.0 {
        dlon += 360.0;
    }
    let mut lon = a.lon + dlon * f;
    if lon > 180.0 {
        lon -= 360.0;
    } else if lon < -180.0 {
        lon += 360.0;
    }
    LonLat { lon, lat: a.lat + (b.lat - a.lat) * f }
}

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
        let maybes = Maybes::new(&gpu, assets.as_ref(), &textures, MAYBES * MAX_CLOUDS as u32)?;
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
            clouds: Vec::new(),
            budget,
            stage_shown: None,
            stage_seconds: -1.0,
            last_dt: 0.0,
            floats: Vec::new(),
            next_id: 1,
            clock,
            undo: Vec::new(),
            redo: Vec::new(),
            camera: CameraControl::new(Camera::new(-30.0, 25.0, 1.0)),
            gestures: Gestures::new(),
            pressing: None,
            pressing_float: None,
            chosen: Vec::new(),
            followed: Vec::new(),
            samples: Default::default(),
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
            paths: Default::default(),
            reduce_motion: false,
            night: false,
            rest: 0.0,
            seen: Vec::new(),
            haptic: false,
            credits_open: false,
            frames: (0, 0.0, 0.0, 0),
            panel_stand_in: None,
            studio: Default::default(),
            exporting: false,
            clip_phase: (1.0, 0.0),
            conduct: Default::default(),
            song: None,
            cues: Vec::new(),
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
            if !f.born(self.clock.day) {
                continue;
            }
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
        self.record(crate::studio::TakeEvent::Pointer(ev));
        let mut gestures = std::mem::take(&mut self.gestures);
        let out = gestures.feed(ev, &*self);
        self.gestures = gestures;
        self.apply(out, ev.t);
    }

    /// A key the shell forwards (the web build, a hardware keyboard).
    pub fn key(&mut self, key: Key) {
        self.record(crate::studio::TakeEvent::Key { t: self.now, key });
        let out = self.gestures.key(key);
        self.apply(out, self.now);
    }

    /// A wheel or trackpad pinch: zoom by `factor` about the point.
    pub fn wheel(&mut self, x: f64, y: f64, factor: f64) {
        self.record(crate::studio::TakeEvent::Wheel { t: self.now, x, y, factor });
        let mut gestures = std::mem::take(&mut self.gestures);
        let out = gestures.wheel(x, y, factor, &*self);
        self.gestures = gestures;
        self.apply(out, self.now);
    }

    /// The device was shaken: undo.
    pub fn shaken(&mut self) {
        self.record(crate::studio::TakeEvent::Shake { t: self.now });
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
            // (D10: a ring at touch-down keeps the choice; `Unchoose` is the tap's carrier)
            Intent::Ring { at } => self.ripples.push(Ripple { at, when: self.seconds, strength: 1.0 }),
            Intent::Choose { float } => {
                if self.floats.iter().any(|f| f.id == float) {
                    self.chosen = vec![float];
                    self.followed.retain(|&c| c != float);
                }
            }
            Intent::Show { float, shown } => {
                if self.floats.iter().any(|f| f.id == float) {
                    self.chosen.retain(|&c| c != float);
                    self.followed.retain(|&c| c != float);
                    match shown {
                        Some(Shown::Cloud) => self.chosen.push(float),
                        Some(Shown::Lamp) => self.followed.push(float),
                        None => {
                            if self.camera.riding == Some(float) {
                                self.camera.riding = None;
                            }
                        }
                    }
                }
            }
            Intent::ChooseAlso { float } => {
                if let Some(i) = self.chosen.iter().position(|&c| c == float) {
                    self.chosen.remove(i);
                    if self.camera.riding == Some(float) {
                        self.camera.riding = None;
                    }
                } else if self.floats.iter().any(|f| f.id == float) {
                    self.followed.retain(|&c| c != float);
                    self.chosen.push(float);
                }
            }
            Intent::Record { on } => {
                if self.studio.replay.is_none() {
                    match (on, self.studio.recording.is_some()) {
                        (true, false) => self.begin_take(self.now),
                        (false, true) => self.end_take(self.now),
                        _ => {}
                    }
                }
            }
            Intent::Conduct { float, gesture } => self.cue(float, gesture),
            Intent::TakeSpeed { speed } => {
                if let Some(t) = self.studio.takes.last_mut() {
                    t.speed = speed.clamp(0.25, 2.0);
                }
            }
            Intent::Export => {
                if self.studio.replay.is_none() && !self.studio.takes.is_empty() {
                    self.studio.export_wanted = true;
                }
            }
            Intent::Unchoose => {
                self.chosen.clear();
                self.followed.clear();
            }
            Intent::Press { at, since } => self.pressing = Some((at, since)),
            Intent::PressFloat { float, since } => self.pressing_float = Some((float, since)),
            Intent::Unpress => {
                self.pressing = None;
                self.pressing_float = None;
            }
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
                        f.held = Held::No;
                        // a new leg from here, its own seed: the path before it stays what it was
                        let mut legs = f.legs();
                        f.seed = (f.seed ^ day.to_bits()).rotate_left(23).wrapping_mul(0x9E37_79B9_7F4A_7C15);
                        legs.push(Leg::new(p, day, f.seed));
                        f.legs = legs;
                        let e = self.rules.put(p, day, f.seed, 0).eddy;
                        f.eddy = [e.0, e.1];
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
                    // it joins the chosen, in front (R6: the others stay out); one chosen as itself
                    // stays so, followed without a cloud (R8)
                    if self.followed.contains(&float) {
                        self.followed.retain(|&c| c != float);
                        self.followed.push(float);
                    } else {
                        self.chosen.retain(|&c| c != float);
                        self.chosen.push(float);
                    }
                    if let Some(f) = self.floats.iter_mut().find(|f| f.id == float) {
                        f.lit_for = 0.0;
                    }
                }
            }
            Intent::Scrub { days } => self.clock.day += days,
            Intent::SetRunning { running } => self.clock.running = running,
            Intent::ToggleCredits => self.credits_open = !self.credits_open,
            Intent::ClearAll => self.clear_all(),
            Intent::Home => {
                self.camera.riding = None;
                let motion = crate::touch::motion::Motion { reduce: self.reduce_motion };
                self.camera.fly_to(HOME, self.now, motion);
            }
        }
    }

    fn put(&mut self, at: LonLat) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        let seed = (self.clock.day.to_bits() ^ (id as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)).rotate_left(17);
        // the water there is already moving: the first eddy velocity is drawn from the measured
        // variance (Truth's `Rules::put`), deterministic for the seed
        let e = self.rules.put(at, self.clock.day, seed, 0).eddy;
        self.floats.push(Float { id, at, put_day: self.clock.day, seed, held: Held::No, hand: Hand::Held, eddy: [e.0, e.1], measured: 1.0, wake: Vec::new(), trail: Vec::new(), lit_for: 0.0, legs: vec![Leg::new(at, self.clock.day, seed)] });
        id
    }

    /// A float taken out past the rim (Motion's `Release { at: None }` while carried, or its own
    /// intent): undo gives it back as it was before the touch began.
    fn take_out(&mut self, float: u32) {
        let Some(i) = self.floats.iter().position(|f| f.id == float) else { return };
        let mut f = self.floats.remove(i);
        self.chosen.retain(|&c| c != float);
        self.followed.retain(|&c| c != float);
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

    /// Every float taken out as one touch (R4): undo gives them all back where they were.
    fn clear_all(&mut self) {
        if self.floats.is_empty() {
            return;
        }
        let mut gone = std::mem::take(&mut self.floats);
        for f in &mut gone {
            f.hand = Hand::Free;
        }
        self.pressing = None;
        self.pressing_float = None;
        self.chosen.clear();
        self.followed.clear();
        self.camera.riding = None;
        self.undo.push(Undo::Cleared(gone));
        self.redo.clear();
    }

    fn undo(&mut self) {
        match self.undo.pop() {
            Some(Undo::Cleared(gone)) => {
                for mut f in gone {
                    f.hand = Hand::Free;
                    f.lit_for = 0.0;
                    self.floats.push(f);
                }
                self.redo.push(Undo::Cleared(Vec::new()));
            }
            Some(Undo::Put(id)) => {
                if let Some(i) = self.floats.iter().position(|f| f.id == id) {
                    let f = self.floats.remove(i);
                    self.redo.push(Undo::Removed(f));
                }
            }
            Some(Undo::Conducted(before)) => {
                let now = self.swap_floats(before);
                self.redo.push(Undo::Conducted(now));
            }
            Some(Undo::Moved { id, from }) => {
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == id) {
                    let now = f.clone();
                    let mut back = from;
                    back.hand = Hand::Free;
                    back.lit_for = 0.0;
                    *f = back;
                    self.redo.push(Undo::Moved { id, from: now });
                }
            }
            Some(Undo::Removed(mut f)) => {
                f.hand = Hand::Free;
                f.lit_for = 0.0;
                self.redo.push(Undo::Put(f.id));
                self.floats.push(f);
            }
            None => {}
        }
    }

    fn redo(&mut self) {
        match self.redo.pop() {
            Some(Undo::Cleared(_)) => {
                let mut gone = std::mem::take(&mut self.floats);
                for f in &mut gone {
                    f.hand = Hand::Free;
                }
                self.camera.riding = None;
                self.undo.push(Undo::Cleared(gone));
            }
            Some(Undo::Conducted(after)) => {
                let now = self.swap_floats(after);
                self.undo.push(Undo::Conducted(now));
            }
            Some(Undo::Removed(mut f)) => {
                f.hand = Hand::Free;
                f.lit_for = 0.0;
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
        self.record(crate::studio::TakeEvent::Frame { t: now });
        let dt = match self.last_t {
            Some(t) => (now - t).clamp(0.0, 0.25),
            None => 0.0,
        };
        self.last_t = Some(now);
        self.last_dt = dt;
        self.now = now;
        if dt > 0.0 {
            self.frames.0 += 1;
            self.frames.1 += dt;
            self.frames.2 = self.frames.2.max(dt);
        }
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
        self.step_worlds();
        let seconds = self.seconds;
        self.cues.retain(|(_, when)| seconds - when < 2.0);
        for f in &mut self.floats {
            f.lit_for += dt;
        }
        let ride_at = self.camera.riding.and_then(|id| self.floats.iter().find(|f| f.id == id && f.born(self.clock.day)).map(|f| f.at));
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
        // every shown float's cloud blooms out (0.35 s) and folds back when it is no longer shown
        // (0.25 s); a folded cloud gives its slot back (R6)
        self.open_clouds();
        for c in &mut self.clouds {
            c.bloom = if c.wanted { (c.bloom + (dt / 0.35) as f32).min(1.0) } else { (c.bloom - (dt / 0.25) as f32).max(0.0) };
        }
        self.clouds.retain(|c| c.wanted || c.bloom > 0.0);
        // the front's maybes bloom out as a float is held (0.35 s, eased out) and fold back when it
        // is let go (0.25 s, eased in); the silver rises and sinks with them (Visual's numbers)
        let held = self.shown().map(|f| (f.id, f.at));
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
        if self.shown().is_some() { 1.0 - (1.0 - b) * (1.0 - b) } else { b * b }
    }

    /// The float in front, whose nearest drifter shows: the one held by a finger, else the one
    /// pressed, else the last chosen, else the one the camera rides (R4: following shows its maybes;
    /// `SHOW_RIDDEN` is the switch).
    fn shown(&self) -> Option<&Float> {
        let by_id = |id: u32| self.floats.iter().find(|f| f.id == id);
        self.floats
            .iter()
            .find(|f| f.hand == Hand::Held)
            .or_else(|| self.pressing_float.and_then(|(id, _)| by_id(id)))
            .or_else(|| self.chosen.last().copied().and_then(by_id))
            .or_else(|| if SHOW_RIDDEN { self.camera.riding.filter(|id| !self.followed.contains(id)).and_then(by_id) } else { None })
    }

    /// Every float whose cloud is out (R6), the front one last: the chosen in order, the ridden, the
    /// pressed, the held; at most `MAX_CLOUDS` (the earliest chosen give way).
    pub fn shown_ids(&self) -> Vec<u32> {
        let mut ids: Vec<u32> = self.chosen.clone();
        let front = |ids: &mut Vec<u32>, id: u32| {
            ids.retain(|&i| i != id);
            ids.push(id);
        };
        if SHOW_RIDDEN {
            if let Some(id) = self.camera.riding {
                // (a ridden float chosen as itself shows no cloud: R8)
                if !ids.contains(&id) && !self.followed.contains(&id) {
                    ids.push(id);
                }
            }
        }
        if let Some((id, _)) = self.pressing_float {
            front(&mut ids, id);
        }
        if let Some(f) = self.floats.iter().find(|f| f.hand == Hand::Held) {
            front(&mut ids, f.id);
        }
        let day = self.clock.day;
        ids.retain(|id| self.floats.iter().any(|f| f.id == *id && f.born(day)));
        if ids.len() > MAX_CLOUDS {
            ids.drain(0..ids.len() - MAX_CLOUDS);
        }
        ids
    }

    /// Marks which clouds are wanted, gives a slot to a newly shown float's, and orders them as
    /// shown (the front last); returns the ids shown.
    fn open_clouds(&mut self) -> Vec<u32> {
        let ids = self.shown_ids();
        for c in &mut self.clouds {
            c.wanted = ids.contains(&c.id);
        }
        for &id in &ids {
            if !self.clouds.iter().any(|c| c.id == id) {
                if let Some(slot) = (0..MAX_CLOUDS as u32).find(|s| !self.clouds.iter().any(|c| c.slot == *s)) {
                    let leg = self.floats.iter().find(|f| f.id == id).and_then(|f| f.leg_at(self.clock.day).map(|li| f.legs[li])).unwrap_or(Leg::new(LonLat::new(0.0, 0.0), f64::NAN, 0));
                    self.clouds.push(Cloud { id, leg, slot, n: 0, reached: f64::INFINITY, keeps: Vec::new(), next_keep: 0.0, bloom: 0.0, wanted: true });
                }
            }
        }
        // in the order shown, the front one last (a folding one first)
        self.clouds.sort_by_key(|c| ids.iter().position(|&i| i == c.id).map_or(0, |p| p + 1));
        ids
    }

    /// The days the clouds out are still behind the date, the most of any (R6: 0 when caught up;
    /// a scrub faster than `MAYBES_BUDGET` half-days a frame shows here).
    pub fn behind(&self) -> f64 {
        let day = self.clock.day;
        self.clouds.iter().filter(|c| c.wanted && c.reached.is_finite()).map(|c| (day.max(c.leg.day) - c.reached).abs()).fold(0.0, f64::max)
    }

    /// The record control's rect as the look drew it (R9; Motion's probe), x, y, w, h in points.
    pub fn record_bounds(&self) -> Option<[f32; 4]> {
        self.look.record_bounds()
    }

    /// The panel's rows as the look drew them (R6): each lamp's id and rect in points.
    /// Each panel mark's words for VoiceOver, in `panel_bounds`'s order (R10: the marks carry no
    /// text on screen).
    pub fn panel_words(&self) -> Vec<(u32, String)> {
        self.look.panel_words()
    }

    pub fn panel_bounds(&self) -> Vec<(u32, [f32; 4])> {
        if let Some(rows) = &self.panel_stand_in {
            return rows.clone();
        }
        self.look.panel_bounds()
    }

    /// Every float not in the hand is where its path says it is on the clock's day (R5): the leg
    /// active then, stepped on the quarter-day grid from its put with Truth's flight, cached, the
    /// place between samples interpolated; the wake is the path's day marks. At most `budget`
    /// flight steps are taken this frame; what remains waits for the next, so a long scrub catches
    /// up over a few frames instead of stalling one.
    fn step_floats(&mut self) {
        self.step_floats_with(FLIGHT_BUDGET);
        self.step_samples(FLIGHT_BUDGET / 2);
    }

    fn step_floats_with(&mut self, mut budget: usize) {
        let day = self.clock.day;
        let mut floats = std::mem::take(&mut self.floats);
        for f in &mut floats {
            if f.hand == Hand::Carried {
                continue;
            }
            if f.legs.is_empty() {
                f.legs = f.legs();
            }
            let Some(li) = f.leg_at(day) else { continue };
            // the earlier legs' paths, up to the day the next leg began (their beads stay drawn)
            for (i, leg) in f.legs.iter().enumerate().take(li) {
                self.ensure_path(f.id, i, *leg, f.legs[i + 1].day, &mut budget);
            }
            let leg = f.legs[li];
            self.ensure_path(f.id, li, leg, day, &mut budget);
            let path = &self.paths[&(f.id, li)];
            let k = path.samples.partition_point(|s| s.day <= day + 1e-9).saturating_sub(1);
            let a = path.samples[k];
            if let Some(b) = path.samples.get(k + 1) {
                let fr = ((day - a.day) / (b.day - a.day)).clamp(0.0, 1.0);
                f.at = lerp_place(a.at, b.at, fr);
            } else {
                f.at = a.at;
            }
            f.eddy = a.eddy;
            f.measured = a.measured;
            f.held = a.held;
            // the wake: a bead at every day mark of every leg up to now
            f.wake.clear();
            f.trail.clear();
            for i in 0..=li {
                let until = if i < li { f.legs[i + 1].day } else { day };
                if let Some(p) = self.paths.get(&(f.id, i)) {
                    for s in &p.samples {
                        if s.day > until + 1e-9 {
                            break;
                        }
                        if s.day.fract() == 0.0 {
                            f.wake.push(Bead { at: s.at, day: s.day, measured: s.measured });
                        }
                        f.trail.push(Bead { at: s.at, day: s.day, measured: s.measured });
                    }
                }
            }
            if f.wake.len() > WAKE_MAX {
                f.wake.drain(0..f.wake.len() - WAKE_MAX);
            }
            if let Some(first) = f.wake.first().map(|b| b.day) {
                let cut = f.trail.iter().position(|b| b.day >= first - 1e-9).unwrap_or(0);
                f.trail.drain(0..cut);
            }
        }
        self.floats = floats;
    }

    /// A leg's path computed to `upto` (as far as the budget allows this frame).
    fn ensure_path(&mut self, id: u32, li: usize, leg: Leg, upto: f64, budget: &mut usize) {
        let path = self.paths.entry((id, li)).or_insert_with(|| Path { leg, samples: Vec::new() });
        Self::extend_path(&self.rules, path, leg, leg.world, upto, budget);
    }

    /// A path of leg `leg` for maybe `k` (0 the float itself) stepped to `upto` on the quarter-day grid.
    fn extend_path(rules: &Rules, path: &mut Path, leg: Leg, k: u32, upto: f64, budget: &mut usize) {
        if path.leg != leg || path.samples.is_empty() {
            let e = match leg.eddy {
                Some(e) => (e[0], e[1]),
                None => rules.put(leg.at, leg.day, leg.seed, k).eddy,
            };
            path.leg = leg;
            path.samples.clear();
            path.samples.push(Sample { day: leg.day, at: leg.at, eddy: [e.0, e.1], measured: 1.0, held: Held::No });
        }
        while *budget > 0 {
            let last = *path.samples.last().unwrap();
            if last.day >= upto - 1e-9 {
                break;
            }
            let next = grid_next(last.day);
            let s = rules.flight(Drift { at: last.at, eddy: (last.eddy[0], last.eddy[1]) }, last.day, next - last.day, leg.seed, k);
            path.samples.push(Sample { day: next, at: s.drift.at, eddy: [s.drift.eddy.0, s.drift.eddy.1], measured: s.measured, held: s.held });
            *budget -= 1;
        }
    }

    /// Steps the chosen clouds' CPU samples toward the date (R8), within what the floats' budget
    /// left this frame; samples of clouds no longer chosen are dropped.
    fn step_samples(&mut self, mut budget: usize) {
        let day = self.clock.day;
        let chosen = self.chosen.clone();
        self.samples.retain(|(id, _), _| chosen.contains(id));
        for id in chosen {
            let Some(leg) = self.floats.iter().find(|f| f.id == id).and_then(|f| f.leg_at(day).map(|li| f.legs[li])) else { continue };
            for k in 1..=FRAME_SAMPLES {
                let path = self.samples.entry((id, k)).or_insert_with(|| Path { leg, samples: Vec::new() });
                Self::extend_path(&self.rules, path, leg, k, day, &mut budget);
            }
        }
    }

    /// How the float is chosen, if it is (R8).
    pub fn shown_as(&self, id: u32) -> Option<Shown> {
        if self.chosen.contains(&id) {
            Some(Shown::Cloud)
        } else if self.followed.contains(&id) {
            Some(Shown::Lamp)
        } else {
            None
        }
    }

    /// The places the camera should hold in view to frame everything chosen (R8): each followed
    /// lamp's place, and each chosen cloud's lamp and `FRAME_SAMPLES` of its maybes on the date
    /// (stepped on the CPU, so a scrub's first frames may lag a little behind the drawn cloud).
    pub fn frame_points(&self) -> Vec<LonLat> {
        let day = self.clock.day;
        let mut out = Vec::new();
        for f in self.floats.iter().filter(|f| f.born(day) && (self.chosen.contains(&f.id) || self.followed.contains(&f.id))) {
            out.push(f.at);
        }
        for ((_, _), p) in self.samples.iter().filter(|((id, _), _)| self.chosen.contains(id)) {
            if p.samples.is_empty() || p.leg.day > day + 1e-9 {
                continue;
            }
            let k = p.samples.partition_point(|s| s.day <= day + 1e-9).saturating_sub(1);
            let a = p.samples[k];
            out.push(match p.samples.get(k + 1) {
                Some(b) => lerp_place(a.at, b.at, ((day - a.day) / (b.day - a.day)).clamp(0.0, 1.0)),
                None => a.at,
            });
        }
        out
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
        self.paths.clear();
        self.step_floats_with(usize::MAX);
        true
    }

    /// Sets the whole scene (the checks): the camera, the date, floats with their wakes simulated.
    pub fn set_scene(&mut self, scene: &Scene) {
        self.floats.clear();
        self.paths.clear();
        self.undo.clear();
        self.redo.clear();
        self.ripples.clear();
        self.camera = CameraControl::new(scene.camera);
        self.gestures = Gestures::new();
        self.clock = Clock { day: scene.day, running: scene.running, rate: RATE };
        self.seconds = scene.seconds;
        self.night = scene.night;
        self.credits_open = scene.credits_open;
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
        self.step_floats_with(usize::MAX);
        let ids: Vec<u32> = self.floats.iter().map(|f| f.id).collect();
        self.chosen = scene.choose.iter().filter_map(|&i| ids.get(i).copied()).collect();
        self.followed = scene.follow.iter().filter_map(|&i| ids.get(i).copied()).filter(|id| !self.chosen.contains(id)).collect();
        self.samples.clear();
        self.step_samples(usize::MAX);
        self.clouds.clear();
        self.bloom = if scene.hold.is_some() || !self.chosen.is_empty() { 1.0 } else { 0.0 };
        // the clouds out at full bloom, caught up to the date (a scene is a still)
        self.open_clouds();
        for c in &mut self.clouds {
            c.bloom = 1.0;
        }
        self.step_clouds(usize::MAX);
        if let Some(f) = self.shown() {
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
        let front = self.shown().map(|f| f.id);
        let ease = |b: f32, opening: bool| if opening { 1.0 - (1.0 - b) * (1.0 - b) } else { b * b };
        let clouds: Vec<LookCloud> = self
            .clouds
            .iter()
            .filter(|c| c.n > 0)
            .filter_map(|c| {
                let f = self.floats.iter().find(|f| f.id == c.id)?;
                Some(LookCloud { id: c.id, at: f.at, origin: c.leg.at, base: c.slot * MAYBES, count: c.n, bloom: ease(c.bloom, c.wanted), front: front == Some(c.id) })
            })
            .collect();
        LookState {
            floats: self
                .floats
                .iter()
                .filter(|f| f.born(self.clock.day))
                .map(|f| {
                    let first = f.legs.first().copied().unwrap_or(Leg::new(f.at, f.put_day, f.seed));
                    LookFloat { id: f.id, at: f.at, lit: (1.0 - (f.lit_for / 3.0)).clamp(0.0, 1.0) as f32, held: f.hand == Hand::Held, chosen: front == Some(f.id), shown: clouds.iter().any(|c| c.id == f.id), followed: self.followed.contains(&f.id), origin: first.at, put_day: first.day, wake: f.trail.iter().map(|b| LookBead { at: b.at, measured: b.measured }).collect(), measured: f.measured }
                })
                .collect(),
            behind: self.behind() as f32,
            recording: self.studio.recording.as_ref().map(|t| (self.now - t.t0) as f32),
            exporting: self.exporting,
            opening: if self.exporting { self.clip_phase.0 } else { 1.0 },
            ending: if self.exporting { self.clip_phase.1 } else { 0.0 },
            lamp_rings: if self.exporting {
                Vec::new()
            } else {
                // (a ring within a few km of a lamp: a tap on it or on its mark)
                self.ripples
                    .iter()
                    .filter_map(|r| self.floats.iter().filter(|f| f.born(self.clock.day)).find(|f| f.at.distance_m(r.at) < 30_000.0).map(|f| (f.id, f.at, (self.seconds - r.when) as f32)))
                    .collect()
            },
            conducting: self.conducting(),
            song: self.song.is_some(),
            cues: self.cues.iter().map(|(c, when)| {
                let mut c = c.clone();
                c.age = (self.seconds - when) as f32;
                c
            }).collect(),
            panel_press: self.gestures.panel_pressing_since(self.now).map(|(id, secs)| (id, crate::touch::motion::thicken(secs) as f32)),
            clip: if self.exporting {
                Some([0.0, 0.0, (crate::studio::CLIP.0 as f64 / crate::studio::CLIP_PPP) as f32, (crate::studio::CLIP.1 as f64 / crate::studio::CLIP_PPP) as f32])
            } else {
                None
            },
            takes: self.studio.takes.len() as u32,
            resting,
            bloom: clouds.iter().find(|c| c.front).map_or(0.0, |c| c.bloom),
            clouds,
            drifter: self.drifter.clone(),
            drifter_label: self.drifter_label.clone(),
            drifter_nearest: if self.drifter.is_empty() { None } else { Some(self.drifter_nearest.min(self.drifter.len() - 1)) },
            rise: self.bloom_eased(),
            night: self.night,
            date: format!("{} {}", day, MONTHS[(month - 1) as usize]),
            note: String::new(),
            date_running: self.clock.running,
            year_day: (self.clock.year_day() / 365.25) as f32,
            seconds: self.seconds as f32,
            date_press: self.gestures.date_pressing_since(self.now).map_or(0.0, |secs| crate::touch::motion::thicken(secs) as f32),
            truth_marks: false,
            credits: crate::sea::CREDIT_LINE.to_string(),
            credits_full: crate::sea::CREDITS.iter().map(|s| s.to_string()).collect(),
            credits_open: self.credits_open,
        }
    }

    /// Every shown float's cloud stepped toward the date, at most `budget` steps each (R6).
    fn step_clouds(&mut self, budget: usize) {
        // every shown float's maybes on the GPU (R5, several since R6): seeded once from its leg's
        // put, stepped on from the day they have reached, a budget of steps a frame each; a rewind
        // restores the nearest keep and steps from there, so a scrub never re-runs a cloud from the put
        let day = self.clock.day;
        let n = if self.budget.scale < 0.75 { MAYBES / 2 } else { MAYBES };
        let mut clouds = std::mem::take(&mut self.clouds);
        for c in &mut clouds {
            if !c.wanted {
                continue; // folding: drawn as it stands
            }
            let Some(f) = self.floats.iter().find(|f| f.id == c.id) else { continue };
            let Some(li) = f.leg_at(day) else { continue };
            let leg = f.legs[li];
            let base = c.slot * MAYBES;
            let same = c.n == n && c.leg == leg;
            let target = day.max(leg.day);
            let mut reached = if same { c.reached } else { f64::INFINITY };
            if reached > target + 1e-9 {
                // behind us: the nearest keep at or before the target, else the put itself
                let keep = if same { c.keeps.iter().rposition(|k| k.day <= target + 1e-9) } else { None };
                match keep {
                    Some(k) => {
                        self.maybes.restore(&self.gpu, &c.keeps[k].keep);
                        reached = c.keeps[k].day;
                        c.keeps.truncate(k + 1);
                        c.next_keep = reached + KEEP_EVERY;
                    }
                    None => {
                        let paths: Vec<GpuPath> = (1..=n)
                            .map(|k| {
                                let e = match leg.eddy {
                                    Some(e) => (e[0], e[1]),
                                    None => self.rules.put(leg.at, leg.day, leg.seed, k).eddy,
                                };
                                GpuPath { lon: leg.at.lon as f32, lat: leg.at.lat as f32, held: 0, measured: 1.0, eddy_u: e.0, eddy_v: e.1 }
                            })
                            .collect();
                        self.maybes.seed_at(&self.gpu, base, &paths);
                        c.keeps.clear();
                        reached = leg.day;
                        c.next_keep = leg.day + KEEP_EVERY;
                        c.leg = leg;
                        c.n = n;
                    }
                }
            }
            // the frame's steps in one submit (Truth's step_many), a keep where one falls due
            let mut steps = 0;
            let mut batch: Vec<Params> = Vec::new();
            let budget = budget;
            while reached < target - 1e-6 && steps < budget {
                let dt = MAYBES_STEP.min(target - reached);
                batch.push(Params::new(&self.rules, reached, dt, leg.seed, n).at_base(base));
                reached += dt;
                steps += 1;
                if reached >= c.next_keep - 1e-9 || batch.len() as u32 >= crate::sea::MAX_STEPS {
                    self.maybes.step_many(&self.gpu, &batch, true);
                    batch.clear();
                    if reached < c.next_keep - 1e-9 {
                        continue;
                    }
                    c.keeps.push(Kept { day: reached, keep: self.maybes.keep_at(&self.gpu, base, n) });
                    if c.keeps.len() > KEEPS_MAX {
                        c.keeps.remove(0);
                    }
                    c.next_keep = reached + KEEP_EVERY;
                }
            }
            if !batch.is_empty() {
                self.maybes.step_many(&self.gpu, &batch, true);
            }
            c.reached = reached;
        }
        self.clouds = clouds;
    }

    /// Draws a frame into `target`.
    pub fn render(&mut self, target: &wgpu::TextureView) {
        self.render_with(target, true);
    }

    /// Draws a frame; `step` lets the clouds take their frame's steps first (a replay has stepped
    /// them already, R9).
    fn render_with(&mut self, target: &wgpu::TextureView, step: bool) {
        let view = self.view();
        self.render_view(target, view, step);
    }

    /// Draws a frame through `view`: the live screen's, or a clip's own size and camera (R10: the
    /// clip is drawn at 9:16 with a camera of its own, not cropped from the take's screen).
    fn render_view(&mut self, target: &wgpu::TextureView, view: View, step: bool) {
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
        let rescaled = self.stage.as_ref().map_or(true, |s| !wanted(s, self.budget.scale, view.width, view.height));
        if rescaled {
            self.stage = Some(self.blit.stage(&self.gpu, view.width, view.height, self.budget.scale));
        }
        if moved || sea_ticked || rescaled {
            if moved && self.budget.frame(self.last_dt) {
                self.stage = Some(self.blit.stage(&self.gpu, view.width, view.height, self.budget.scale));
            }
            let stage = self.stage.as_ref().unwrap();
            let mut u = uniform;
            u.frame = [stage.width as f32, stage.height as f32, (stage.width as f64 / view.width as f64 * view.ppp) as f32, view.reading as f32];
            self.gpu.queue.write_buffer(&self.stage_view_buffer, 0, bytemuck::bytes_of(&u));
            if self.exporting && self.clip_phase.1 > 0.0 {
                // the clip's last beat: touch feedback fades out with it (R10)
                let fade = 1.0 - self.clip_phase.1;
                let rings: Vec<_> = self.ripples.iter().map(|r| crate::look::sea::Ripple { at: r.at, when: r.when, strength: r.strength * fade }).collect();
                self.sea.prepare(&self.gpu, &rings);
            } else {
                self.sea.prepare(&self.gpu, &self.ripples);
            }
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
            self.frames.3 += 1;
        }
        if step {
            self.step_clouds(MAYBES_BUDGET);
        }
        let state = self.look_state();
        let d = view.disc;
        self.look.prepare(&self.gpu, &state, (view.width as f64 / view.ppp) as f32, (view.height as f64 / view.ppp) as f32, view.reading as f32, [d.cx as f32, d.cy as f32, d.r as f32]);
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
            self.look.draw(&mut rp, &self.view_bind, &self.maybes);
        }
        self.gpu.queue.submit([enc.finish()]);
    }

    /// A frame: time passes, then it is drawn.
    pub fn frame(&mut self, target: &wgpu::TextureView, now: f64) {
        if let Some(take) = &self.studio.recording {
            if now - take.t0 >= crate::studio::TAKE_MAX {
                self.end_take(now);
            }
        }
        self.advance(now);
        self.render(target);
    }

    /// An intent from outside the touch (the checks, a shell's button): applied, and recorded in a
    /// take as it runs (R9).
    pub fn command(&mut self, intent: Intent) {
        self.record(crate::studio::TakeEvent::Intent { t: self.now, intent });
        self.intend(intent);
    }

    fn record(&mut self, ev: crate::studio::TakeEvent) {
        if self.studio.replay.is_none() {
            if let Some(take) = &mut self.studio.recording {
                take.events.push(ev);
            }
        }
    }

    /// The state a take starts from (R9).
    fn take_start(&self) -> crate::studio::TakeStart {
        crate::studio::TakeStart {
            floats: self.floats.clone(),
            next_id: self.next_id,
            clock: self.clock,
            camera: self.camera.camera,
            chosen: self.chosen.clone(),
            followed: self.followed.clone(),
            riding: self.camera.riding,
            seconds: self.seconds,
            night: self.night,
            reduce_motion: self.reduce_motion,
            credits_open: self.credits_open,
        }
    }

    /// Puts the app at a take's start, at the shell's time `now`, and resets to rest what the start
    /// doesn't hold (the camera's inertia, the gestures, the ripples; the clouds caught up and
    /// bloomed), so a live take and its replay begin alike.
    fn apply_start(&mut self, s: &crate::studio::TakeStart, now: f64) {
        self.floats = s.floats.clone();
        for f in &mut self.floats {
            f.hand = Hand::Free;
        }
        self.next_id = s.next_id;
        self.clock = s.clock;
        self.camera = CameraControl::new(s.camera);
        self.camera.riding = s.riding;
        self.chosen = s.chosen.clone();
        self.followed = s.followed.clone();
        self.seconds = s.seconds;
        self.night = s.night;
        self.reduce_motion = s.reduce_motion;
        self.credits_open = s.credits_open;
        self.gestures = Gestures::new();
        self.ripples.clear();
        self.conduct.clear();
        self.cues.clear();
        self.pressing = None;
        self.pressing_float = None;
        self.last_t = Some(now);
        self.now = now;
        self.rest = if self.clock.running { 0.0 } else { 1.0 };
        self.step_floats_with(usize::MAX);
        self.samples.clear();
        self.step_samples(usize::MAX);
        self.clouds.clear();
        self.open_clouds();
        for c in &mut self.clouds {
            c.bloom = 1.0;
        }
        self.bloom = if self.shown().is_some() { 1.0 } else { 0.0 };
        self.step_clouds(usize::MAX);
    }

    /// The song the shell plays from its start while a take records (R12), by name; None for none.
    pub fn set_song(&mut self, name: Option<String>) {
        self.song = name;
    }

    /// The song's name, if the shell has one.
    pub fn song(&self) -> Option<&str> {
        self.song.as_deref()
    }

    /// Whether a take is being recorded now (the shell starts and stops the song by it).
    pub fn recording(&self) -> bool {
        self.studio.recording.is_some()
    }

    /// While exporting, where the take's song begins in the clip and how long it runs, seconds
    /// (R12); None without a song or at another speed.
    pub fn export_song(&self) -> Option<(f64, f64)> {
        self.studio.replay.as_ref()?.take.song_in_clip()
    }

    /// Whether the hand conducts (R11): while a take is recorded, and while one is replayed for its
    /// export.
    pub fn conducting(&self) -> bool {
        self.studio.recording.is_some() || self.studio.replay.is_some()
    }

    /// Each lamp's leg cut where it is now, the flight carried on unchanged (the same world, its eddy
    /// velocity), so a take's lamps have short legs whose worlds the hand can choose among (R11). Any
    /// legs after the date (a scrub back) are dropped: the take writes the future afresh.
    fn cut_legs(&mut self) {
        let day = self.clock.day;
        // the cut is on the quarter-day grid, at the first mark at or after the date, with the path's
        // own sample there (place and eddy): the new leg's flight is the old one's, bit for bit (R11,
        // Truth: `conduct::tests`), and the lamp is where it was drawn
        let mut cuts = Vec::new();
        for f in &self.floats {
            if !f.born(day) || f.hand != Hand::Free {
                continue;
            }
            let Some(li) = f.leg_at(day) else { continue };
            cuts.push((f.id, li, f.legs()[li]));
        }
        for (id, li, leg) in cuts {
            let mut budget = usize::MAX;
            self.ensure_path(id, li, leg, day, &mut budget);
            let s = self.paths[&(id, li)].samples.iter().copied().find(|s| s.day >= day - 1e-9);
            let Some(f) = self.floats.iter_mut().find(|f| f.id == id) else { continue };
            let mut legs = f.legs();
            legs.truncate(li + 1);
            if let Some(s) = s.filter(|s| s.day > leg.day + 1e-9) {
                legs.push(Leg { at: s.at, day: s.day, seed: leg.seed, world: leg.world, eddy: Some(s.eddy) });
            }
            f.legs = legs;
        }
    }

    /// Every lamp's worlds stepped to the date while the hand conducts (R11); dropped otherwise.
    fn step_worlds(&mut self) {
        if !self.conducting() {
            self.conduct.clear();
            return;
        }
        let day = self.clock.day;
        let mut live = Vec::new();
        for f in &self.floats {
            if !f.born(day) || f.hand != Hand::Free {
                continue;
            }
            let Some(li) = f.leg_at(day) else { continue };
            let legs = f.legs();
            let leg = legs[li];
            live.push(f.id);
            let w = self.conduct.entry(f.id).or_insert_with(|| crate::conduct::Worlds::new(&self.rules, leg, li));
            if !w.serves(&leg, li) {
                *w = crate::conduct::Worlds::new(&self.rules, leg, li);
            }
            w.step_to(&self.rules, day);
            // the next mark a share at a time, so no frame steps a whole mark (Truth, R11)
            w.prepare(&self.rules);
        }
        self.conduct.retain(|id, _| live.contains(id));
    }

    /// A conductor's cue (R11): each lamp aimed at (one, or every lamp out) takes the world that did
    /// what the gesture did; its leg becomes that world and a new one begins where it now is.
    fn cue(&mut self, float: Option<u32>, gesture: ConductGesture) {
        if !self.conducting() {
            return;
        }
        self.step_worlds();
        let day = self.clock.day;
        let disc = self.disc();
        let cam = self.camera.camera;
        let project = move |p: LonLat| {
            let (xy, facing) = disc.project(&cam, p);
            (facing > 0.0).then_some(xy)
        };
        let targets: Vec<u32> = match float {
            Some(id) => vec![id],
            None => self.conduct.keys().copied().collect::<std::collections::BTreeSet<_>>().into_iter().collect(),
        };
        let mut before = Vec::new();
        for id in targets {
            let Some(w) = self.conduct.get(&id) else { continue };
            let Some(choice) = w.choose(&self.rules, gesture, day, self.clock.rate, &project) else { continue };
            let chosen = w.place(choice.k, day);
            let places = w.places(day);
            let fan: Vec<LonLat> = choice.fan.iter().map(|k| w.place(*k, day).at).collect();
            let li = w.leg_index;
            // the leg becomes world k to the next grid mark, and the next leg begins there with world
            // k's exact state, still in world k: the lamp's path is that world's, bit for bit (Truth)
            let (cut_day, cut) = w.cut(choice.k, day);
            let Some(f) = self.floats.iter_mut().find(|f| f.id == id) else { continue };
            before.push(f.clone());
            let mut legs = f.legs();
            legs.truncate(li + 1);
            legs[li].world = choice.k;
            if cut_day > legs[li].day + 1e-9 {
                let seed = legs[li].seed;
                legs.push(Leg { at: cut.at, day: cut_day, seed, world: choice.k, eddy: Some([cut.eddy.0, cut.eddy.1]) });
            }
            f.legs = legs;
            let from = f.at;
            f.at = chosen.at;
            f.eddy = [chosen.eddy.0, chosen.eddy.1];
            f.lit_for = 0.0;
            let cue = crate::look::LookCue { id, worlds: places, near: choice.near, chosen: choice.k, at: chosen.at, from, fan, age: 0.0 };
            self.cues.push((cue, self.seconds));
        }
        if !before.is_empty() {
            self.undo.push(Undo::Conducted(before));
            self.redo.clear();
        }
        self.step_floats_with(usize::MAX);
        self.step_worlds();
    }

    /// Puts these floats in place of the ones with their ids, returning those (undo of a cue).
    fn swap_floats(&mut self, list: Vec<Float>) -> Vec<Float> {
        let mut out = Vec::new();
        for mut f in list {
            if let Some(cur) = self.floats.iter_mut().find(|c| c.id == f.id) {
                f.hand = Hand::Free;
                f.lit_for = 0.0;
                out.push(std::mem::replace(cur, f));
            }
        }
        out
    }

    /// A take begins at the shell's time `now` (R9).
    fn begin_take(&mut self, now: f64) {
        self.cut_legs();
        // the conductor's tempo while a take records (R11, Truth: CONDUCTOR.md › In the app)
        self.clock.rate = crate::conduct::TEMPO;
        let start = self.take_start();
        self.apply_start(&start, now);
        self.studio.recording = Some(crate::studio::Take { start, events: Vec::new(), t0: now, t1: now, width: self.width, height: self.height, ppp: self.ppp, trim: None, end: None, speed: 1.0, framing: crate::studio::Framing::default(), song: self.song.clone() });
    }

    /// The take ends and is kept, the newest last (R9).
    fn end_take(&mut self, now: f64) {
        if let Some(mut take) = self.studio.recording.take() {
            take.t1 = now;
            take.end = Some(self.take_start());
            self.clock.rate = RATE;
            if take.duration() > 0.2 {
                self.studio.takes.push(take);
                if self.studio.takes.len() > crate::studio::TAKES_KEPT {
                    self.studio.takes.remove(0);
                }
            }
        }
    }

    /// Whether the reader asked to export the newest take since the shell last asked (R9).
    pub fn take_export_wanted(&mut self) -> bool {
        std::mem::take(&mut self.studio.export_wanted)
    }

    /// Begins exporting the newest take (or `index`) at `fps`: the live state is put aside and the
    /// app set to the take's start and size. Returns the frames to come, 0 if there is no take.
    pub fn export_begin(&mut self, index: Option<usize>, fps: f64) -> u32 {
        if self.studio.replay.is_some() {
            return 0;
        }
        let Some(take) = index.map_or(self.studio.takes.last(), |i| self.studio.takes.get(i)).cloned() else { return 0 };
        let live = Stash {
            start: self.take_start(),
            undo: self.undo.clone(),
            redo: self.redo.clone(),
            budget: (self.budget.locked, self.budget.scale),
            size: (self.width, self.height, self.ppp),
            recording: self.studio.recording.take(),
            now: self.now,
            seen: self.seen.len(),
        };
        self.budget.locked = true;
        self.budget.scale = 1.0;
        self.resize(take.width, take.height, take.ppp);
        self.apply_start(&take.start, take.t0);
        let frames = ((take.clip_seconds() * fps).ceil() as u32).max(1);
        let cam = crate::studio::ClipCamera::new(take.start.camera);
        self.studio.replay = Some(crate::studio::Replay { take, next_event: 0, frame: 0, frames, fps, live, cam, held: 0.0 });
        frames
    }

    /// The clip's camera for the frame last exported, and the take's camera it follows (R10: the check
    /// writes them to a trace, so a lurch can be plotted rather than eyed).
    pub fn export_camera(&self) -> Option<(Camera, Camera)> {
        self.studio.replay.as_ref().map(|r| (r.cam.cam, self.camera.camera))
    }

    /// The export's size in pixels: the clip's 9:16 (`studio::CLIP`).
    pub fn export_size(&self) -> Option<(u32, u32)> {
        self.studio.replay.as_ref().map(|_| crate::studio::CLIP)
    }

    /// The pixel size an export's offscreen target needs: the clip's own (R10: drawn at its size,
    /// not cropped).
    pub fn export_target_size(&self) -> Option<(u32, u32)> {
        self.export_size()
    }

    /// The clip's view: its own size at `studio::CLIP_PPP`, through `cam` (R10).
    fn clip_view(&self, cam: Camera) -> View {
        let (w, h) = crate::studio::CLIP;
        let mut v = self.view();
        v.width = w;
        v.height = h;
        v.ppp = crate::studio::CLIP_PPP;
        v.reading = Self::reading_for(w, h, v.ppp);
        v.camera = cam;
        v.disc = View::layout(w, h, v.ppp, cam);
        v
    }

    /// The live state drawn as a clip's frame would draw it (R10, for the check's `render --clip`
    /// and a scene's look "as exported"): the clip's size and the live camera, chrome out, into a
    /// target of `studio::CLIP`.
    pub fn render_clip(&mut self, target: &wgpu::TextureView) {
        let view = self.clip_view(self.camera.camera);
        self.exporting = true;
        self.clip_phase = (1.0, 0.0);
        self.render_view(target, view, true);
        self.exporting = false;
    }

    /// Renders the export's next frame into `target` (an `Offscreen` of `export_target_size`) and
    /// returns its pixels as BGRA rows, top down, cropped to `export_size`; None when it is done.
    pub fn export_frame(&mut self, target: &crate::gpu::Offscreen) -> Option<Vec<u8>> {
        use crate::studio::ClipPart;
        let (t_out, part, through, dt) = {
            let r = self.studio.replay.as_ref()?;
            if r.frame >= r.frames {
                return None;
            }
            let (t, part, through) = r.take.clip_at(r.frame as f64 / r.fps);
            (t, part, through, 1.0 / r.fps)
        };
        // every event up to this frame's time, fed as the shell fed them live
        loop {
            let ev = {
                let r = self.studio.replay.as_mut().unwrap();
                match r.take.events.get(r.next_event) {
                    Some(e) if e.t() <= t_out + 1e-9 => {
                        r.next_event += 1;
                        e.clone()
                    }
                    _ => break,
                }
            };
            use crate::studio::TakeEvent as E;
            match ev {
                E::Frame { t } => {
                    self.advance(t);
                    self.step_clouds(MAYBES_BUDGET);
                }
                E::Pointer(p) => self.pointer(p),
                E::Wheel { x, y, factor, .. } => self.wheel(x, y, factor),
                E::Key { key, .. } => self.key(key),
                E::Shake { .. } => self.shaken(),
                E::Intent { intent, .. } => self.intend(intent),
            }
        }
        // the held beats: the date held, the sea's drawn motion running on (put back after the frame,
        // so the replay's own state is exactly the take's)
        let held = self.studio.replay.as_ref().unwrap().held;
        let seconds = self.seconds;
        if part != ClipPart::During {
            let r = self.studio.replay.as_mut().unwrap();
            r.held += dt;
        }
        self.seconds += held;
        self.clip_phase = match part {
            ClipPart::Before => (through as f32, 0.0),
            ClipPart::During => (1.0, 0.0),
            ClipPart::After => (1.0, through as f32),
        };
        // what the clip leans toward (R11): the lamps a cue has touched, else every lamp out
        let lamps = {
            let day = self.clock.day;
            let out: Vec<&Float> = self.floats.iter().filter(|f| f.born(day) && f.hand == Hand::Free).collect();
            let conducted: Vec<LonLat> = out.iter().filter(|f| f.legs().iter().any(|l| l.world != 0)).map(|f| f.at).collect();
            let view = self.clip_view(Camera { zoom: 1.0, ..self.camera.camera });
            let (w, _) = crate::studio::CLIP;
            crate::studio::Lean {
                lamps: if conducted.is_empty() { out.iter().map(|f| f.at).collect() } else { conducted },
                r1: view.disc.r,
                half: w as f64 / crate::studio::CLIP_PPP / 2.0,
            }
        };
        let cam = {
            let r = self.studio.replay.as_mut().unwrap();
            let framing = r.take.framing;
            r.cam.step(self.camera.camera, part, through, dt, &framing, &lamps)
        };
        let view = self.clip_view(cam);
        self.haptic = false;
        self.exporting = true;
        self.render_view(&target.view, view, false);
        self.exporting = false;
        self.clip_phase = (1.0, 0.0);
        self.seconds = seconds;
        let rgba = target.read(&self.gpu);
        let r = self.studio.replay.as_mut().unwrap();
        r.frame += 1;
        // RGBA in, BGRA out
        let mut out = rgba;
        for p in out.chunks_exact_mut(4) {
            p.swap(0, 2);
        }
        Some(out)
    }

    /// Feeds the export's remaining events and says how far the replay ended from where the take
    /// did: the most any float, or the camera's centre, is apart, in km (R9: the check of
    /// determinism; 0 when the replay is exact). None without an export or a recorded end.
    pub fn export_drift(&mut self) -> Option<f64> {
        loop {
            let ev = {
                let r = self.studio.replay.as_mut()?;
                match r.take.events.get(r.next_event) {
                    Some(e) => {
                        r.next_event += 1;
                        e.clone()
                    }
                    None => break,
                }
            };
            use crate::studio::TakeEvent as E;
            match ev {
                E::Frame { t } => {
                    self.advance(t);
                    self.step_clouds(MAYBES_BUDGET);
                }
                E::Pointer(p) => self.pointer(p),
                E::Wheel { x, y, factor, .. } => self.wheel(x, y, factor),
                E::Key { key, .. } => self.key(key),
                E::Shake { .. } => self.shaken(),
                E::Intent { intent, .. } => self.intend(intent),
            }
        }
        let end = self.studio.replay.as_ref()?.take.end.clone()?;
        let mut worst = LonLat::new(end.camera.lon, end.camera.lat).distance_m(LonLat::new(self.camera.camera.lon, self.camera.camera.lat));
        if end.floats.len() != self.floats.len() {
            return Some(f64::INFINITY);
        }
        for (a, b) in end.floats.iter().zip(&self.floats) {
            worst = worst.max(a.at.distance_m(b.at));
        }
        Some(worst / 1000.0)
    }

    /// Ends an export: the live state put back as it was (R9).
    pub fn export_end(&mut self) {
        let Some(r) = self.studio.replay.take() else { return };
        let live = r.live;
        self.resize(live.size.0, live.size.1, live.size.2);
        self.apply_start(&live.start, live.now);
        self.undo = live.undo;
        self.redo = live.redo;
        self.budget.locked = live.budget.0;
        self.budget.scale = live.budget.1;
        self.studio.recording = live.recording;
        self.seen.truncate(live.seen);
        self.haptic = false;
    }

    /// The frames since last asked, as one line for a log: how many, the mean and the worst interval
    /// in milliseconds, how many redrew the stage, the stage's scale, the floats and the maybes. The
    /// device prints it every few seconds; so does the web's console. The count starts again.
    pub fn stats(&mut self) -> String {
        let (n, sum, worst, stage) = std::mem::replace(&mut self.frames, (0, 0.0, 0.0, 0));
        let mean = if n > 0 { sum / n as f64 * 1000.0 } else { 0.0 };
        format!(
            "adrift: {n} frames, {mean:.1} ms mean, {:.1} ms worst, stage redrawn {stage}x at {:.2}, {} floats, {} maybes in {} clouds",
            worst * 1000.0,
            self.budget.scale,
            self.floats.len(),
            self.clouds.iter().map(|c| c.n).sum::<u32>(),
            self.clouds.len()
        )
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
