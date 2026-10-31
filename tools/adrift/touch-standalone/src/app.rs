//! A stand-in for `adrift-core`'s `App`: the state the gestures probe and the intents applied as
//! `app.rs` applies them (kept in step with it by hand), over a sea of circles for land and floats
//! that drift only as a plan says. No GPU, so a plan plays in milliseconds.
use crate::gpu::View;
use crate::model::*;
use crate::touch::{CameraControl, Gestures, Out};

#[derive(Clone, Debug)]
pub enum Undo {
    Put(u32),
    Moved { id: u32, from: Float },
    Removed(Float),
}

/// Stand-ins for `sea::Rules` and `sea::Coast`, as far as the probe reads them.
pub struct Coast {
    pub land: Vec<[f64; 3]>,
}
impl Coast {
    pub fn is_land(&self, p: LonLat) -> bool {
        self.land.iter().any(|c| {
            let mut d = p.lon - c[0];
            if d > 180.0 {
                d -= 360.0
            } else if d < -180.0 {
                d += 360.0
            }
            d.hypot(p.lat - c[1]) <= c[2]
        })
    }
}
pub struct Rules {
    pub coast: Coast,
}

pub struct App {
    pub rules: Rules,
    pub floats: Vec<Float>,
    pub next_id: u32,
    pub clock: Clock,
    pub undo: Vec<Undo>,
    pub camera: CameraControl,
    pub gestures: Gestures,
    pub width: u32,
    pub height: u32,
    pub ppp: f64,
    pub reading: f64,
    pub reduce_motion: bool,
    /// Every intent applied, in order.
    pub applied: Vec<Intent>,
}

impl App {
    pub fn new(width: u32, height: u32, ppp: f64, camera: Camera) -> App {
        let short = (width.min(height) as f64) / ppp;
        let reading = (1.0 + 0.2 * ((short - 430.0) / (1032.0 - 430.0)).clamp(0.0, 1.0)).clamp(1.0, 1.2);
        App {
            rules: Rules { coast: Coast { land: vec![] } },
            floats: vec![],
            next_id: 1,
            clock: Clock { day: 0.0, running: true, rate: 2.0 },
            undo: vec![],
            camera: CameraControl::new(camera),
            gestures: Gestures::new(),
            width,
            height,
            ppp,
            reading,
            reduce_motion: false,
            applied: vec![],
        }
    }

    pub fn disc(&self) -> Disc {
        View::layout(self.width, self.height, self.ppp, self.camera.camera)
    }

    pub fn view(&self) -> View {
        View { width: self.width, height: self.height, ppp: self.ppp, reading: self.reading, disc: self.disc(), camera: self.camera.camera, seconds: 0.0, clock: self.clock }
    }

    pub fn is_land(&self, p: LonLat) -> bool {
        self.rules.coast.is_land(p)
    }

    /// As `App::hit`: the nearest float within reach, else the place.
    pub fn hit(&self, x: f64, y: f64) -> Hit {
        let disc = self.disc();
        let reach = 14.0 * self.reading;
        let mut best: Option<(f64, u32)> = None;
        for f in &self.floats {
            let (p, facing) = disc.project(&self.camera.camera, f.at);
            if facing < 0.05 {
                continue;
            }
            let d = ((p[0] - x).powi(2) + (p[1] - y).powi(2)).sqrt();
            if d < reach && best.map_or(true, |b| d < b.0) {
                best = Some((d, f.id));
            }
        }
        let place = disc.unproject(&self.camera.camera, x, y);
        if let Some((_, id)) = best {
            let f = self.floats.iter().find(|f| f.id == id).unwrap();
            return Hit::Float(id, place.unwrap_or(f.at));
        }
        match place {
            Some(p) if self.is_land(p) => Hit::Land(p),
            Some(p) => Hit::Sea(p),
            None => Hit::Off,
        }
    }

    pub fn put(&mut self, at: LonLat) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.floats.push(Float { id, at, put_day: self.clock.day, seed: id as u64, held: Held::No, hand: Hand::Held, wake: vec![], lit_for: 0.0 });
        id
    }

    pub fn pointer(&mut self, ev: PointerEvent) {
        let mut g = std::mem::take(&mut self.gestures);
        let out = g.feed(ev, &*self);
        self.gestures = g;
        self.apply(out, ev.t);
    }

    pub fn tick(&mut self, now: f64, dt: f64) {
        let mut g = std::mem::take(&mut self.gestures);
        let out = g.tick(now, &*self);
        self.gestures = g;
        self.apply(out, now);
        let ride_at = self.camera.riding.and_then(|id| self.floats.iter().find(|f| f.id == id).map(|f| f.at));
        if self.camera.riding.is_some() && ride_at.is_none() {
            self.camera.riding = None;
        }
        let motion = crate::touch::Motion { reduce: self.reduce_motion };
        self.camera.tick(now, dt, ride_at, self.floats.is_empty(), self.gestures.touching(), motion);
    }

    pub fn apply(&mut self, out: Out, now: f64) {
        let disc = self.disc();
        let motion = crate::touch::Motion { reduce: self.reduce_motion };
        for m in out.camera {
            self.camera.apply(&disc, m, now, motion);
        }
        for i in out.intents {
            self.intend(i);
        }
    }

    /// `App::intend`, line for line where it matters to the gestures.
    pub fn intend(&mut self, intent: Intent) {
        self.applied.push(intent);
        match intent {
            Intent::Ring { .. } => {}
            Intent::Put { at } => {
                let id = self.put(at);
                self.gestures.float_put(id);
                self.undo.push(Undo::Put(id));
            }
            Intent::Hold { float } => {
                if let Some(f) = self.floats.iter_mut().find(|f| f.id == float) {
                    f.hand = Hand::Held;
                    self.undo.push(Undo::Moved { id: float, from: f.clone() });
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
                match at {
                    Some(p) => {
                        let moved = self.floats[i].hand == Hand::Carried;
                        let f = &mut self.floats[i];
                        f.hand = Hand::Free;
                        if moved {
                            f.at = p;
                            f.put_day = self.clock.day;
                            f.wake.clear();
                        } else if matches!(self.undo.last(), Some(Undo::Moved { id, .. }) if *id == float) {
                            self.undo.pop();
                        }
                    }
                    None => {
                        let f = self.floats.remove(i);
                        if matches!(self.undo.last(), Some(Undo::Moved { id, .. }) if *id == float) {
                            self.undo.pop();
                        }
                        self.undo.push(Undo::Removed(f));
                    }
                }
            }
            Intent::Undo => match self.undo.pop() {
                Some(Undo::Put(id)) => self.floats.retain(|f| f.id != id),
                Some(Undo::Moved { id, from }) => {
                    if let Some(f) = self.floats.iter_mut().find(|f| f.id == id) {
                        let mut back = from;
                        back.hand = Hand::Free;
                        *f = back;
                    }
                }
                Some(Undo::Removed(mut f)) => {
                    f.hand = Hand::Free;
                    self.floats.push(f);
                }
                None => {}
            },
            Intent::Ride { float } => {
                if self.camera.riding == Some(float) {
                    self.camera.riding = None;
                } else if self.floats.iter().any(|f| f.id == float) {
                    self.camera.riding = Some(float);
                }
            }
            Intent::Scrub { days } => self.clock.day += days,
            Intent::SetRunning { running } => self.clock.running = running,
        }
    }
}
