//! Bats (a working title): a summer dusk above Bracken Cave, north of San Antonio, where millions of
//! Mexican free-tailed bats come out of a sinkhole in a turning column and stream off over the hills
//! to feed. You are the hawk that waits for them: a finger dives into the stream, the bats it nears
//! swerve, and their swerve runs on down the stream from bat to bat.
//!
//! `colony.rs` is the bats (their rules), `land.wgsl` the sky and the land, `sound.rs` the detector's
//! clicks. The watcher stands on a rise `EYE` metres up, the cave's mouth `colony::CAVE_Z` ahead.

pub mod colony;
pub mod sound;

use colony::{len, Colony, Stage, CAVE_X, CAVE_Z};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};

/// Bats at most (the colony is millions; these are the ones drawn).
const N: usize = 56_000;
/// The watcher's eye above the cave mouth's ground, metres; the view's height, degrees.
const EYE: f32 = 14.0;
const FOV: f32 = 75.0;
/// Where the horizon sits, as a share of the screen's height from the top.
const HORIZON: f32 = 0.79;
/// Dusk deepens over this many seconds, then holds.
const DUSK_S: f32 = 300.0;
/// The emergence is already under way when the toy opens: this many seconds of it are run first.
const PREWARM_S: f32 = 70.0;
/// A real wingspan, metres (bats, the hawk), and how much larger the hawk is drawn, to be seen.
const BAT_SPAN: f32 = 0.30;
const HAWK_SPAN: f32 = 1.3;
const HAWK_DRAWN: f32 = 1.7;
/// How near the hawk must come for a bat to see it, metres.
const SEEN: f32 = 16.0;

struct Hawk {
    id: u64,
    /// the finger's point (points) and how far along the view the hawk flies (camera depth)
    at: [f64; 2],
    depth: f32,
    p: [f32; 3],
    v: [f32; 3],
    /// seconds since the finger lifted (the hawk flies off), or None while it is down
    gone: Option<f32>,
    wing: f32,
}

pub struct Bats {
    colony: Colony,
    sky: Backdrop,
    painter: Painter,
    sound: sound::Sound,
    hawks: Vec<Hawk>,
    t: f64,
    size_pt: [f64; 2],
    swerves: f32,
}

/// The camera: points a radian (focal length), and the pitch up.
fn camera(size_pt: [f64; 2]) -> (f32, f32) {
    let h = size_pt[1] as f32;
    let f = 0.5 * h / (FOV.to_radians() * 0.5).tan();
    let pitch = ((HORIZON - 0.5) * h / f).atan();
    (f, pitch)
}

impl Bats {
    /// Where a point (metres) is on screen (points), and its depth along the view, if ahead.
    fn project(&self, p: [f32; 3]) -> Option<([f32; 2], f32)> {
        let (f, th) = camera(self.size_pt);
        let r = [p[0], p[1] - EYE, p[2]];
        let y = r[1] * th.cos() - r[2] * th.sin();
        let z = r[1] * th.sin() + r[2] * th.cos();
        if z < 1.0 {
            return None;
        }
        let w = self.size_pt[0] as f32;
        let h = self.size_pt[1] as f32;
        Some(([w * 0.5 + f * r[0] / z, h * 0.5 - f * y / z], z))
    }

    /// The point (metres) at screen point `q` (points) and camera depth `z`.
    fn unproject(&self, q: [f64; 2], z: f32) -> [f32; 3] {
        let (f, th) = camera(self.size_pt);
        let x = (q[0] as f32 - self.size_pt[0] as f32 * 0.5) / f * z;
        let y = -(q[1] as f32 - self.size_pt[1] as f32 * 0.5) / f * z;
        [
            x,
            y * th.cos() + z * th.sin() + EYE,
            -y * th.sin() + z * th.cos(),
        ]
    }

    /// How deep a finger at `q` reaches: the nearest bat under it on screen, else the column.
    fn depth_at(&self, q: [f64; 2]) -> f32 {
        let mut best = (40.0f32, None);
        for b in &self.colony.bats {
            if b.stage == Stage::Roost {
                continue;
            }
            if let Some((s, z)) = self.project(b.p) {
                let d = ((s[0] - q[0] as f32).powi(2) + (s[1] - q[1] as f32).powi(2)).sqrt();
                if d < best.0 {
                    best = (d, Some(z));
                }
            }
        }
        best.1.unwrap_or(CAVE_Z)
    }

    fn paint_bats(&mut self, k: f32, f: f32) {
        // the bats: dark against the sky, each a pair of beating wings; far ones a smoke
        let ink = [0.035, 0.028, 0.04];
        for b in &self.colony.bats {
            if b.stage == Stage::Roost || b.p[1] < -0.5 {
                continue;
            }
            let Some((c, z)) = self.project(b.p) else {
                continue;
            };
            // a jinking bat throws its wings wide: the ripple shows as a dark flicker running on
            let jinking = colony::jinking(b);
            let span = BAT_SPAN * f / z * k * (1.0 + 0.8 * jinking);
            let side = {
                let h = (b.v[0] * b.v[0] + b.v[2] * b.v[2]).sqrt().max(1e-3);
                [b.v[2] / h, 0.0, -b.v[0] / h]
            };
            let flap = b.wing.sin();
            let half = BAT_SPAN * 0.5;
            let tip = |s: f32| {
                [
                    b.p[0] + side[0] * half * s,
                    b.p[1] + half * 0.55 * flap,
                    b.p[2] + side[2] * half * s,
                ]
            };
            let (Some((l, _)), Some((r, _))) = (self.project(tip(-1.0)), self.project(tip(1.0)))
            else {
                continue;
            };
            let px = |p: [f32; 2]| [p[0] * k, p[1] * k];
            let thins = if b.stage == Stage::Stream {
                ((colony::STREAM_END - b.s) / 700.0).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let fade = (-z / 2600.0).exp() * thins;
            let cover = (span / 1.6).clamp(0.12, 1.0);
            let a = (0.92 * fade * cover * (1.0 + jinking)).min(1.0);
            let col = [ink[0], ink[1], ink[2], a];
            let w = (span * 0.11).clamp(0.55, 2.5);
            if span < 1.4 {
                self.painter.disc(px(c), 0.7, 0.8, col);
            } else {
                self.painter.line(px(l), px(c), w, 0.7, col);
                self.painter.line(px(c), px(r), w, 0.7, col);
            }
        }
    }

    fn paint_hawks(&mut self, k: f32, f: f32) {
        // the hawks: you
        for h in &self.hawks {
            let Some((_, z)) = self.project(h.p) else {
                continue;
            };
            let fade = h.gone.map_or(1.0, |g| (1.0 - g / 2.5).max(0.0));
            let speed = len(h.v);
            // stooping it folds its wings back
            let fold = ((speed - 14.0) / 16.0).clamp(0.0, 0.35);
            // drawn at least big enough to see under a finger, wherever it flies
            let at_least = 46.0 * z / f;
            let span = (HAWK_SPAN * HAWK_DRAWN).max(at_least) * (1.0 - fold * 0.6);
            let dir = {
                let n = (h.v[0] * h.v[0] + h.v[2] * h.v[2]).sqrt().max(1e-3);
                [h.v[0] / n, h.v[2] / n]
            };
            // seen from below, as a hawk is against the sky: the wings drawn across its way on the
            // glass, so it never shows edge-on
            let Some((ahead, _)) = self.project([
                h.p[0] + dir[0] * 4.0,
                h.p[1] + h.v[1] / speed.max(1.0) * 4.0,
                h.p[2] + dir[1] * 4.0,
            ]) else {
                continue;
            };
            let Some((c, _)) = self.project(h.p) else {
                continue;
            };
            let fwd = {
                let d = [ahead[0] - c[0], ahead[1] - c[1]];
                let n = (d[0] * d[0] + d[1] * d[1]).sqrt().max(1e-3);
                [d[0] / n, d[1] / n]
            };
            let across = [-fwd[1], fwd[0]];
            let half = span * f / z * 0.5;
            let beat = if fold > 0.3 { 0.0 } else { h.wing.sin() * 0.12 };
            // (along its way, across it), in points from its body
            let at = |a: f32, x: f32| {
                [
                    (c[0] + fwd[0] * a * half + across[0] * x * half) * k,
                    (c[1] + fwd[1] * a * half + across[1] * x * half) * k,
                ]
            };
            let col = [0.03, 0.022, 0.02, 0.97 * fade];
            let sweep = fold * 0.6;
            let w = (0.07 * half * k).max(1.0);
            for sd in [-1.0f32, 1.0] {
                // the wing, filled: from the body out to its leading edge (bent at the wrist) and
                // its trailing edge, the hand swept back to a tip
                let wrist = [0.16 - sweep * 0.3 + beat, sd * (0.5 - fold * 0.15)];
                let tip = [-0.2 - sweep + beat, sd * (1.0 - fold * 0.35)];
                for j in 0..=6 {
                    let u = j as f32 / 6.0;
                    let root = [0.14 - 0.34 * u, sd * 0.05];
                    // the leading edge to the wrist and on to the tip; the trailing edge back in
                    let out = if u < 0.5 {
                        let v = u / 0.5;
                        [
                            wrist[0] + (tip[0] - wrist[0]) * v,
                            wrist[1] + (tip[1] - wrist[1]) * v,
                        ]
                    } else {
                        let v = (u - 0.5) / 0.5;
                        [
                            tip[0] + (-0.22 - tip[0]) * v,
                            tip[1] + (sd * 0.3 - tip[1]) * v,
                        ]
                    };
                    self.painter
                        .line(at(root[0], root[1]), at(out[0], out[1]), w, 1.0, col);
                }
            }
            // the body, the head ahead, the fanned tail behind
            self.painter
                .line(at(0.28, 0.0), at(-0.25, 0.0), w * 1.8, 1.0, col);
            for x in [-0.12f32, -0.04, 0.04, 0.12] {
                self.painter
                    .line(at(-0.2, 0.0), at(-0.55, x), w * 1.1, 1.0, col);
            }
        }
    }

    /// 0 as the toy opens (the sun just down), 1 at the deepest it goes.
    fn dusk(&self) -> f32 {
        ((self.colony.t - PREWARM_S) / DUSK_S).clamp(0.0, 1.0)
    }
}

impl Toy for Bats {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let gpu = &ctx.gpu;
        let mut colony = Colony::new(N, 0xBA75_0001);
        let dt = 0.25;
        for _ in 0..(PREWARM_S / dt) as usize {
            colony.step(dt);
        }
        Ok(Bats {
            colony,
            sky: Backdrop::new(gpu, "bats-land", include_str!("land.wgsl")),
            painter: Painter::new(gpu),
            sound: sound::Sound::default(),
            hawks: Vec::new(),
            t: 0.0,
            size_pt: ctx.points(),
            swerves: 0.0,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
    }

    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        let q = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                self.hawks.retain(|h| h.id != e.id);
                let depth = self.depth_at(q);
                let p = self.unproject(q, depth);
                // it comes in from above and behind, already stooping
                self.hawks.push(Hawk {
                    id: e.id,
                    at: q,
                    depth,
                    p: [p[0] - 3.0, p[1] + 9.0, p[2] - 6.0],
                    v: [0.0, -16.0, 8.0],
                    gone: None,
                    wing: 0.0,
                });
            }
            Phase::Move => {
                if let Some(h) = self
                    .hawks
                    .iter_mut()
                    .find(|h| h.id == e.id && h.gone.is_none())
                {
                    h.at = q;
                }
            }
            _ => {
                if let Some(h) = self
                    .hawks
                    .iter_mut()
                    .find(|h| h.id == e.id && h.gone.is_none())
                {
                    h.gone = Some(0.0);
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = dt as f32;
        self.colony.step(dt);
        let mut swerved = self.colony.swerved;
        let mut fastest = 0.0f32;
        let mut caught = false;
        let targets: Vec<[f32; 3]> = self
            .hawks
            .iter()
            .map(|h| self.unproject(h.at, h.depth))
            .collect();
        for (h, target) in self.hawks.iter_mut().zip(targets) {
            match h.gone.as_mut() {
                None => {
                    // a spring to the finger, quick but not instant, so a flick is a stoop
                    let w = 7.5;
                    for k in 0..3 {
                        let a = w * w * (target[k] - h.p[k]) - 2.0 * w * h.v[k];
                        h.v[k] += a * dt;
                    }
                }
                Some(g) => {
                    // it climbs away
                    *g += dt;
                    h.v[1] += 30.0 * dt;
                    h.v[2] += 4.0 * dt;
                }
            }
            for k in 0..3 {
                h.p[k] += h.v[k] * dt;
            }
            h.wing += dt * std::f32::consts::TAU * 2.2;
            fastest = fastest.max(len(h.v));
            if h.gone.is_none_or(|g| g < 1.0) {
                let (n, c) = self.colony.hawk(h.p, h.v, SEEN);
                swerved += n;
                caught |= c.is_some();
            }
        }
        self.hawks.retain(|h| h.gone.is_none_or(|g| g < 2.5));
        self.colony.hawks = self
            .hawks
            .iter()
            .filter(|h| h.gone.is_none_or(|g| g < 1.0))
            .map(|h| h.p)
            .collect();
        if caught {
            ctx.haptic();
        }
        // the clicks: the bats near the cave mouth (the detector's there), and every swerve a flurry
        self.swerves = self.swerves * (-dt * 4.0).exp() + swerved as f32;
        let near = self
            .colony
            .bats
            .iter()
            .filter(|b| b.stage == Stage::Column && b.p[1] < 60.0)
            .count() as f32;
        self.sound.set(sound::Mix {
            clicks: near * 0.22 + self.swerves * 6.0,
            hawk: fastest,
            dusk: self.dusk(),
        });
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let k = ctx.ppp as f32;
        let size = [ctx.width, ctx.height];
        let (f, pitch) = camera(self.size_pt);
        let dusk = self.dusk();
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        self.sky.render(
            gpu,
            &mut enc,
            target,
            size,
            &BackdropParams {
                time: self.t as f32,
                scale: 1.0,
                center: [0.0, 0.0],
                p: [f * k, pitch, dusk, EYE, CAVE_Z, 7.0, CAVE_X, 0.0],
            },
        );
        self.paint_bats(k, f);
        self.paint_hawks(k, f);
        self.painter.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.sound.synth())
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        // the streams a little past the column's top, for the plans to dive at
        let mut v = Vec::new();
        for (k, name) in [(0, "stream"), (1, "stream2")] {
            let (p, _) = colony::stream_at(k, 260.0);
            if let Some((s, _)) = self.project(p) {
                v.push((name.to_string(), [s[0] as f64, s[1] as f64]));
            }
        }
        if let Some((s, _)) = self.project([CAVE_X, 60.0, CAVE_Z]) {
            v.push(("column".to_string(), [s[0] as f64, s[1] as f64]));
        }
        v
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let a = self.anchors();
        let at = |n: &str| a.iter().find(|x| x.0 == n).map(|x| x.1);
        let s = at("stream").unwrap_or([self.size_pt[0] * 0.6, self.size_pt[1] * 0.4]);
        Some(info::Demo::new(
            "You're the hawk. Dive into the bats.",
            vec![info::Stroke::drag(
                0.4,
                0.9,
                [s[0] - 70.0, s[1] - 110.0],
                [s[0] + 30.0, s[1] + 40.0],
            )],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::lines(&[
            "At dusk, millions of free-tailed bats climb out of Bracken Cave, in the Texas Hill Country, circling up in a column and streaming away to feed.",
            "A bat that sees a hawk jinks aside, and its neighbours copy it a moment later, so the swerve runs on through the flight.",
            "You are the hawk: dive into the bats and watch the ripple run.",
        ]))
    }
}

toy_kit::ios_shell!(Bats);
