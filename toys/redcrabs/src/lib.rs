//! Red Crabs (a working title): Christmas Island in the wet season, seen from above, the forest floor,
//! a road and the shore. Red crabs walk out of the rainforest to the sea in their thousands (`crabs.rs`)
//! when the rain comes, hide in tree shade from the midday sun and follow the barrier fences
//! (`fence.rs`) that lead them to the tunnel under the road. A finger draws more fence; a tap brings a
//! shower. painting-company row 217's Terra holds it.

pub mod crabs;
pub mod fence;

use crabs::{Island, State, BODY, RULES, SHOWER_M, SHOWER_S};
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::switches::Switch;
use toy_kit::{info, Ctx, Phase, PointerEvent, Toy};

/// Where it is: Christmas Island, in the Indian Ocean (lon, lat).
pub const PLACE: (f64, f64) = (105.63, -10.48);

/// The screen's shorter side is this many metres of forest floor.
const WIDTH_M: f32 = 7.0;
/// A touch that moves less than this (points) in less than this long (seconds) is a tap.
const TAP_PT: f64 = 12.0;
const TAP_S: f64 = 0.6;

struct Finger {
    id: u64,
    from: [f64; 2],
    t: f64,
    laying: bool,
}

pub struct RedCrabs {
    island: Island,
    ground: Backdrop,
    painter: Painter,
    finger: Option<Finger>,
    /// points a metre
    k: f32,
    t: f64,
    size_pt: [f64; 2],
}

fn scale(size: [f64; 2]) -> f32 {
    (size[0].min(size[1]) as f32 / WIDTH_M).max(1e-3)
}

/// How bright the day is (0.42 at night .. 1 at noon) for a sun of `sun`.
fn daylight(sun: f32) -> f32 {
    0.42 + 0.58 * sun.powf(0.5)
}

impl RedCrabs {
    fn m(&self, p: [f64; 2]) -> [f32; 2] {
        [p[0] as f32 / self.k, p[1] as f32 / self.k]
    }

    fn paint_ground_marks(&mut self, px: f32) {
        // wet ground where showers fell
        for (c, w) in &self.island.puddles {
            let c = [c[0] * px, c[1] * px];
            self.painter.disc(
                c,
                SHOWER_M * px * 0.7,
                SHOWER_M * px * 0.5,
                [0.0, 0.02, 0.03, 0.3 * w],
            );
        }
    }

    fn paint_crabs(&mut self, px: f32, lit: f32) {
        let lit = lit.max(0.7);
        let red = [0.98 * lit, 0.16 * lit, 0.08 * lit];
        let dark = [0.55 * lit, 0.07 * lit, 0.04 * lit];
        for c in &self.island.crabs {
            if self.island.in_tunnel(c.p) || c.p[1] < -0.2 {
                continue;
            }
            let r = BODY * c.size * px;
            let at = [c.p[0] * px, c.p[1] * px];
            let (s, co) = c.heading.sin_cos();
            let fwd = [co, s];
            let side = [-s, co];
            let (alpha, shrink) = match c.state {
                State::Rest => (0.55, 0.8),
                _ => (1.0, 1.0),
            };
            let r = r * shrink;
            // the legs, scuttling sideways as it walks
            let wob = if c.state == State::Rest {
                0.0
            } else {
                c.gait.sin() * 0.25
            };
            for k in [-0.35f32, 0.35] {
                let o = [
                    at[0] + fwd[0] * r * (k + wob * 0.3),
                    at[1] + fwd[1] * r * (k + wob * 0.3),
                ];
                let a = [o[0] + side[0] * r * 1.9, o[1] + side[1] * r * 1.9];
                let b = [o[0] - side[0] * r * 1.9, o[1] - side[1] * r * 1.9];
                self.painter
                    .line(a, b, r * 0.16, 0.8, [dark[0], dark[1], dark[2], alpha]);
            }
            self.painter
                .disc(at, r, 0.9, [red[0], red[1], red[2], alpha]);
            for k in [-1.0f32, 1.0] {
                let cl = [
                    at[0] + fwd[0] * r * 1.2 + side[0] * r * 0.55 * k,
                    at[1] + fwd[1] * r * 1.2 + side[1] * r * 0.55 * k,
                ];
                self.painter
                    .disc(cl, r * 0.38, 0.8, [red[0] * 0.9, red[1], red[2], alpha]);
            }
        }
    }

    fn paint_fences(&mut self, px: f32, lit: f32) {
        let mesh = [0.80 * lit, 0.84 * lit, 0.88 * lit, 0.95];
        let post = [0.30 * lit, 0.30 * lit, 0.32 * lit, 1.0];
        for line in &self.island.fences.lines {
            for w in line.windows(2) {
                let a = [w[0][0] * px, w[0][1] * px];
                let b = [w[1][0] * px, w[1][1] * px];
                self.painter.line(a, b, 0.028 * px, 1.0, mesh);
            }
            for p in line.iter().step_by(4) {
                self.painter
                    .disc([p[0] * px, p[1] * px], 0.04 * px, 1.0, post);
            }
        }
    }

    fn paint_sky(&mut self, px: f32, sun: f32) {
        // the canopy's shade, deepest when the sun is high
        let a = 0.12 + 0.42 * sun;
        for t in &self.island.trees {
            self.painter.disc(
                [t.c[0] * px, t.c[1] * px],
                t.r * px * 0.85,
                t.r * px * 0.3,
                [0.0, 0.03, 0.0, a],
            );
        }
        // the showers' rain, streaking down
        for (i, s) in self.island.showers.iter().enumerate() {
            let fade = (s.left / SHOWER_S).min(1.0) * (SHOWER_S - s.left).min(0.5) * 2.0;
            for j in 0..70u32 {
                let h = |k: u32| {
                    let x =
                        (j.wrapping_mul(2_654_435_761) ^ (i as u32 * 97 + k)).wrapping_mul(40_503);
                    (x >> 8) as f32 / (1u32 << 24) as f32
                };
                let ang = h(1) * std::f32::consts::TAU;
                let rr = h(2).sqrt() * SHOWER_M;
                let fall = (self.t as f32 * (2.0 + h(3)) + h(4)).fract();
                let c = [
                    s.c[0] + ang.cos() * rr,
                    s.c[1] + ang.sin() * rr - (1.0 - fall) * 0.6,
                ];
                let a = [c[0] * px, c[1] * px];
                let b = [(c[0] - 0.02) * px, (c[1] + 0.18) * px];
                self.painter
                    .line(a, b, 0.008 * px, 0.8, [0.85, 0.9, 1.0, 0.45 * fade]);
                if fall > 0.85 {
                    self.painter.ring(
                        [c[0] * px, (c[1] + 0.6 * (1.0 - fall)) * px],
                        (fall - 0.85) * 0.6 * px,
                        0.01 * px,
                        0.8,
                        [0.8, 0.9, 1.0, 0.35 * fade],
                    );
                }
            }
        }
    }
}

/// The island as it opens: the rain has just been, and the crabs are on their way.
fn opening(w: f32, h: f32) -> Island {
    let mut island = Island::new(w, h, 0x00C2_AB5E);
    island.rain([w * 0.4, h * 0.15]);
    for _ in 0..180 {
        island.step(1.0 / 30.0);
    }
    island.t = 0.0;
    island
}

impl Toy for RedCrabs {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let k = scale(size_pt);
        let (w, h) = (size_pt[0] as f32 / k, size_pt[1] as f32 / k);
        let island = opening(w, h);
        Ok(RedCrabs {
            island,
            ground: Backdrop::new(&ctx.gpu, "redcrabs-ground", include_str!("ground.wgsl")),
            painter: Painter::new(&ctx.gpu),
            finger: None,
            k,
            t: 0.0,
            size_pt,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        let size = ctx.points();
        if (size[0] - self.size_pt[0]).abs() < 1.0 && (size[1] - self.size_pt[1]).abs() < 1.0 {
            return;
        }
        self.size_pt = size;
        self.k = scale(size);
        let on = self.island.on;
        self.island = opening(size[0] as f32 / self.k, size[1] as f32 / self.k);
        self.island.on = on;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = self.m([e.x, e.y]);
        match e.phase {
            Phase::Down => {
                if let Some(f) = &self.finger {
                    if f.laying {
                        self.island.fences.end();
                    }
                }
                self.finger = Some(Finger {
                    id: e.id,
                    from: [e.x, e.y],
                    t: self.t,
                    laying: false,
                });
            }
            Phase::Move => {
                let Some(f) = self.finger.as_mut().filter(|f| f.id == e.id) else {
                    return;
                };
                if !f.laying && (e.x - f.from[0]).hypot(e.y - f.from[1]) > TAP_PT {
                    f.laying = true;
                    let start = [f.from[0] as f32 / self.k, f.from[1] as f32 / self.k];
                    self.island.fences.begin(start);
                    ctx.haptic();
                }
                if f.laying {
                    self.island.fences.extend(p);
                }
            }
            _ => {
                if !self.finger.as_ref().is_some_and(|f| f.id == e.id) {
                    return;
                }
                let Some(f) = self.finger.take() else {
                    return;
                };
                if matches!(e.phase, Phase::Cancel) {
                    if f.laying {
                        self.island.fences.end();
                    }
                } else if f.laying {
                    self.island.fences.extend(p);
                    self.island.fences.end();
                } else if self.t - f.t < TAP_S {
                    self.island.rain(p);
                    ctx.haptic();
                }
            }
        }
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        let (w, h) = (self.island.w, self.island.road[0]);
        self.island.rain([w * 0.5, h * 0.5]);
        ctx.haptic();
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = (dt as f32).clamp(0.0, 0.1);
        // two half steps keep the crabs off each other's backs on a slow frame
        self.island.step(dt * 0.5);
        self.island.step(dt * 0.5);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let px = self.k * ctx.ppp as f32;
        let sun = self.island.sun();
        let lit = daylight(sun);
        let i = &self.island;
        let params = BackdropParams {
            time: self.t as f32,
            scale: px,
            center: [i.w * 0.5, i.h * 0.5],
            p: [
                lit, sun, i.road[0], i.road[1], i.shore, i.sea, i.gap[0], i.gap[1],
            ],
        };
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.ground
            .render(&ctx.gpu, &mut enc, target, size, &params);
        self.paint_ground_marks(px);
        self.paint_crabs(px, lit);
        self.paint_fences(px, lit);
        self.paint_sky(px, sun);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let i = &self.island;
        let k = self.k as f64;
        let at = |x: f32, y: f32| [x as f64 * k, y as f64 * k];
        vec![
            ("forest".into(), at(i.w * 0.5, i.road[0] * 0.4)),
            ("above-left".into(), at(i.w * 0.12, i.road[0] - 1.6)),
            ("above-mid".into(), at(i.w * 0.45, i.road[0] - 2.6)),
            ("gap".into(), at((i.gap[0] + i.gap[1]) * 0.5, i.road[0])),
            ("below".into(), at(i.w * 0.3, (i.road[1] + i.shore) * 0.5)),
        ]
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Red crabs marching to the sea. Draw a fence.",
            vec![info::Stroke::drag(
                0.4,
                1.2,
                [s[0] * 0.15, s[1] * 0.3],
                [s[0] * 0.55, s[1] * 0.36],
            )],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "On Christmas Island the wet season's rain sets millions of red crabs walking from the forest to the sea to spawn.",
                "They hide in shade from the midday sun, which dries them out.",
                "A fence turns them along it to the tunnels under the road. Drag to build one; tap for rain.",
            ],
            "",
        ))
    }

    fn switches(&self) -> Vec<Switch> {
        Switch::all(&RULES, &self.island.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(r) = self.island.on.get_mut(i) {
            *r = on;
        }
    }
}

toy_kit::ios_shell!(RedCrabs);
