//! Huddle (a working title): emperor penguins on the sea ice of Atka Bay in a winter wind, seen from
//! above. The ones on the windward edge get cold, peel off, walk round and press in again on the lee
//! side, so the huddle turns over and creeps downwind; now and then a wave of small steps runs through
//! it. A finger drawn across the ice turns the wind; a touch on a penguin starts a wave.
//!
//! `flock.rs` is the penguins (their rules), `ice.wgsl` the ice and the light, `sound.rs` the wind
//! and the calls. Everything runs at `PACE` times the real speed.

pub mod flock;
pub mod sound;

use flock::Huddle as Flock;
use toy_kit::backdrop::{Backdrop, BackdropParams};
use toy_kit::paint::Painter;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};

/// Penguins in the huddle.
const N: usize = 480;
/// Points a metre on screen, and how many times the real pace the huddle moves.
const PT_PER_M: f64 = 50.0;
const PACE: f32 = 4.0;
/// Seconds (real, before `PACE`) the huddle has already stood when the toy opens.
const PREWARM_S: f32 = 240.0;
/// A penguin's shadow on the ice, metres (x west, y south): the sun a few degrees up.
const SHADOW: [f32; 2] = [0.6, 1.3];
/// The day's wind, metres a second (the paper's days ran to 8.3 m/s).
const WIND: f32 = 7.0;
/// Snowflakes blowing over the scene.
const FLAKES: usize = 260;

struct Touch {
    id: u64,
    from: [f64; 2],
    last: [f64; 2],
    tl: f64,
    dragging: bool,
}

pub struct Huddle {
    flock: Flock,
    ice: Backdrop,
    painter: Painter,
    sound: sound::Sound,
    /// the world point (metres) at the screen's middle
    eye: [f32; 2],
    flakes: Vec<[f32; 3]>,
    touch: Option<Touch>,
    t: f64,
    size_pt: [f64; 2],
    calls_owed: f32,
}

impl Huddle {
    fn to_screen(&self, p: [f32; 2]) -> [f64; 2] {
        [
            self.size_pt[0] * 0.5 + (p[0] - self.eye[0]) as f64 * PT_PER_M,
            self.size_pt[1] * 0.5 + (p[1] - self.eye[1]) as f64 * PT_PER_M,
        ]
    }

    fn to_world(&self, q: [f64; 2]) -> [f32; 2] {
        [
            self.eye[0] + ((q[0] - self.size_pt[0] * 0.5) / PT_PER_M) as f32,
            self.eye[1] + ((q[1] - self.size_pt[1] * 0.5) / PT_PER_M) as f32,
        ]
    }

    fn touch_up(&mut self, ctx: &mut Ctx, q: [f64; 2]) {
        let Some(t) = self.touch.take() else {
            return;
        };
        if t.dragging {
            return;
        }
        let w = self.to_world(q);
        if let Some(i) = self.flock.at(w, flock::SPACING * 1.2) {
            self.flock.nudge(i);
            let pan = ((q[0] / self.size_pt[0]) * 2.0 - 1.0) as f32;
            self.sound.call(pan, 1.0);
            ctx.haptic();
        }
    }

    fn paint_penguins(&mut self, k: f32) {
        let s = (PT_PER_M as f32) * k;
        let px = |p: [f32; 2], eye: [f32; 2], size: [f64; 2]| -> [f32; 2] {
            [
                (size[0] as f32 * 0.5 + (p[0] - eye[0]) * PT_PER_M as f32) * k,
                (size[1] as f32 * 0.5 + (p[1] - eye[1]) * PT_PER_M as f32) * k,
            ]
        };
        let (eye, size) = (self.eye, self.size_pt);
        // long shadows first: the sun is low in the north, so each falls to the south (down the
        // screen) and a little west
        for b in &self.flock.birds {
            let a = px(b.p, eye, size);
            let z = px([b.p[0] - SHADOW[0] * 0.35, b.p[1] + SHADOW[1]], eye, size);
            self.painter
                .line(a, z, 0.085 * s, 0.07 * s, [0.20, 0.26, 0.45, 0.16]);
        }
        for (i, b) in self.flock.birds.iter().enumerate() {
            // each its own: a little bigger or smaller, its head turned its own way, and inside the
            // huddle its head mostly down, so its gold shows less
            let h = |n: u32| {
                let x = (i as u32)
                    .wrapping_mul(2_654_435_761)
                    .wrapping_add(n.wrapping_mul(40_503));
                ((x ^ (x >> 15)).wrapping_mul(2_246_822_519) >> 8) as f32 / (1u32 << 24) as f32
            };
            let s = s * (0.88 + 0.22 * h(1));
            let turn_head = (h(2) - 0.5) * 0.06;
            let shown = (0.30 + 0.7 * b.exposed.max(b.moving)) * (0.6 + 0.4 * h(3));
            let f = b.face;
            let side = [-f[1], f[0]];
            // the waddle: a sway side to side as it walks or steps
            let sway = (b.gait).sin() * 0.018 * b.moving.max(if b.step < 1.4 { 0.6 } else { 0.0 });
            let at = |a: f32, x: f32| {
                px(
                    [
                        b.p[0] + f[0] * a + side[0] * (x + sway),
                        b.p[1] + f[1] * a + side[1] * (x + sway),
                    ],
                    eye,
                    size,
                )
            };
            // the back: near black with a blue sheen, frosted where the wind has caught it
            let tone = 0.85 + 0.3 * h(4);
            let back = [0.07 * tone, 0.08 * tone, 0.11 * tone, 1.0];
            self.painter
                .line(at(-0.09, 0.0), at(0.05, 0.0), 0.115 * s, 0.6, back);
            let sheen = [0.30, 0.36, 0.48, 0.22];
            self.painter.line(
                at(-0.06, -0.03),
                at(0.03, -0.03),
                0.045 * s,
                0.05 * s,
                sheen,
            );
            if b.frost > 0.02 {
                let fr = [0.90, 0.94, 0.99, 0.8 * b.frost];
                self.painter
                    .line(at(-0.09, 0.0), at(0.02, 0.0), 0.085 * s, 0.035 * s, fr);
            }
            // the head, its gold ear patches, the bill
            self.painter
                .disc(at(0.12, turn_head), 0.062 * s, 0.6, [0.05, 0.05, 0.07, 1.0]);
            let gold = [0.98, 0.80, 0.36, 0.9 * shown];
            // the ear patches, seen from above: gold streaks down the sides of the head and neck
            self.painter.line(
                at(0.12, 0.05 + turn_head),
                at(0.06, 0.06),
                0.014 * s,
                0.01 * s,
                gold,
            );
            self.painter.line(
                at(0.12, -0.05 + turn_head),
                at(0.06, -0.06),
                0.014 * s,
                0.01 * s,
                gold,
            );
            self.painter.line(
                at(0.16, turn_head * 1.6),
                at(0.22, turn_head * 2.6),
                0.012 * s,
                0.6,
                [0.12, 0.10, 0.10, 1.0],
            );
            self.painter.line(
                at(0.165, turn_head * 1.7),
                at(0.20, turn_head * 2.4),
                0.007 * s,
                0.6,
                [0.95, 0.55, 0.42, 1.0],
            );
        }
    }

    fn paint_snow(&mut self, k: f32) {
        let w = self.flock.wind;
        let len = 0.08 + 0.03 * self.flock.gust;
        for fl in &self.flakes {
            let a = self.to_screen([fl[0], fl[1]]);
            let b = self.to_screen([fl[0] + w[0] * len, fl[1] + w[1] * len]);
            let p = |q: [f64; 2]| [(q[0] as f32) * k, (q[1] as f32) * k];
            self.painter.line(
                p(a),
                p(b),
                0.7 * k * fl[2],
                0.8,
                [1.0, 1.0, 1.0, 0.75 * fl[2]],
            );
        }
    }

    fn blow_flakes(&mut self, dt: f32) {
        let w = self.flock.wind;
        let v = 0.7 * self.flock.gust;
        let half = [
            (self.size_pt[0] / PT_PER_M * 0.5) as f32 + 0.3,
            (self.size_pt[1] / PT_PER_M * 0.5) as f32 + 0.3,
        ];
        let eye = self.eye;
        for (i, fl) in self.flakes.iter_mut().enumerate() {
            let wob = ((self.t as f32) * 3.0 + i as f32).sin() * 0.3;
            fl[0] += (w[0] * v - w[1] * wob) * dt * fl[2];
            fl[1] += (w[1] * v + w[0] * wob) * dt * fl[2];
            for a in 0..2 {
                let rel = fl[a] - eye[a];
                if rel > half[a] {
                    fl[a] -= 2.0 * half[a];
                } else if rel < -half[a] {
                    fl[a] += 2.0 * half[a];
                }
            }
        }
    }
}

impl Toy for Huddle {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let gpu = &ctx.gpu;
        // the wind off the ice cap to the south, blowing north and a little west
        let mut flock = Flock::new(N, [-0.35, -0.94], 0x5EA1_CE01);
        let dt = 0.1;
        for _ in 0..(PREWARM_S * PACE / dt) as usize {
            flock.step(dt);
        }
        let (c, _) = flock.middle();
        let mut seed = 0x1234_5678u32;
        let mut r = || {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            (seed >> 8) as f32 / (1u32 << 24) as f32
        };
        let pts = ctx.points();
        let span = [(pts[0] / PT_PER_M) as f32, (pts[1] / PT_PER_M) as f32];
        let flakes = (0..FLAKES)
            .map(|_| {
                [
                    c[0] + (r() - 0.5) * span[0],
                    c[1] + (r() - 0.5) * span[1],
                    0.4 + 0.8 * r(),
                ]
            })
            .collect();
        Ok(Huddle {
            flock,
            ice: Backdrop::new(gpu, "huddle-ice", include_str!("ice.wgsl")),
            painter: Painter::new(gpu),
            sound: sound::Sound::default(),
            eye: c,
            flakes,
            touch: None,
            t: 0.0,
            size_pt: pts,
            calls_owed: 0.0,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let q = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                self.touch = Some(Touch {
                    id: e.id,
                    from: q,
                    last: q,
                    tl: self.t,
                    dragging: false,
                });
            }
            Phase::Move => {
                let Some(t) = self.touch.as_mut().filter(|t| t.id == e.id) else {
                    return;
                };
                let d = [q[0] - t.from[0], q[1] - t.from[1]];
                if (d[0] * d[0] + d[1] * d[1]).sqrt() > 14.0 {
                    t.dragging = true;
                }
                if t.dragging {
                    // the wind blows the way the finger goes, as hard as it goes
                    let step = [q[0] - t.last[0], q[1] - t.last[1]];
                    let l = (step[0] * step[0] + step[1] * step[1]).sqrt();
                    if l > 2.0 {
                        let secs = (self.t - t.tl).max(1.0 / 120.0);
                        let speed = (l / secs / 45.0) as f32;
                        let dir = [d[0] as f32, d[1] as f32];
                        let g = self.flock.gust;
                        self.flock.blow(dir, g + (speed - g) * 0.3);
                        t.last = q;
                        t.tl = self.t;
                    }
                }
            }
            _ => {
                if self.touch.as_ref().is_some_and(|t| t.id == e.id) {
                    self.touch_up(ctx, q);
                }
            }
        }
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = dt as f32;
        // in steps no longer than a tenth of a second of the huddle's time
        let sim = dt * PACE;
        let n = (sim / 0.1).ceil().max(1.0) as usize;
        for _ in 0..n {
            self.flock.step(sim / n as f32);
            // a call now and then as one presses back in
            for p in std::mem::take(&mut self.flock.joined) {
                self.calls_owed += 0.35;
                if self.calls_owed >= 1.0 {
                    self.calls_owed -= 1.0;
                    let x = (p[0] - self.eye[0]) as f64 * PT_PER_M / self.size_pt[0] * 2.0;
                    self.sound.call(x as f32, 0.5);
                }
            }
        }
        // the view drifts with the huddle
        let (c, _) = self.flock.middle();
        let k = (dt * 0.25).min(1.0);
        self.eye[0] += (c[0] - self.eye[0]) * k;
        self.eye[1] += (c[1] - self.eye[1]) * k;
        self.blow_flakes(dt);
        self.sound.wind(self.flock.gust);
        // a gust the finger raised dies back to the day's wind; its direction stays
        if self.touch.is_none() {
            self.flock.gust += (WIND - self.flock.gust) * (dt * 0.08).min(1.0);
        }
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let gpu = &ctx.gpu;
        let k = ctx.ppp as f32;
        let size = [ctx.width, ctx.height];
        let (c, r) = self.flock.middle();
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        let w = self.flock.wind;
        self.ice.render(
            gpu,
            &mut enc,
            target,
            size,
            &BackdropParams {
                time: self.t as f32,
                scale: (PT_PER_M * ctx.ppp) as f32,
                center: self.eye,
                p: [w[0], w[1], self.flock.gust, 0.8, c[0], c[1], r, 0.0],
            },
        );
        self.paint_penguins(k);
        self.paint_snow(k);
        self.painter.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.sound.synth())
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let (c, r) = self.flock.middle();
        let w = self.flock.wind;
        vec![
            ("middle".to_string(), self.to_screen(c)),
            (
                "windward".to_string(),
                self.to_screen([c[0] - w[0] * r * 0.8, c[1] - w[1] * r * 0.8]),
            ),
        ]
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Penguins in a blizzard. Drag to turn the wind.",
            vec![info::Stroke::drag(
                0.4,
                1.2,
                [s[0] * 0.15, s[1] * 0.80],
                [s[0] * 0.85, s[1] * 0.70],
            )],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "Emperor penguins on the windward edge get cold; the ones inside stay warm.",
                "A cold one walks round and presses in on the lee side, so the huddle creeps downwind.",
                "Every half minute or so a wave of tiny steps runs through it.",
                "Drag to turn the wind. Touch a penguin to start a wave.",
            ],
            "",
        ))
    }
}

toy_kit::ios_shell!(Huddle);
