//! Mosquito Bay (a working title), the Paint by Moonlight toy: Puerto Mosquito on Vieques, Puerto Rico, at
//! night, seen over a kayak's side, the water in Paint by Moonlight's night paper. The bay is thick with
//! dinoflagellates that flash blue when the water around them is stirred (`bay.rs`), so a finger drawn
//! through it paints with light, fish streak where they swim and dart from the finger, and a shake brings a
//! shower whose every drop sparks. Each flash spends a cell's light, which comes back slowly; and the moon,
//! when it is up, drowns the light out (its glints in the app's moonlight paints). painting-company row 359.

pub mod bay;
pub mod sound;

use bay::{Bay, CELL, RULES};
use sound::{Sound, Splash};
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::switches::Switch;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};

/// Where it is: Puerto Mosquito, on the south coast of Vieques, Puerto Rico (lon, lat).
pub const PLACE: (f64, f64) = (-65.443, 18.098);

/// The screen is this many centimetres of water across.
const WIDTH_CM: f32 = 60.0;
/// The moon rises or sets on the water in about this long (seconds, a time constant).
const MOON_S: f32 = 0.7;

struct Finger {
    id: u64,
    at: [f32; 2],
    t: f64,
}

pub struct MosquitoBay {
    bay: Bay,
    field: Field,
    texels: Vec<[f32; 4]>,
    sound: Sound,
    fingers: Vec<Finger>,
    /// points a centimetre
    k: f32,
    t: f64,
    moon: f32,
    size_pt: [f64; 2],
}

impl MosquitoBay {
    fn cm(&self, p: [f64; 2]) -> [f32; 2] {
        [p[0] as f32 / self.k, p[1] as f32 / self.k]
    }
    fn pan(&self, at: [f32; 2]) -> f32 {
        (at[0] / self.bay.w * 2.0 - 1.0).clamp(-1.0, 1.0) * 0.8
    }
    fn splash(&self, at: [f32; 2], amp: f32, tau: f32, bright: f32, delay: f32) -> Splash {
        Splash {
            delay,
            amp,
            tau,
            bright,
            pan: self.pan(at),
        }
    }
}

/// The paddle's last stroke, still glowing as the toy opens: a sweep that curls back on itself (cm, on a
/// screen `w` by `h`).
fn paddle_stroke(w: f32, h: f32) -> Vec<[f32; 2]> {
    (0..=40)
        .map(|i| {
            let u = i as f32 / 40.0;
            let a = 3.6 - u * 5.6;
            let r = w * (0.36 - 0.22 * u);
            [w * 0.5 + a.cos() * r, h * 0.66 + a.sin() * r * 0.8]
        })
        .collect()
}

impl Toy for MosquitoBay {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let k = size_pt[0] as f32 / WIDTH_CM;
        let h = size_pt[1] as f32 / k;
        let mut bay = Bay::new(WIDTH_CM, h, 0x0B10_BA7);
        // the fish have been about a while, and the paddle has just come out of the water
        for _ in 0..30 {
            bay.step(1.0 / 15.0);
        }
        let stroke = paddle_stroke(WIDTH_CM, h);
        for pair in stroke.windows(2) {
            bay.drag(pair[0], pair[1], 0.02);
            bay.step(0.02);
        }
        for _ in 0..8 {
            bay.step(1.0 / 30.0);
        }
        let field = Field::new(
            &ctx.gpu,
            "biobay-water",
            [bay.gw as u32, bay.gh as u32],
            include_str!("bay.wgsl"),
            None,
        );
        Ok(MosquitoBay {
            bay,
            field,
            texels: Vec::new(),
            sound: Sound::default(),
            fingers: Vec::new(),
            k,
            t: 0.0,
            moon: 0.0,
            size_pt,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        // the water keeps its size; the screen shows it from the top left at the same scale
        self.size_pt = ctx.points();
        self.k = self.size_pt[0] as f32 / WIDTH_CM;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = self.cm([e.x, e.y]);
        match e.phase {
            Phase::Down => {
                self.fingers.retain(|f| f.id != e.id);
                self.fingers.push(Finger {
                    id: e.id,
                    at: p,
                    t: self.t,
                });
                self.bay.touch(p);
                ctx.haptic();
                self.sound.send(&[self.splash(p, 0.22, 0.06, 0.45, 0.0)]);
            }
            Phase::Move => {
                let t = self.t;
                if let Some(f) = self.fingers.iter_mut().find(|f| f.id == e.id) {
                    let (a, dt) = (f.at, (t - f.t).max(1.0 / 120.0) as f32);
                    f.at = p;
                    f.t = t;
                    self.bay.drag(a, p, dt);
                }
            }
            _ => self.fingers.retain(|f| f.id != e.id),
        }
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        self.bay.rain();
        ctx.haptic();
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        let dt = (dt as f32).clamp(0.0, 0.1);
        self.t = t;
        self.bay.step(dt);
        let up = if self.bay.on[0] { 0.0 } else { 1.0 };
        self.moon += (up - self.moon) * (1.0 - (-dt / MOON_S).exp());
        let mut splashes = Vec::new();
        for (i, p) in self.bay.drops.iter().enumerate().take(6) {
            let h = ((p[0] * 13.1 + p[1] * 7.7).sin() * 0.5 + 0.5).fract();
            splashes.push(self.splash(*p, 0.05 + 0.06 * h, 0.012, 0.8, i as f32 * 0.009));
        }
        for p in self.bay.darted.drain(..).collect::<Vec<_>>() {
            splashes.push(self.splash(p, 0.16, 0.05, 0.6, 0.0));
        }
        if !splashes.is_empty() {
            self.sound.send(&splashes);
        }
        // the swish: as loud as the fingers are stirring
        let stirring: f32 = self
            .fingers
            .iter()
            .map(|f| if t - f.t < 0.08 { 1.0 } else { 0.0 })
            .sum();
        let at = self
            .fingers
            .first()
            .map(|f| f.at)
            .unwrap_or([self.bay.w * 0.5, 0.0]);
        self.sound.swish(stirring.min(1.0) * 0.5, self.pan(at));
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        self.bay.texels(&mut self.texels);
        self.field.upload(&ctx.gpu, &self.texels);
        let s = self.k * ctx.ppp as f32;
        let params = BackdropParams {
            time: self.t as f32,
            scale: s,
            center: [
                self.size_pt[0] as f32 / self.k * 0.5,
                self.size_pt[1] as f32 / self.k * 0.5,
            ],
            p: [self.moon, CELL, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.field
            .render(&ctx.gpu, &mut enc, target, size, &params, true);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn synth(&mut self) -> Option<Box<dyn Synth>> {
        Some(self.sound.synth())
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let s = self.size_pt;
        vec![
            ("left".into(), [s[0] * 0.15, s[1] * 0.55]),
            ("right".into(), [s[0] * 0.85, s[1] * 0.55]),
            ("middle".into(), [s[0] * 0.5, s[1] * 0.55]),
        ]
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Plankton that glow when stirred. Drag to stir.",
            vec![info::Stroke::drag(
                0.4,
                1.2,
                [s[0] * 0.2, s[1] * 0.5],
                [s[0] * 0.8, s[1] * 0.6],
            )],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::lines(&[
            "Mosquito Bay in Puerto Rico is full of tiny plankton that flash blue whenever the water around them is stirred.",
            "Each flash spends some of their light, which comes back slowly. A bright moon drowns the glow out.",
            "Drag to stir. Fish dart from your finger. Shake for rain.",
        ]))
    }

    fn switches(&self) -> Vec<Switch> {
        Switch::all(&RULES, &self.bay.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(r) = self.bay.on.get_mut(i) {
            *r = on;
        }
    }
}

toy_kit::ios_shell!(MosquitoBay);
