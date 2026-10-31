//! Pointer events to intents (R0's plain vocabulary).
//!
//! One finger: down on the sea and up within a beat is a tap (a ring); resting a beat puts a float
//! and holds it (the finger then carries it; lifting sets it down, or takes it out past the rim);
//! down on a float and resting holds it (its maybes), moving carries it; a tap on a float rides it;
//! moving before the beat is the globe's drag, with a fling at the lift. Two fingers zoom and turn.

use super::motion::{BEAT, REST_SLOP, TAP};
use crate::model::{Hit, Intent, Phase, PointerEvent};

#[derive(Clone, Copy, Debug, PartialEq)]
enum State {
    Idle,
    /// Down, undecided: a tap, a rest or a drag.
    Pressed { hit: Hit, x0: f64, y0: f64, t0: f64 },
    /// Dragging the globe.
    Orbit,
    /// Holding a float (put just now, or picked up), carrying it when it moves.
    Holding { float: u32, moved: bool },
    /// Two fingers.
    Pinch,
}

/// What the gestures ask the app to do beyond intents: the camera's own moves.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraMove {
    Turn { dx: f64, dy: f64 },
    Fling { vx: f64, vy: f64 },
    Zoom { factor: f64 },
    Stop,
}

#[derive(Default)]
pub struct Out {
    pub intents: Vec<Intent>,
    pub camera: Vec<CameraMove>,
}

struct Finger {
    id: u64,
    x: f64,
    y: f64,
    t: f64,
    /// The last few moves, for the fling.
    recent: Vec<(f64, f64, f64)>,
}

pub struct Gestures {
    state: State,
    fingers: Vec<Finger>,
    pinch_distance: f64,
    put_float: Option<u32>,
}

impl Default for Gestures {
    fn default() -> Self {
        Self::new()
    }
}

impl Gestures {
    pub fn new() -> Gestures {
        Gestures { state: State::Idle, fingers: Vec::new(), pinch_distance: 0.0, put_float: None }
    }

    /// The app tells which float the last `Put` made (so the finger goes on holding it).
    pub fn float_put(&mut self, id: u32) {
        self.put_float = Some(id);
        if let State::Pressed { .. } = self.state {
            self.state = State::Holding { float: id, moved: false };
        }
    }

    /// An event, with what was under it and under the finger now.
    pub fn feed(&mut self, ev: PointerEvent, hit: impl Fn(f64, f64) -> Hit) -> Out {
        let mut out = Out::default();
        match ev.phase {
            Phase::Down => {
                self.fingers.push(Finger { id: ev.id, x: ev.x, y: ev.y, t: ev.t, recent: vec![(ev.x, ev.y, ev.t)] });
                if self.fingers.len() == 1 {
                    self.state = State::Pressed { hit: hit(ev.x, ev.y), x0: ev.x, y0: ev.y, t0: ev.t };
                    out.camera.push(CameraMove::Stop);
                } else if self.fingers.len() == 2 {
                    if let State::Holding { float, .. } = self.state {
                        // a second finger ends a carry where the float is
                        let f = &self.fingers[0];
                        let at = match hit(f.x, f.y) {
                            Hit::Sea(p) | Hit::Land(p) | Hit::Float(_, p) => Some(p),
                            _ => None,
                        };
                        out.intents.push(Intent::Release { float, at });
                    }
                    self.state = State::Pinch;
                    self.pinch_distance = self.distance();
                }
            }
            Phase::Move => {
                let Some(i) = self.fingers.iter().position(|f| f.id == ev.id) else { return out };
                let (px, py) = (self.fingers[i].x, self.fingers[i].y);
                self.fingers[i].x = ev.x;
                self.fingers[i].y = ev.y;
                self.fingers[i].t = ev.t;
                self.fingers[i].recent.push((ev.x, ev.y, ev.t));
                if self.fingers[i].recent.len() > 6 {
                    self.fingers[i].recent.remove(0);
                }
                match self.state {
                    State::Pressed { hit: h, x0, y0, .. } => {
                        let far = ((ev.x - x0).powi(2) + (ev.y - y0).powi(2)).sqrt() > REST_SLOP;
                        if far {
                            match h {
                                Hit::Float(id, _) => {
                                    // a drag that begins on a float carries it
                                    self.state = State::Holding { float: id, moved: true };
                                    out.intents.push(Intent::Hold { float: id });
                                    if let Hit::Sea(p) | Hit::Land(p) | Hit::Float(_, p) = hit(ev.x, ev.y) {
                                        out.intents.push(Intent::Carry { float: id, to: p });
                                    }
                                }
                                _ => {
                                    self.state = State::Orbit;
                                    out.camera.push(CameraMove::Turn { dx: ev.x - x0, dy: ev.y - y0 });
                                }
                            }
                        }
                    }
                    State::Orbit => out.camera.push(CameraMove::Turn { dx: ev.x - px, dy: ev.y - py }),
                    State::Holding { float, .. } => {
                        self.state = State::Holding { float, moved: true };
                        if let Hit::Sea(p) | Hit::Land(p) | Hit::Float(_, p) = hit(ev.x, ev.y) {
                            out.intents.push(Intent::Carry { float, to: p });
                        }
                    }
                    State::Pinch => {
                        if self.fingers.len() >= 2 {
                            let d = self.distance();
                            if self.pinch_distance > 1.0 && d > 1.0 {
                                out.camera.push(CameraMove::Zoom { factor: d / self.pinch_distance });
                            }
                            self.pinch_distance = d;
                            // the two fingers' centre turns the globe
                            let (cx, cy) = self.centre();
                            let j = 1 - i.min(1);
                            let (ox, oy) = (self.fingers[j].x, self.fingers[j].y);
                            let (pcx, pcy) = ((px + ox) / 2.0, (py + oy) / 2.0);
                            out.camera.push(CameraMove::Turn { dx: cx - pcx, dy: cy - pcy });
                        }
                    }
                    State::Idle => {}
                }
            }
            Phase::Up | Phase::Cancel => {
                let Some(i) = self.fingers.iter().position(|f| f.id == ev.id) else { return out };
                let finger = self.fingers.remove(i);
                match self.state {
                    State::Pressed { hit: h, t0, .. } => {
                        if ev.t - t0 <= TAP + BEAT {
                            match h {
                                Hit::Sea(p) => out.intents.push(Intent::Ring { at: p }),
                                Hit::Float(id, _) => out.intents.push(Intent::Ride { float: id }),
                                _ => {}
                            }
                        }
                        self.state = State::Idle;
                    }
                    State::Orbit => {
                        if let Some((vx, vy)) = Self::speed(&finger) {
                            out.camera.push(CameraMove::Fling { vx, vy });
                        }
                        self.state = State::Idle;
                    }
                    State::Holding { float, .. } => {
                        let at = match hit(ev.x, ev.y) {
                            Hit::Sea(p) | Hit::Land(p) | Hit::Float(_, p) => Some(p),
                            _ => None,
                        };
                        out.intents.push(Intent::Release { float, at: if ev.phase == Phase::Cancel { at.or(None) } else { at } });
                        self.state = State::Idle;
                    }
                    State::Pinch => {
                        if self.fingers.len() < 2 {
                            self.state = State::Idle;
                            self.fingers.clear();
                        }
                    }
                    State::Idle => {}
                }
                if self.fingers.is_empty() {
                    self.state = State::Idle;
                }
            }
        }
        out
    }

    /// Time passes: a finger that has rested a beat on the sea puts a float; on a float, holds it.
    pub fn tick(&mut self, now: f64) -> Out {
        let mut out = Out::default();
        if let State::Pressed { hit, t0, .. } = self.state {
            if now - t0 >= BEAT && self.fingers.len() == 1 {
                match hit {
                    Hit::Sea(p) => {
                        out.intents.push(Intent::Put { at: p });
                        // (the app answers with float_put; until then the state stays pressed)
                    }
                    Hit::Float(id, _) => {
                        self.state = State::Holding { float: id, moved: false };
                        out.intents.push(Intent::Hold { float: id });
                    }
                    _ => {}
                }
            }
        }
        out
    }

    pub fn is_holding(&self) -> Option<u32> {
        match self.state {
            State::Holding { float, .. } => Some(float),
            _ => None,
        }
    }

    fn distance(&self) -> f64 {
        if self.fingers.len() < 2 {
            return 0.0;
        }
        let (a, b) = (&self.fingers[0], &self.fingers[1]);
        ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
    }

    fn centre(&self) -> (f64, f64) {
        let (a, b) = (&self.fingers[0], &self.fingers[1]);
        ((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
    }

    /// Points a second over the finger's last moves, if it was moving.
    fn speed(f: &Finger) -> Option<(f64, f64)> {
        let r = &f.recent;
        if r.len() < 2 {
            return None;
        }
        let (x0, y0, t0) = r[0];
        let (x1, y1, t1) = r[r.len() - 1];
        let dt = t1 - t0;
        if dt < 1e-3 {
            return None;
        }
        let (vx, vy) = ((x1 - x0) / dt, (y1 - y0) / dt);
        if vx.abs() + vy.abs() < 30.0 {
            return None;
        }
        Some((vx, vy))
    }
}
