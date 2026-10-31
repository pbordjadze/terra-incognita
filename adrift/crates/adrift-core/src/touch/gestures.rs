//! Pointer events to intents: the one state machine of VOCABULARY.md. One finger does everything;
//! a touch that moves before the beat is the globe's, a touch that rests the beat is about what is
//! under it. Nothing here reads the state but through the `Probe`.

use super::camera::{CameraRig, Frame};
use super::motion::{self, Motion};
use crate::model::{Camera, FloatId, Hand, Intent, LonLat, Phase, PointerEvent};
use std::collections::{BTreeMap, VecDeque};

/// What is under a point of the screen, as the state knows it this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    /// A float within `REACH` reading points (the nearest), and where it is.
    Float(FloatId, LonLat),
    /// The date drawn at the foot: the control for scrubbing and pausing.
    Date,
    Sea(LonLat),
    Land(LonLat),
    /// Off the globe's disc.
    Off,
}

/// Keys the shells forward (the web build; a hardware keyboard).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Undo,
    Redo,
}

/// The questions `Gestures` asks of the state. `App` answers them from the state and the camera it
/// mirrored last frame.
pub trait Probe {
    fn hit(&self, x: f64, y: f64) -> Hit;
    /// Where a float is now; None once it is gone (taken out, undone).
    fn float_at(&self, id: FloatId) -> Option<LonLat>;
    /// Whether any float is on the sea (the idle turn holds still for one).
    fn any_floats(&self) -> bool;
    fn playing(&self) -> bool;
    /// The clock's day (days since 2000-01-01).
    fn day(&self) -> f64;
    /// The id the state will give the next float put, so the hand can name it before it exists.
    fn next_float_id(&self) -> FloatId;
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Finger {
    x: f64,
    y: f64,
}

#[derive(Clone, Debug, PartialEq)]
enum State {
    Idle,
    /// A finger down on the sea, the ring thickening.
    Pressing { id: u32, x0: f64, y0: f64, t0: f64, at: LonLat },
    /// A finger down on a float.
    Resting { id: u32, x0: f64, y0: f64, t0: f64, float: FloatId },
    /// A finger down on land or off the disc: silent unless it drags.
    Down { id: u32, x0: f64, y0: f64 },
    /// A float in hand: the one just put, or one held past the beat.
    Holding { id: u32, x0: f64, y0: f64, float: FloatId, carried: bool, over: Hand },
    /// The globe's drag.
    /// The globe's drag: the finger, and the point of the globe it took.
    Orbit { id: u32, x: f64, y: f64, grab: Option<LonLat> },
    Pinch { a: u32, b: u32 },
    /// Three fingers: waiting for a swipe.
    Three { x0: f64, done: bool },
    DateDown { id: u32, x0: f64, day0: f64, scrubbing: bool },
}

/// The state machine. One per screen.
pub struct Gestures {
    state: State,
    fingers: BTreeMap<u32, Finger>,
    /// The last tap's lift, for a double tap.
    last_tap: Option<(f64, f64, f64)>,
    /// The drag's last samples, for the fling.
    samples: VecDeque<(f64, f64, f64)>,
    pub camera: CameraRig,
    frame: Frame,
}

impl Gestures {
    pub fn new(camera: Camera, motion: Motion, frame: Frame, now: f64) -> Gestures {
        Gestures { state: State::Idle, fingers: BTreeMap::new(), last_tap: None, samples: VecDeque::new(), camera: CameraRig::new(camera, motion, now), frame }
    }

    /// The screen changed (its size, the disc): the camera's turns are measured in it.
    pub fn set_frame(&mut self, frame: Frame) {
        self.frame = frame;
    }
    pub fn frame(&self) -> &Frame {
        &self.frame
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
    /// How long the finger has rested, for the look's ring (0 when not pressing).
    pub fn pressing_since(&self, now: f64) -> Option<f64> {
        match self.state {
            State::Pressing { t0, .. } => Some(now - t0),
            _ => None,
        }
    }

    fn slop(&self) -> f64 {
        motion::SLOP * self.frame.reading
    }

    fn touching(&self) -> bool {
        !self.fingers.is_empty()
    }

    fn hand(&self, probe: &dyn Probe, x: f64, y: f64) -> Hand {
        match probe.hit(x, y) {
            Hit::Sea(p) | Hit::Float(_, p) => Hand::Sea(p),
            Hit::Land(p) => Hand::Land(p),
            // the date lies over the globe or off it; what matters is the water under the finger
            Hit::Date => match self.camera.unproject(x, y, &self.frame) {
                Some(p) => Hand::Land(p),
                None => Hand::Off,
            },
            Hit::Off => Hand::Off,
        }
    }

    /// Undo by the system's shake (the device).
    pub fn shaken(&mut self, out: &mut Vec<Intent>) {
        if matches!(self.state, State::Idle) {
            out.push(Intent::Undo);
        }
    }

    /// Undo and redo from a keyboard (the web build).
    pub fn key(&mut self, key: Key, out: &mut Vec<Intent>) {
        if matches!(self.state, State::Idle) {
            out.push(match key {
                Key::Undo => Intent::Undo,
                Key::Redo => Intent::Redo,
            });
        }
    }

    /// A mouse's wheel or a trackpad's pinch (the web build): zoom by `factor` about the cursor,
    /// what is under it staying under it. Nothing a finger does is changed by it.
    pub fn wheel(&mut self, x: f64, y: f64, factor: f64, now: f64) {
        if !matches!(self.state, State::Idle | State::Orbit { .. }) {
            return;
        }
        self.camera.touched(now);
        self.camera.pinch(factor, x, y, x, y, &self.frame);
    }

    /// A pointer event. Pushes the intents it means onto `out`.
    pub fn event(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Vec<Intent>) {
        match e.phase {
            Phase::Down => self.down(e, probe, out),
            Phase::Move => self.moved(e, probe, out),
            Phase::Up => self.up(e, probe, out),
            Phase::Cancel => self.cancel(e, out),
        }
    }

    fn down(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Vec<Intent>) {
        self.fingers.insert(e.id, Finger { x: e.x, y: e.y });
        self.camera.touched(e.t);
        let n = self.fingers.len();
        match (&self.state, n) {
            (State::Idle, 1) => {
                self.samples.clear();
                self.state = match probe.hit(e.x, e.y) {
                    Hit::Sea(at) => {
                        out.push(Intent::Ring { at });
                        out.push(Intent::Press { at, since: e.t });
                        State::Pressing { id: e.id, x0: e.x, y0: e.y, t0: e.t, at }
                    }
                    Hit::Float(float, _) => State::Resting { id: e.id, x0: e.x, y0: e.y, t0: e.t, float },
                    Hit::Date => State::DateDown { id: e.id, x0: e.x, day0: probe.day(), scrubbing: false },
                    Hit::Land(_) | Hit::Off => State::Down { id: e.id, x0: e.x, y0: e.y },
                };
            }
            // a second finger before the beat, or during the globe's drag: a pinch
            (State::Pressing { id, .. } | State::Resting { id, .. } | State::Down { id, .. } | State::Orbit { id, .. }, 2) => {
                let a = *id;
                if matches!(self.state, State::Pressing { .. }) {
                    out.push(Intent::Unpress);
                }
                if self.camera.leave() {
                    out.push(Intent::Leave);
                }
                self.state = State::Pinch { a, b: e.id };
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

    fn moved(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Vec<Intent>) {
        let Some(prev) = self.fingers.get(&e.id).copied() else { return };
        self.fingers.insert(e.id, Finger { x: e.x, y: e.y });
        let slop = self.slop();
        match self.state.clone() {
            State::Pressing { id, x0, y0, t0, .. } | State::Resting { id, x0, y0, t0, .. } if id == e.id => {
                let strayed = (e.x - x0).hypot(e.y - y0) > slop;
                if strayed {
                    if e.t - t0 >= motion::BEAT {
                        // the beat passed without a tick: it was a rest
                        self.rest_elapsed(probe, out);
                        self.moved(e, probe, out);
                        return;
                    }
                    if matches!(self.state, State::Pressing { .. }) {
                        out.push(Intent::Unpress);
                    }
                    self.begin_orbit(id, x0, y0, e, out);
                }
            }
            State::Down { id, x0, y0 } if id == e.id => {
                if (e.x - x0).hypot(e.y - y0) > slop {
                    self.begin_orbit(id, x0, y0, e, out);
                }
            }
            State::Holding { id, x0, y0, float, carried, .. } if id == e.id => {
                let carried = carried || (e.x - x0).hypot(e.y - y0) > slop;
                if carried {
                    if self.camera.leave() {
                        out.push(Intent::Leave);
                    }
                    let over = self.hand(probe, e.x, e.y);
                    out.push(Intent::Carry { id: float, to: over });
                    self.state = State::Holding { id, x0, y0, float, carried: true, over };
                }
            }
            State::Orbit { id, x, y, grab } if id == e.id => {
                self.camera.drag(grab, e.x, e.y, e.x - x, e.y - y, &self.frame);
                self.sample(e);
                self.state = State::Orbit { id, x: e.x, y: e.y, grab };
            }
            State::Pinch { a, b } if e.id == a || e.id == b => {
                let other = self.fingers[&if e.id == a { b } else { a }];
                let d0 = (prev.x - other.x).hypot(prev.y - other.y).max(1.0);
                let d1 = (e.x - other.x).hypot(e.y - other.y).max(1.0);
                let (ax, ay) = ((prev.x + other.x) / 2.0, (prev.y + other.y) / 2.0);
                let (bx, by) = ((e.x + other.x) / 2.0, (e.y + other.y) / 2.0);
                self.camera.pinch(d1 / d0, ax, ay, bx, by, &self.frame);
            }
            State::Three { x0, done: false } => {
                let dx = self.mean_x() - x0;
                if dx.abs() >= motion::SWIPE {
                    out.push(if dx < 0.0 { Intent::Undo } else { Intent::Redo });
                    self.state = State::Three { x0, done: true };
                }
            }
            State::DateDown { id, x0, day0, scrubbing } if id == e.id => {
                let dx = e.x - x0;
                if scrubbing || dx.abs() > slop {
                    if !scrubbing {
                        out.push(Intent::Play(false));
                    }
                    out.push(Intent::Scrub { day: day0 + dx * motion::days_per_point(self.frame.width) });
                    self.state = State::DateDown { id, x0, day0, scrubbing: true };
                }
            }
            _ => {}
        }
    }

    fn begin_orbit(&mut self, id: u32, x0: f64, y0: f64, e: &PointerEvent, out: &mut Vec<Intent>) {
        if self.camera.leave() {
            out.push(Intent::Leave);
        }
        let grab = self.camera.unproject(x0, y0, &self.frame);
        self.camera.drag(grab, e.x, e.y, e.x - x0, e.y - y0, &self.frame);
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

    fn up(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Vec<Intent>) {
        if self.fingers.remove(&e.id).is_none() {
            return;
        }
        match self.state.clone() {
            State::Pressing { id, t0, .. } if id == e.id => {
                if e.t - t0 >= motion::BEAT {
                    self.rest_elapsed(probe, out);
                    self.up_from_hold(e, probe, out);
                    return;
                }
                out.push(Intent::Unpress);
                self.tapped(e, probe);
                self.state = State::Idle;
            }
            State::Resting { id, t0, float, .. } if id == e.id => {
                if e.t - t0 >= motion::BEAT {
                    self.rest_elapsed(probe, out);
                    self.up_from_hold(e, probe, out);
                    return;
                }
                if self.camera.riding() != Some(float) {
                    self.camera.ride(float);
                    out.push(Intent::Ride { id: float });
                }
                self.tapped(e, probe);
                self.state = State::Idle;
            }
            State::Down { id, .. } if id == e.id => {
                self.tapped(e, probe);
                self.state = State::Idle;
            }
            State::Holding { id, .. } if id == e.id => self.up_from_hold(e, probe, out),
            State::Orbit { id, .. } if id == e.id => {
                let (vx, vy) = self.velocity(e.t);
                self.camera.fling(vx, vy, e.t);
                self.state = State::Idle;
            }
            State::Pinch { a, b } if e.id == a || e.id == b => {
                // the drag goes on with the other finger, anchored where it is now
                let rest = if e.id == a { b } else { a };
                let f = self.fingers[&rest];
                self.samples.clear();
                let grab = self.camera.unproject(f.x, f.y, &self.frame);
                self.state = State::Orbit { id: rest, x: f.x, y: f.y, grab };
            }
            State::Three { .. } => {
                if self.fingers.is_empty() {
                    self.state = State::Idle;
                }
            }
            State::DateDown { id, scrubbing, .. } if id == e.id => {
                if !scrubbing {
                    out.push(Intent::Play(!probe.playing()));
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

    fn up_from_hold(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Vec<Intent>) {
        if let State::Holding { float, carried, .. } = self.state {
            if !carried {
                out.push(Intent::Release { id: float, at: None });
            } else {
                match self.hand(probe, e.x, e.y) {
                    Hand::Sea(p) => out.push(Intent::Release { id: float, at: Some(p) }),
                    Hand::Land(_) => out.push(Intent::Release { id: float, at: None }),
                    Hand::Off => out.push(Intent::TakeOut { id: float }),
                }
            }
        }
        self.state = State::Idle;
    }

    /// A tap lifted: a second within the double tap's time and distance flies in about it.
    fn tapped(&mut self, e: &PointerEvent, _probe: &dyn Probe) {
        if let Some((t, x, y)) = self.last_tap {
            if e.t - t <= motion::DOUBLE_TAP_S && (e.x - x).hypot(e.y - y) <= motion::DOUBLE_TAP_PT {
                self.camera.double_tap(self.camera.unproject(e.x, e.y, &self.frame), e.t);
                self.last_tap = None;
                return;
            }
        }
        self.last_tap = Some((e.t, e.x, e.y));
    }

    fn cancel(&mut self, e: &PointerEvent, out: &mut Vec<Intent>) {
        self.fingers.remove(&e.id);
        match self.state.clone() {
            State::Pressing { id, .. } if id == e.id => {
                out.push(Intent::Unpress);
                self.state = State::Idle;
            }
            State::Holding { id, float, .. } if id == e.id => {
                out.push(Intent::Release { id: float, at: None });
                self.state = State::Idle;
            }
            State::Pinch { a, b } if e.id == a || e.id == b => {
                let rest = if e.id == a { b } else { a };
                match self.fingers.get(&rest).copied() {
                    Some(f) => {
                        let grab = self.camera.unproject(f.x, f.y, &self.frame);
                        self.state = State::Orbit { id: rest, x: f.x, y: f.y, grab };
                    }
                    None => self.state = State::Idle,
                }
            }
            State::Three { .. } if !self.fingers.is_empty() => {}
            State::Resting { id, .. } | State::Down { id, .. } | State::Orbit { id, .. } | State::DateDown { id, .. } if id == e.id => {
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
    fn rest_elapsed(&mut self, probe: &dyn Probe, out: &mut Vec<Intent>) {
        match self.state.clone() {
            State::Pressing { id, x0, y0, at, .. } => {
                let float = probe.next_float_id();
                out.push(Intent::Put { id: float, at });
                self.state = State::Holding { id, x0, y0, float, carried: false, over: Hand::Sea(at) };
            }
            State::Resting { id, x0, y0, float, .. } => {
                out.push(Intent::Hold { id: float });
                let over = match probe.float_at(float) {
                    Some(p) => Hand::Sea(p),
                    None => Hand::Off,
                };
                self.state = State::Holding { id, x0, y0, float, carried: false, over };
            }
            _ => {}
        }
    }

    /// Every frame: the beat elapsing under a resting finger, and the camera's own motion.
    /// `dt` seconds since the last tick.
    pub fn tick(&mut self, now: f64, dt: f64, probe: &dyn Probe, out: &mut Vec<Intent>) {
        match self.state {
            State::Pressing { t0, .. } | State::Resting { t0, .. } if now - t0 >= motion::BEAT => self.rest_elapsed(probe, out),
            _ => {}
        }
        let ridden = self.camera.riding().and_then(|id| probe.float_at(id));
        let touching = self.touching();
        if let Some(c) = self.camera.frame(now, dt, &self.frame, ridden, !probe.any_floats(), touching) {
            out.push(Intent::Camera(c));
        }
    }
}
