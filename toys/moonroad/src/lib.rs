//! Moon Road (a working title), in homage to Paint by Moonlight: Lake Superior from the beach at
//! moonrise, the water a numbered template of facets on Paint by Moonlight's dark paper (`lake.rs`). The moon paints it:
//! every facet that tilts the moon's light to your eye flashes, then keeps a moonlight paint, so the road of
//! light down the water from the moon to you fills in, number by number. A tap is a gust that ruffles
//! the water and spreads the road; a drag walks you along the shore, and the road comes with you,
//! leaving what it painted behind. Shake for a fresh page. painting-company row 333.

pub mod lake;
pub mod sound;

use lake::{Lake, DU, DV, EYE_M, MOON_EL, NU, NV};
use sound::{Chime, Sound};
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::sound::pentatonic;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};

/// Where it is: the beach at Marquette, Michigan, looking out over Lake Superior (lon, lat).
pub const PLACE: (f64, f64) = (-87.38, 46.55);

/// The horizon, a share of the screen's height from the top.
const HORIZON: f64 = 0.41;
/// The lens: points a radian, as a multiple of the screen's width.
const LENS: f64 = 1.95;
/// The moon has been painting this long when the toy opens (seconds), in steps this long (big ones:
/// the slopes' walk is exact at any step, and the opening is to be quick).
const OPENING_S: f64 = 9.0;
const OPENING_STEP_S: f64 = 0.15;
/// A drag walks the water under the finger, taken at least this near and at most this far (metres),
/// so the walk is never a crawl near the shore nor a leap toward the horizon.
const WALK_M: [f64; 2] = [3.0, 30.0];
/// A held finger gusts again this often (seconds).
const HOLD_GUST_S: f64 = 0.55;
/// At most this many notes a second from the painting.
const NOTES_PER_S: f64 = 7.0;

struct Finger {
    id: u64,
    at: [f64; 2],
    last_gust: f64,
}

pub struct MoonRoad {
    lake: Lake,
    field: Field,
    texels: Vec<[f32; 4]>,
    sound: Sound,
    fingers: Vec<Finger>,
    t: f64,
    size_pt: [f64; 2],
    note_budget: f64,
}

impl MoonRoad {
    fn horizon(&self) -> f64 {
        self.size_pt[1] * HORIZON
    }
    fn focal(&self) -> f64 {
        self.size_pt[0] * LENS
    }
    /// Metres out to the water under a point (None above the horizon).
    fn out_at(&self, y: f64) -> Option<f64> {
        let a = (y - self.horizon()) / self.focal();
        (a > 0.002).then(|| EYE_M / a.tan())
    }
    /// The water under a point: (metres along the shore, metres out).
    fn water_at(&self, p: [f64; 2]) -> Option<[f64; 2]> {
        let d = self.out_at(p[1])?;
        Some([
            self.lake.x_eye + (p[0] - self.size_pt[0] * 0.5) / self.focal() * d,
            d,
        ])
    }
    fn near_m(size: [f64; 2]) -> f64 {
        let a = size[1] * (1.0 - HORIZON) / (size[0] * LENS);
        EYE_M / a.tan()
    }
}

impl Toy for MoonRoad {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let mut lake = Lake::new(MoonRoad::near_m(size_pt), 0x0D0D_3E11);
        lake.slow = ctx.reduce_motion;
        for _ in 0..(OPENING_S / OPENING_STEP_S) as usize {
            lake.step(OPENING_STEP_S);
        }
        let field = Field::new(
            &ctx.gpu,
            "moonroad-lake",
            [NU as u32, NV as u32],
            include_str!("lake.wgsl"),
            None,
        );
        Ok(MoonRoad {
            lake,
            field,
            texels: Vec::new(),
            sound: Sound::default(),
            fingers: Vec::new(),
            t: 0.0,
            size_pt,
            note_budget: 0.0,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
    }

    fn settings(&mut self, ctx: &mut Ctx) {
        self.lake.slow = ctx.reduce_motion;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                self.fingers.retain(|f| f.id != e.id);
                self.fingers.push(Finger {
                    id: e.id,
                    at: p,
                    last_gust: self.t,
                });
                if let Some(w) = self.water_at(p) {
                    self.lake.gust(w, 1.0);
                    ctx.haptic();
                }
            }
            Phase::Move => {
                let depth = self
                    .out_at(p[1])
                    .unwrap_or(WALK_M[1])
                    .clamp(WALK_M[0], WALK_M[1]);
                let f = self.focal();
                // only the first finger down walks; the others gust where they are
                let first = self.fingers.first().map(|f| f.id);
                if let Some(fi) = self.fingers.iter_mut().find(|f| f.id == e.id) {
                    if first == Some(e.id) {
                        // the water under the finger stays under it: you walk the other way
                        self.lake.x_eye -= (p[0] - fi.at[0]) / f * depth;
                    }
                    fi.at = p;
                }
            }
            _ => self.fingers.retain(|f| f.id != e.id),
        }
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        self.lake.clear();
        ctx.haptic();
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = dt.clamp(0.0, 0.1);
        // a finger held still keeps gusting
        let mut gusts = Vec::new();
        for f in &mut self.fingers {
            if t - f.last_gust > HOLD_GUST_S {
                f.last_gust = t;
                gusts.push(f.at);
            }
        }
        for p in gusts {
            if let Some(w) = self.water_at(p) {
                self.lake.gust(w, 0.8);
            }
        }
        self.lake.step(dt);
        // the notes of the painting: the nearest new cells, a few a second
        self.note_budget = (self.note_budget + dt * NOTES_PER_S).min(3.0);
        let mut fresh = self.lake.fresh.clone();
        fresh.sort_by(|a, b| a[1].total_cmp(&b[1]));
        let mut chimes = Vec::new();
        for w in fresh.iter().take(3) {
            if self.note_budget < 1.0 {
                break;
            }
            self.note_budget -= 1.0;
            let az = w[0] / w[1];
            let degree = (az * 40.0).round() as i32 + 2 - (w[1].ln() * 0.8) as i32;
            chimes.push(Chime {
                freq: pentatonic(392.0, degree),
                amp: (0.16 * (5.0 / w[1]).min(1.0)).max(0.025) as f32,
                pan: (az * 4.0).clamp(-0.8, 0.8) as f32,
            });
        }
        if !chimes.is_empty() {
            self.sound.send(&chimes);
        }
        let windy = self
            .lake
            .gusts
            .iter()
            .map(|g| g.power * (-(t - g.t).max(0.0) / 3.0).exp())
            .sum::<f64>();
        self.sound.wind(windy.min(1.0) as f32);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let ppp = ctx.ppp;
        self.lake.texels(&mut self.texels);
        self.field.upload(&ctx.gpu, &self.texels);
        let params = BackdropParams {
            time: self.t as f32,
            scale: ppp as f32,
            center: [0.0, 0.0],
            p: [
                (self.horizon() * ppp) as f32,
                (self.focal() * ppp) as f32,
                self.lake.x_eye as f32,
                MOON_EL as f32,
                DU as f32,
                DV as f32,
                self.lake.v0 as f32,
                EYE_M as f32,
            ],
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
        let hy = self.horizon();
        vec![
            ("road".into(), [s[0] * 0.5, hy + (s[1] - hy) * 0.45]),
            ("near".into(), [s[0] * 0.62, hy + (s[1] - hy) * 0.8]),
        ]
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        let hy = self.horizon();
        Some(info::Demo::new(
            "The moon paints the lake. Tap for a gust.",
            vec![info::Stroke::tap(
                0.5,
                [s[0] * 0.55, hy + (s[1] - hy) * 0.55],
            )],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::lines(&[
            "The moon is low over Lake Superior. Only water tilted just right sends its light to your eye, so the road always points at you.",
            "The moon paints each bit it lights. A gust tilts more, and the road spreads.",
            "Drag to walk the shore; the road follows.",
        ]))
    }
}

toy_kit::ios_shell!(MoonRoad);
