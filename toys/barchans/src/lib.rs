//! Barchans (a working title): crescent dunes walking across the Atlantic Sahara near Tarfaya, seen
//! from straight above in a low morning sun. One rule makes them (`dunes.rs`): the wind lifts sand
//! wherever it reaches, drops it a little downwind, and it sticks more readily on sand; the lee of a
//! crest is out of the wind, and too steep a slope slides. A finger dragged turns the wind; held still,
//! it pours sand, a new heap the wind shapes into a dune and sends walking. A big slip face, sliding,
//! sings (`sound.rs`). painting-company row 327.

pub mod dunes;
pub mod sound;

use dunes::{Dunes, SLAB};
use sound::Sound;
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::paint::Painter;
use toy_kit::{info, Ctx, Phase, PointerEvent, Synth, Toy};

/// Where it is: the barchan corridor south of Tarfaya, Morocco (lon, lat).
pub const PLACE: (f64, f64) = (-12.70, 27.62);

/// The field, cells (a torus: a taller screen sees it wrap, seamlessly); its width spans the screen.
pub const FIELD: [usize; 2] = [256, 560];
const ACROSS: usize = FIELD[0];
pub const SEED: u64 = 0xD00E_5EED;
/// The trade wind, straight down the screen: the scene is turned so that down is south-south-west,
/// where the north-north-east trade carries the sand (the map's north is 20° right of up).
pub const TRADE: [f32; 2] = [0.0, 1.0];
/// Picks of the rule a second, at rest and in a gust.
const PICKS_S: f32 = 450_000.0;
const GRAINS: usize = 500;

struct Grain {
    p: [f32; 2],
    age: f32,
    life: f32,
}

struct Finger {
    id: u64,
    at: [f64; 2],
    still_since: f64,
}

pub struct Barchans {
    dunes: Dunes,
    field: Field,
    cells: Vec<[f32; 4]>,
    painter: Painter,
    sound: Sound,
    grains: Vec<Grain>,
    /// where the wind is turning to, and how hard a finger is blowing it (0..1)
    want: [f32; 2],
    gust: f32,
    acc: f32,
    finger: Option<Finger>,
    t: f64,
    size_pt: [f64; 2],
    cell_pt: f64,
}

impl Barchans {
    fn cell(&self, p: [f64; 2]) -> [f32; 2] {
        [(p[0] / self.cell_pt) as f32, (p[1] / self.cell_pt) as f32]
    }

    fn spawn(&mut self) -> Grain {
        let d = &mut self.dunes;
        for _ in 0..12 {
            let i = (d.rng.next_u64() % (d.w * d.h) as u64) as usize;
            if d.slabs[i] > 1 {
                return Grain {
                    p: [(i % d.w) as f32 + d.rng.f(), (i / d.w) as f32 + d.rng.f()],
                    age: 0.0,
                    life: 0.3 + 0.5 * d.rng.f(),
                };
            }
        }
        Grain {
            p: [d.rng.f() * d.w as f32, d.rng.f() * d.h as f32],
            age: 0.0,
            life: 0.2,
        }
    }
}

impl Toy for Barchans {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let cell_pt = size_pt[0] / ACROSS as f64;
        // the field as the dune rule left it after a long run (examples/opening.rs builds it)
        let dunes = Dunes::from_bytes(&ctx.assets.read("opening.bin")?, SEED, TRADE)?;
        let down = dunes.h;
        let field = Field::new(
            &ctx.gpu,
            "barchans",
            [ACROSS as u32, down as u32],
            include_str!("dunes.wgsl"),
            None,
        );
        let mut toy = Barchans {
            cells: vec![[0.0; 4]; ACROSS * down],
            dunes,
            field,
            painter: Painter::new(&ctx.gpu),
            sound: Sound::default(),
            grains: Vec::new(),
            want: TRADE,
            gust: 0.0,
            acc: 0.0,
            finger: None,
            t: 0.0,
            size_pt,
            cell_pt,
        };
        for _ in 0..GRAINS {
            let mut g = toy.spawn();
            g.age = g.life * toy.dunes.rng.f();
            toy.grains.push(g);
        }
        Ok(toy)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
        self.cell_pt = self.size_pt[0] / ACROSS as f64;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                if self.finger.is_none() {
                    self.finger = Some(Finger {
                        id: e.id,
                        at: p,
                        still_since: self.t,
                    });
                }
            }
            Phase::Move => {
                let t = self.t;
                if let Some(f) = self.finger.as_mut().filter(|f| f.id == e.id) {
                    let d = [p[0] - f.at[0], p[1] - f.at[1]];
                    let l = (d[0] * d[0] + d[1] * d[1]).sqrt();
                    if l > 3.0 {
                        // the finger's way is the wind's
                        self.want = [(d[0] / l) as f32, (d[1] / l) as f32];
                        self.gust = (self.gust + (l as f32) * 0.02).min(1.0);
                        f.at = p;
                        f.still_since = t;
                        if self.gust > 0.6 {
                            ctx.haptic();
                        }
                    }
                }
            }
            _ => {
                if self.finger.as_ref().is_some_and(|f| f.id == e.id) {
                    self.finger = None;
                }
            }
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = (dt as f32).min(0.1);
        // the wind turns toward where the finger sent it, over a second or so
        let w = self.dunes.wind;
        let k = (dt * 2.2).min(1.0);
        let mut nw = [
            w[0] + (self.want[0] - w[0]) * k,
            w[1] + (self.want[1] - w[1]) * k,
        ];
        let l = (nw[0] * nw[0] + nw[1] * nw[1]).sqrt().max(1e-4);
        nw = [nw[0] / l, nw[1] / l];
        self.dunes.wind = nw;
        // a finger held still pours sand
        if let Some(f) = &self.finger {
            if t - f.still_since > 0.25 {
                let c = self.cell([f.at[0], f.at[1]]);
                self.dunes.pour(c, 6.0, dt * 30.0);
                if (t * 8.0).fract() < (dt as f64 * 8.0) {
                    ctx.haptic();
                }
            }
        }
        self.gust = (self.gust - dt * 0.7).max(0.0);
        self.acc += dt * PICKS_S * (1.0 + 2.5 * self.gust);
        let n = self.acc as usize;
        self.acc -= n as f32;
        self.dunes.run(n);
        // the grains in saltation
        let speed = (24.0 + 40.0 * self.gust) * dt;
        for i in 0..self.grains.len() {
            let g = &mut self.grains[i];
            g.age += dt;
            g.p[0] += nw[0] * speed;
            g.p[1] += nw[1] * speed;
            if g.age > g.life {
                self.grains[i] = self.spawn();
            }
        }
        let carried = std::mem::take(&mut self.dunes.carried) as f32;
        let sang = std::mem::take(&mut self.dunes.sang) as f32;
        let flux = carried / (dt * PICKS_S).max(1.0);
        let song = (sang / dt / 900.0).min(1.0);
        self.sound
            .set(0.45 + 0.5 * self.gust, (flux * 1.5).min(1.0), song, 100.0);
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let d = &self.dunes;
        for (c, &s) in self.cells.iter_mut().zip(&d.slabs) {
            c[0] = s as f32 * SLAB;
        }
        self.field.upload(&ctx.gpu, &self.cells);
        let ppp = ctx.ppp;
        let s = (self.cell_pt * ppp) as f32;
        let params = BackdropParams {
            time: self.t as f32,
            scale: s,
            center: [
                (self.size_pt[0] / self.cell_pt * 0.5) as f32,
                (self.size_pt[1] / self.cell_pt * 0.5) as f32,
            ],
            p: [d.wind[0], d.wind[1], 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        // the grains: short pale streaks, faint, along the wind
        let len = 0.9 + 1.6 * self.gust;
        for g in &self.grains {
            let a = (g.age / g.life * std::f32::consts::PI).sin() * (0.10 + 0.18 * self.gust);
            let p0 = [
                g.p[0].rem_euclid(d.w as f32) * s,
                g.p[1].rem_euclid(d.h as f32) * s,
            ];
            let p1 = [p0[0] - d.wind[0] * len * s, p0[1] - d.wind[1] * len * s];
            self.painter
                .glow(p0, p1, 0.18 * s, 0.25 * s, [1.0, 0.93, 0.78, a]);
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
            "Walking dunes of the Sahara. Drag to turn the wind.",
            vec![info::Stroke::drag(
                0.4,
                1.2,
                [s[0] * 0.25, s[1] * 0.35],
                [s[0] * 0.75, s[1] * 0.5],
            )],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "Real wind lifts sand wherever it reaches and drops it a little downwind, where it sticks best on sand.",
                "Behind every crest is a lee the wind can't reach, and a slope too steep slides.",
                "So heaps grow into crescents with horns downwind, and the small ones walk faster than the big.",
                "Drag to turn the wind; hold still to pour sand. A big dune's sliding face hums.",
            ],
            "",
        ))
    }
}

toy_kit::ios_shell!(Barchans);
