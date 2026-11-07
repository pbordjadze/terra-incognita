//! Pointer events to intents: the one state machine of VOCABULARY.md. One finger does everything;
//! a touch that moves before the beat is the globe's, a touch that rests the beat is about what is
//! under it. Nothing here reads the state but through the `Probe`.

use super::camera::CameraMove;
use super::motion::{self, Motion};
use crate::model::{ConductGesture, Disc, Intent, LonLat, Phase, PointerEvent, Shown};
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
    let baseline = (frame.disc.cy + frame.disc.r + motion::DATE_BELOW_DISC * frame.reading)
        .min(frame.height - motion::DATE_FOOT);
    Rect {
        x: (frame.width - w) / 2.0,
        y: baseline - 10.0 - motion::DATE_BAND / 2.0,
        w,
        h: motion::DATE_BAND,
    }
}

/// Where the record control lives until the look says exactly (R9): the top right corner, opposite
/// the panel, `RECORD_SIZE` reading points square, `RECORD_TOP` points down (under the status bar)
/// and `RECORD_EDGE` in from the right.
pub fn record_spot(frame: &Frame) -> Rect {
    let s = motion::RECORD_SIZE * frame.reading;
    Rect {
        x: frame.width - motion::RECORD_EDGE - s,
        y: motion::RECORD_TOP,
        w: s,
        h: s,
    }
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
    /// Whether the hand conducts (R11, `App::conducting`: a take recording, or replayed for export).
    fn conducting(&self) -> bool {
        false
    }
    /// The lamps on the globe's face, their ids and screen points (R11: a conductor's gesture goes to
    /// the lamp it starts nearest).
    fn lamps_on_screen(&self) -> Vec<(u32, f64, f64)> {
        Vec::new()
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
    Pressing {
        id: u64,
        x0: f64,
        y0: f64,
        t0: f64,
        at: LonLat,
    },
    /// A finger down on a float.
    Resting {
        id: u64,
        x0: f64,
        y0: f64,
        t0: f64,
        float: u32,
        at: LonLat,
    },
    /// A finger down on land or off the disc: silent unless it drags.
    Down {
        id: u64,
        x0: f64,
        y0: f64,
        t0: f64,
    },
    /// A float in hand: the one just put (`float` None until the app names it), or one held past
    /// the beat. `from` is where it was lifted; `over` where the hand is now.
    Holding {
        id: u64,
        x0: f64,
        y0: f64,
        float: Option<u32>,
        from: LonLat,
        carried: bool,
        over: Hit,
    },
    /// The globe's drag: the finger, and the point of the globe it took.
    Orbit {
        id: u64,
        x: f64,
        y: f64,
        grab: Option<LonLat>,
    },
    /// Two fingers; `zoom0` the zoom when they landed and `scale` their distance's ratio since,
    /// so a pinch out past the whole globe can be known at the lift.
    Pinch {
        a: u64,
        b: u64,
        zoom0: f64,
        scale: f64,
    },
    /// Three fingers: waiting for a swipe (left undo, right redo, up home, down clear the sea).
    Three {
        x0: f64,
        y0: f64,
        done: bool,
    },
    /// A finger on the date: `was_running` is the clock as the finger found it, given back when
    /// the scrub (or the flick after it) ends.
    DateDown {
        id: u64,
        x0: f64,
        t0: f64,
        day: f64,
        scrubbing: bool,
        was_running: bool,
    },
    /// A finger down on the credit line, or anywhere while the page of credits is open: a tap
    /// toggles the page; a drag is the globe's.
    CreditsDown {
        id: u64,
        x0: f64,
        y0: f64,
    },
    /// A finger on a lamp's row in the panel (R6): a tap toggles it among the chosen, the beat
    /// rides it, a drag is the globe's.
    PanelDown {
        id: u64,
        x0: f64,
        y0: f64,
        t0: f64,
        float: u32,
        rode: bool,
    },
    /// A finger on the record control (R9): `done` once it has acted (the stop at touch-down, or
    /// the export after a rest), so the lift does nothing more.
    RecordDown {
        id: u64,
        x0: f64,
        y0: f64,
        t0: f64,
        done: bool,
    },
    /// One finger conducting (R11): its stroke in `Gestures::stroke`; `target` the lamp it began
    /// nearest (None: every lamp out); `fired` once its gesture is given (a sweep at its peak speed, a
    /// circle at three quarters of a turn), so the lift adds nothing; `fermata` once it has rested the
    /// beat (the date held till the lift, then given back as `was_running`); `peak` the stroke's
    /// fastest so far, points a second.
    Conduct {
        id: u64,
        x0: f64,
        y0: f64,
        t0: f64,
        target: Option<u32>,
        fired: bool,
        fermata: bool,
        was_running: bool,
        peak: f64,
    },
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
    /// The conducting finger's stroke, (t, x, y) (R11).
    stroke: Vec<(f64, f64, f64)>,
}

impl Default for Gestures {
    fn default() -> Self {
        Self::new()
    }
}

impl Gestures {
    pub fn new() -> Gestures {
        Gestures {
            state: State::Idle,
            fingers: BTreeMap::new(),
            last_tap: None,
            samples: VecDeque::new(),
            date_spin: 0.0,
            date_restore: None,
            date_target: None,
            last_shake: None,
            now: 0.0,
            last_cycle: None,
            stroke: Vec::new(),
        }
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
            State::DateDown {
                scrubbing: false, ..
            } => "date",
            State::DateDown {
                scrubbing: true, ..
            } => "scrubbing",
            State::CreditsDown { .. } => "credits",
            State::PanelDown { .. } => "panel",
            State::RecordDown { .. } => "record",
            State::Conduct { fermata: true, .. } => "fermata",
            State::Conduct { .. } => "conduct",
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

    /// The lamp whose mark in the panel a finger is on and how long it has been there (R10), for the
    /// look to swell that lamp out on the sea while its mark is pressed (`motion::thicken`); None when
    /// no finger is on a mark, or the finger has turned into a drag of the globe.
    pub fn panel_pressing_since(&self, now: f64) -> Option<(u32, f64)> {
        match self.state {
            State::PanelDown {
                t0,
                float,
                rode: false,
                ..
            } => Some((float, now - t0)),
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
            out.intents.push(if again {
                Intent::ClearAll
            } else {
                Intent::Undo
            });
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
            out.camera.push(CameraMove::Pinch {
                grab: probe.hit(x, y).place(),
                ax: x,
                ay: y,
                bx: x,
                by: y,
                factor,
            });
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
                    self.state = State::CreditsDown {
                        id: e.id,
                        x0: e.x,
                        y0: e.y,
                    };
                    return;
                }
                // (R11) while a take records, one finger on the globe conducts: nothing rings, is
                // put or chosen; the record control, the date and the panel answer as ever
                if probe.conducting()
                    && matches!(hit, Hit::Sea(_) | Hit::Float(..) | Hit::Land(_) | Hit::Off)
                {
                    self.last_tap = None;
                    let reach = motion::CONDUCT_NEAR * probe.frame().reading;
                    let target = probe
                        .lamps_on_screen()
                        .into_iter()
                        .map(|(id, x, y)| ((x - e.x).hypot(y - e.y), id))
                        .filter(|(d, _)| *d <= reach)
                        .min_by(|a, b| a.0.total_cmp(&b.0))
                        .map(|(_, id)| id);
                    self.stroke.clear();
                    self.stroke.push((e.t, e.x, e.y));
                    self.state = State::Conduct {
                        id: e.id,
                        x0: e.x,
                        y0: e.y,
                        t0: e.t,
                        target,
                        fired: false,
                        fermata: false,
                        was_running: probe.running(),
                        peak: 0.0,
                    };
                    return;
                }
                self.state = match hit {
                    Hit::Sea(at) => {
                        out.intents.push(Intent::Ring { at });
                        out.intents.push(Intent::Press { at, since: e.t });
                        State::Pressing {
                            id: e.id,
                            x0: e.x,
                            y0: e.y,
                            t0: e.t,
                            at,
                        }
                    }
                    Hit::Float(float, at) => {
                        out.intents.push(Intent::PressFloat { float, since: e.t });
                        State::Resting {
                            id: e.id,
                            x0: e.x,
                            y0: e.y,
                            t0: e.t,
                            float,
                            at,
                        }
                    }
                    Hit::Date => {
                        // the date taken in hand where it is: a glide still under way ends here,
                        // and the clock as it was before that scrub is the one this touch keeps
                        self.date_target = None;
                        let was_running = self.date_restore.take().unwrap_or(probe.running());
                        if was_running != probe.running() {
                            out.intents.push(Intent::SetRunning {
                                running: was_running,
                            });
                        }
                        State::DateDown {
                            id: e.id,
                            x0: e.x,
                            t0: e.t,
                            day: probe.day(),
                            scrubbing: false,
                            was_running,
                        }
                    }
                    Hit::Record => {
                        self.last_tap = None;
                        if probe.recording() {
                            // the take ends at this touch-down, so the touch that stops it is not
                            // in it, and the clip is asked for at once (the share sheet follows)
                            out.intents.push(Intent::Record { on: false });
                            out.intents.push(Intent::Export);
                            State::RecordDown {
                                id: e.id,
                                x0: e.x,
                                y0: e.y,
                                t0: e.t,
                                done: true,
                            }
                        } else {
                            State::RecordDown {
                                id: e.id,
                                x0: e.x,
                                y0: e.y,
                                t0: e.t,
                                done: false,
                            }
                        }
                    }
                    Hit::Panel(float) => {
                        // the row's cloud shows while the finger is on it, as a float's does
                        out.intents.push(Intent::PressFloat { float, since: e.t });
                        self.last_tap = None;
                        State::PanelDown {
                            id: e.id,
                            x0: e.x,
                            y0: e.y,
                            t0: e.t,
                            float,
                            rode: false,
                        }
                    }
                    Hit::Credits => State::CreditsDown {
                        id: e.id,
                        x0: e.x,
                        y0: e.y,
                    },
                    Hit::Land(_) | Hit::Off => State::Down {
                        id: e.id,
                        x0: e.x,
                        y0: e.y,
                        t0: e.t,
                    },
                };
            }
            // a second finger before the beat, or during the globe's drag: a pinch
            (
                State::Pressing { id, .. }
                | State::Resting { id, .. }
                | State::Down { id, .. }
                | State::Orbit { id, .. }
                | State::CreditsDown { id, .. }
                | State::PanelDown { id, .. }
                | State::Conduct { id, .. },
                2,
            ) => {
                if matches!(
                    self.state,
                    State::Pressing { .. } | State::Resting { .. } | State::PanelDown { .. }
                ) {
                    out.intents.push(Intent::Unpress);
                }
                // (R11) a conducting finger joined by a second: the camera's, and a held date given back
                if let State::Conduct {
                    fermata: true,
                    was_running,
                    ..
                } = self.state
                {
                    out.intents.push(Intent::SetRunning {
                        running: was_running,
                    });
                }
                // the second finger rings the sea where it lands too, as the first did: a pinch
                // leaves two rings (R7); not while conducting, where the sea answers only the cues
                if let (Hit::Sea(at), false) = (probe.hit(e.x, e.y), probe.conducting()) {
                    out.intents.push(Intent::Ring { at });
                }
                self.state = State::Pinch {
                    a: id,
                    b: e.id,
                    zoom0: probe.frame().zoom,
                    scale: 1.0,
                };
            }
            // a third finger ends the pinch where it is and waits for a swipe
            (State::Pinch { .. }, 3) => {
                self.state = State::Three {
                    x0: self.mean_x(),
                    y0: self.mean_y(),
                    done: false,
                };
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
        let Some(prev) = self.fingers.get(&e.id).copied() else {
            return;
        };
        self.fingers.insert(e.id, Finger { x: e.x, y: e.y });
        let slop = Self::slop(probe);
        if let State::Conduct { id, .. } = self.state {
            if id == e.id {
                self.conduct_moved(e, probe, out);
                return;
            }
        }
        match self.state {
            State::Pressing { id, x0, y0, t0, .. } | State::Resting { id, x0, y0, t0, .. }
                if id == e.id =>
            {
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
            State::RecordDown {
                id,
                x0,
                y0,
                done: false,
                ..
            } if id == e.id => {
                if (e.x - x0).hypot(e.y - y0) > slop {
                    self.begin_orbit(id, x0, y0, e, probe, out);
                }
            }
            State::PanelDown {
                id, x0, y0, rode, ..
            } if id == e.id => {
                if (e.x - x0).hypot(e.y - y0) > slop {
                    if !rode {
                        out.intents.push(Intent::Unpress);
                    }
                    self.begin_orbit(id, x0, y0, e, probe, out);
                }
            }
            State::Holding {
                id,
                x0,
                y0,
                float,
                from,
                carried,
                ..
            } if id == e.id => {
                let carried = carried || (e.x - x0).hypot(e.y - y0) > slop;
                if carried {
                    let over = probe.hit(e.x, e.y);
                    if let (Some(float), Some(to)) = (float, over.place()) {
                        out.intents.push(Intent::Carry { float, to });
                    }
                    // (off the disc the float stays where it was last over water, in hand, until
                    // `Intent::Carry` can say "over the rim": DEMANDS.md)
                    self.state = State::Holding {
                        id,
                        x0,
                        y0,
                        float,
                        from,
                        carried: true,
                        over,
                    };
                }
            }
            State::Orbit { id, x, y, grab } if id == e.id => {
                out.camera.push(CameraMove::Drag {
                    grab,
                    x: e.x,
                    y: e.y,
                    dx: e.x - x,
                    dy: e.y - y,
                });
                self.sample(e);
                self.state = State::Orbit {
                    id,
                    x: e.x,
                    y: e.y,
                    grab,
                };
            }
            State::Pinch { a, b, zoom0, scale } if e.id == a || e.id == b => {
                let other = self.fingers[&if e.id == a { b } else { a }];
                let d0 = (prev.x - other.x).hypot(prev.y - other.y).max(1.0);
                let d1 = (e.x - other.x).hypot(e.y - other.y).max(1.0);
                let (ax, ay) = ((prev.x + other.x) / 2.0, (prev.y + other.y) / 2.0);
                let (bx, by) = ((e.x + other.x) / 2.0, (e.y + other.y) / 2.0);
                out.camera.push(CameraMove::Pinch {
                    grab: probe.hit(ax, ay).place(),
                    ax,
                    ay,
                    bx,
                    by,
                    factor: d1 / d0,
                });
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
                    out.camera.push(CameraMove::Twist {
                        x: bx,
                        y: by,
                        angle: turn,
                    });
                }
                self.state = State::Pinch {
                    a,
                    b,
                    zoom0,
                    scale: scale * d1 / d0,
                };
            }
            State::Three {
                x0,
                y0,
                done: false,
            } => {
                let (dx, dy) = (self.mean_x() - x0, self.mean_y() - y0);
                if dx.abs() >= motion::SWIPE || dy.abs() >= motion::SWIPE {
                    out.intents.push(if dx.abs() >= dy.abs() {
                        if dx < 0.0 {
                            Intent::Undo
                        } else {
                            Intent::Redo
                        }
                    } else if dy < 0.0 {
                        Intent::Home
                    } else {
                        Intent::ClearAll
                    });
                    self.state = State::Three { x0, y0, done: true };
                }
            }
            State::DateDown {
                id,
                x0,
                t0,
                day,
                scrubbing,
                was_running,
            } if id == e.id => {
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
                    self.state = State::DateDown {
                        id,
                        x0,
                        t0,
                        day,
                        scrubbing: true,
                        was_running,
                    };
                }
            }
            _ => {}
        }
    }

    fn begin_orbit(
        &mut self,
        id: u64,
        x0: f64,
        y0: f64,
        e: &PointerEvent,
        probe: &dyn Probe,
        out: &mut Out,
    ) {
        let grab = probe.hit(x0, y0).place();
        out.camera.push(CameraMove::Drag {
            grab,
            x: e.x,
            y: e.y,
            dx: e.x - x0,
            dy: e.y - y0,
        });
        self.samples.clear();
        self.sample(e);
        self.state = State::Orbit {
            id,
            x: e.x,
            y: e.y,
            grab,
        };
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
        let recent: Vec<_> = self
            .samples
            .iter()
            .filter(|s| now - s.0 <= motion::FLING_WINDOW)
            .collect();
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
        if let State::Conduct { id, .. } = self.state {
            if id == e.id {
                self.stroke.push((e.t, e.x, e.y));
                self.conduct_up(e, probe, out);
                self.state = State::Idle;
                return;
            }
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
            State::Resting {
                id, t0, float, at, ..
            } if id == e.id => {
                if e.t - t0 >= motion::BEAT {
                    self.rest_elapsed(probe, out);
                    self.up_from_hold(e, probe, out);
                    return;
                }
                out.intents.push(Intent::Unpress);
                // a tap turns the lamp to its next way of being chosen: its cloud, itself, let go;
                // the second tap of a double tap on it gives back what the first changed, so a
                // double tap on a lamp only flies in
                let again = self.last_tap.is_some_and(|(t, x, y)| {
                    e.t - t <= motion::DOUBLE_TAP_S
                        && (e.x - x).hypot(e.y - y) <= motion::DOUBLE_TAP_PT
                });
                match self.last_cycle {
                    Some((f, before)) if again && f == float => {
                        out.intents.push(Intent::Show {
                            float,
                            shown: before,
                        });
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
                self.state = State::Orbit {
                    id: rest,
                    x: f.x,
                    y: f.y,
                    grab: probe.hit(f.x, f.y).place(),
                };
            }
            State::Three { .. } => {
                if self.fingers.is_empty() {
                    self.state = State::Idle;
                }
            }
            State::DateDown { id, scrubbing, .. } if id == e.id => {
                if !scrubbing {
                    // a tap: pause or play; while the page of credits is open, it closes the page
                    out.intents.push(if probe.credits_open() {
                        Intent::ToggleCredits
                    } else {
                        Intent::SetRunning {
                            running: !probe.running(),
                        }
                    });
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
        if let State::Holding {
            float: Some(float),
            from,
            carried,
            ..
        } = self.state
        {
            if !carried {
                out.intents.push(Intent::Release {
                    float,
                    at: Some(from),
                });
            } else {
                match probe.hit(e.x, e.y) {
                    Hit::Sea(p) | Hit::Float(_, p) => {
                        out.intents.push(Intent::Release { float, at: Some(p) })
                    }
                    Hit::Land(_) | Hit::Date | Hit::Credits | Hit::Panel(_) | Hit::Record => {
                        out.intents.push(Intent::Carry { float, to: from });
                        out.intents.push(Intent::Release {
                            float,
                            at: Some(from),
                        });
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
            if e.t - t <= motion::DOUBLE_TAP_S && (e.x - x).hypot(e.y - y) <= motion::DOUBLE_TAP_PT
            {
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
            // (R11) the system took the conducting finger: no cue, a held date given back
            State::Conduct {
                id,
                fermata,
                was_running,
                ..
            } if id == e.id => {
                if fermata {
                    out.intents.push(Intent::SetRunning {
                        running: was_running,
                    });
                }
                self.state = State::Idle;
            }
            State::Holding {
                id, float, from, ..
            } if id == e.id => {
                if let Some(float) = float {
                    out.intents.push(Intent::Carry { float, to: from });
                    out.intents.push(Intent::Release {
                        float,
                        at: Some(from),
                    });
                }
                self.state = State::Idle;
            }
            State::Pinch { a, b, .. } if e.id == a || e.id == b => {
                let rest = if e.id == a { b } else { a };
                match self.fingers.get(&rest).copied() {
                    Some(f) => {
                        self.state = State::Orbit {
                            id: rest,
                            x: f.x,
                            y: f.y,
                            grab: None,
                        }
                    }
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
            State::Down { id, .. }
            | State::Orbit { id, .. }
            | State::DateDown { id, .. }
            | State::CreditsDown { id, .. }
            | State::RecordDown { id, .. }
                if id == e.id =>
            {
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
                self.state = State::Holding {
                    id,
                    x0,
                    y0,
                    float: None,
                    from: at,
                    carried: false,
                    over: Hit::Sea(at),
                };
            }
            State::Resting {
                id,
                x0,
                y0,
                float,
                at,
                ..
            } => {
                out.intents.push(Intent::Hold { float });
                let from = probe.float_at(float).unwrap_or(at);
                self.state = State::Holding {
                    id,
                    x0,
                    y0,
                    float: Some(float),
                    from,
                    carried: false,
                    over: Hit::Sea(from),
                };
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
            State::Pressing { t0, .. } | State::Resting { t0, .. } if now - t0 >= motion::BEAT => {
                self.rest_elapsed(probe, &mut out)
            }
            // (R11) a conducting finger that rests the beat without moving holds the date: a fermata
            State::Conduct {
                id,
                x0,
                y0,
                t0,
                target,
                fired: false,
                fermata: false,
                peak,
                ..
            } if now - t0 >= motion::BEAT
                && self
                    .stroke
                    .iter()
                    .all(|&(_, x, y)| (x - x0).hypot(y - y0) <= Self::slop(probe)) =>
            {
                let was_running = probe.running();
                if was_running {
                    out.intents.push(Intent::SetRunning { running: false });
                }
                self.state = State::Conduct {
                    id,
                    x0,
                    y0,
                    t0,
                    target,
                    fired: false,
                    fermata: true,
                    was_running,
                    peak,
                };
            }
            // a rest on the date (the finger still for the beat) opens the page of credits
            State::DateDown {
                t0,
                scrubbing: false,
                ..
            } if now - t0 >= motion::BEAT && !probe.credits_open() => {
                out.intents.push(Intent::ToggleCredits);
                self.state = State::Idle;
            }
            // a rest on a lamp's row rides it (the finger may still be down)
            // a rest on the record control, not recording: the studio's menu (R13: a journey clip of
            // the chosen lamps, the last take again, the lamps copied or pasted)
            State::RecordDown {
                id,
                x0,
                y0,
                t0,
                done: false,
            } if now - t0 >= motion::BEAT => {
                if !probe.recording() {
                    out.intents.push(Intent::Menu);
                }
                self.state = State::RecordDown {
                    id,
                    x0,
                    y0,
                    t0,
                    done: true,
                };
            }
            State::PanelDown {
                id,
                x0,
                y0,
                t0,
                float,
                rode: false,
            } if now - t0 >= motion::BEAT => {
                out.intents.push(Intent::Unpress);
                self.panel_rest(float, probe, &mut out);
                self.state = State::PanelDown {
                    id,
                    x0,
                    y0,
                    t0,
                    float,
                    rode: true,
                };
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
        let pending: f64 = out
            .intents
            .iter()
            .map(|i| {
                if let Intent::Scrub { days } = i {
                    *days
                } else {
                    0.0
                }
            })
            .sum();
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

    /// A conducting finger moves (R11): a loop of three quarters of a turn is a circle, given at once;
    /// a stroke is a sweep, given at its peak speed (the moment the hand's speed has fallen back from
    /// its fastest), not on the lift, so the sea answers the beat as the hand makes it.
    fn conduct_moved(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Out) {
        let State::Conduct {
            id,
            x0,
            y0,
            t0,
            target,
            fired,
            fermata,
            was_running,
            mut peak,
        } = self.state
        else {
            return;
        };
        let last = *self.stroke.last().unwrap();
        self.stroke.push((e.t, e.x, e.y));
        if fired || fermata {
            return;
        }
        let reading = probe.frame().reading;
        let (length, turn) = stroke_shape(&self.stroke, motion::STROKE_STEP * reading);
        if turn.abs() >= motion::CIRCLE_TURN {
            out.intents.push(Intent::Conduct {
                float: target,
                gesture: ConductGesture::Circle {
                    turn,
                    secs: e.t - t0,
                },
            });
            self.state = State::Conduct {
                id,
                x0,
                y0,
                t0,
                target,
                fired: true,
                fermata,
                was_running,
                peak,
            };
            return;
        }
        let _ = last;
        // the hand's speed over the last SPEED_WINDOW (so a 120 Hz finger's jitter is averaged and a
        // 20 Hz one is read the same)
        let n = self.stroke.len();
        let from = (0..n - 1)
            .rev()
            .find(|&i| e.t - self.stroke[i].0 >= motion::SPEED_WINDOW)
            .unwrap_or(0);
        let (ft, fx, fy) = self.stroke[from];
        let speed = (e.x - fx).hypot(e.y - fy) / (e.t - ft).max(1.0 / 240.0);
        let mut fire = false;
        if speed >= peak {
            peak = speed;
        } else if peak >= motion::SWEEP_PEAK * reading
            && speed <= motion::SWEEP_FALL * peak
            && length >= motion::SWEEP_LENGTH * reading
            && turn.abs() < motion::SWEEP_CURVE
            && e.t - t0 >= motion::SWEEP_AGE
        {
            // past the peak: the stroke so far is the sweep (a stroke still curving may be a circle)
            fire = true;
        }
        if fire {
            out.intents.push(Intent::Conduct {
                float: target,
                gesture: ConductGesture::Sweep {
                    dx: e.x - x0,
                    dy: e.y - y0,
                    secs: e.t - t0,
                },
            });
        }
        self.state = State::Conduct {
            id,
            x0,
            y0,
            t0,
            target,
            fired: fire,
            fermata,
            was_running,
            peak,
        };
    }

    /// The conducting finger lifts (R11): a fermata gives the date back; a gesture not yet given is
    /// given now (a stroke that never slowed before the lift is a sweep, a short touch a tap).
    fn conduct_up(&mut self, e: &PointerEvent, probe: &dyn Probe, out: &mut Out) {
        let State::Conduct {
            x0,
            y0,
            t0,
            target,
            fired,
            fermata,
            was_running,
            ..
        } = self.state
        else {
            return;
        };
        if fermata {
            out.intents.push(Intent::SetRunning {
                running: was_running,
            });
            return;
        }
        if fired {
            return;
        }
        let reading = probe.frame().reading;
        let (length, turn) = stroke_shape(&self.stroke, motion::STROKE_STEP * reading);
        let gesture = if turn.abs() >= motion::CIRCLE_TURN {
            ConductGesture::Circle {
                turn,
                secs: e.t - t0,
            }
        } else if length >= motion::SWEEP_LENGTH * reading {
            ConductGesture::Sweep {
                dx: e.x - x0,
                dy: e.y - y0,
                secs: e.t - t0,
            }
        } else {
            ConductGesture::Tap { x: x0, y: y0 }
        };
        out.intents.push(Intent::Conduct {
            float: target,
            gesture,
        });
    }

    /// Whether a conducting finger is resting and holding the date (R12, a fermata), for the look: a
    /// paused date while conducting may also be a tap on the date band, which this is not.
    pub fn fermata(&self) -> bool {
        matches!(self.state, State::Conduct { fermata: true, .. })
    }

    /// The conducting finger's stroke so far, (t, x, y) points (R11), for the look to draw the hand.
    pub fn conduct_stroke(&self) -> Option<&[(f64, f64, f64)]> {
        matches!(self.state, State::Conduct { .. }).then_some(&self.stroke[..])
    }

    /// A tap on a lamp's row: it joins the chosen, in front, or, chosen already, is let go; a lamp
    /// newly chosen out of sight or at the globe's rim is turned toward the reader (R6).
    /// (R10) The tap rings the water at the lamp itself, in the open sea where the reader can see it
    /// answer: the tie between a mark at the screen's corner and its lamp out in the world.
    fn panel_tap(&mut self, float: u32, probe: &dyn Probe, out: &mut Out) {
        if let Some(at) = probe.float_at(float) {
            out.intents.push(Intent::Ring { at });
        }
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
            match probe.choices().into_iter().rfind(|&c| c != float) {
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

/// A stroke's length and its turn, radians (in `conduct.rs`'s sense: positive clockwise as seen on a
/// y-down screen), over its
/// steps of at least `step` points (smaller jitters skipped, so a resting finger never turns) (R11).
pub fn stroke_shape(pts: &[(f64, f64, f64)], step: f64) -> (f64, f64) {
    let Some(&(_, x0, y0)) = pts.first() else {
        return (0.0, 0.0);
    };
    let (mut length, mut turn) = (0.0, 0.0);
    let mut prev = (x0, y0);
    let mut last: Option<(f64, f64)> = None;
    for &(_, x, y) in &pts[1..] {
        let (dx, dy) = (x - prev.0, y - prev.1);
        let d = dx.hypot(dy);
        if d < step {
            continue;
        }
        length += d;
        if let Some((ax, ay)) = last {
            // measured as `conduct.rs` measures a world's path on the screen (y down: positive is
            // clockwise as seen), so a loop picks the worlds that turned the way the hand did
            turn += (ax * dy - ay * dx).atan2(ax * dx + ay * dy);
        }
        last = Some((dx, dy));
        prev = (x, y);
    }
    (length, turn)
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
            (v[0] * c[0] + v[1] * c[1] + v[2] * c[2])
                .clamp(-1.0, 1.0)
                .acos()
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
        conducting: bool,
    }

    impl Fake {
        fn new() -> Fake {
            Fake {
                chosen: RefCell::new(Vec::new()),
                followed: RefCell::new(Vec::new()),
                day: RefCell::new(100.0),
                running: RefCell::new(true),
                behind: 0.0,
                conducting: false,
            }
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
            Frame {
                width: 393.0,
                height: 852.0,
                reading: 1.0,
                disc: Disc {
                    cx: 196.5,
                    cy: 400.0,
                    r: 180.0,
                },
                zoom: 1.0,
            }
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
        fn conducting(&self) -> bool {
            self.conducting
        }
        fn lamps_on_screen(&self) -> Vec<(u32, f64, f64)> {
            // two lamps on the face (the panel's stand-in rows lie at x 30, off the disc)
            vec![(7, 150.0, 400.0), (8, 260.0, 470.0)]
        }
        fn behind(&self) -> f64 {
            self.behind
        }
    }

    fn ev(phase: Phase, x: f64, y: f64, t: f64) -> PointerEvent {
        PointerEvent {
            id: 0,
            phase,
            x,
            y,
            t,
        }
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
        let first = g.feed(
            PointerEvent {
                id: 1,
                phase: Phase::Down,
                x: 150.0,
                y: 400.0,
                t: 0.0,
            },
            &p,
        );
        let second = g.feed(
            PointerEvent {
                id: 2,
                phase: Phase::Down,
                x: 250.0,
                y: 420.0,
                t: 0.05,
            },
            &p,
        );
        let rings: Vec<LonLat> = first
            .intents
            .iter()
            .chain(&second.intents)
            .filter_map(|i| {
                if let Intent::Ring { at } = i {
                    Some(*at)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(
            rings,
            vec![LonLat::new(15.0, 40.0), LonLat::new(25.0, 42.0)]
        );
        assert!(!second
            .intents
            .iter()
            .any(|i| matches!(i, Intent::Put { .. })));
    }

    #[test]
    fn panel_marks_choose_two_and_let_one_go_and_ring_at_their_lamps() {
        let (mut g, p) = (Gestures::new(), Fake::new());
        let a = tap(&mut g, &p, 30.0, 110.0, 0.0);
        let b = tap(&mut g, &p, 30.0, 150.0, 1.0);
        assert_eq!(*p.chosen.borrow(), vec![1, 2]);
        for i in a.iter().chain(&b) {
            assert!(!matches!(i, Intent::Put { .. } | Intent::Unchoose), "{i:?}");
        }
        // (R10) each tap rings the water once, at its own lamp, never under the finger
        let rings = |v: &[Intent]| {
            v.iter()
                .filter_map(|i| {
                    if let Intent::Ring { at } = i {
                        Some(*at)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(rings(&a), vec![p.float_at(1).unwrap()]);
        assert_eq!(rings(&b), vec![p.float_at(2).unwrap()]);
        // (R8) a second tap on a row: that lamp as itself, its cloud folded; a third lets it go
        let c = tap(&mut g, &p, 30.0, 110.0, 2.0);
        assert_eq!(*p.chosen.borrow(), vec![2]);
        assert_eq!(*p.followed.borrow(), vec![1]);
        tap(&mut g, &p, 30.0, 110.0, 2.5);
        assert!(p.followed.borrow().is_empty() && *p.chosen.borrow() == vec![2]);
        assert_eq!(rings(&c).len(), 1);
        let mut g2 = Gestures::new();
        tap(&mut g2, &p, 30.0, 190.0, 3.0);
        let out = g2.feed(ev(Phase::Down, 30.0, 190.0, 3.2), &p);
        assert!(out
            .camera
            .iter()
            .all(|m| !matches!(m, CameraMove::Fly { .. })));
    }

    #[test]
    fn a_rest_on_a_row_rides_it_once() {
        let (mut g, p) = (Gestures::new(), Fake::new());
        p.apply(&g.feed(ev(Phase::Down, 30.0, 150.0, 0.0), &p));
        // (R10) while the finger rests on the mark, its lamp is the one pressed, for the look's swell
        assert_eq!(g.panel_pressing_since(0.25), Some((2, 0.25)));
        let out = g.tick(0.5, &p);
        assert!(
            out.intents.contains(&Intent::Ride { float: 2 }),
            "{:?}",
            out.intents
        );
        assert_eq!(g.panel_pressing_since(0.55), None);
        assert!(g
            .tick(0.6, &p)
            .intents
            .iter()
            .all(|i| !matches!(i, Intent::Ride { .. })));
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
        assert!(
            step > 0.0 && step <= 0.2 * motion::SCRUB_RATE / 60.0 + 1e-9,
            "{step}"
        );
    }

    fn conductor() -> (Gestures, Fake) {
        let mut p = Fake::new();
        p.conducting = true;
        (Gestures::new(), p)
    }

    fn conducts(out: &Out) -> Vec<(Option<u32>, ConductGesture)> {
        out.intents
            .iter()
            .filter_map(|i| {
                if let Intent::Conduct { float, gesture } = i {
                    Some((*float, *gesture))
                } else {
                    None
                }
            })
            .collect()
    }

    #[test]
    fn conducting_a_tap_near_a_lamp_cues_it_and_rings_nothing() {
        let (mut g, p) = conductor();
        let down = g.feed(ev(Phase::Down, 160.0, 410.0, 0.0), &p);
        assert!(down.intents.is_empty(), "{:?}", down.intents);
        let up = g.feed(ev(Phase::Up, 161.0, 410.0, 0.1), &p);
        assert_eq!(
            conducts(&up),
            vec![(Some(7), ConductGesture::Tap { x: 160.0, y: 410.0 })]
        );
        assert!(up.intents.iter().all(|i| !matches!(
            i,
            Intent::Ring { .. } | Intent::Put { .. } | Intent::Ride { .. } | Intent::Unchoose
        )));
        // open water, far from both lamps: every lamp
        g.feed(ev(Phase::Down, 300.0, 250.0, 1.0), &p);
        let up = g.feed(ev(Phase::Up, 300.0, 250.0, 1.1), &p);
        assert_eq!(
            conducts(&up),
            vec![(None, ConductGesture::Tap { x: 300.0, y: 250.0 })]
        );
    }

    /// A stroke east, 200 points over 0.3 s, fastest half-way (a smoothstep); frames at `fps`.
    fn stroke(g: &mut Gestures, p: &Fake, fps: f64) -> (Vec<(f64, Out)>, Out) {
        let mut outs = Vec::new();
        g.feed(ev(Phase::Down, 100.0, 300.0, 0.0), p);
        let x = |t: f64| {
            let u = (t / 0.3).min(1.0);
            100.0 + 200.0 * u * u * (3.0 - 2.0 * u)
        };
        let mut t = 0.0;
        while t < 0.4 {
            t += 1.0 / fps;
            outs.push((t, g.feed(ev(Phase::Move, x(t), 300.0, t), p)));
        }
        let up = g.feed(ev(Phase::Up, x(t), 300.0, t + 0.01), p);
        (outs, up)
    }

    #[test]
    fn conducting_a_sweep_is_given_at_its_peak_not_on_the_lift() {
        for fps in [60.0, 20.0] {
            let (mut g, p) = conductor();
            let (outs, up) = stroke(&mut g, &p, fps);
            let fired: Vec<_> = outs
                .iter()
                .filter_map(|(t, o)| conducts(o).first().map(|c| (*t, c.1)))
                .collect();
            assert_eq!(fired.len(), 1, "{fps} fps: {fired:?}");
            let (t, gesture) = fired[0];
            // the peak is at 0.15 s; given as the hand slows from it, well before the lift (0.4 s)
            assert!(
                t > 0.14 && t < 0.28,
                "{fps} fps: given at {t}, not near the peak"
            );
            match gesture {
                ConductGesture::Sweep { dx, dy, secs } => assert!(
                    dx > 80.0 && dy.abs() < 1.0 && (secs - t).abs() < 1e-9,
                    "{gesture:?}"
                ),
                other => panic!("{other:?}"),
            }
            // the start (100, 300) is more than 90 points from both lamps: every lamp
            assert!(
                conducts(&outs.iter().find(|(tt, _)| *tt == t).unwrap().1)[0]
                    .0
                    .is_none()
            );
            assert!(
                conducts(&up).is_empty(),
                "the lift gave another: {:?}",
                up.intents
            );
        }
    }

    #[test]
    fn conducting_a_loop_is_a_circle_at_three_quarters_of_a_turn() {
        let (mut g, p) = conductor();
        g.feed(ev(Phase::Down, 260.0, 400.0, 0.0), &p);
        let mut given = Vec::new();
        for i in 1..=40 {
            let a = i as f64 / 40.0 * 2.0 * std::f64::consts::PI;
            // round (200, 400) at 60 points, clockwise as seen (y down)
            let (x, y) = (200.0 + 60.0 * a.cos(), 400.0 + 60.0 * a.sin());
            let o = g.feed(ev(Phase::Move, x, y, i as f64 * 0.02), &p);
            given.extend(conducts(&o).into_iter().map(|c| (i, c)));
        }
        let up = g.feed(ev(Phase::Up, 260.0, 400.0, 0.9), &p);
        assert_eq!(given.len(), 1, "{given:?}");
        let (i, (target, gesture)) = given[0];
        assert!((28..=32).contains(&i), "given at step {i}");
        // it began 17 points from lamp 8's... no: (260, 400) is 70 from lamp 8 at (260, 470), 110 from 7
        assert_eq!(target, Some(8));
        assert!(
            matches!(gesture, ConductGesture::Circle { turn, .. } if turn > 4.0),
            "{gesture:?}"
        );
        assert!(conducts(&up).is_empty());
    }

    #[test]
    fn conducting_a_rest_holds_the_date_till_the_lift() {
        let (mut g, p) = conductor();
        p.apply(&g.feed(ev(Phase::Down, 300.0, 250.0, 0.0), &p));
        let o = g.tick(0.45, &p);
        assert_eq!(o.intents, vec![Intent::SetRunning { running: false }]);
        p.apply(&o);
        assert_eq!(g.describe(), "fermata");
        assert!(g.fermata());
        let up = g.feed(ev(Phase::Up, 302.0, 250.0, 2.0), &p);
        assert_eq!(up.intents, vec![Intent::SetRunning { running: true }]);
    }

    #[test]
    fn conducting_two_fingers_are_the_cameras_and_one_finger_never_turns_the_globe() {
        let (mut g, p) = conductor();
        g.feed(ev(Phase::Down, 300.0, 250.0, 0.0), &p);
        let o = g.feed(ev(Phase::Move, 330.0, 250.0, 0.05), &p);
        assert!(
            o.camera
                .iter()
                .all(|m| !matches!(m, CameraMove::Drag { .. })),
            "{:?}",
            o.camera
        );
        let mut b = ev(Phase::Down, 200.0, 250.0, 0.1);
        b.id = 2;
        let o = g.feed(b, &p);
        assert!(o.intents.iter().all(|i| !matches!(i, Intent::Ring { .. })));
        assert_eq!(g.describe(), "pinch");
        // not conducting, the hand is as ever: a tap on the sea rings it
        let (mut g, p) = (Gestures::new(), Fake::new());
        let o = g.feed(ev(Phase::Down, 300.0, 250.0, 0.0), &p);
        assert!(conducts(&o).is_empty());
    }

    /// A Conduct given, by finger (or none).
    type Given = (Option<u32>, ConductGesture);

    /// Feeds one finger along `path(u)`, u 0..1 over `secs`, at `hz` events a second; returns every
    /// Conduct given with the time it came, and the lift's.
    fn draw(
        g: &mut Gestures,
        p: &Fake,
        t0: f64,
        secs: f64,
        hz: f64,
        path: impl Fn(f64) -> (f64, f64),
    ) -> (Vec<(f64, Given)>, Vec<Given>) {
        let (x0, y0) = path(0.0);
        let mut got = Vec::new();
        got.extend(
            conducts(&g.feed(ev(Phase::Down, x0, y0, t0), p))
                .into_iter()
                .map(|c| (t0, c)),
        );
        let n = (secs * hz).ceil() as usize;
        for i in 1..=n {
            let t = t0 + secs * i as f64 / n as f64;
            let (x, y) = path(i as f64 / n as f64);
            got.extend(
                conducts(&g.feed(ev(Phase::Move, x, y, t), p))
                    .into_iter()
                    .map(|c| (t, c)),
            );
            got.extend(conducts(&g.tick(t, p)).into_iter().map(|c| (t, c)));
        }
        let (x, y) = path(1.0);
        let up = conducts(&g.feed(ev(Phase::Up, x, y, t0 + secs + 0.01), p));
        (got, up)
    }

    fn kind(g: &ConductGesture) -> &'static str {
        match g {
            ConductGesture::Tap { .. } => "tap",
            ConductGesture::Sweep { .. } => "sweep",
            ConductGesture::Circle { .. } => "circle",
        }
    }

    /// Exactly one gesture of `want` for the whole touch.
    fn one(
        got: &[(f64, (Option<u32>, ConductGesture))],
        up: &[(Option<u32>, ConductGesture)],
        want: &str,
        what: &str,
    ) {
        let all: Vec<&ConductGesture> = got
            .iter()
            .map(|g| &g.1 .1)
            .chain(up.iter().map(|g| &g.1))
            .collect();
        assert_eq!(all.len(), 1, "{what}: {all:?}");
        assert_eq!(kind(all[0]), want, "{what}: {all:?}");
    }

    #[test]
    fn conducting_plausible_touches_are_read_as_meant_at_120_and_20_events_a_second() {
        use std::f64::consts::PI;
        for hz in [120.0, 20.0] {
            // a sloppy tap: the finger rolls 12 points in a tenth of a second
            let (mut g, p) = conductor();
            let (got, up) = draw(&mut g, &p, 0.0, 0.1, hz, |u| {
                (300.0 + 12.0 * u, 250.0 + 4.0 * u)
            });
            one(&got, &up, "tap", &format!("sloppy tap {hz}"));
            // a quick sloppy tap: 18 points in 50 ms (360 pt/s, but too short to be a stroke)
            let (mut g, p) = conductor();
            let (got, up) = draw(&mut g, &p, 0.0, 0.05, hz, |u| (300.0 + 18.0 * u, 250.0));
            one(&got, &up, "tap", &format!("quick sloppy tap {hz}"));
            // a slow sweep: 120 points over a second, never fast (given at the lift)
            let (mut g, p) = conductor();
            let (got, up) = draw(&mut g, &p, 0.0, 1.0, hz, |u| {
                (100.0 + 120.0 * u, 300.0 - 10.0 * u)
            });
            one(&got, &up, "sweep", &format!("slow sweep {hz}"));
            // a sweep ending in a curl (the hand turning back as it slows): still a sweep, given at its peak
            let (mut g, p) = conductor();
            let (got, up) = draw(&mut g, &p, 0.0, 0.5, hz, |u| {
                let a = (u * 1.4).min(1.0);
                let s = a * a * (3.0 - 2.0 * a);
                let curl = ((u - 0.7).max(0.0) / 0.3) * PI * 0.6;
                (
                    100.0 + 180.0 * s + 20.0 * curl.sin(),
                    300.0 - 20.0 * (1.0 - curl.cos()),
                )
            });
            one(&got, &up, "sweep", &format!("sweep with a curl {hz}"));
            assert!(up.is_empty(), "given at the lift, not the peak");
            // a small circle: 20 points across, half a second
            let (mut g, p) = conductor();
            let (got, up) = draw(&mut g, &p, 0.0, 0.5, hz, |u| {
                (
                    300.0 + 20.0 * (2.0 * PI * u).cos(),
                    250.0 + 20.0 * (2.0 * PI * u).sin(),
                )
            });
            one(&got, &up, "circle", &format!("small circle {hz}"));
            // a lopsided loop, an ellipse 60 by 25, uneven speed
            let (mut g, p) = conductor();
            let (got, up) = draw(&mut g, &p, 0.0, 0.7, hz, |u| {
                (
                    300.0 + 60.0 * (2.0 * PI * u).cos(),
                    250.0 - 25.0 * (2.0 * PI * u).sin(),
                )
            });
            one(&got, &up, "circle", &format!("ellipse {hz}"));
            if let ConductGesture::Circle { turn, .. } = got[0].1 .1 {
                // drawn anticlockwise as seen (y down, the sine negated): negative in the seam's sense
                assert!(turn < 0.0, "{turn}");
            }
            // a slow loop, two seconds, given at three quarters
            let (mut g, p) = conductor();
            let (got, up) = draw(&mut g, &p, 0.0, 2.0, hz, |u| {
                (
                    300.0 + 50.0 * (2.0 * PI * u).cos(),
                    250.0 + 50.0 * (2.0 * PI * u).sin(),
                )
            });
            one(&got, &up, "circle", &format!("slow loop {hz}"));
            assert!(got[0].0 > 1.3 && got[0].0 < 1.75, "given at {}", got[0].0);
        }
    }

    #[test]
    fn conducting_a_two_finger_move_that_starts_as_one_finger_cues_nothing() {
        // the first finger lands and flicks 36 points, slowing, before the second lands 70 ms later
        // (past its peak by 33 ms: without the stroke's least age it would have cued a sweep)
        let (mut g, p) = conductor();
        let mut got = Vec::new();
        got.extend(conducts(&g.feed(ev(Phase::Down, 200.0, 300.0, 0.0), &p)));
        let mut x = 200.0;
        for i in 1..=8 {
            let t = i as f64 / 120.0;
            x += 9.0 - i as f64;
            got.extend(conducts(&g.feed(ev(Phase::Move, x, 300.0, t), &p)));
        }
        let mut b = ev(Phase::Down, 300.0, 380.0, 0.07);
        b.id = 2;
        got.extend(conducts(&g.feed(b, &p)));
        assert_eq!(g.describe(), "pinch");
        for i in 1..=20 {
            let t = 0.07 + i as f64 / 120.0;
            got.extend(conducts(
                &g.feed(ev(Phase::Move, 240.0 + 3.0 * i as f64, 300.0, t), &p),
            ));
            let mut m = ev(Phase::Move, 300.0 + 3.0 * i as f64, 380.0, t);
            m.id = 2;
            got.extend(conducts(&g.feed(m, &p)));
        }
        got.extend(conducts(&g.feed(ev(Phase::Up, 300.0, 300.0, 0.3), &p)));
        let mut u = ev(Phase::Up, 360.0, 380.0, 0.31);
        u.id = 2;
        got.extend(conducts(&g.feed(u, &p)));
        assert!(got.is_empty(), "a camera move cued {got:?}");
    }
}
