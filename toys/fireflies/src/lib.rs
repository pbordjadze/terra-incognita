//! Fireflies (a working title; painting-company row 204): the Earth at night, every patch of its
//! lights a firefly blinking its own rhythm. Each sees its neighbours' flashes a moment late and is
//! hurried on by them (sim.rs: Mirollo–Strogatz pulse coupling), so the lights fall into step, a
//! city with its neighbours first, then whole regions, then a continent; the seas keep continents
//! each to their own beat. A finger turns the globe; a tap nudges the lights under it out of step.
//!
//! The fireflies are the measured lights themselves (NASA's Black Marble, Terra's night sheet), never
//! marks over them: each lit pixel follows its nearest firefly (look.rs, night.wgsl).

pub mod look;
pub mod sim;

use serde::{Deserialize, Serialize};
use sim::{Swarm, Tune};
use toy_kit::{Camera, Ctx, Globe, Phase, PointerEvent, Toy};

/// Where the globe first faces: the North Atlantic's two lit shores, the Americas and Europe.
const START: Camera = Camera::new(-38.0, 30.0, 1.0);
/// A finger's reach for a nudge, in points.
const NUDGE_PT: f64 = 26.0;

pub struct Fireflies {
    globe: Globe,
    swarm: Swarm,
    look: look::Look,
    soft: bool,
    downs: std::collections::HashMap<u64, ([f64; 2], f64, f64)>,
    fingers_seen: usize,
    /// Where each finger is now.
    at: std::collections::HashMap<u64, [f64; 2]>,
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
        let r_km = NUDGE_PT * self.globe.reading() / self.globe.disc().r * sim::EARTH_KM;
        if self.swarm.nudge(at.to_xyz(), r_km.max(40.0)) > 0 {
            ctx.haptic();
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
        // never smaller than the room: a pinch out stops at the whole globe filling the screen's width
        globe.hand.min_zoom = 1.0;
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
        self.look.upload(&ctx.gpu, &self.swarm, self.soft);
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
