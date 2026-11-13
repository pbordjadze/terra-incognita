//! Catatumbo (a working title): night over Lake Maracaibo, where the Catatumbo river meets the lake and
//! lightning flickers on most nights of the year. Your finger is the ground: touch the lake and a
//! stroke grows down to it, forking by the rule real lightning follows (`bolt.rs`), and flashes on the
//! water; hold and it strikes again down the same channel. The thunder is the bolt's own (`sound.rs`):
//! every piece of the channel cracks, and each crack reaches you after sound has crossed the distance.
//! Touch the sky and a crawler spreads under the cloud. Left alone, the storm keeps flickering.
//! painting-company row 327.

pub mod bolt;
pub mod sound;

use bolt::{Bolt, Rng, CELL_M, NZ};
use sound::{Pulse, Thunder, SOUND_MS};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};

/// Where it is: the mouth of the Catatumbo, south-west Lake Maracaibo (lon, lat).
pub const PLACE: (f64, f64) = (-71.65, 9.45);

/// The horizon, a share of the screen's height from the top.
const HORIZON: f64 = 0.44;
/// The eye is this high over the water (metres), and the screen's bottom edge is this far out.
const EYE_M: f64 = 60.0;
const NEAREST_M: f64 = 150.0;
/// The leader's descent, seconds (a real one takes a few hundredths; slowed so it is seen).
const LEAD_S: f64 = 0.42;
/// A held finger draws strokes down the channel, this far apart (seconds), at most this many.
const RESTRIKE_S: (f64, f64) = (0.09, 0.26);
const MAX_STROKES: usize = 9;
/// A stroke's light dies in this long.
const STROKE_S: f64 = 0.16;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Ground,
    Sheet,
}

struct Strike {
    kind: Kind,
    bolt: Bolt,
    /// the foot (ground) or the start (sheet), points on screen
    at: [f64; 2],
    /// metres to it
    dist: f64,
    born: f64,
    /// the return strokes' times (seconds)
    strokes: Vec<f64>,
    finger: Option<u64>,
    /// how bright its strokes are (the storm's own far ones are dimmer)
    power: f32,
    thundered: usize,
    seed: u64,
}

pub struct Catatumbo {
    painter: Painter,
    sky: Backdrop,
    thunder: Thunder,
    strikes: Vec<Strike>,
    rng: Rng,
    t: f64,
    size_pt: [f64; 2],
    next_storm: f64,
    flicker: (f64, [f64; 2], f64),
    reduce_motion: bool,
}

impl Catatumbo {
    fn horizon(&self) -> f64 {
        self.size_pt[1] * HORIZON
    }

    /// Points a metre at one metre's distance (the lens), so the bottom edge is `NEAREST_M` out.
    fn focal(&self) -> f64 {
        NEAREST_M * (self.size_pt[1] - self.horizon()) / EYE_M
    }

    /// Metres out to the water under a point below the horizon.
    fn distance_at(&self, y: f64) -> f64 {
        EYE_M * self.focal() / (y - self.horizon()).max(1.5)
    }

    /// The screen point (points) of a place `lateral` metres right of the foot's line and `alt` metres
    /// up, at `dist` metres out, for a foot drawn at `foot_x`.
    fn screen(&self, foot_x: f64, lateral: f64, alt: f64, dist: f64) -> [f64; 2] {
        let f = self.focal() / dist;
        [foot_x + lateral * f, self.horizon() + (EYE_M - alt) * f]
    }

    fn strike_at(&mut self, ctx: &mut Ctx, p: [f64; 2], finger: Option<u64>, power: f32) {
        let seed = self.rng.next();
        if p[1] > self.horizon() + 1.0 {
            let bolt = Bolt::strike(seed);
            let dist = self.distance_at(p[1]);
            self.strikes.push(Strike {
                kind: Kind::Ground,
                bolt,
                at: p,
                dist,
                born: self.t,
                strokes: vec![self.t + LEAD_S],
                finger,
                power,
                thundered: 0,
                seed,
            });
        } else {
            let bolt = Bolt::sheet(seed, 420);
            // the cloud base is ~1.2 km up: the higher on the screen, the nearer overhead
            let up = (self.horizon() - p[1]).max(4.0) / self.focal();
            let dist = ((NZ as f64 - 1.0) * CELL_M - EYE_M) / up.max(0.05);
            self.strikes.push(Strike {
                kind: Kind::Sheet,
                bolt,
                at: p,
                dist,
                born: self.t,
                strokes: vec![self.t + LEAD_S * 0.8],
                finger,
                power,
                thundered: 0,
                seed,
            });
            ctx.haptic();
        }
    }

    /// The points (points on screen) of a strike's cells.
    fn cell_points(&self, s: &Strike) -> Vec<[f64; 2]> {
        let b = &s.bolt;
        match s.kind {
            Kind::Ground => {
                let foot = b.foot() as f64;
                b.cells
                    .iter()
                    .map(|c| {
                        let lateral = (c.x as f64 - foot) * CELL_M;
                        let alt = (b.nz as f64 - 1.0 - c.z as f64) * CELL_M;
                        self.screen(s.at[0], lateral, alt.max(0.0), s.dist)
                    })
                    .collect()
            }
            Kind::Sheet => {
                // flat under the deck, foreshortened: wide across, shallow in depth
                let k = (self.focal() / s.dist * CELL_M * 0.8).clamp(7.0, 16.0);
                let (cx, cz) = (b.nx as f64 * 0.5, b.nz as f64 * 0.5);
                b.cells
                    .iter()
                    .map(|c| {
                        [
                            s.at[0] + (c.x as f64 - cx) * k,
                            s.at[1] + (c.z as f64 - cz) * k * 0.35,
                        ]
                    })
                    .collect()
            }
        }
    }

    /// The thunder of one stroke: a crack from every piece of the channel, after its distance.
    fn thunder_of(&self, s: &Strike, first: bool) -> Vec<Pulse> {
        let b = &s.bolt;
        let foot_lateral = (s.at[0] - self.size_pt[0] * 0.5) * s.dist / self.focal();
        let foot = b.foot() as f64;
        let mut out = Vec::new();
        for (k, c) in b.cells.iter().enumerate().skip(1) {
            if !first && !c.main {
                continue;
            }
            let (x, alt, depth) = match s.kind {
                Kind::Ground => (
                    foot_lateral + (c.x as f64 - foot) * CELL_M,
                    (b.nz as f64 - 1.0 - c.z as f64) * CELL_M,
                    s.dist,
                ),
                Kind::Sheet => (
                    foot_lateral + (c.x as f64 - b.nx as f64 * 0.5) * CELL_M,
                    (NZ as f64 - 1.0) * CELL_M,
                    s.dist + (c.z as f64 - b.nz as f64 * 0.5) * CELL_M,
                ),
            };
            let d = (x * x + (alt - EYE_M).powi(2) + depth * depth).sqrt().max(20.0);
            let strength = if c.main { 1.0 } else { 0.35 + c.current };
            // a little jitter: the channel's kinks spread each piece's arrival
            let j = (hash(s.seed ^ k as u64) - 0.5) * 0.02;
            out.push(Pulse {
                delay: d / SOUND_MS + j,
                amp: (0.7 * strength as f64 * (220.0 / d).min(1.6)) as f32
                    * if first { 1.0 } else { 0.5 }
                    * s.power,
                tau: (0.03 + d / 9_000.0) as f32,
                bright: (1.0 - d / 2500.0).clamp(0.0, 1.0) as f32,
                pan: (x / d) as f32,
            });
        }
        out
    }

    /// The storm on its own: far strokes and flickers in the deck now and then.
    fn storm(&mut self, ctx: &mut Ctx) {
        if self.t < self.next_storm {
            return;
        }
        self.next_storm = self.t + self.rng.range(1.2, 4.5);
        let w = self.size_pt[0];
        let hy = self.horizon();
        if self.rng.f() < 0.4 {
            // a far ground stroke, just below the horizon
            let p = [self.rng.range(w * 0.1, w * 0.9), hy + self.rng.range(4.0, 22.0)];
            self.strike_at(ctx, p, None, 0.55);
        } else {
            let p = [self.rng.range(0.0, w), self.rng.range(hy * 0.25, hy * 0.92)];
            self.flicker = (self.t, p, self.rng.range(0.25, 0.7));
        }
    }

    /// How bright a strike is at time t (0 dark), and whether its leader is still coming.
    fn light(&self, s: &Strike, t: f64) -> f32 {
        let mut l = 0.0f32;
        for &st in &s.strokes {
            let a = t - st;
            if a >= 0.0 {
                l += (-(a / STROKE_S) * 3.0).exp() as f32;
            }
        }
        l * s.power
    }
}

fn hash(x: u64) -> f64 {
    let mut r = Rng(x.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    r.f()
}

/// A segment broken into a jagged line, finer the longer it is on screen: lightning is crooked at
/// every scale. Midpoints pushed sideways by a seeded amount; stops below `min` points.
fn jag(a: [f64; 2], b: [f64; 2], seed: u64, min: f64, out: &mut Vec<[f64; 2]>) {
    let d = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
    if d < min {
        out.push(b);
        return;
    }
    let n = [-(b[1] - a[1]) / d, (b[0] - a[0]) / d];
    let off = (hash(seed) - 0.5) * d * 0.32;
    let m = [(a[0] + b[0]) * 0.5 + n[0] * off, (a[1] + b[1]) * 0.5 + n[1] * off];
    jag(a, m, seed.wrapping_mul(31).wrapping_add(1), min, out);
    jag(m, b, seed.wrapping_mul(37).wrapping_add(2), min, out);
}

impl Toy for Catatumbo {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let sky = Backdrop::new(&ctx.gpu, "catatumbo-sky", include_str!("sky.wgsl"));
        let thunder = Thunder::new();
        thunder.rain(1.0);
        Ok(Catatumbo {
            painter: Painter::new(&ctx.gpu),
            sky,
            thunder,
            strikes: Vec::new(),
            rng: Rng(0xCA7A_7B00),
            t: 0.0,
            size_pt: ctx.points(),
            next_storm: 0.6,
            flicker: (-10.0, [0.0, 0.0], 0.0),
            reduce_motion: ctx.reduce_motion,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.reduce_motion = ctx.reduce_motion;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => self.strike_at(ctx, p, Some(e.id), 1.0),
            Phase::Move => {
                // a finger dragged far from its stroke calls a new one there
                let held = self
                    .strikes
                    .iter()
                    .rposition(|s| s.finger == Some(e.id));
                if let Some(i) = held {
                    let s = &self.strikes[i];
                    let far = ((s.at[0] - p[0]).powi(2) + (s.at[1] - p[1]).powi(2)).sqrt() > 44.0;
                    if far && self.t - s.born > LEAD_S + 0.12 {
                        self.strikes[i].finger = None;
                        self.strike_at(ctx, p, Some(e.id), 1.0);
                    }
                }
            }
            _ => {
                for s in &mut self.strikes {
                    if s.finger == Some(e.id) {
                        s.finger = None;
                    }
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, _dt: f64) {
        self.t = t;
        self.storm(ctx);
        let reduce = self.reduce_motion;
        let mut pulses = Vec::new();
        let mut buzz = false;
        for i in 0..self.strikes.len() {
            // a held finger keeps the channel striking
            let s = &self.strikes[i];
            let last = *s.strokes.last().unwrap_or(&s.born);
            if s.finger.is_some() && !reduce && t > last && s.strokes.len() < MAX_STROKES {
                let gap = RESTRIKE_S.0 + (RESTRIKE_S.1 - RESTRIKE_S.0) * hash(s.seed ^ s.strokes.len() as u64);
                if t - last > gap {
                    self.strikes[i].strokes.push(t);
                }
            }
            let s = &self.strikes[i];
            let due = s.strokes.iter().filter(|&&st| st <= t).count();
            if due > s.thundered {
                pulses.extend(self.thunder_of(s, s.thundered == 0));
                buzz |= s.power > 0.9;
                self.strikes[i].thundered = due;
            }
        }
        if buzz {
            ctx.haptic();
        }
        if !pulses.is_empty() {
            self.thunder.send(&pulses);
        }
        self.strikes.retain(|s| {
            s.finger.is_some() || t - s.strokes.last().copied().unwrap_or(s.born) < 1.6
        });
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let ppp = ctx.ppp;
        let t = self.t;
        let hy = self.horizon();
        let cap = if self.reduce_motion { 0.3 } else { 1.0 };
        // the brightest flash lights the deck
        let mut flash = (0.0f32, [self.size_pt[0] * 0.5, hy * 0.5]);
        for s in &self.strikes {
            let l = self.light(s, t);
            if l > flash.0 {
                let up = match s.kind {
                    Kind::Ground => self.screen(s.at[0], 0.0, 900.0, s.dist)[1].max(hy * 0.12),
                    Kind::Sheet => s.at[1],
                };
                flash = (l, [s.at[0], up]);
            }
        }
        let (ft, fp, fa) = self.flicker;
        let fage = t - ft;
        let flick = if fage < 0.6 {
            // a flicker comes as two or three quick pulses
            fa * ((fage * 31.0).sin().max(0.0) * (-fage * 5.0).exp())
        } else {
            0.0
        } as f32;
        let params = BackdropParams {
            time: t as f32,
            scale: ppp as f32,
            center: [0.0, 0.0],
            p: [
                (flash.0.min(1.6) * cap),
                (flash.1[0] * ppp) as f32,
                (flash.1[1] * ppp) as f32,
                (hy * ppp) as f32,
                flick * cap,
                (fp[0] * ppp) as f32,
                (fp[1] * ppp) as f32,
                1.0,
            ],
        };
        let strikes = std::mem::take(&mut self.strikes);
        for s in &strikes {
            self.paint_strike(s, ppp, cap);
        }
        self.strikes = strikes;
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.sky.render(&ctx.gpu, &mut enc, target, size, &params);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.thunder.synth())
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Lake Maracaibo's storm. Touch the water.",
            vec![info::Stroke::tap(0.5, [s[0] * 0.55, s[1] * 0.72])],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "Real lightning breaks the air a step at a time, likeliest where the pull is strongest, so it forks.",
                "Your finger is the ground it reaches for; hold, and it strikes again down the same channel.",
                "Every piece of the channel thunders; sound takes 3 seconds a kilometre, so the near pieces crack first and the far ones roll in.",
            ],
            "",
        ))
    }
}

impl Catatumbo {
    fn paint_strike(&mut self, s: &Strike, ppp: f64, cap: f32) {
        let t = self.t;
        let pts = self.cell_points(s);
        let age = t - s.born;
        let first = s.strokes[0];
        let leading = t < first;
        let reveal = if leading { (age / (first - s.born)).clamp(0.0, 1.0) as f32 } else { 1.0 };
        let light = self.light(s, t).min(1.6) * cap;
        let after = if leading {
            0.0
        } else {
            (-(t - s.strokes.last().copied().unwrap_or(first)) * 1.6).exp() as f32
        };
        let near = (self.focal() / s.dist).clamp(0.05, 8.0);
        let width = (0.8 + near * 0.6).min(3.0);
        let foot_y = s.at[1];
        let ground = s.kind == Kind::Ground;
        let mut line = Vec::new();
        for (k, c) in s.bolt.cells.iter().enumerate().skip(1) {
            if c.order > reveal {
                break;
            }
            // after the stroke, only the channels that carried current still show
            let cur = c.current;
            let (alpha, core) = if leading {
                (0.22 + 0.25 * cur, false)
            } else {
                let a = light * cur.powf(1.5) + after * 0.25 * cur;
                if a < 0.01 {
                    continue;
                }
                (a, true)
            };
            let a = pts[c.parent];
            let b = pts[k];
            line.clear();
            line.push(a);
            jag(a, b, s.seed ^ (k as u64) << 7, 7.0, &mut line);
            let r = (width * (0.35 + 0.9 * cur as f64)) as f32;
            for w in line.windows(2) {
                let (pa, pb) = (w[0], w[1]);
                let (qa, qb) = (px(pa, ppp), px(pb, ppp));
                let violet = [0.62, 0.52, 1.0, alpha * 0.22];
                let p = ppp as f32;
                self.painter.glow(qa, qb, 0.0, r * 9.0 * p, violet);
                if core {
                    self.painter
                        .glow(qa, qb, r * 0.5 * p, 1.2 * p, [1.0, 0.97, 1.0, alpha.min(1.0)]);
                } else {
                    self.painter
                        .glow(qa, qb, r * 0.3 * p, 1.0 * p, [0.75, 0.7, 1.0, alpha]);
                }
                // its reflection in the lake, broken by the ripples
                if ground && pa[1] < foot_y && pb[1] < foot_y {
                    let m = |p: [f64; 2]| {
                        let y = 2.0 * foot_y - p[1];
                        let d = y - foot_y;
                        let wob = (hash((y * 0.5) as u64 ^ (t * 12.0) as u64) - 0.5) * (2.0 + d * 0.06);
                        [p[0] + wob, y]
                    };
                    let (ra, rb) = (m(pa), m(pb));
                    if ra[1] < self.size_pt[1] + 40.0 {
                        self.painter.glow(
                            px(ra, ppp),
                            px(rb, ppp),
                            r * 0.4 * ppp as f32,
                            r * 3.0 * ppp as f32,
                            [0.70, 0.62, 1.0, alpha * 0.30],
                        );
                    }
                }
            }
        }
        // where it meets the water: a splash of light, and while it comes, a streamer rising to meet it
        if ground {
            let f = px(s.at, ppp);
            if leading && reveal > 0.75 {
                let up = ((reveal - 0.75) / 0.25) as f64 * 30.0 * near.min(2.0);
                let top = px([s.at[0], s.at[1] - up], ppp);
                self.painter
                    .glow(f, top, (width * 0.3 * ppp) as f32, (2.0 * ppp) as f32, [0.8, 0.75, 1.0, 0.6]);
            }
            if !leading {
                let rr = (14.0 + 50.0 * near.min(3.0)) * ppp;
                self.painter.glow(
                    f,
                    [f[0] + 0.01, f[1]],
                    0.0,
                    rr as f32,
                    [0.85, 0.75, 1.0, (light * 0.55 + after * 0.1).min(0.9)],
                );
            }
        }
    }
}

fn px(p: [f64; 2], ppp: f64) -> [f32; 2] {
    [(p[0] * ppp) as f32, (p[1] * ppp) as f32]
}

toy_kit::ios_shell!(Catatumbo);
