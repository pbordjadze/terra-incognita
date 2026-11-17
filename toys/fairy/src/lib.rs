//! Fairy Circles (a working title): the bare discs in the grassland of the NamibRand Nature Reserve,
//! Namibia, from a drone in a low morning sun. One rule makes them (`veld.rs`): grasses compete for
//! scarce rain, and their roots draw water from the ground nearby, so a uniform sward breaks into a
//! lattice of bare discs, each ringed by the grass that drinks from it. A finger pressed is a shower: the
//! grass there greens and the circles close. A shake is a drought: the circles widen and the grass breaks
//! into spots. painting-company row 327.

pub mod veld;

use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::paint::Painter;
use toy_kit::switches::Switch;
use toy_kit::{info, Ctx, Phase, PointerEvent, Toy};
use veld::{sward, Veld, RAIN_SCARCE, RULES};

/// Where it is: the fairy-circle grassland of the NamibRand Nature Reserve, Namibia (lon, lat).
pub const PLACE: (f64, f64) = (16.0, -25.0);

/// The field, cells (a torus: a taller screen sees it wrap, seamlessly); its width spans the screen.
/// 8 discs across and 20 rows of them down, so the opening lattice closes round the torus.
pub const FIELD: [usize; 2] = [96, 208];
pub const SEED: u64 = 0xFA1C_1C1E;
/// The model's time a second (each step of `veld::DT`).
const PACE: f32 = 6.0;
/// Steps run before the first frame, so the seeded lattice has settled into the rule's own.
const SETTLE: usize = 240;
/// The low sun's way across the screen (from the upper left).
const SUN: [f32; 2] = [-0.75, -0.66];

struct Drop {
    p: [f32; 2],
    age: f32,
}

pub struct FairyCircles {
    veld: Veld,
    field: Field,
    cells: Vec<[f32; 4]>,
    painter: Painter,
    fingers: Vec<(u64, [f64; 2])>,
    drops: Vec<Drop>,
    acc: f32,
    t: f64,
    buzz: f64,
    size_pt: [f64; 2],
    cell_pt: f64,
}

impl FairyCircles {
    fn cell(&self, p: [f64; 2]) -> [f32; 2] {
        [(p[0] / self.cell_pt) as f32, (p[1] / self.cell_pt) as f32]
    }

    /// The finger's shower: rain on the ground under it, and drops landing round it.
    fn rain(&mut self, ctx: &mut Ctx, dt: f32) {
        for k in 0..self.fingers.len() {
            let at = self.fingers[k].1;
            let c = self.cell(at);
            self.veld.shower_at(c, 6.0, dt * 2.2);
            for _ in 0..3 {
                let r = 7.0 * self.veld.rng.f().sqrt();
                let a = std::f32::consts::TAU * self.veld.rng.f();
                self.drops.push(Drop {
                    p: [c[0] + r * a.cos(), c[1] + r * a.sin()],
                    age: 0.0,
                });
            }
        }
        if !self.fingers.is_empty() && self.t - self.buzz > 0.35 {
            self.buzz = self.t;
            ctx.haptic();
        }
        for d in &mut self.drops {
            d.age += dt;
        }
        self.drops.retain(|d| d.age < 0.5);
    }
}

impl Toy for FairyCircles {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let cell_pt = size_pt[0] / FIELD[0] as f64;
        let mut veld = Veld::new(FIELD[0], FIELD[1], SEED);
        veld.run(SETTLE);
        let field = Field::new(
            &ctx.gpu,
            "fairy",
            [FIELD[0] as u32, FIELD[1] as u32],
            include_str!("veld.wgsl"),
            None,
        );
        Ok(FairyCircles {
            cells: vec![[0.0; 4]; FIELD[0] * FIELD[1]],
            veld,
            field,
            painter: Painter::new(&ctx.gpu),
            fingers: Vec::new(),
            drops: Vec::new(),
            acc: 0.0,
            t: 0.0,
            buzz: 0.0,
            size_pt,
            cell_pt,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
        self.cell_pt = self.size_pt[0] / FIELD[0] as f64;
    }

    fn pointer(&mut self, _ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => self.fingers.push((e.id, p)),
            Phase::Move => {
                if let Some(f) = self.fingers.iter_mut().find(|f| f.0 == e.id) {
                    f.1 = p;
                }
            }
            _ => self.fingers.retain(|f| f.0 != e.id),
        }
    }

    fn shaken(&mut self, ctx: &mut Ctx) {
        // a dry year: the rain fails for a while
        self.veld.drought = 1.0;
        ctx.haptic();
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let dt = (dt as f32).min(0.1);
        self.rain(ctx, dt);
        self.veld.drought = (self.veld.drought - dt / 14.0).max(0.0);
        self.acc += dt * PACE / veld::DT;
        let n = self.acc as usize;
        self.acc -= n as f32;
        self.veld.run(n.min(10));
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let v = &self.veld;
        let bs = sward(RAIN_SCARCE);
        for (i, c) in self.cells.iter_mut().enumerate() {
            *c = [v.b[i] / bs, v.shower[i], v.water[i], 0.0];
        }
        self.field.upload(&ctx.gpu, &self.cells);
        let s = (self.cell_pt * ctx.ppp) as f32;
        let params = BackdropParams {
            time: self.t as f32,
            scale: s,
            center: [
                (self.size_pt[0] / self.cell_pt * 0.5) as f32,
                (self.size_pt[1] / self.cell_pt * 0.5) as f32,
            ],
            p: [SUN[0], SUN[1], 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
        };
        // the shower's drops: small rings spreading on the ground where they land
        for d in &self.drops {
            let u = d.age / 0.5;
            let p = [d.p[0] * s, d.p[1] * s];
            self.painter.ring(
                p,
                (0.3 + 1.2 * u) * s,
                0.12 * s,
                0.1 * s,
                [0.85, 0.9, 1.0, 0.45 * (1.0 - u)],
            );
        }
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.field
            .render(&ctx.gpu, &mut enc, target, size, &params, true);
        self.painter.render(&ctx.gpu, &mut enc, target, size, None);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Fairy circles in Namib grass. Press to rain.",
            vec![info::Stroke::hold(0.4, 1.6, [s[0] * 0.5, s[1] * 0.45])],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "In Namibia's NamibRand, millions of bare circles lie in the grass, spaced like a honeycomb.",
                "Rain is scarce, and grass roots draw water from the ground nearby, leaving bare discs ringed by taller grass.",
                "Press to rain: the circles close. Shake for a drought.",
            ],
            "",
        ))
    }

    fn switches(&self) -> Vec<Switch> {
        Switch::all(&RULES, &self.veld.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(o) = self.veld.on.get_mut(i) {
            *o = on;
        }
    }
}

toy_kit::ios_shell!(FairyCircles);
