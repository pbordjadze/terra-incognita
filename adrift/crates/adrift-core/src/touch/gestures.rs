//! Pointer events to intents: the one state machine of VOCABULARY.md. One finger does everything;
//! a touch that moves before the beat is the globe's, a touch that rests the beat is about what is
//! under it. Nothing here reads the state but through the `Probe`.

use super::camera::CameraMove;
use super::motion::{self, Motion};
use crate::model::{Disc, Intent, LonLat, Phase, PointerEvent, Shown};
use std::collections::{BTreeMap, VecDeque};

/// What is under a point of the screen, as the state knows it this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    /// A float within reach (the nearest), and the place under the finger.
    Float(u32, LonLat),
    /// The date drawn under the globe: the control for scrubbing and pausing.
    Date,
    /// The credit line drawn along the foot: a tap opens the page of credits.
    Credits,
    /// A lamp's row in the panel off the globe (R6, `App::panel_bounds()`): a tap chooses it beside
    /// the others or lets it go; a rest rides it. Never a place: nothing is put or rung through it.
    Panel(u32),
    /// The studio's record control (R9): a tap starts a take; while recording, a touch stops it at
    /// touch-down and exports it; a rest on it exports the newest take again. Never a place.
    Record,
    Sea(LonLat),
    Land(LonLat),
    /// Off the globe's disc.
    Off,
}

impl Hit {
    /// The place under the finger, if it is on the globe.
    pub fn place(self) -> Option<LonLat> {
        match self {
            Hit::Float(_, p) | Hit::Sea(p) | Hit::Land(p) => Some(p),
            Hit::Date | Hit::Credits | Hit::Panel(_) | Hit::Record | Hit::Off => None,
        }
    }
}

/// Keys the shells forward (the web build; a hardware keyboard).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Key {
    Undo,
    Redo,
    /// `h` or `0`: the camera flies home.
    Home,
    /// Backspace or Delete: every float out, as one touch.
    ClearAll,
}

/// The screen as drawn this frame, in points: its size, the reading scale, the disc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub width: f64,
    pub height: f64,
    /// Terra's reading scale: 1 on a phone, 1.2 on the 13-inch iPad; slop and reach grow with it.
    pub reading: f64,
    pub disc: Disc,
    /// The camera's zoom as drawn (the over-pinch is measured against its least).
    pub zoom: f64,
}

/// A rectangle of the screen, points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl Rect {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x <= self.x + self.w && y >= self.y && y <= self.y + self.h
    }
}

/// Where the date lives until the look says exactly (`Look::date_bounds()`): its baseline
/// `DATE_BELOW_DISC` reading points under the disc's bottom edge, clamped to `DATE_FOOT` above
/// the screen's foot (so past zoom 2 it lies over the globe), the middle half of the width,
/// `DATE_BAND` tall about the glyphs. A drag there scrubs and a tap pauses.
pub fn date_band(frame: &Frame) -> Rect {
    let w = frame.width * 0.5;
    let baseline = (frame.disc.cy + frame.disc.r + motion::DATE_BELOW_DISC * frame.reading).min(frame.height - motion::DATE_FOOT);
    Rect { x: (frame.width - w) / 2.0, y: baseline - 10.0 - motion::DATE_BAND / 2.0, w, h: motion::DATE_BAND }
}

/// Where the record control lives until the look says exactly (R9): the top right corner, opposite
/// the panel, `RECORD_SIZE` reading points square, `RECORD_TOP` points down (under the status bar)
/// and `RECORD_EDGE` in from the right.
pub fn record_spot(frame: &Frame) -> Rect {
    let s = motion::RECORD_SIZE * frame.reading;
    Rect { x: frame.width - motion::RECORD_EDGE - s, y: motion::RECORD_TOP, w: s, h: s }
}

/// The questions `Gestures` asks of the state. The app answers them from the state and the camera
/// as drawn this frame (touch/probe.rs).
pub trait Probe {
    fn hit(&self, x: f64, y: f64) -> Hit;
    /// Where a float is now; None once it is gone.
    fn float_at(&self, id: u32) -> Option<LonLat>;
    /// The float the camera drifts with.
    fn riding(&self) -> Option<u32>;
    fn running(&self) -> bool;
    /// The clock's day.
    fn day(&self) -> f64;
    fn frame(&self) -> Frame;
    fn motion(&self) -> Motion;
    /// The page of credits is open: any tap closes it, and the beat does nothing.
    fn credits_open(&self) -> bool;
    /// Whether a float is among the chosen (R6).
    fn is_chosen(&self, _id: u32) -> bool {
        false
    }
    /// How a float is chosen (R8): as itself, as its cloud, or not.
    fn shown_as(&self, _id: u32) -> Option<Shown> {
        None
    }
    /// Every float chosen either way (R8), the front last.
    fn choices(&self) -> Vec<u32> {
        Vec::new()
    }
    /// The places that frame everything chosen (R8, `App::frame_points`).
    fn frame_points(&self) -> Vec<LonLat> {
        Vec::new()
    }
    /// A take is being recorded (R9).
    fn recording(&self) -> bool {
        false
    }
    /// How many takes are kept (R9).
    fn takes(&self) -> usize {
        0
    }
    /// The days the slowest cloud out trails the date (R6, `App::behind()`): the scrub's brake.
    fn behind(&self) -> f64 {
        0.0
    }
}

/// What the gestures ask of the app: intents for the state, moves for the camera.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Out {
    pub intents: Vec<Intent>,
    pub camera: Vec<CameraMove>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Finger {
    x: f64,
    y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Idle,
    /// A finger down on the sea, the ring thickening.
    Pressing { id: u64, x0: f64, y0: f64, t0: f64, at: LonLat },
    /// A finger down on a float.
    Resting { id: u64, x0: f64, y0: f64, t0: f64, float: u32, at: LonLat },
    /// A finger down on land or off the disc: silent unless it drags.
    Down { id: u64, x0: f64, y0: f64, t0: f64 },
    /// A float in hand: the one just put (`float` None until the app names it), or one held past
    /// the beat. `from` is where it was lifted; `over` where the hand is now.
    Holding { id: u64, x0: f64, y0: f64, float: Option<u32>, from: LonLat, carried: bool, over: Hit },
    /// The globe's drag: the finger, and the point of the globe it took.
    Orbit { id: u64, x: f64, y: f64, grab: Option<LonLat> },
    /// Two fingers; `zoom0` the zoom when they landed and `scale` their distance's ratio since,
    /// so a pinch out past the whole globe can be known at the lift.
    Pinch { a: u64, b: u64, zoom0: f64, scale: f64 },
    /// Three fingers: waiting for a swipe (left undo, right redo, up home, down clear the sea).
    Three { x0: f64, y0: f64, done: bool },
    /// A finger on the date: `was_running` is the clock as the finger found it, given back when
    /// the scrub (or the flick after it) ends.
    DateDown { id: u64, x0: f64, t0: f64, day: f64, scrubbing: bool, was_running: bool },
    /// A finger down on the credit line, or anywhere while the page of credits is open: a tap
    /// toggles the page; a drag is the globe's.
    CreditsDown { id: u64, x0: f64, y0: f64 },
    /// A finger on a lamp's row in the panel (R6): a tap toggles it among the chosen, the beat
    /// rides it, a drag is the globe's.
    PanelDown { id: u64, x0: f64, y0: f64, t0: f64, float: u32, rode: bool },
    /// A finger on the record control (R9): `done` once it has acted (the stop at touch-down, or
    /// the export after a rest), so the lift does nothing more.
    RecordDown { id: u64, x0: f64, y0: f64, t0: f64, done: bool },
}

/// The state machine. One per screen.
pub struct Gestures {
    state: State,
    fingers: BTreeMap<u64, Finger>,
    /// The last tap's lift, for a double tap.
    last_tap: Option<(f64, f64, f64)>,
    /// The drag's last samples, for the fling (and a scrub's, for the date's flick).
    samples: VecDeque<(f64, f64, f64)>,
    /// The date running on after a flick, days a second.
    date_spin: f64,
    /// Whether the clock ran before the scrub or flick now in progress: given back when it ends.
    date_restore: Option<bool>,
    /// The day the hand has asked for (a scrub, or a flick running on): the date goes after it no
    /// faster than the speed limit (`motion::scrub_step`), so every cloud keeps up (R6).
    date_target: Option<f64>,
    /// The last shake, for a second one that clears the sea.
    last_shake: Option<f64>,
    /// The clock as last seen, for the shake and the flick.
    now: f64,
    /// The lamp the last tap turned, and how it was chosen before (R8): a double tap gives it back.
    last_cycle: Option<(u32, Option<Shown>)>,
}

impl Default for Gestures {
    fn default() -> Self {
        Self::new()
    }
}

impl Gestures {
    pub fn new() -> Gestures {
        Gestures { state: State::Idle, fingers: BTreeMap::new(), last_tap: None, samples: VecDeque::new(), date_spin: 0.0, date_restore: None, date_target: None, last_shake: None, now: 0.0, last_cycle: None }
    }

    /// The app answers `Intent::Put` with the float it made, so the finger goes on holding it.
    pub fn float_put(&mut self, id: u32) {
        if let State::Holding { float: None, .. } = &mut self.state {
            if let State::Holding { float, .. } = &mut self.state {
                *float = Some(id);
            }
        }
    }

    /// What the finger is doing, for the checks and the workbench.
    pub fn describe(&self) -> &'static str {
        match self.state {
            State::Idle => "idle",
            State::Pressing { .. } => "pressing",
            State::Resting { .. } => "resting",
            State::Down { .. } => "down",
            State::Holding { carried: false, .. } => "holding",
            State::Holding { carried: true, .. } => "carrying",
            State::Orbit { .. } => "orbit",
            State::Pinch { .. } => "pinch",
            State::Three { .. } => "three",
            State::DateDown { scrubbing: false, .. } => "date",
            State::DateDown { scrubbing: true, .. } => "scrubbing",
            State::CreditsDown { .. } => "credits",
            State::PanelDown { .. } => "panel",
            State::RecordDown { .. } => "record",
        }
    }

    /// How long the finger has rested on the sea, for the look's ring (None when not pressing).
    pub fn pressing_since(&self, now: f64) -> Option<f64> {
        match self.state {
            State::Pressing { t0, .. } => Some(now - t0),
            _ => None,
        }
    }

    /// How long a finger has been on the date (a tap in the making, a rest toward the credits, or
    /// a scrub), for the look's band (`LookState.date_press` by `motion::thicken`); None when no
    /// finger is on it (a flick runs on without one).
    pub fn date_pressing_since(&self, now: f64) -> Option<f64> {
        match self.state {
            State::DateDown { t0, .. } => Some(now - t0),
            _ => None,
        }
    }

    /// The float in hand, if any.
    pub fn is_holding(&self) -> Option<u32> {
        match self.state {
            State::Holding { float, .. } => float,
            _ => None,
        }
    }

    /// Whether any finger is down.
    pub fn touching(&self) -> bool {
        !self.fingers.is_empty()
    }

    fn slop(probe: &dyn Probe) -> f64 {
        motion::SLOP * probe.frame().reading
    }

    /// The system's shake (the device): one shake undoes the last touch; a second within
    /// `SHAKE_AGAIN` of the first clears the sea (every float out as one touch, undoable).
    pub fn shaken(&mut self) -> Out {
        let mut out = Out::default();
        if matches!(self.state, State::Idle) {
            let again = self.last_shake.is_some_and(|t| {
                let dt = self.now - t;
                dt >= motion::SHAKE_AGAIN.0 && dt <= motion::SHAKE_AGAIN.1
            });
            out.intents.push(if again { Intent::ClearAll } else { Intent::Undo });
            self.last_shake = Some(self.now);
        }
        out
    }

    /// Undo and redo from a keyboard (the web build).
    pub fn key(&mut self, key: Key) -> Out {
        let mut out = Out::default();
        if matches!(self.state, State::Idle) {
            out.intents.push(match key {
                Key::Undo => Intent::Undo,
                Key::Redo => Intent::Redo,
                Key::Home => Intent::Home,
                Key::ClearAll => Intent::ClearAll,
            });
        }
        out
    }

    /// A mouse's wheel or a trackpad's pinch (the web build): zoom by `factor` about the cursor,
    /// what is under it staying under it. Nothing a finger does is changed by it.
    pub fn wheel(&mut self, x: f64, y: f64, factor: f64, probe: &dyn Probe) -> Out {
        let mut out = Out::default();
        if matches!(self.state, State::Idle | State::Orbit { .. }) {
            out.camera.push(CameraMove::Stop);
            out.camera.push(CameraMove::Pinch { grab: probe.hit(x, y).place(), ax: x, ay: y, bx: x, by: y, factor });
        }
        out
    }

    /// A pointer event: the intents and camera moves it means.
    pub fn feed(&mut self, e: PointerEvent, probe: &dyn Probe) -> Out {
        let mut out = Out::default();
        match e.phase {
            Phase::Down => self.down(&e, probe, &mut out),
            Phase::Move => self.moved(&e, probe, &mut out),
            Phase::Up => self.up(&e, probe, &mut out),
            Phase::Cancel => self.cancel(&e, &mut out),
        }
        out
    }

    fn down(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Out) {
        self.fingers.insert(e.id, Finger { x: e.x, y: e.y });
        self.now = e.t;
        out.camera.push(CameraMove::Stop);
        // a touch stops the date's flick, as it stops the globe's; the date still goes on to the day
        // the hand last asked for, and the clock is given back when it is there (`tick`)
        if self.date_spin != 0.0 {
            self.date_spin = 0.0;
            self.date_target = Some(probe.day());
        }
        let n = self.fingers.len();
        match (self.state, n) {
            (State::Idle, 1) => {
                self.samples.clear();
                let hit = probe.hit(e.x, e.y);
                // the page of credits is open: a tap anywhere closes it (a drag on the date still
                // scrubs; any other drag is the globe's)
                if probe.credits_open() && hit != Hit::Date {
                    self.state = State::CreditsDown { id: e.id, x0: e.x, y0: e.y };
                    return;
                }
                self.state = match hit {
                    Hit::Sea(at) => {
                        out.intents.push(Intent::Ring { at });
                        out.intents.push(Intent::Press { at, since: e.t });
                        State::Pressing { id: e.id, x0: e.x, y0: e.y, t0: e.t, at }
                    }
                    Hit::Float(float, at) => {
                        out.intents.push(Intent::PressFloat { float, since: e.t });
                        State::Resting { id: e.id, x0: e.x, y0: e.y, t0: e.t, float, at }
                    }
                    Hit::Date => {
                        // the date taken in hand where it is: a glide still under way ends here,
                        // and the clock as it was before that scrub is the one this touch keeps
                        self.date_target = None;
                        let was_running = self.date_restore.take().unwrap_or(probe.running());
                        if was_running != probe.running() {
                            out.intents.push(Intent::SetRunning { running: was_running });
                        }
                        State::DateDown { id: e.id, x0: e.x, t0: e.t, day: probe.day(), scrubbing: false, was_running }
                    }
                    Hit::Record => {
                        self.last_tap = None;
                        if probe.recording() {
                            // the take ends at this touch-down, so the touch that stops it is not
                            // in it, and the clip is asked for at once (the share sheet follows)
                            out.intents.push(Intent::Record { on: false });
                            out.intents.push(Intent::Export);
                            State::RecordDown { id: e.id, x0: e.x, y0: e.y, t0: e.t, done: true }
                        } else {
                            State::RecordDown { id: e.id, x0: e.x, y0: e.y, t0: e.t, done: false }
                        }
                    }
                    Hit::Panel(float) => {
                        // the row's cloud shows while the finger is on it, as a float's does
                        out.intents.push(Intent::PressFloat { float, since: e.t });
                        self.last_tap = None;
                        State::PanelDown { id: e.id, x0: e.x, y0: e.y, t0: e.t, float, rode: false }
                    }
                    Hit::Credits => State::CreditsDown { id: e.id, x0: e.x, y0: e.y },
                    Hit::Land(_) | Hit::Off => State::Down { id: e.id, x0: e.x, y0: e.y, t0: e.t },
                };
            }
            // a second finger before the beat, or during the globe's drag: a pinch
            (State::Pressing { id, .. } | State::Resting { id, .. } | State::Down { id, .. } | State::Orbit { id, .. } | State::CreditsDown { id, .. } | State::PanelDown { id, .. }, 2) => {
                if matches!(self.state, State::Pressing { .. } | State::Resting { .. } | State::PanelDown { .. }) {
                    out.intents.push(Intent::Unpress);
                }
                // the second finger rings the sea where it lands too, as the first did: a pinch
                // leaves two rings (R7)
                if let Hit::Sea(at) = probe.hit(e.x, e.y) {
                    out.intents.push(Intent::Ring { at });
                }
                self.state = State::Pinch { a: id, b: e.id, zoom0: probe.frame().zoom, scale: 1.0 };
            }
            // a third finger ends the pinch where it is and waits for a swipe
            (State::Pinch { .. }, 3) => {
                self.state = State::Three { x0: self.mean_x(), y0: self.mean_y(), done: false };
            }
            // a float in hand, or the date: more fingers are ignored
            _ => {}
        }
    }

    fn mean_x(&self) -> f64 {
        let n = self.fingers.len().max(1) as f64;
        self.fingers.values().map(|f| f.x).sum::<f64>() / n
    }
    fn mean_y(&self) -> f64 {
        let n = self.fingers.len().max(1) as f64;
        self.fingers.values().map(|f| f.y).sum::<f64>() / n
    }

    fn moved(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Out) {
        let Some(prev) = self.fingers.get(&e.id).copied() else { return };
        self.fingers.insert(e.id, Finger { x: e.x, y: e.y });
        let slop = Self::slop(probe);
        match self.state {
            State::Pressing { id, x0, y0, t0, .. } | State::Resting { id, x0, y0, t0, .. } if id == e.id => {
                if (e.x - x0).hypot(e.y - y0) > slop {
                    if e.t - t0 >= motion::BEAT {
                        // the beat passed without a tick: it was a rest
                        self.rest_elapsed(probe, out);
                        self.moved(e, probe, out);
                        return;
                    }
                    out.intents.push(Intent::Unpress);
                    self.begin_orbit(id, x0, y0, e, probe, out);
                }
            }
            State::Down { id, x0, y0, .. } | State::CreditsDown { id, x0, y0 } if id == e.id => {
                if (e.x - x0).hypot(e.y - y0) > slop {
                    self.begin_orbit(id, x0, y0, e, probe, out);
                }
            }
            State::RecordDown { id, x0, y0, done: false, .. } if id == e.id => {
                if (e.x - x0).hypot(e.y - y0) > slop {
                    self.begin_orbit(id, x0, y0, e, probe, out);
                }
            }
            State::PanelDown { id, x0, y0, rode, .. } if id == e.id => {
                if (e.x - x0).hypot(e.y - y0) > slop {
                    if !rode {
                        out.intents.push(Intent::Unpress);
                    }
                    self.begin_orbit(id, x0, y0, e, probe, out);
                }
            }
            State::Holding { id, x0, y0, float, from, carried, .. } if id == e.id => {
                let carried = carried || (e.x - x0).hypot(e.y - y0) > slop;
                if carried {
                    let over = probe.hit(e.x, e.y);
                    if let (Some(float), Some(to)) = (float, over.place()) {
                        out.intents.push(Intent::Carry { float, to });
                    }
                    // (off the disc the float stays where it was last over water, in hand, until
                    // `Intent::Carry` can say "over the rim": DEMANDS.md)
                    self.state = State::Holding { id, x0, y0, float, from, carried: true, over };
                }
            }
            State::Orbit { id, x, y, grab } if id == e.id => {
                out.camera.push(CameraMove::Drag { grab, x: e.x, y: e.y, dx: e.x - x, dy: e.y - y });
                self.sample(e);
                self.state = State::Orbit { id, x: e.x, y: e.y, grab };
            }
            State::Pinch { a, b, zoom0, scale } if e.id == a || e.id == b => {
                let other = self.fingers[&if e.id == a { b } else { a }];
                let d0 = (prev.x - other.x).hypot(prev.y - other.y).max(1.0);
                let d1 = (e.x - other.x).hypot(e.y - other.y).max(1.0);
                let (ax, ay) = ((prev.x + other.x) / 2.0, (prev.y + other.y) / 2.0);
                let (bx, by) = ((e.x + other.x) / 2.0, (e.y + other.y) / 2.0);
                out.camera.push(CameraMove::Pinch { grab: probe.hit(ax, ay).place(), ax, ay, bx, by, factor: d1 / d0 });
                // and turned about each other: the orb rolls with them (R8), about their midpoint
                let a0 = (prev.y - other.y).atan2(prev.x - other.x);
                let a1 = (e.y - other.y).atan2(e.x - other.x);
                let mut turn = a1 - a0;
                if turn > std::f64::consts::PI {
                    turn -= std::f64::consts::TAU;
                } else if turn < -std::f64::consts::PI {
                    turn += std::f64::consts::TAU;
                }
                if turn.abs() > 1e-9 {
                    out.camera.push(CameraMove::Twist { x: bx, y: by, angle: turn });
                }
                self.state = State::Pinch { a, b, zoom0, scale: scale * d1 / d0 };
            }
            State::Three { x0, y0, done: false } => {
                let (dx, dy) = (self.mean_x() - x0, self.mean_y() - y0);
                if dx.abs() >= motion::SWIPE || dy.abs() >= motion::SWIPE {
                    out.intents.push(if dx.abs() >= dy.abs() {
                        if dx < 0.0 { Intent::Undo } else { Intent::Redo }
                    } else if dy < 0.0 {
                        Intent::Home
                    } else {
                        Intent::ClearAll
                    });
                    self.state = State::Three { x0, y0, done: true };
                }
            }
            State::DateDown { id, x0, t0, day, scrubbing, was_running } if id == e.id => {
                let dx = e.x - x0;
                if scrubbing || dx.abs() > slop {
                    if !scrubbing {
                        // the clock rests while the finger moves it, and is given back after
                        if was_running {
                            out.intents.push(Intent::SetRunning { running: false });
                        }
                        self.date_restore = Some(was_running);
                        self.samples.clear();
                    }
                    self.sample(e);
                    // the hand says where the date should be; the date goes there in `tick`, no
                    // faster than the speed limit
                    self.date_target = Some(day + dx * motion::days_per_point(probe.frame().width));
                    self.state = State::DateDown { id, x0, t0, day, scrubbing: true, was_running };
                }
            }
            _ => {}
        }
    }

    fn begin_orbit(&mut self, id: u64, x0: f64, y0: f64, e: &PointerEvent, probe: &dyn Probe, out: &mut Out) {
        let grab = probe.hit(x0, y0).place();
        out.camera.push(CameraMove::Drag { grab, x: e.x, y: e.y, dx: e.x - x0, dy: e.y - y0 });
        self.samples.clear();
        self.sample(e);
        self.state = State::Orbit { id, x: e.x, y: e.y, grab };
    }

    fn sample(&mut self, e: &PointerEvent) {
        self.samples.push_back((e.t, e.x, e.y));
        while self.samples.len() > 8 {
            self.samples.pop_front();
        }
    }

    /// The fling's velocity, points a second, from the drag's last samples: those within the
    /// window of the lift, or, when events came sparsely, the last two if the finger was still
    /// moving when it lifted (a finger that stopped a while before lifting has no fling).
    fn velocity(&self, now: f64) -> (f64, f64) {
        let recent: Vec<_> = self.samples.iter().filter(|s| now - s.0 <= motion::FLING_WINDOW).collect();
        let (first, last) = if recent.len() >= 2 {
            (*recent[0], *recent[recent.len() - 1])
        } else {
            let n = self.samples.len();
            if n < 2 || now - self.samples[n - 1].0 > motion::FLING_STOPPED {
                return (0.0, 0.0);
            }
            (self.samples[n - 2], self.samples[n - 1])
        };
        let dt = last.0 - first.0;
        if dt < 0.005 {
            return (0.0, 0.0);
        }
        ((last.1 - first.1) / dt, (last.2 - first.2) / dt)
    }

    fn up(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Out) {
        if self.fingers.remove(&e.id).is_none() {
            return;
        }
        match self.state {
            State::Pressing { id, t0, at, .. } if id == e.id => {
                if e.t - t0 >= motion::BEAT {
                    self.rest_elapsed(probe, out);
                    self.up_from_hold(e, probe, out);
                    return;
                }
                // a tap: the ring alone; and the chosen float, if any, is let go with its ride
                out.intents.push(Intent::Unpress);
                out.intents.push(Intent::Unchoose);
                out.camera.push(CameraMove::Leave);
                self.tapped(e, Some(at), out);
                self.state = State::Idle;
            }
            State::Resting { id, t0, float, at, .. } if id == e.id => {
                if e.t - t0 >= motion::BEAT {
                    self.rest_elapsed(probe, out);
                    self.up_from_hold(e, probe, out);
                    return;
                }
                out.intents.push(Intent::Unpress);
                // a tap turns the lamp to its next way of being chosen: its cloud, itself, let go;
                // the second tap of a double tap on it gives back what the first changed, so a
                // double tap on a lamp only flies in
                let again = self.last_tap.is_some_and(|(t, x, y)| e.t - t <= motion::DOUBLE_TAP_S && (e.x - x).hypot(e.y - y) <= motion::DOUBLE_TAP_PT);
                match self.last_cycle {
                    Some((f, before)) if again && f == float => {
                        out.intents.push(Intent::Show { float, shown: before });
                        if before.is_some() {
                            out.camera.push(CameraMove::Follow { float });
                        }
                        self.last_cycle = None;
                    }
                    _ => {
                        self.last_cycle = Some((float, probe.shown_as(float)));
                        self.cycle(float, probe, out);
                    }
                }
                self.tapped(e, Some(at), out);
                self.state = State::Idle;
            }
            State::Down { id, .. } if id == e.id => {
                self.tapped(e, probe.hit(e.x, e.y).place(), out);
                self.state = State::Idle;
            }
            State::Holding { id, .. } if id == e.id => self.up_from_hold(e, probe, out),
            State::Orbit { id, .. } if id == e.id => {
                let (vx, vy) = self.velocity(e.t);
                if vx != 0.0 || vy != 0.0 {
                    out.camera.push(CameraMove::Fling { vx, vy });
                }
                self.state = State::Idle;
            }
            State::Pinch { a, b, zoom0, scale } if e.id == a || e.id == b => {
                // pinched out past the whole globe and let go: the camera flies home
                if zoom0 * scale < motion::ZOOM_MIN * motion::OVERPINCH {
                    out.intents.push(Intent::Home);
                    self.state = State::Idle;
                    return;
                }
                // the drag goes on with the other finger, anchored where it is now
                let rest = if e.id == a { b } else { a };
                let f = self.fingers[&rest];
                self.samples.clear();
                self.state = State::Orbit { id: rest, x: f.x, y: f.y, grab: probe.hit(f.x, f.y).place() };
            }
            State::Three { .. } => {
                if self.fingers.is_empty() {
                    self.state = State::Idle;
                }
            }
            State::DateDown { id, scrubbing, .. } if id == e.id => {
                if !scrubbing {
                    // a tap: pause or play; while the page of credits is open, it closes the page
                    out.intents.push(if probe.credits_open() { Intent::ToggleCredits } else { Intent::SetRunning { running: !probe.running() } });
                } else {
                    if !probe.motion().reduce {
                        // let go moving: the date runs on, slowing (a flick)
                        let (vx, _) = self.velocity(e.t);
                        self.date_spin = vx * motion::days_per_point(probe.frame().width);
                    }
                    if self.date_spin.abs() < motion::DATE_SPIN_REST {
                        // no flick; the clock is given back when the date has reached the finger
                        // (`tick`), at once if it is there already
                        self.date_spin = 0.0;
                    }
                    self.state = State::Idle;
                    self.settle_date(probe, out);
                }
                self.state = State::Idle;
            }
            State::PanelDown { id, rode: true, .. } if id == e.id => self.state = State::Idle,
            State::RecordDown { id, t0, done, .. } if id == e.id => {
                // a tap starts a take (the touch that starts it is not in it either: the take
                // begins at the lift)
                if !done && e.t - t0 < motion::BEAT {
                    out.intents.push(Intent::Record { on: true });
                }
                self.state = State::Idle;
            }
            State::PanelDown { id, t0, float, .. } if id == e.id => {
                out.intents.push(Intent::Unpress);
                if e.t - t0 >= motion::BEAT {
                    self.panel_rest(float, probe, out);
                } else {
                    self.panel_tap(float, probe, out);
                }
                self.state = State::Idle;
            }
            State::CreditsDown { id, .. } if id == e.id => {
                // a tap (the finger stayed within the slop, or it would be an orbit by now)
                out.intents.push(Intent::ToggleCredits);
                self.state = State::Idle;
            }
            _ => {
                if self.fingers.is_empty() {
                    self.state = State::Idle;
                }
            }
        }
    }

    /// The hand opens: not carried, nothing changed; carried over sea, set down there; over land,
    /// back where it was lifted; off the disc, taken out.
    fn up_from_hold(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Out) {
        if let State::Holding { float: Some(float), from, carried, .. } = self.state {
            if !carried {
                out.intents.push(Intent::Release { float, at: Some(from) });
            } else {
                match probe.hit(e.x, e.y) {
                    Hit::Sea(p) | Hit::Float(_, p) => out.intents.push(Intent::Release { float, at: Some(p) }),
                    Hit::Land(_) | Hit::Date | Hit::Credits | Hit::Panel(_) | Hit::Record => {
                        out.intents.push(Intent::Carry { float, to: from });
                        out.intents.push(Intent::Release { float, at: Some(from) });
                    }
                    Hit::Off => out.intents.push(Intent::Release { float, at: None }),
                }
            }
        }
        self.state = State::Idle;
    }

    /// A tap lifted: a second within the double tap's time and distance flies in about it.
    fn tapped(&mut self, e: &PointerEvent, at: Option<LonLat>, out: &mut Out) {
        if let Some((t, x, y)) = self.last_tap {
            if e.t - t <= motion::DOUBLE_TAP_S && (e.x - x).hypot(e.y - y) <= motion::DOUBLE_TAP_PT {
                out.camera.push(CameraMove::Fly { at });
                self.last_tap = None;
                return;
            }
        }
        self.last_tap = Some((e.t, e.x, e.y));
    }

    fn cancel(&mut self, e: &PointerEvent, out: &mut Out) {
        self.fingers.remove(&e.id);
        match self.state {
            State::Holding { id, float, from, .. } if id == e.id => {
                if let Some(float) = float {
                    out.intents.push(Intent::Carry { float, to: from });
                    out.intents.push(Intent::Release { float, at: Some(from) });
                }
                self.state = State::Idle;
            }
            State::Pinch { a, b, .. } if e.id == a || e.id == b => {
                let rest = if e.id == a { b } else { a };
                match self.fingers.get(&rest).copied() {
                    Some(f) => self.state = State::Orbit { id: rest, x: f.x, y: f.y, grab: None },
                    None => self.state = State::Idle,
                }
            }
            State::Three { .. } if !self.fingers.is_empty() => {}
            State::Pressing { id, .. } if id == e.id => {
                out.intents.push(Intent::Unpress);
                self.state = State::Idle;
            }
            State::Resting { id, .. } if id == e.id => {
                out.intents.push(Intent::Unpress);
                self.state = State::Idle;
            }
            State::PanelDown { id, rode, .. } if id == e.id => {
                if !rode {
                    out.intents.push(Intent::Unpress);
                }
                self.state = State::Idle;
            }
            State::Down { id, .. } | State::Orbit { id, .. } | State::DateDown { id, .. } | State::CreditsDown { id, .. } | State::RecordDown { id, .. } if id == e.id => {
                self.state = State::Idle;
            }
            _ => {
                if self.fingers.is_empty() {
                    self.state = State::Idle;
                }
            }
        }
    }

    /// The beat passed with the finger resting: the sea's press puts a float, a float's rest holds it.
    fn rest_elapsed(&mut self, probe: &dyn Probe, out: &mut Out) {
        match self.state {
            State::Pressing { id, x0, y0, at, .. } => {
                out.intents.push(Intent::Put { at });
                self.state = State::Holding { id, x0, y0, float: None, from: at, carried: false, over: Hit::Sea(at) };
            }
            State::Resting { id, x0, y0, float, at, .. } => {
                out.intents.push(Intent::Hold { float });
                let from = probe.float_at(float).unwrap_or(at);
                self.state = State::Holding { id, x0, y0, float: Some(float), from, carried: false, over: Hit::Sea(from) };
            }
            _ => {}
        }
    }

    /// Every frame: the beat elapsing under a resting finger; the date running on after a flick.
    pub fn tick(&mut self, now: f64, probe: &dyn Probe) -> Out {
        let mut out = Out::default();
        let dt = (now - self.now).clamp(0.0, 0.05);
        self.now = now;
        match self.state {
            State::Pressing { t0, .. } | State::Resting { t0, .. } if now - t0 >= motion::BEAT => self.rest_elapsed(probe, &mut out),
            // a rest on the date (the finger still for the beat) opens the page of credits
            State::DateDown { t0, scrubbing: false, .. } if now - t0 >= motion::BEAT && !probe.credits_open() => {
                out.intents.push(Intent::ToggleCredits);
                self.state = State::Idle;
            }
            // a rest on a lamp's row rides it (the finger may still be down)
            // a rest on the record control, not recording, with a take kept: export it again
            State::RecordDown { id, x0, y0, t0, done: false } if now - t0 >= motion::BEAT => {
                if probe.takes() > 0 && !probe.recording() {
                    out.intents.push(Intent::Export);
                }
                self.state = State::RecordDown { id, x0, y0, t0, done: true };
            }
            State::PanelDown { id, x0, y0, t0, float, rode: false } if now - t0 >= motion::BEAT => {
                out.intents.push(Intent::Unpress);
                self.panel_rest(float, probe, &mut out);
                self.state = State::PanelDown { id, x0, y0, t0, float, rode: true };
            }
            _ => {}
        }
        if self.date_spin != 0.0 && self.fingers.is_empty() {
            if self.date_spin.abs() < motion::DATE_SPIN_REST {
                // the flick has come to rest
                self.date_spin = 0.0;
            } else if dt > 0.0 {
                // the flick moves the day asked for; the date follows it under the speed limit
                let to = self.date_target.unwrap_or(probe.day()) + self.date_spin * dt;
                self.date_target = Some(to);
                self.date_spin = motion::fling_decay(self.date_spin, dt);
            }
        }
        if let Some(to) = self.date_target {
            let step = motion::scrub_step(to - probe.day(), dt, probe.behind());
            if step != 0.0 {
                out.intents.push(Intent::Scrub { days: step });
            }
        }
        self.settle_date(probe, &mut out);
        // while the camera follows, what is chosen is framed (R8): every place to hold, as its
        // centre and widest angle from it; never while a finger is down
        if probe.riding().is_some() && self.fingers.is_empty() {
            if let Some((centre, spread, n)) = frame_of(&probe.frame_points()) {
                out.camera.push(CameraMove::Frame { centre, spread, n });
            }
        }
        out
    }

    /// The date has reached the day asked for, no finger scrubs it and no flick runs: the scrub is
    /// over, and the clock is given back as the finger found it.
    fn settle_date(&mut self, probe: &dyn Probe, out: &mut Out) {
        if matches!(self.state, State::DateDown { .. }) || self.date_spin != 0.0 {
            return;
        }
        // (the intents of this tick are applied after it: what the last step leaves is what counts)
        let pending: f64 = out.intents.iter().map(|i| if let Intent::Scrub { days } = i { *days } else { 0.0 }).sum();
        if let Some(to) = self.date_target {
            if (to - probe.day() - pending).abs() > motion::SCRUB_THERE {
                return;
            }
            self.date_target = None;
        }
        if let Some(running) = self.date_restore.take() {
            out.intents.push(Intent::SetRunning { running });
        }
    }

    /// A tap on a lamp's row: it joins the chosen, in front, or, chosen already, is let go; a lamp
    /// newly chosen out of sight or at the globe's rim is turned toward the reader (R6).
    fn panel_tap(&mut self, float: u32, probe: &dyn Probe, out: &mut Out) {
        self.cycle(float, probe, out);
    }

    /// One touch, three ways (R8): a lamp not chosen is chosen as its cloud (its maybes, the heart
    /// of it); chosen as its cloud, the cloud folds back and the lamp is chosen as itself (its
    /// journey, to watch); chosen as itself, it is let go. Each lamp its own way, mixed freely, the
    /// same on the globe and on the panel; the camera follows everything chosen (`Follow`, framed
    /// each tick), and a lamp let go hands the follow to another still chosen, if any.
    fn cycle(&mut self, float: u32, probe: &dyn Probe, out: &mut Out) {
        let next = match probe.shown_as(float) {
            None => Some(Shown::Cloud),
            Some(Shown::Cloud) => Some(Shown::Lamp),
            Some(Shown::Lamp) => None,
        };
        match next {
            // chosen as its cloud and followed: the app's `Ride` (it joins the chosen in front)
            Some(Shown::Cloud) => out.intents.push(Intent::Ride { float }),
            _ => out.intents.push(Intent::Show { float, shown: next }),
        }
        if next == Some(Shown::Lamp) {
            out.camera.push(CameraMove::Follow { float });
        } else if next.is_none() && probe.riding() == Some(float) {
            match probe.choices().into_iter().filter(|&c| c != float).last() {
                Some(other) => out.camera.push(CameraMove::Follow { float: other }),
                None => out.camera.push(CameraMove::Leave),
            }
        }
    }

    /// A rest on a lamp's row: the camera follows again (R8), framing everything chosen, after a
    /// hand had turned it away; a lamp not chosen is chosen as its cloud first.
    fn panel_rest(&mut self, float: u32, probe: &dyn Probe, out: &mut Out) {
        if probe.shown_as(float).is_none() {
            out.intents.push(Intent::Ride { float });
        } else {
            out.camera.push(CameraMove::Follow { float });
        }
    }
}

/// The centre of a set of places on the sphere (their mean direction) and the widest angle of any
/// from it, radians; None for none.
pub fn frame_of(points: &[LonLat]) -> Option<(LonLat, f64, usize)> {
    if points.is_empty() {
        return None;
    }
    let mut c = [0.0; 3];
    for p in points {
        let v = p.to_xyz();
        c = [c[0] + v[0], c[1] + v[1], c[2] + v[2]];
    }
    let n = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
    if n < 1e-9 {
        // spread all round the globe: no centre holds them, so the whole globe from where it is
        return Some((points[0], std::f64::consts::PI, points.len()));
    }
    let c = [c[0] / n, c[1] / n, c[2] / n];
    let spread = points
        .iter()
        .map(|p| {
            let v = p.to_xyz();
            (v[0] * c[0] + v[1] * c[1] + v[2] * c[2]).clamp(-1.0, 1.0).acos()
        })
        .fold(0.0, f64::max);
    Some((LonLat::from_xyz(c), spread, points.len()))
}

#[cfg(test)]
mod tests {
    //! The panel's touch and the scrub's speed limit against a stand-in probe: a panel of rows down
    //! the left edge (y 100.., 40 points a row, ids 1..), the date band at y 600, sea elsewhere.
    use super::*;
    use std::cell::RefCell;

    struct Fake {
        chosen: RefCell<Vec<u32>>,
        followed: RefCell<Vec<u32>>,
        day: RefCell<f64>,
        running: RefCell<bool>,
        behind: f64,
    }

    impl Fake {
        fn new() -> Fake {
            Fake { chosen: RefCell::new(Vec::new()), followed: RefCell::new(Vec::new()), day: RefCell::new(100.0), running: RefCell::new(true), behind: 0.0 }
        }
        /// Applies the intents as the app would (the parts these tests read).
        fn apply(&self, out: &Out) {
            for i in &out.intents {
                match *i {
                    Intent::ChooseAlso { float } => {
                        let mut c = self.chosen.borrow_mut();
                        if let Some(p) = c.iter().position(|&x| x == float) {
                            c.remove(p);
                        } else {
                            c.push(float);
                        }
                    }
                    Intent::Ride { float } => {
                        if !self.chosen.borrow().contains(&float) {
                            self.followed.borrow_mut().retain(|&x| x != float);
                            self.chosen.borrow_mut().push(float);
                        }
                    }
                    Intent::Show { float, shown } => {
                        self.chosen.borrow_mut().retain(|&x| x != float);
                        self.followed.borrow_mut().retain(|&x| x != float);
                        match shown {
                            Some(Shown::Cloud) => self.chosen.borrow_mut().push(float),
                            Some(Shown::Lamp) => self.followed.borrow_mut().push(float),
                            None => {}
                        }
                    }
                    Intent::Scrub { days } => *self.day.borrow_mut() += days,
                    Intent::SetRunning { running } => *self.running.borrow_mut() = running,
                    _ => {}
                }
            }
        }
    }

    impl Probe for Fake {
        fn hit(&self, x: f64, y: f64) -> Hit {
            if x < 60.0 && (100.0..300.0).contains(&y) {
                return Hit::Panel(1 + ((y - 100.0) / 40.0) as u32);
            }
            if (580.0..620.0).contains(&y) {
                return Hit::Date;
            }
            Hit::Sea(LonLat::new(x / 10.0, y / 10.0))
        }
        fn float_at(&self, id: u32) -> Option<LonLat> {
            Some(LonLat::new(id as f64 * 10.0, 0.0))
        }
        fn riding(&self) -> Option<u32> {
            None
        }
        fn running(&self) -> bool {
            *self.running.borrow()
        }
        fn day(&self) -> f64 {
            *self.day.borrow()
        }
        fn frame(&self) -> Frame {
            Frame { width: 393.0, height: 852.0, reading: 1.0, disc: Disc { cx: 196.5, cy: 400.0, r: 180.0 }, zoom: 1.0 }
        }
        fn motion(&self) -> Motion {
            Motion { reduce: false }
        }
        fn credits_open(&self) -> bool {
            false
        }
        fn is_chosen(&self, id: u32) -> bool {
            self.chosen.borrow().contains(&id)
        }
        fn shown_as(&self, id: u32) -> Option<Shown> {
            if self.chosen.borrow().contains(&id) {
                Some(Shown::Cloud)
            } else if self.followed.borrow().contains(&id) {
                Some(Shown::Lamp)
            } else {
                None
            }
        }
        fn behind(&self) -> f64 {
            self.behind
        }
    }

    fn ev(phase: Phase, x: f64, y: f64, t: f64) -> PointerEvent {
        PointerEvent { id: 0, phase, x, y, t }
    }

    fn tap(g: &mut Gestures, p: &Fake, x: f64, y: f64, t: f64) -> Vec<Intent> {
        let mut all = Vec::new();
        for (ph, dt) in [(Phase::Down, 0.0), (Phase::Up, 0.08)] {
            let out = g.feed(ev(ph, x, y, t + dt), p);
            p.apply(&out);
            all.extend(out.intents);
        }
        all
    }

    #[test]
    fn a_pinch_rings_the_sea_under_both_fingers() {
        let (mut g, p) = (Gestures::new(), Fake::new());
        let first = g.feed(PointerEvent { id: 1, phase: Phase::Down, x: 150.0, y: 400.0, t: 0.0 }, &p);
        let second = g.feed(PointerEvent { id: 2, phase: Phase::Down, x: 250.0, y: 420.0, t: 0.05 }, &p);
        let rings: Vec<LonLat> = first.intents.iter().chain(&second.intents).filter_map(|i| if let Intent::Ring { at } = i { Some(*at) } else { None }).collect();
        assert_eq!(rings, vec![LonLat::new(15.0, 40.0), LonLat::new(25.0, 42.0)]);
        assert!(!second.intents.iter().any(|i| matches!(i, Intent::Put { .. })));
    }

    #[test]
    fn panel_rows_choose_two_and_let_one_go_and_never_ring() {
        let (mut g, p) = (Gestures::new(), Fake::new());
        let a = tap(&mut g, &p, 30.0, 110.0, 0.0);
        let b = tap(&mut g, &p, 30.0, 150.0, 1.0);
        assert_eq!(*p.chosen.borrow(), vec![1, 2]);
        for i in a.iter().chain(&b) {
            assert!(!matches!(i, Intent::Ring { .. } | Intent::Put { .. } | Intent::Unchoose), "{i:?}");
        }
        // (R8) a second tap on a row: that lamp as itself, its cloud folded; a third lets it go
        let c = tap(&mut g, &p, 30.0, 110.0, 2.0);
        assert_eq!(*p.chosen.borrow(), vec![2]);
        assert_eq!(*p.followed.borrow(), vec![1]);
        tap(&mut g, &p, 30.0, 110.0, 2.5);
        assert!(p.followed.borrow().is_empty() && *p.chosen.borrow() == vec![2]);
        assert!(c.iter().all(|i| !matches!(i, Intent::Ring { .. })));
        let mut g2 = Gestures::new();
        tap(&mut g2, &p, 30.0, 190.0, 3.0);
        let out = g2.feed(ev(Phase::Down, 30.0, 190.0, 3.2), &p);
        assert!(out.camera.iter().all(|m| !matches!(m, CameraMove::Fly { .. })));
    }

    #[test]
    fn a_rest_on_a_row_rides_it_once() {
        let (mut g, p) = (Gestures::new(), Fake::new());
        p.apply(&g.feed(ev(Phase::Down, 30.0, 150.0, 0.0), &p));
        let out = g.tick(0.5, &p);
        assert!(out.intents.contains(&Intent::Ride { float: 2 }), "{:?}", out.intents);
        assert!(g.tick(0.6, &p).intents.iter().all(|i| !matches!(i, Intent::Ride { .. })));
        let up = g.feed(ev(Phase::Up, 30.0, 150.0, 0.7), &p);
        assert!(up.intents.is_empty(), "{:?}", up.intents);
        assert_eq!(g.describe(), "idle");
    }

    #[test]
    fn a_fast_scrub_is_held_to_the_limit_and_lands_where_the_finger_left_it() {
        let (mut g, p) = (Gestures::new(), Fake::new());
        p.apply(&g.feed(ev(Phase::Down, 100.0, 600.0, 0.0), &p));
        // a fling of the hand: 300 points (300 days) in a twentieth of a second
        p.apply(&g.tick(0.05, &p));
        p.apply(&g.feed(ev(Phase::Move, 400.0, 600.0, 0.05), &p));
        assert!(!*p.running.borrow());
        let mut t = 0.05;
        let mut worst: f64 = 0.0;
        for _ in 0..90 {
            t += 1.0 / 60.0;
            let before = p.day();
            p.apply(&g.tick(t, &p));
            worst = worst.max((p.day() - before).abs());
        }
        assert!(worst <= motion::SCRUB_RATE / 60.0 + 1e-9, "{worst}");
        assert!((p.day() - 400.0).abs() < 1e-9, "{}", p.day());
        // still held: the clock waits for the lift
        assert!(!*p.running.borrow());
        p.apply(&g.feed(ev(Phase::Move, 400.0, 600.0, t), &p));
        p.apply(&g.feed(ev(Phase::Up, 400.0, 600.0, t + 0.3), &p));
        assert!(*p.running.borrow());
    }

    #[test]
    fn clouds_behind_slow_the_scrub_but_never_stop_it() {
        let mut p = Fake::new();
        p.behind = 100.0;
        let mut g = Gestures::new();
        p.apply(&g.feed(ev(Phase::Down, 100.0, 600.0, 0.0), &p));
        p.apply(&g.tick(0.02, &p));
        p.apply(&g.feed(ev(Phase::Move, 300.0, 600.0, 0.02), &p));
        let before = p.day();
        p.apply(&g.tick(0.02 + 1.0 / 60.0, &p));
        let step = p.day() - before;
        assert!(step > 0.0 && step <= 0.2 * motion::SCRUB_RATE / 60.0 + 1e-9, "{step}");
    }
}
