//! Pointer events to intents: the one state machine of VOCABULARY.md. One finger does everything;
//! a touch that moves before the beat is the globe's, a touch that rests the beat is about what is
//! under it. Nothing here reads the state but through the `Probe`.

use super::camera::CameraMove;
use super::motion::{self, Motion};
use crate::model::{Disc, Intent, LonLat, Phase, PointerEvent};
use std::collections::{BTreeMap, VecDeque};

/// What is under a point of the screen, as the state knows it this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    /// A float within reach (the nearest), and the place under the finger.
    Float(u32, LonLat),
    /// The date drawn at the foot: the control for scrubbing and pausing.
    Date,
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
            Hit::Date | Hit::Off => None,
        }
    }
}

/// Keys the shells forward (the web build; a hardware keyboard).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Undo,
    Redo,
}

/// The screen as drawn this frame, in points: its size, the reading scale, the disc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub width: f64,
    pub height: f64,
    /// Terra's reading scale: 1 on a phone, 1.2 on the 13-inch iPad; slop and reach grow with it.
    pub reading: f64,
    pub disc: Disc,
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

/// Where the date lives: a band at the foot of the screen, centred, the middle half of the width
/// and `DATE_BAND` points tall above the bottom inset. The look draws the date in it; a drag there
/// scrubs and a tap pauses. The one place both read.
pub fn date_band(frame: &Frame) -> Rect {
    let w = frame.width * 0.5;
    Rect { x: (frame.width - w) / 2.0, y: frame.height - motion::DATE_INSET - motion::DATE_BAND, w, h: motion::DATE_BAND }
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
    Pinch { a: u64, b: u64 },
    /// Three fingers: waiting for a swipe.
    Three { x0: f64, done: bool },
    DateDown { id: u64, x0: f64, t0: f64, day: f64, scrubbing: bool },
}

/// The state machine. One per screen.
pub struct Gestures {
    state: State,
    fingers: BTreeMap<u64, Finger>,
    /// The last tap's lift, for a double tap.
    last_tap: Option<(f64, f64, f64)>,
    /// The drag's last samples, for the fling.
    samples: VecDeque<(f64, f64, f64)>,
}

impl Default for Gestures {
    fn default() -> Self {
        Self::new()
    }
}

impl Gestures {
    pub fn new() -> Gestures {
        Gestures { state: State::Idle, fingers: BTreeMap::new(), last_tap: None, samples: VecDeque::new() }
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
        }
    }

    /// How long the finger has rested on the sea, for the look's ring (None when not pressing).
    pub fn pressing_since(&self, now: f64) -> Option<f64> {
        match self.state {
            State::Pressing { t0, .. } => Some(now - t0),
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

    /// Undo by the system's shake (the device).
    pub fn shaken(&mut self) -> Out {
        let mut out = Out::default();
        if matches!(self.state, State::Idle) {
            out.intents.push(Intent::Undo);
        }
        out
    }

    /// Undo and redo from a keyboard (the web build). Redo waits on `Intent::Redo` (DEMANDS.md).
    pub fn key(&mut self, key: Key) -> Out {
        let mut out = Out::default();
        if matches!(self.state, State::Idle) {
            match key {
                Key::Undo => out.intents.push(Intent::Undo),
                Key::Redo => {}
            }
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
        out.camera.push(CameraMove::Stop);
        let n = self.fingers.len();
        match (self.state, n) {
            (State::Idle, 1) => {
                self.samples.clear();
                self.state = match probe.hit(e.x, e.y) {
                    Hit::Sea(at) => {
                        out.intents.push(Intent::Ring { at });
                        State::Pressing { id: e.id, x0: e.x, y0: e.y, t0: e.t, at }
                    }
                    Hit::Float(float, at) => State::Resting { id: e.id, x0: e.x, y0: e.y, t0: e.t, float, at },
                    Hit::Date => State::DateDown { id: e.id, x0: e.x, t0: e.t, day: probe.day(), scrubbing: false },
                    Hit::Land(_) | Hit::Off => State::Down { id: e.id, x0: e.x, y0: e.y, t0: e.t },
                };
            }
            // a second finger before the beat, or during the globe's drag: a pinch
            (State::Pressing { id, .. } | State::Resting { id, .. } | State::Down { id, .. } | State::Orbit { id, .. }, 2) => {
                self.state = State::Pinch { a: id, b: e.id };
            }
            // a third finger ends the pinch where it is and waits for a swipe
            (State::Pinch { .. }, 3) => {
                self.state = State::Three { x0: self.mean_x(), done: false };
            }
            // a float in hand, or the date: more fingers are ignored
            _ => {}
        }
    }

    fn mean_x(&self) -> f64 {
        let n = self.fingers.len().max(1) as f64;
        self.fingers.values().map(|f| f.x).sum::<f64>() / n
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
                    self.begin_orbit(id, x0, y0, e, probe, out);
                }
            }
            State::Down { id, x0, y0, .. } if id == e.id => {
                if (e.x - x0).hypot(e.y - y0) > slop {
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
            State::Pinch { a, b } if e.id == a || e.id == b => {
                let other = self.fingers[&if e.id == a { b } else { a }];
                let d0 = (prev.x - other.x).hypot(prev.y - other.y).max(1.0);
                let d1 = (e.x - other.x).hypot(e.y - other.y).max(1.0);
                let (ax, ay) = ((prev.x + other.x) / 2.0, (prev.y + other.y) / 2.0);
                let (bx, by) = ((e.x + other.x) / 2.0, (e.y + other.y) / 2.0);
                out.camera.push(CameraMove::Pinch { grab: probe.hit(ax, ay).place(), ax, ay, bx, by, factor: d1 / d0 });
            }
            State::Three { x0, done: false } => {
                let dx = self.mean_x() - x0;
                if dx.abs() >= motion::SWIPE {
                    if dx < 0.0 {
                        out.intents.push(Intent::Undo);
                    }
                    // (a swipe right waits on `Intent::Redo`: DEMANDS.md)
                    self.state = State::Three { x0, done: true };
                }
            }
            State::DateDown { id, x0, t0, day, scrubbing } if id == e.id => {
                let dx = e.x - x0;
                if scrubbing || dx.abs() > slop {
                    if !scrubbing {
                        out.intents.push(Intent::SetRunning { running: false });
                    }
                    let to = day + dx * motion::days_per_point(probe.frame().width);
                    let now = probe.day();
                    if (to - now).abs() > 1e-9 {
                        out.intents.push(Intent::Scrub { days: to - now });
                    }
                    self.state = State::DateDown { id, x0, t0, day, scrubbing: true };
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
                // a tap: the ring alone
                self.tapped(e, Some(at), out);
                self.state = State::Idle;
            }
            State::Resting { id, t0, float, at, .. } if id == e.id => {
                if e.t - t0 >= motion::BEAT {
                    self.rest_elapsed(probe, out);
                    self.up_from_hold(e, probe, out);
                    return;
                }
                if probe.riding() != Some(float) {
                    out.intents.push(Intent::Ride { float });
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
            State::Pinch { a, b } if e.id == a || e.id == b => {
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
                    out.intents.push(Intent::SetRunning { running: !probe.running() });
                }
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
                    Hit::Land(_) | Hit::Date => {
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
            State::Pinch { a, b } if e.id == a || e.id == b => {
                let rest = if e.id == a { b } else { a };
                match self.fingers.get(&rest).copied() {
                    Some(f) => self.state = State::Orbit { id: rest, x: f.x, y: f.y, grab: None },
                    None => self.state = State::Idle,
                }
            }
            State::Three { .. } if !self.fingers.is_empty() => {}
            State::Pressing { id, .. } | State::Resting { id, .. } | State::Down { id, .. } | State::Orbit { id, .. } | State::DateDown { id, .. } if id == e.id => {
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

    /// Every frame: the beat elapsing under a resting finger.
    pub fn tick(&mut self, now: f64, probe: &dyn Probe) -> Out {
        let mut out = Out::default();
        match self.state {
            State::Pressing { t0, .. } | State::Resting { t0, .. } if now - t0 >= motion::BEAT => self.rest_elapsed(probe, &mut out),
            _ => {}
        }
        out
    }
}
