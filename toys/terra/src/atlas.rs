//! On the Earth: its touches (a light raises its toy's card; a gathering of lights is come down toward; a
//! bare touch rings), the card's own touches (a tap opens it, a swipe down puts it away), and the Earth's
//! slow turn while no one touches it.

use std::sync::atomic::Ordering;
use toy_kit::{Camera, Ctx, LonLat, Phase, PointerEvent};

use crate::doors::Hit;
use crate::{card, ease, fly, Flight, State, Terra, Up};

/// A swipe down this far (points) on the card puts it away.
const SWIPE_AWAY: f64 = 60.0;

impl Terra {
    pub(crate) fn atlas_pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        self.last_touch = e.t;
        self.life.beckon = None;
        let pts = ctx.points();
        // the card's own fingers: a tap opens it, a swipe down puts it away; the Earth never feels them
        let up = self.card.is_some_and(|c| c.gone.is_none());
        if e.phase == Phase::Down && up && self.downs.is_empty() && card::on_card(pts, [e.x, e.y]) {
            self.card_finger = Some((e.id, [e.x, e.y], e.t));
            return;
        }
        if self.card_finger_pointer(e) {
            return;
        }
        match e.phase {
            Phase::Down => {
                self.downs.insert(e.id, ([e.x, e.y], e.t, false));
                self.flight = None;
            }
            Phase::Move => {
                if let Some(d) = self.downs.get_mut(&e.id) {
                    if (e.x - d.0[0]).hypot(e.y - d.0[1]) > 10.0 {
                        d.2 = true;
                    }
                }
            }
            Phase::Up | Phase::Cancel => {}
        }
        let many = self.downs.len() > 1;
        self.globe.pointer(e);
        if matches!(e.phase, Phase::Up | Phase::Cancel) {
            if let Some((at, t0, moved)) = self.downs.remove(&e.id) {
                if e.phase == Phase::Up && !moved && !many && e.t - t0 < 0.5 {
                    self.tap(ctx, at, e.t);
                }
            }
        }
    }

    /// The card's own finger: a swipe down puts the card away, a still tap opens it. Whether it was that finger.
    fn card_finger_pointer(&mut self, e: PointerEvent) -> bool {
        let Some((id, at, t0)) = self.card_finger else {
            return false;
        };
        if id != e.id {
            return false;
        }
        if e.phase == Phase::Move && e.y - at[1] > SWIPE_AWAY {
            self.card_finger = None;
            self.put_card_away(e.t);
        } else if e.phase == Phase::Up {
            self.card_finger = None;
            let still = (e.x - at[0]).hypot(e.y - at[1]) < 12.0;
            if still && e.t - t0 < 0.6 && self.card.is_some_and(|c| c.gone.is_none()) {
                self.open_card(e.t);
            }
        } else if e.phase == Phase::Cancel {
            self.card_finger = None;
        }
        true
    }

    fn tap(&mut self, ctx: &mut Ctx, at: [f64; 2], t: f64) {
        match self.life.hit(&self.globe, at) {
            Some(Hit::Door(i)) => self.raise_card(ctx, i, t),
            Some(Hit::Many(ll)) => {
                // a gathering: the Earth comes down toward it until its lights part
                self.put_card_away(t);
                let from = self.globe.camera;
                let to = Camera {
                    lon: ll.lon,
                    lat: ll.lat,
                    zoom: (from.zoom * 2.6).min(self.globe.hand.max_zoom),
                    roll: from.roll,
                };
                self.fly_to(to, t);
                self.strike(2, 0.18, 0.0);
            }
            None => {
                // something happens anywhere: a soft ring where the finger was, and the nearest light answers;
                // a card up goes away
                self.put_card_away(t);
                if let Some(ll) = self.globe.unproject(at[0], at[1]) {
                    self.life.ring(ll, t);
                    let pan = ((at[0] / ctx.points()[0]) * 2.0 - 1.0) as f32;
                    self.strike(-3, 0.16, pan * 0.5);
                    if let Some(n) = self.life.nearest(&self.globe, at) {
                        self.life.glint(n, t + 0.25);
                    }
                }
            }
        }
    }

    /// Light `i` was touched: it stirs and rings, its toy is made, its card rises with the toy going in it,
    /// and the Earth turns so the light sits above the card.
    pub(crate) fn raise_card(&mut self, ctx: &mut Ctx, i: usize, t: f64) {
        self.life.stir(i, t);
        self.life.tried(i);
        let pan = ((self.life.spot(&self.globe, i).0[0] / ctx.points()[0]) * 2.0 - 1.0) as f32;
        self.strike(self.life.doors[i].degree, 0.32, pan * 0.6);
        self.strike(self.life.doors[i].degree + 5, 0.12, pan * 0.6);
        ctx.haptic();
        if self.card.is_some_and(|c| c.gone.is_none()) && self.door == i {
            return;
        }
        let mut room = match (self.life.doors[i].open)(ctx, &self.assets) {
            Ok(r) => r,
            Err(e) => {
                toy_kit::log(&format!(
                    "terra: {} did not open: {e:#}",
                    self.life.doors[i].id
                ));
                return;
            }
        };
        room.settings(ctx.night, ctx.reduce_motion);
        room.update(t);
        *self.room_sound.lock().unwrap() = room.synth();
        // (another card up gives way at once: one toy is held at a time)
        let risen = self.card.is_some_and(|c| c.gone.is_none());
        self.room = Some(room);
        self.door = i;
        self.card = Some(Up {
            t0: if risen { t - card::RISE * 0.5 } else { t },
            gone: None,
        });
        self.bring_above_card(ctx, i, t);
    }

    /// Turns the Earth so light `i` sits in the open part of the screen above the card, at the zoom it is at.
    fn bring_above_card(&mut self, ctx: &Ctx, i: usize, t: f64) {
        let pts = ctx.points();
        let top = card::shape(pts).card[1];
        let (p, _) = self.life.spot(&self.globe, i);
        let want_y = (top * 0.55).max(80.0);
        // already well placed (in the open, and facing the reader rather than at the Earth's rim): leave it be
        let d = self.globe.disc();
        let rim = (p[0] - d.cx).hypot(p[1] - d.cy) / d.r.max(1.0);
        if p[1] > 60.0 && p[1] < top - 40.0 && p[0] > 40.0 && p[0] < pts[0] - 40.0 && rim < 0.7 {
            return;
        }
        let at = self.life.doors[i].at;
        let from = self.globe.camera;
        let r = self.globe.disc().r.max(1.0);
        // on the light's meridian, the camera's centre that many points below it
        let dy = (self.globe.disc().cy - want_y) / r;
        let lat = (at[1] - dy.clamp(-0.9, 0.9).asin().to_degrees()).clamp(-80.0, 80.0);
        let to = Camera {
            lon: at[0],
            lat,
            zoom: from.zoom,
            roll: 0.0,
        };
        self.fly_to(to, t);
    }

    pub(crate) fn fly_to(&mut self, to: Camera, t: f64) {
        let from = self.globe.camera;
        let (a, b) = (
            LonLat::new(from.lon, from.lat).to_xyz(),
            LonLat::new(to.lon, to.lat).to_xyz(),
        );
        let w = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
            .clamp(-1.0, 1.0)
            .acos();
        let dur = if self.reduce_motion {
            0.01
        } else {
            0.6 + 0.4 * w + 0.2 * (to.zoom / from.zoom).ln().abs()
        };
        self.flight = Some(Flight {
            from,
            to,
            t0: t,
            dur,
        });
    }

    /// The card's picture grows to the glass: in.
    pub(crate) fn open_card(&mut self, t: f64) {
        if self.room.is_none() || !matches!(self.state, State::Atlas) {
            return;
        }
        self.flight = None;
        // the Earth's other fingers are let go, so its hand is not left holding one
        for (id, (at, _, _)) in std::mem::take(&mut self.downs) {
            self.globe.pointer(PointerEvent {
                id,
                phase: Phase::Cancel,
                x: at[0],
                y: at[1],
                t,
            });
        }
        self.card_finger = None;
        self.strike(self.life.doors[self.door].degree + 2, 0.14, 0.0);
        self.state = State::Opening { t0: t };
    }

    /// The card falls away and its toy is let go.
    pub(crate) fn put_card_away(&mut self, t: f64) {
        if let Some(c) = self.card.as_mut() {
            if c.gone.is_none() {
                c.gone = Some(t);
            }
        }
    }

    /// Back toward the Earth: the toy shrinks into its card, which stays up over the Earth where the reader was.
    pub(crate) fn leave(&mut self, ctx: &mut Ctx, t: f64) {
        self.strike(self.life.doors[self.door].degree - 5, 0.18, 0.0);
        ctx.haptic();
        // the toy lets go of every finger still on it, and its demo stops (it would play on into the Earth)
        if let Some(r) = self.room.as_mut() {
            for (id, at) in self.inside_fingers.drain() {
                r.pointer(PointerEvent {
                    id,
                    phase: Phase::Cancel,
                    x: at[0],
                    y: at[1],
                    t,
                });
            }
        }
        self.inside_fingers.clear();
        ctx.hide_info();
        self.pinch = None;
        self.earth_finger = None;
        self.card = Some(Up {
            t0: t - card::RISE,
            gone: None,
        });
        self.state = State::Closing { t0: t };
    }

    /// Each frame on the Earth: a flight under way, the slow turn, and a card going away.
    pub(crate) fn atlas_step(&mut self, ctx: &Ctx, t: f64, dt: f64) {
        if let Some(f) = self.flight {
            let u = (t - f.t0) / f.dur;
            self.globe.camera = fly(f.from, f.to, u.min(1.0));
            if u >= 1.0 {
                self.flight = None;
            }
        } else if self.card.is_none() {
            self.drift_step(ctx, t, dt);
        }
        if let Some(Up { gone: Some(t1), .. }) = self.card {
            if t - t1 >= card::RISE {
                self.card = None;
                self.room = None;
                *self.room_sound.lock().unwrap() = None;
                self.room_level.store(0f32.to_bits(), Ordering::Relaxed);
                self.veil.target = None;
            }
        }
    }

    /// Left alone, the Earth turns slowly under the reader (west to east, as it does), lingering where its
    /// lights face the reader and crossing the empty sea between them more briskly, so a minute's look never
    /// turns them all away (painting-company row 246: untouched, the opening had turned to bare Pacific by 40 s).
    fn drift_step(&mut self, ctx: &Ctx, t: f64, dt: f64) {
        if self.reduce_motion || self.globe.hand.busy() || !self.downs.is_empty() {
            return;
        }
        let k = ease((t - self.last_touch - 4.0) / 3.0);
        let [w, h] = ctx.points();
        let seen: f64 = self
            .life
            .shown(&self.globe)
            .iter()
            .filter(|s| s.facing > 0.35)
            .map(|s| {
                // a light counts the more the nearer the middle of the screen it is
                let dx = (s.at[0] / w - 0.5).abs() * 2.0;
                let dy = (s.at[1] / h - 0.5).abs() * 2.0;
                (1.0 - dx.max(dy)).clamp(0.0, 1.0).sqrt() * s.doors.len() as f64
            })
            .sum();
        let want = 1.6 * (2.6 / (1.0 + 1.3 * seen)).clamp(0.3, 2.6);
        self.drift += (want - self.drift) * (dt / 3.0).min(1.0);
        self.globe.camera.lon -= self.drift * k * dt;
    }
}
