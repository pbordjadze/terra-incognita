//! Bubblers (a working title): low tide on a beach at Phuket, seen from straight above, a hand's
//! breadth of sand filling the screen. Sand-bubbler crabs feed out from their burrows in spokes,
//! leaving the sifted sand behind them as little balls, and the beach fills with starbursts nobody
//! drew (`beach.rs`). A tap is a footstep: the near crabs bolt, their neighbours see them go and bolt
//! too, and a while later the eyes come up again. A finger drawn through the sand leaves a furrow they
//! work around. Shaken, or now and then on its own, the tide comes in and takes it all.
//! painting-company row 327.

pub mod beach;
pub mod sound;

use beach::{Beach, State, CELL};
use sound::{Sound, Tick};
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::paint::Painter;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};

/// Where it is: a beach on Phuket's west coast (lon, lat).
pub const PLACE: (f64, f64) = (98.30, 7.90);

/// The screen is this many centimetres across.
const WIDTH_CM: f32 = 30.0;
/// The tide went out this long before the toy opens (seconds): the first crabs are just digging out.
const OPENING_S: f32 = 4.0;
/// The tide comes on its own after this long untouched by one (seconds).
const TIDE_EVERY_S: f32 = 210.0;

pub struct Bubblers {
    beach: Beach,
    field: Field,
    cells: Vec<[f32; 4]>,
    painter: Painter,
    sound: Sound,
    k: f32,
    t: f64,
    last_tide: f32,
    fingers: Vec<(u64, [f32; 2], f64)>,
    size_pt: [f64; 2],
}

impl Bubblers {
    fn cm(&self, p: [f64; 2]) -> [f32; 2] {
        [p[0] as f32 / self.k, p[1] as f32 / self.k]
    }

    fn rustle(&self, at: [f32; 2], amp: f32) -> Tick {
        Tick {
            delay: 0.0,
            amp,
            tau: 0.07,
            bright: 0.55,
            pan: (at[0] / self.beach.w * 2.0 - 1.0) * 0.7,
        }
    }
}

impl Toy for Bubblers {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let k = size_pt[0] as f32 / WIDTH_CM;
        let h = size_pt[1] as f32 / k;
        let mut beach = Beach::new(WIDTH_CM, h, 0x0B0B_B1E5);
        let dt = 0.05;
        for _ in 0..(OPENING_S / dt) as usize {
            beach.step(dt);
        }
        // the clock starts at zero for the toy; the beach keeps its own
        let field = Field::new(
            &ctx.gpu,
            "bubblers-sand",
            [beach.gw as u32, beach.gh as u32],
            include_str!("sand.wgsl"),
            None,
        );
        let n = beach.gw * beach.gh;
        Ok(Bubblers {
            last_tide: beach.t,
            beach,
            field,
            cells: vec![[0.0; 4]; n],
            painter: Painter::new(&ctx.gpu),
            sound: Sound::default(),
            k,
            t: 0.0,
            fingers: Vec::new(),
            size_pt,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        // the beach keeps its size; the screen shows it from the top left at the same scale
        self.size_pt = ctx.points();
        self.k = self.size_pt[0] as f32 / WIDTH_CM;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = self.cm([e.x, e.y]);
        match e.phase {
            Phase::Down => {
                self.fingers.retain(|f| f.0 != e.id);
                self.fingers.push((e.id, p, self.t));
                self.beach.footstep(p, 1.0);
                ctx.haptic();
                self.sound.send(&[self.rustle(p, 0.25)]);
            }
            Phase::Move => {
                let t = self.t;
                let mut step = None;
                if let Some(f) = self.fingers.iter_mut().find(|f| f.0 == e.id) {
                    let last = f.1;
                    self.beach.furrow(last, p);
                    f.1 = p;
                    if t - f.2 > 0.14 {
                        f.2 = t;
                        step = Some(p);
                    }
                }
                if let Some(q) = step {
                    self.beach.footstep(q, 0.6);
                }
            }
            _ => self.fingers.retain(|f| f.0 != e.id),
        }
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        if self.beach.tide.is_none() {
            self.beach.bring_tide();
            self.last_tide = self.beach.t;
            ctx.haptic();
        }
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = (dt as f32).min(0.1);
        self.beach.step(dt);
        if self.beach.t - self.last_tide > TIDE_EVERY_S && self.beach.tide.is_none() {
            self.beach.bring_tide();
            self.last_tide = self.beach.t;
        }
        let mut ticks = Vec::new();
        for (i, p) in self.beach.laid.iter().enumerate().take(6) {
            let h = ((p[0] * 131.0 + p[1] * 71.0).sin() * 0.5 + 0.5).fract();
            ticks.push(Tick {
                delay: i as f32 * 0.011 + h * dt,
                amp: 0.10 + 0.08 * h,
                tau: 0.0035,
                bright: 0.75 + 0.25 * h,
                pan: (p[0] / self.beach.w * 2.0 - 1.0) * 0.8,
            });
        }
        let bolted = self.beach.bolted.len();
        if bolted > 0 {
            let at = self.beach.bolted[0];
            ticks.push(self.rustle(at, (0.06 * bolted as f32).min(0.3)));
        }
        if !ticks.is_empty() {
            self.sound.send(&ticks);
        }
        let wash = self
            .beach
            .front()
            .map(|f| (1.0 - f / self.beach.h).clamp(0.0, 1.0))
            .unwrap_or(0.0);
        self.sound.wash(wash);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let b = &self.beach;
        if b.dirty {
            for i in 0..b.gw * b.gh {
                self.cells[i] = [b.worked[i], b.wet[i], b.furrow[i], 0.0];
            }
            self.field.upload(&ctx.gpu, &self.cells);
            self.beach.dirty = false;
        }
        let b = &self.beach;
        let ppp = ctx.ppp as f32;
        let s = self.k * ppp;
        let px = |p: [f32; 2]| [p[0] * s, p[1] * s];
        let params = BackdropParams {
            time: b.t,
            scale: s,
            center: [
                self.size_pt[0] as f32 / self.k * 0.5,
                self.size_pt[1] as f32 / self.k * 0.5,
            ],
            p: [
                b.front().unwrap_or(-100.0),
                b.h,
                CELL,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        };
        let front = b.front().unwrap_or(f32::MAX);
        // the burrows
        for c in &b.crabs {
            if c.home[1] > front || !c.dug {
                continue;
            }
            // a collar of dug-out sand, lit on its far side; a soft hollow; the dark hole in it
            let z = 0.85 + 0.3 * ((c.home[0] * 7.1 + c.home[1] * 3.3).sin() * 0.5 + 0.5);
            self.painter.disc(
                px([c.home[0] + 0.06, c.home[1] + 0.07]),
                0.48 * z * s,
                0.2 * s,
                [0.98, 0.93, 0.82, 0.35],
            );
            self.painter.disc(
                px([c.home[0] - 0.04, c.home[1] - 0.05]),
                0.42 * z * s,
                0.22 * s,
                [0.45, 0.38, 0.30, 0.35],
            );
            self.painter.disc(
                px([c.home[0] - 0.03, c.home[1] - 0.03]),
                0.2 * z * s,
                0.12 * s,
                [0.10, 0.08, 0.06, 0.92],
            );
        }
        // the balls: a shadow, the ball, its light
        for p in &b.pellets {
            let grow = ((b.t - p.born) / 0.18).clamp(0.0, 1.0);
            let melt = if p.gone >= 0.0 {
                1.0 - ((b.t - p.gone) / 1.5).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let r = p.r * grow * (0.5 + 0.5 * melt);
            if r <= 0.0 {
                continue;
            }
            let a = melt;
            // a ball of the sand itself: dark and wet when it is rolled, paling as it dries
            let dry = ((b.t - p.born) / 50.0).clamp(0.0, 1.0);
            let tone = 0.05 * p.shade;
            let wet = [0.62, 0.55, 0.45];
            let pale = [0.86, 0.80, 0.68];
            let col = |i: usize| wet[i] + (pale[i] - wet[i]) * dry + tone;
            self.painter.disc(
                px([p.p[0] + r * 0.35, p.p[1] + r * 0.45]),
                r * 1.05 * s,
                r * 0.7 * s,
                [0.28, 0.22, 0.16, 0.30 * a],
            );
            self.painter.disc(
                px(p.p),
                r * s,
                r * 0.35 * s,
                [col(0), col(1), col(2), a],
            );
            // the light on its upper left, the grains on it
            self.painter.disc(
                px([p.p[0] - r * 0.3, p.p[1] - r * 0.3]),
                r * 0.55 * s,
                r * 0.5 * s,
                [1.0, 0.96, 0.88, (0.16 + 0.1 * dry) * a],
            );
        }
        // the crabs
        for c in &b.crabs {
            paint_crab(&mut self.painter, c, s, b.t);
        }
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.field
            .render(&ctx.gpu, &mut enc, target, size, &params, true);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.sound.synth())
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Sand-bubbler crabs at low tide. Tap to step.",
            vec![info::Stroke::tap(0.5, [s[0] * 0.5, s[1] * 0.45])],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "Real sand-bubbler crabs feed outward from their burrows, sifting the sand and leaving it behind in little balls.",
                "Each run starts beside the last, on the least-sifted sand, so the balls spread into starbursts.",
                "A crab that sees a neighbour bolt bolts too. Your tap is a footstep; drag to draw in the sand.",
                "Shake, and the tide comes in.",
            ],
            "",
        ))
    }
}

fn paint_crab(p: &mut Painter, c: &beach::Crab, s: f32, t: f32) {
    let px = |q: [f32; 2]| [q[0] * s, q[1] * s];
    let body = 0.48 * c.size;
    let f = c.facing;
    let (fx, fy) = (f.cos(), f.sin());
    let at = |along: f32, across: f32| {
        [
            c.p[0] + fx * along - fy * across,
            c.p[1] + fy * along + fx * across,
        ]
    };
    match c.state {
        State::Hidden(_) => {}
        State::Peek(_) => {
            // two eyes on their stalks at the mouth of the hole
            let wob = (t * 3.0 + c.gait).sin() * 0.03;
            for side in [-1.0f32, 1.0] {
                let e = at(0.05, side * 0.16 + wob);
                p.disc(px(e), 0.07 * s, 0.03 * s, [0.10, 0.09, 0.08, 1.0]);
            }
        }
        _ => {
            // the shadow, the legs, the body, its mottling, the claws and the eyes
            p.disc(
                px([c.p[0] + 0.12, c.p[1] + 0.15]),
                body * 1.25 * s,
                body * 0.5 * s,
                [0.25, 0.2, 0.15, 0.28],
            );
            for side in [-1.0f32, 1.0] {
                for leg in 0..4 {
                    let a = (leg as f32 - 1.5) * 0.42;
                    let swing =
                        (c.gait + leg as f32 * 1.7 + side * 0.8).sin() * 0.18 * c.moving.min(1.0);
                    let root = at(a * body * 0.8, side * body * 0.7);
                    let knee = at(a * body * 1.4 + swing * 0.5, side * body * 1.45);
                    let foot = at(a * body * 1.9 + swing, side * body * 1.95);
                    let col = [0.58, 0.48, 0.36, 1.0];
                    p.line(px(root), px(knee), 0.055 * s, 0.03 * s, col);
                    p.line(px(knee), px(foot), 0.04 * s, 0.03 * s, col);
                }
            }
            p.disc(px(c.p), body * s, 0.06 * s, [0.80, 0.70, 0.55, 1.0]);
            for (i, (u, v)) in [(0.15, 0.2), (-0.2, -0.15), (0.05, -0.28), (-0.25, 0.22)]
                .iter()
                .enumerate()
            {
                let r = body * (0.18 + 0.05 * i as f32);
                p.disc(
                    px(at(u * body, v * body)),
                    r * s,
                    r * 0.6 * s,
                    [0.55, 0.44, 0.33, 0.55],
                );
            }
            p.disc(
                px(at(-0.1 * body, -0.1 * body)),
                body * 0.5 * s,
                body * 0.5 * s,
                [1.0, 0.96, 0.88, 0.25],
            );
            for side in [-1.0f32, 1.0] {
                let claw = at(body * 0.95, side * body * 0.35);
                p.disc(px(claw), body * 0.22 * s, 0.03 * s, [0.86, 0.78, 0.66, 1.0]);
                let eye = at(body * 0.85, side * body * 0.55);
                p.disc(px(eye), 0.065 * s, 0.025 * s, [0.08, 0.07, 0.06, 1.0]);
            }
        }
    }
}

toy_kit::ios_shell!(Bubblers);
