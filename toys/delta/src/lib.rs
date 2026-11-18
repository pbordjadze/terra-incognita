//! The Delta (a working title): the Okavango Delta, Botswana, from the air in the flood season. Rain that
//! fell on the Angolan highlands months before comes down the Panhandle and fans out across land so flat
//! (about 1 m in 3,400) that a termite mound decides where it goes, and it never reaches the sea: it sinks
//! into the Kalahari sand (`land.rs`). The phone held flat is that land: its tilt (`Ctx::sensors`) leans
//! the fan, and the flood follows. A finger pressed on the land is a low place the water runs to; with no
//! sense of tilt (a desktop browser) it is the only way to steer.

pub mod land;

use land::{Land, RULES, TILT, TILT_MAX};
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::switches::Switch;
use toy_kit::{info, Ctx, Phase, PointerEvent, Toy};

/// Where it is: the Okavango Delta, Botswana (lon, lat).
pub const PLACE: (f64, f64) = (22.9, -19.3);
/// The land, cells: its width spans the screen.
pub const GRID: [usize; 2] = [72, 156];
pub const SEED: u64 = 0x0CA7_A460;
/// Flow substeps a second (the flow's speed and its stability both hang on it).
const RATE: f64 = 1440.0;
/// Seconds of flood run before the first frame, so the toy opens in flood.
const SETTLE: f64 = 2.0;
/// How quickly the land's lean follows the phone's (seconds): a hand trembles.
const EASE: f32 = 0.25;
/// The low sun's way across the screen (from the upper left).
const SUN: [f32; 2] = [-0.7, -0.7];

pub struct Delta {
    land: Land,
    field: Field,
    cells: Vec<[f32; 4]>,
    /// The phone's lean, eased (screen directions, the sine of the tilt).
    lean: [f32; 2],
    fingers: Vec<(u64, [f64; 2])>,
    acc: f64,
    t: f64,
    size_pt: [f64; 2],
    cell_pt: f64,
}

impl Delta {
    /// The phone's lean as the sensors give it, clamped: none without a sense of tilt.
    fn phone_lean(ctx: &Ctx) -> [f32; 2] {
        let Some(d) = ctx.sensors.downhill() else {
            return [0.0, 0.0];
        };
        let (x, y) = (d[0] as f32, d[1] as f32);
        let m = (x * x + y * y).sqrt();
        let k = if m > TILT_MAX { TILT_MAX / m } else { 1.0 };
        [x * k, y * k]
    }

    fn cell(&self, p: [f64; 2]) -> [f32; 2] {
        [(p[0] / self.cell_pt) as f32, (p[1] / self.cell_pt) as f32]
    }

    /// Runs the flood `secs` seconds.
    fn run(&mut self, secs: f64) {
        self.land.tilt = [self.lean[0] * TILT, self.lean[1] * TILT];
        self.land.press = self.fingers.last().map(|f| self.cell(f.1));
        self.acc += secs * RATE;
        let n = self.acc.floor();
        self.acc -= n;
        if n > 0.0 {
            self.land.run(secs as f32, n as usize);
        }
    }
}

impl Toy for Delta {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let cell_pt = size_pt[0] / GRID[0] as f64;
        let field = Field::new(
            &ctx.gpu,
            "delta",
            [GRID[0] as u32, GRID[1] as u32],
            include_str!("land.wgsl"),
            None,
        );
        let mut d = Delta {
            land: Land::new(GRID[0], GRID[1], SEED),
            field,
            cells: vec![[0.0; 4]; GRID[0] * GRID[1]],
            lean: [0.0, 0.0],
            fingers: Vec::new(),
            acc: 0.0,
            t: 0.0,
            size_pt,
            cell_pt,
        };
        // the flood already down the fan, in coarse steps of a second
        for _ in 0..SETTLE as usize {
            d.run(1.0);
        }
        Ok(d)
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
        self.cell_pt = self.size_pt[0] / GRID[0] as f64;
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = [e.x, e.y];
        match e.phase {
            Phase::Down => {
                self.fingers.push((e.id, p));
                ctx.haptic();
            }
            Phase::Move => {
                if let Some(f) = self.fingers.iter_mut().find(|f| f.0 == e.id) {
                    f.1 = p;
                }
            }
            _ => self.fingers.retain(|f| f.0 != e.id),
        }
    }

    fn update(&mut self, ctx: &mut Ctx, t: f64, dt: f64) {
        self.t = t;
        let want = Self::phone_lean(ctx);
        let k = 1.0 - (-(dt as f32) / EASE).exp();
        self.lean = [
            self.lean[0] + (want[0] - self.lean[0]) * k,
            self.lean[1] + (want[1] - self.lean[1]) * k,
        ];
        self.run(dt.min(0.1));
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        let l = &self.land;
        for (i, c) in self.cells.iter_mut().enumerate() {
            let y = (i / l.w) as f32;
            *c = [
                l.water[i],
                l.height(i) + land::FALL * y,
                l.green[i],
                l.mound[i],
            ];
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
            p: [
                SUN[0],
                SUN[1],
                if l.on[1] { 1.0 } else { 0.0 },
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            ],
        };
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.field
            .render(&ctx.gpu, &mut enc, target, size, &params, true);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "The Okavango in flood. Tilt the phone, or press.",
            vec![info::Stroke::hold(0.4, 2.6, [s[0] * 0.8, s[1] * 0.55])],
        ))
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "Rain from Angola reaches Botswana months later and fans out over land that falls 1 m in 3,400.",
                "So flat that islands a metre high, many begun as termite mounds, steer it. It never reaches the sea: it sinks into the sand.",
                "Hold the phone flat and tilt it: the flood follows. Or press where it should go.",
            ],
            "",
        ))
    }

    fn switches(&self) -> Vec<Switch> {
        Switch::all(&RULES, &self.land.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(o) = self.land.on.get_mut(i) {
            *o = on;
        }
    }
}

toy_kit::ios_shell!(Delta);
