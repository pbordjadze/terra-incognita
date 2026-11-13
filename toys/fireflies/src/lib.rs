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
pub mod sound;

use serde::{Deserialize, Serialize};
use sim::{Swarm, Tune};
use toy_kit::{Camera, Ctx, Globe, Phase, PointerEvent, Toy};

/// Where the globe first faces: the Americas, north over south, a tall screen's shape (the zoom
/// is the screen's: `start_zoom`).
const START: Camera = Camera::new(-80.0, 24.0, 1.0);
/// A finger's reach for a nudge, in points.
const NUDGE_PT: f64 = 26.0;
/// A touch's ring of light: how fast it spreads on the screen (points a second), how long it can
/// reach (seconds), and how wide it is drawn (points).
const RING_PT_S: f64 = 280.0;
const RING_REACH_S: f64 = 2.0;
const RING_WIDTH_PT: f64 = 12.0;
/// The latitudes the view's centre settles within at rest (where the lights are: a pole never fills
/// the view), and how long after the fingers lift it starts to settle (seconds).
const SETTLE_LAT: (f64, f64) = (-38.0, 60.0);
const SETTLE_AFTER: f64 = 1.0;
/// When one side of the view shows more of the Earth than this (degrees), the view is the whole globe,
/// and it settles on a region's centre of light.
const WHOLE_HALF_ANGLE: f64 = 35.0;
/// How far the whole globe looks for a brighter region: its light counts less by e for each this
/// many degrees away.
const LIGHT_REACH: f64 = 70.0;
/// A region's reach (degrees): the light within this of a place is that place's light.
const REGION: f64 = 20.0;

/// The lit regions' centres of light: every place on a 4-degree grid within the settling latitudes,
/// the light of the fireflies within REGION of it, and their light-weighted centre (lon, lat, light),
/// keeping the places with any light. Computed once (about 11 million products).
fn light_centres(swarm: &Swarm) -> Vec<(f64, f64, f64)> {
    let cosr = REGION.to_radians().cos();
    let mut out = Vec::new();
    let mut lat = SETTLE_LAT.0;
    while lat <= SETTLE_LAT.1 {
        let mut lon = -180.0;
        while lon < 180.0 {
            let p = toy_kit::LonLat::new(lon, lat).to_xyz();
            let (mut sum, mut m) = (0.0, [0.0; 3]);
            for f in &swarm.flies {
                if p[0] * f.xyz[0] + p[1] * f.xyz[1] + p[2] * f.xyz[2] > cosr {
                    sum += f.weight;
                    for k in 0..3 {
                        m[k] += f.weight * f.xyz[k];
                    }
                }
            }
            if sum > 0.0 {
                let c = toy_kit::LonLat::from_xyz(m);
                out.push((c.lon, c.lat.clamp(SETTLE_LAT.0, SETTLE_LAT.1), sum));
            }
            lon += 4.0;
        }
        lat += 4.0;
    }
    out
}

/// A lit crowd: a firefly whose light with its neighbours' is at least this (a city's region, about
/// the brightest quarter of the fireflies: Ohio 4.0, Lisbon 3.1; not New Brunswick's 1.2).
const CROWD_LIGHT: f64 = 2.8;

/// Closer than this the lights' sheet shows its pixels (about 10 km each): the zoom stops here.
const MAX_ZOOM: f64 = 5.0;

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
    ear: sound::Ear,
    /// Each firefly's light with its neighbours' (a crowd's light).
    crowd: Vec<f64>,
    /// The lit regions' centres of light (`light_centres`).
    lights: Vec<(f64, f64, f64)>,
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
        let (_, last) = self
            .swarm
            .touch(at.to_xyz(), r_km, RING_REACH_S * speed, speed);
        self.rings.retain(|g| self.swarm.t - g.t0 < g.life);
        if self.rings.len() >= look::MAX_RINGS {
            self.rings.remove(0);
        }
        self.rings.push(look::Ring {
            at: at.to_xyz(),
            t0: self.swarm.t,
            speed: RING_PT_S / r,
            life: (last + 0.6).clamp(1.2, 4.0),
        });
        ctx.haptic();
        self.ear
            .touch((x / (self.globe.view().width as f64 / self.globe.view().ppp)) as f32);
    }

    /// Listens to the screen: each patch's glow, for the chimes (sound.rs).
    fn listen(&mut self) {
        let v = self.globe.view();
        let (w, h) = (v.width as f64 / v.ppp, v.height as f64 / v.ppp);
        let c = toy_kit::LonLat::new(self.globe.camera.lon, self.globe.camera.lat).to_xyz();
        let mut cells = [(0f32, 0f32, 0u32); sound::COLS * sound::ROWS];
        for (i, f) in self.swarm.flies.iter().enumerate() {
            if c[0] * f.xyz[0] + c[1] * f.xyz[1] + c[2] * f.xyz[2] < 0.05 {
                continue;
            }
            let ([x, y], facing) = self.globe.project(toy_kit::LonLat::new(f.lon, f.lat));
            if facing <= 0.05 || x < 0.0 || y < 0.0 || x >= w || y >= h {
                continue;
            }
            let k = ((y / h * sound::ROWS as f64) as usize).min(sound::ROWS - 1) * sound::COLS
                + ((x / w * sound::COLS as f64) as usize).min(sound::COLS - 1);
            let wt = f.weight as f32;
            let g = self.swarm.glow(i, false);
            cells[k].0 += wt * g;
            cells[k].1 += wt;
            cells[k].2 += (g > 0.5) as u32;
        }
        self.ear.hear(&cells, self.soft, self.swarm.t);
    }

    /// The view's half-width as an angle on the globe (radians): how much of the Earth one side of
    /// the screen's shorter half shows.
    fn half_angle(&self) -> f64 {
        let v = self.globe.view();
        let short = (v.width.min(v.height) as f64 / v.ppp) / 2.0;
        (short / self.globe.disc().r).min(1.0).asin()
    }

    /// The firefly in a lit crowd (CROWD_LIGHT: a city's region, not a lone island's speck) nearest the screen's centre,
    /// and how far it is (radians).
    fn nearest_crowd(&self) -> (usize, f64) {
        let c = toy_kit::LonLat::new(self.globe.camera.lon, self.globe.camera.lat).to_xyz();
        let (mut best, mut bd) = (0, f64::MIN);
        for (i, f) in self.swarm.flies.iter().enumerate() {
            let d = c[0] * f.xyz[0] + c[1] * f.xyz[1] + c[2] * f.xyz[2];
            if d > bd
                && self.crowd[i] >= CROWD_LIGHT
                && f.lat > SETTLE_LAT.0
                && f.lat < SETTLE_LAT.1
            {
                (bd, best) = (d, i);
            }
        }
        (best, bd.clamp(-1.0, 1.0).acos())
    }

    /// The lit region whose centre of light the whole globe settles on: the brightest near the view's
    /// centre (its light discounted by how far it is), and how far it is (radians).
    fn nearest_light_centre(&self) -> ((f64, f64), f64) {
        let c = toy_kit::LonLat::new(self.globe.camera.lon, self.globe.camera.lat).to_xyz();
        let mut best = (f64::MIN, (0.0, 0.0), 0.0);
        for l in &self.lights {
            let x = toy_kit::LonLat::new(l.0, l.1).to_xyz();
            let d = (c[0] * x[0] + c[1] * x[1] + c[2] * x[2])
                .clamp(-1.0, 1.0)
                .acos();
            let score = l.2 * (-d / LIGHT_REACH.to_radians()).exp();
            if score > best.0 {
                best = (score, (l.0, l.1), d);
            }
        }
        (best.1, best.2)
    }

    /// Whatever the hand did, the globe at rest settles (a twist's roll kept: Home sets it upright): its
    /// centre within the lit latitudes, and a lit crowd in the middle of the view (at whole-globe
    /// zoom too). It waits a second after the fingers lift, so a fling plays out first.
    fn steady(&mut self, dt: f64) {
        if self.globe.hand.busy() {
            self.idle = 0.0;
            self.drifting = false;
            return;
        }
        self.idle += dt;
        if self.idle < SETTLE_AFTER {
            return;
        }
        let k = 1.0 - (-dt * 1.6).exp();
        let cam = &mut self.globe.camera;
        cam.lat += (cam.lat.clamp(SETTLE_LAT.0, SETTLE_LAT.1) - cam.lat) * k;
        let half = self.half_angle();
        // far out (a whole hemisphere in view) the view settles on a lit region's centre of light;
        // closer in, on the nearest lit crowd
        let whole = half > WHOLE_HALF_ANGLE.to_radians();
        let ((lon, lat), far) = if whole {
            self.nearest_light_centre()
        } else {
            let (i, far) = self.nearest_crowd();
            ((self.swarm.flies[i].lon, self.swarm.flies[i].lat), far)
        };
        let (start, stop) = if whole { (0.08, 0.02) } else { (0.15, 0.05) };
        if far > start * half {
            self.drifting = true;
        }
        if far < stop * half {
            self.drifting = false;
        }
        if std::env::var_os("FIREFLIES_DRIFT").is_some() {
            let c = self.globe.camera;
            eprintln!(
                "t {:.2} cam {:.1},{:.1} roll {:.1} z{:.1} target {lon:.1},{lat:.1} ({}) at {:.1} deg drifting {}",
                self.swarm.t, c.lon, c.lat, c.roll, c.zoom, if whole { "light centre" } else { "crowd" }, far.to_degrees(), self.drifting
            );
        }
        if self.drifting {
            let cam = &mut self.globe.camera;
            let dlon = (lon - cam.lon + 540.0).rem_euclid(360.0) - 180.0;
            cam.lon += dlon * k;
            cam.lat += (lat - cam.lat) * k;
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
        // a pinch out goes past the whole globe and springs back (the kit's rest_zoom)
        globe.hand.min_zoom = 0.8;
        // at rest it comes back to the opening's size: on a tall screen (the phone) the whole globe is
        // a narrow band in black, so it rests closer there
        globe.hand.rest_zoom = globe.camera.zoom;
        globe.hand.max_zoom = MAX_ZOOM;
        globe.set_home();
        let seed = std::env::var("FIREFLIES_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0x5eed_f1e5);
        let swarm = Swarm::from_bytes(&ctx.assets.read("fireflies.bin")?, Tune::default(), seed)?;
        let look = look::Look::new(&ctx.gpu, &globe, swarm.flies.len());
        let lights = light_centres(&swarm);
        let crowd = swarm
            .flies
            .iter()
            .map(|f| {
                f.weight
                    + f.near
                        .iter()
                        .map(|&(j, _)| swarm.flies[j as usize].weight)
                        .sum::<f64>()
            })
            .collect();
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
            ear: sound::Ear::new(),
            lights,
            crowd,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.globe.resize(ctx.width, ctx.height, ctx.ppp);
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        // (two fingers: the kit's hand pinches about what they hold and twists, as in every globe app)
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
        self.listen();
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
        self.look
            .upload(&ctx.gpu, &self.swarm, self.soft, &self.rings, width);
        self.look.draw(&ctx.gpu, &self.globe, target);
    }

    fn synth(&mut self) -> Option<Box<dyn toy_kit::Synth>> {
        Some(Box::new(sound::Chimes::new(self.ear.queue.clone())))
    }

    fn demo(&self, _ctx: &Ctx) -> Option<toy_kit::info::Demo> {
        // a tap on a lit crowd: its lights sparkle out of step and the field pulls them back
        let at = self.globe.spot_for(&[
            [2.35, 48.86],
            [9.19, 45.46],
            [-0.13, 51.51],
            [31.24, 30.04],
            [3.38, 6.52],
            [77.21, 28.61],
            [139.69, 35.69],
            [-74.0, 40.71],
            [-99.13, 19.43],
            [-46.63, -23.55],
        ]);
        Some(toy_kit::info::Demo::new(
            "Each city light is a firefly. Tap to knock them out of step.",
            vec![toy_kit::info::Stroke::tap(0.5, at)],
        ))
    }

    fn rules(&self) -> Option<toy_kit::info::Rules> {
        Some(toy_kit::info::Rules::lines(&[
                "Some fireflies flash in unison, like those in the Smoky Mountains.",
                "Each flashes on its own rhythm, but seeing a neighbour light up nudges it a little sooner. No one leads, yet slowly they fall into step, in waves.",
                "Here every city light is a firefly, and green is a flash. Tap to knock a crowd out of step, and watch it fall back in.",
        ]))
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.soft = ctx.reduce_motion;
        self.globe.hand.reduce_motion = ctx.reduce_motion;
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
