//! Fireflies (a working title; painting-company row 204): the Earth at night, every patch of its
//! lights a firefly blinking its own rhythm. Each sees its neighbours' flashes a moment late and is
//! hurried on by them (sim.rs: Mirollo–Strogatz pulse coupling), so the lights fall into step, a
//! city with its neighbours first, then whole regions, then a continent; the seas keep continents
//! each to their own beat. A finger turns the globe; a tap sends a ring of light out from the finger
//! that sets off the lights it reaches; the hand is never left lost in the dark (a view over empty sea
//! or a pole drifts back to the lights).
//!
//! The fireflies are the measured lights themselves (NASA's Black Marble, Terra's night sheet), never
//! marks over them: each lit pixel follows its nearest firefly (look.rs, night.wgsl).

pub mod look;
pub mod sim;

use serde::{Deserialize, Serialize};
use sim::{Swarm, Tune};
use toy_kit::{Camera, Ctx, Globe, Phase, PointerEvent, Toy};

/// Where the globe first faces: the Americas, north over south, a tall screen's shape (the zoom
/// is the screen's: `start_zoom`).
const START: Camera = Camera::new(-72.0, 20.0, 1.0);
/// A finger's reach for a nudge, in points.
const NUDGE_PT: f64 = 26.0;
/// A touch's ring of light: how fast it spreads on the screen (points a second), how long it can
/// reach (seconds), and how wide it is drawn (points).
const RING_PT_S: f64 = 280.0;
const RING_REACH_S: f64 = 2.0;
const RING_WIDTH_PT: f64 = 2.5;
/// Fewer fireflies than this in the middle of the screen and, the hand at rest, the view drifts to the nearest lights.
const LOST_BELOW: usize = 25;
/// The view's latitude the hand comes back within at rest, so a pole never fills it.
const TILT_LIMIT: f64 = 58.0;

/// The opening zoom: on a tall screen the whole globe would be a narrow band (the phone: half the
/// screen black above and below), so it starts closer there, filling about 70% of the height.
fn start_zoom(points: [f64; 2]) -> f64 {
    let short = points[0].min(points[1]);
    (0.35 * points[0].max(points[1]) / (short / 2.0 - 14.0)).clamp(1.0, 1.7)
}

pub struct Fireflies {
    globe: Globe,
    swarm: Swarm,
    look: look::Look,
    soft: bool,
    downs: std::collections::HashMap<u64, ([f64; 2], f64, f64)>,
    fingers_seen: usize,
    /// Where each finger is now.
    at: std::collections::HashMap<u64, [f64; 2]>,
    rings: Vec<look::Ring>,
    /// Seconds since the last finger lifted.
    idle: f64,
    /// Drifting to the lights (until it is over the lit place it was going to).
    drifting: bool,
}

#[derive(Serialize, Deserialize)]
struct Saved {
    camera: Camera,
    phases: Vec<f32>,
}

impl Fireflies {
    fn tap(&mut self, ctx: &mut Ctx, x: f64, y: f64) {
        let Some(at) = self.globe.unproject(x, y) else {
            return;
        };
        let r = self.globe.disc().r;
        let km_pt = sim::EARTH_KM / r;
        let r_km = (NUDGE_PT * self.globe.reading() * km_pt).max(40.0);
        let speed = RING_PT_S * km_pt;
        let (_, last) = self.swarm.touch(at.to_xyz(), r_km, RING_REACH_S * speed, speed);
        self.rings.retain(|g| self.swarm.t - g.t0 < g.life);
        if self.rings.len() >= look::MAX_RINGS {
            self.rings.remove(0);
        }
        self.rings.push(look::Ring { at: at.to_xyz(), t0: self.swarm.t, speed: RING_PT_S / r, life: (last + 0.6).clamp(1.2, 4.0) });
        ctx.haptic();
    }

    /// The fireflies in the middle of the screen (a corner's coast doesn't make a view), and the nearest to the screen's centre that is in a crowd (a lit coast
    /// or a land's field of lights, not a lone island's speck).
    fn in_view(&self) -> (usize, usize) {
        let [w, h] = [self.globe.view().width as f64 / self.globe.view().ppp, self.globe.view().height as f64 / self.globe.view().ppp];
        let c = toy_kit::LonLat::new(self.globe.camera.lon, self.globe.camera.lat).to_xyz();
        let (mut n, mut best, mut bd) = (0, 0, f64::MIN);
        for (i, f) in self.swarm.flies.iter().enumerate() {
            let d = c[0] * f.xyz[0] + c[1] * f.xyz[1] + c[2] * f.xyz[2];
            if d > bd && f.near.len() >= 10 {
                (bd, best) = (d, i);
            }
            if d > 0.1 {
                let ([x, y], facing) = self.globe.project(toy_kit::LonLat::new(f.lon, f.lat));
                if facing > 0.1 && (x - w / 2.0).abs() <= 0.35 * w && (y - h / 2.0).abs() <= 0.3 * h {
                    n += 1;
                }
            }
        }
        (n, best)
    }

    /// The hand at rest is never left lost: the tilt eases back within the limit, and a view with
    /// (almost) no lights in it drifts to the nearest.
    fn steady(&mut self, dt: f64) {
        if self.globe.hand.fingers() > 0 {
            self.idle = 0.0;
            self.drifting = false;
            return;
        }
        self.idle += dt;
        if self.idle < 0.8 {
            return;
        }
        let cam = &mut self.globe.camera;
        if cam.lat.abs() > TILT_LIMIT {
            let lim = TILT_LIMIT * cam.lat.signum();
            cam.lat -= (cam.lat - lim) * (1.0 - (-dt * 2.5).exp());
        }
        let (n, near) = self.in_view();
        if std::env::var_os("FIREFLIES_DRIFT").is_some() {
            let (c, f) = (self.globe.camera, &self.swarm.flies[near]);
            eprintln!("t {:.2} in view {n} cam {:.1},{:.1} z{:.1} target {:.1},{:.1}", self.swarm.t, c.lon, c.lat, c.zoom, f.lon, f.lat);
        }
        if n < LOST_BELOW {
            self.drifting = true;
        }
        let f = &self.swarm.flies[near];
        let there = (f.lon - self.globe.camera.lon).abs() + (f.lat - self.globe.camera.lat).abs() < 1.0 / self.globe.camera.zoom;
        if there {
            self.drifting = false;
        }
        if self.drifting {
            let k = 1.0 - (-dt * 1.4).exp();
            let cam = &mut self.globe.camera;
            let dlon = (f.lon - cam.lon + 540.0).rem_euclid(360.0) - 180.0;
            cam.lon += dlon * k;
            cam.lat += (f.lat.clamp(-TILT_LIMIT, TILT_LIMIT) - cam.lat) * k;
        }
    }
}

impl Toy for Fireflies {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let mut globe = Globe::new(
            &ctx.gpu,
            ctx.assets.as_ref(),
            ctx.width,
            ctx.height,
            ctx.ppp,
            10,
        )?;
        globe.night = 1.0;
        globe.camera = START;
        globe.camera.zoom = start_zoom(ctx.points());
        // a pinch out goes a little past the whole globe and springs back to it (the kit's rest_zoom)
        globe.hand.min_zoom = 0.8;
        globe.hand.rest_zoom = 1.0;
        let seed = std::env::var("FIREFLIES_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0x5eed_f1e5);
        let swarm = Swarm::from_bytes(&ctx.assets.read("fireflies.bin")?, Tune::default(), seed)?;
        let look = look::Look::new(&ctx.gpu, &globe, swarm.flies.len());
        Ok(Fireflies {
            globe,
            swarm,
            look,
            soft: ctx.reduce_motion,
            downs: Default::default(),
            fingers_seen: 0,
            at: Default::default(),
            rings: Vec::new(),
            idle: 0.0,
            drifting: false,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        // two fingers: the kit's hand zooms about the screen's centre; here the place between the
        // fingers stays between them, so a pinch goes toward what the fingers are on (and two fingers
        // moving together carry the globe)
        let mid = |at: &std::collections::HashMap<u64, [f64; 2]>| {
            let v: Vec<[f64; 2]> = at.values().copied().collect();
            (v.len() == 2).then(|| [(v[0][0] + v[1][0]) / 2.0, (v[0][1] + v[1][1]) / 2.0])
        };
        let before = if e.phase == Phase::Move && self.at.contains_key(&e.id) { mid(&self.at) } else { None };
        let held = before.and_then(|m| self.globe.unproject(m[0], m[1]));
        self.globe.pointer(e);
        match e.phase {
            Phase::Down | Phase::Move => {
                if e.phase == Phase::Down || self.at.contains_key(&e.id) {
                    self.at.insert(e.id, [e.x, e.y]);
                }
            }
            Phase::Up | Phase::Cancel => {
                self.at.remove(&e.id);
            }
        }
        if let (Some(place), Some(m)) = (held, mid(&self.at)) {
            let (q, facing) = self.globe.project(place);
            if facing > 0.0 {
                let (dx, dy) = (m[0] - q[0], m[1] - q[1]);
                let r = self.globe.disc().r;
                if dx.hypot(dy) > 1e-6 {
                    self.globe.camera = self.globe.camera.turned([dy, dx, 0.0], dx.hypot(dy) / r);
                }
            }
        }
        match e.phase {
            Phase::Down => {
                if self.downs.is_empty() {
                    self.fingers_seen = 0;
                }
                self.fingers_seen += 1;
                self.downs.insert(e.id, ([e.x, e.y], e.t, 0.0));
            }
            Phase::Move => {
                if let Some(d) = self.downs.get_mut(&e.id) {
                    d.2 = d.2.max((e.x - d.0[0]).hypot(e.y - d.0[1]));
                }
            }
            Phase::Up => {
                if let Some((p, t0, moved)) = self.downs.remove(&e.id) {
                    if self.fingers_seen == 1 && moved < 10.0 && e.t - t0 < 0.4 {
                        self.tap(ctx, p[0], p[1]);
                    }
                }
            }
            Phase::Cancel => {
                self.downs.remove(&e.id);
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.globe.update(&ctx.gpu, ctx.assets.as_ref(), t, dt);
        self.swarm.step(dt);
        self.steady(dt);
        if std::env::var_os("FIREFLIES_ORDER").is_some() && (t * 2.0).fract() < dt * 2.0 {
            let o = |lo0: f64, lo1: f64, la0: f64, la1: f64| {
                move |f: &sim::Firefly| f.lon >= lo0 && f.lon < lo1 && f.lat >= la0 && f.lat < la1
            };
            eprintln!(
                "t {t:6.1}  all {:.2}  US-east {:.2}  N.America {:.2}  Europe {:.2}  India {:.2}  Brazil {:.2}",
                self.swarm.order(|_| true),
                self.swarm.order(o(-90.0, -70.0, 30.0, 45.0)),
                self.swarm.order(o(-130.0, -60.0, 20.0, 60.0)),
                self.swarm.order(o(-10.0, 30.0, 36.0, 60.0)),
                self.swarm.order(o(68.0, 90.0, 8.0, 32.0)),
                self.swarm.order(o(-55.0, -35.0, -30.0, -5.0)),
            );
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        self.globe.upload(&ctx.gpu);
        let width = RING_WIDTH_PT / self.globe.disc().r;
        self.look.upload(&ctx.gpu, &self.swarm, self.soft, &self.rings, width);
        self.look.draw(&ctx.gpu, &self.globe, target);
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.soft = ctx.reduce_motion;
    }

    fn save(&self) -> Option<String> {
        serde_json::to_string(&Saved {
            camera: self.globe.camera,
            phases: self.swarm.phases(),
        })
        .ok()
    }

    fn restore(&mut self, _ctx: &mut Ctx, saved: &str) {
        if let Ok(s) = serde_json::from_str::<Saved>(saved) {
            self.globe.camera = s.camera;
            self.swarm.set_phases(&s.phases);
        }
    }
}

toy_kit::ios_shell!(Fireflies);
