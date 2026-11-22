//! Shimmering (a working title): a colony of giant honeybees (Apis dorsata) on its single open comb, hung
//! under a cliff's overhang in Chitwan, Nepal, seen face on in the evening. When a hornet hovers near, a few
//! bees under it flip their abdomens up, their neighbours follow, and a wave ripples across the curtain of
//! bees in a fraction of a second; the waves drive the hornet off (`comb.rs`, Kastberger et al. 2008,
//! SOURCES.md). The finger is the hornet; left alone, a hornet comes on its own.

pub mod comb;

use comb::{Comb, CELL, RULES};
use toy_kit::backdrop::BackdropParams;
use toy_kit::field::Field;
use toy_kit::switches::Switch;
use toy_kit::{info, Ctx, Phase, PointerEvent, Toy};

/// Where it is: a cliff by the Rapti river at Chitwan, Nepal (lon, lat).
pub const PLACE: (f64, f64) = (84.43, 27.58);

/// The screen's shorter side is this many centimetres of comb.
const WIDTH_CM: f32 = 46.0;

pub struct Shimmering {
    comb: Comb,
    field: Field,
    texels: Vec<[f32; 4]>,
    finger: Option<u64>,
    /// points a centimetre
    k: f32,
    t: f64,
    heading: f32,
    size_pt: [f64; 2],
}

fn scale(size: [f64; 2]) -> f32 {
    (size[0].min(size[1]) as f32 / WIDTH_CM).max(1e-3)
}

fn field(ctx: &Ctx, comb: &Comb) -> Field {
    Field::new(
        &ctx.gpu,
        "shimmering-comb",
        [comb.gw as u32, comb.gh as u32],
        include_str!("comb.wgsl"),
        None,
    )
}

impl Shimmering {
    fn cm(&self, p: [f64; 2]) -> [f32; 2] {
        [p[0] as f32 / self.k, p[1] as f32 / self.k]
    }
}

impl Toy for Shimmering {
    fn new(ctx: &mut Ctx) -> anyhow::Result<Self> {
        let size_pt = ctx.points();
        let k = scale(size_pt);
        let mut comb = Comb::new(size_pt[0] as f32 / k, size_pt[1] as f32 / k, 0x0D05_A7A0);
        // the hornet is already on its way in as the toy opens
        for _ in 0..60 {
            comb.step(1.0 / 60.0);
        }
        let field = field(ctx, &comb);
        Ok(Shimmering {
            comb,
            field,
            texels: Vec::new(),
            finger: None,
            k,
            t: 0.0,
            heading: 0.0,
            size_pt,
        })
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        self.size_pt = ctx.points();
        self.k = scale(self.size_pt);
        let (w, h) = (
            self.size_pt[0] as f32 / self.k,
            self.size_pt[1] as f32 / self.k,
        );
        if (w - self.comb.w).abs() > CELL || (h - self.comb.h).abs() > CELL {
            let on = self.comb.on;
            self.comb = Comb::new(w, h, 0x0D05_A7A0);
            self.comb.on = on;
            self.field = field(ctx, &self.comb);
        }
    }

    fn pointer(&mut self, ctx: &mut Ctx, e: PointerEvent) {
        let p = self.cm([e.x, e.y]);
        match e.phase {
            Phase::Down => {
                self.finger = Some(e.id);
                self.comb.hornet.finger = Some(p);
                ctx.haptic();
            }
            Phase::Move if self.finger == Some(e.id) => self.comb.hornet.finger = Some(p),
            Phase::Move => {}
            _ => {
                if self.finger == Some(e.id) {
                    self.finger = None;
                    self.comb.hornet.finger = None;
                }
            }
        }
    }

    fn update(&mut self, _ctx: &mut Ctx, t: f64, dt: f64) {
        let dt = (dt as f32).clamp(0.0, 0.1);
        self.t = t;
        self.comb.step(dt);
        // the hornet faces the comb, leaning into its flight
        let v = self.comb.hornet.v;
        let lean = (v[0] * 0.02).clamp(-0.7, 0.7);
        self.heading += (lean - self.heading) * (1.0 - (-dt / 0.15).exp());
    }

    fn draw(&mut self, ctx: &mut Ctx, target: &wgpu::TextureView) {
        self.comb.texels(&mut self.texels);
        self.field.upload(&ctx.gpu, &self.texels);
        let (w, h) = (self.comb.w, self.comb.h);
        let (cx, top, a, b) = comb::shape(w, h);
        let hp = self.comb.hornet.p;
        let params = BackdropParams {
            time: self.t as f32,
            scale: self.k * ctx.ppp as f32,
            center: [
                self.size_pt[0] as f32 / self.k * 0.5,
                self.size_pt[1] as f32 / self.k * 0.5,
            ],
            p: [CELL, hp[0], hp[1], self.heading, cx, top, a, b],
        };
        let size = [ctx.width, ctx.height];
        let mut enc = ctx.gpu.device.create_command_encoder(&Default::default());
        self.field
            .render(&ctx.gpu, &mut enc, target, size, &params, true);
        ctx.gpu.queue.submit([enc.finish()]);
    }

    fn anchors(&self) -> Vec<(String, [f64; 2])> {
        let s = self.size_pt;
        vec![
            ("left".into(), [s[0] * 0.25, s[1] * 0.4]),
            ("right".into(), [s[0] * 0.75, s[1] * 0.4]),
            ("middle".into(), [s[0] * 0.5, s[1] * 0.45]),
        ]
    }

    fn demo(&self, _ctx: &Ctx) -> Option<info::Demo> {
        let s = self.size_pt;
        Some(info::Demo::new(
            "Giant honeybees shimmer at a hornet. Drag near them.",
            vec![info::Stroke::drag(
                0.4,
                1.6,
                [s[0] * 0.3, s[1] * 0.35],
                [s[0] * 0.65, s[1] * 0.5],
            )],
        ))
    }

    fn style(&self) -> Option<info::Style> {
        Some(info::Style {
            name: "Calculated Movements",
            maker: "Larry Cuba",
            place: "Santa Cruz, California",
            year: 1985,
        })
    }

    fn rules(&self) -> Option<info::Rules> {
        Some(info::Rules::new(
            &[
                "Giant honeybees hang in open curtains under cliffs. When a hornet hovers close, a few bees flip their abdomens up.",
                "Each bee flips when its neighbour does, then rests a moment, so a wave races across the comb.",
                "Your finger is the hornet. The waves drive it off.",
            ],
            "",
        ))
    }

    fn switches(&self) -> Vec<Switch> {
        Switch::all(&RULES, &self.comb.on)
    }

    fn set_switch(&mut self, _ctx: &mut Ctx, i: usize, on: bool) {
        if let Some(r) = self.comb.on.get_mut(i) {
            *r = on;
        }
    }
}

toy_kit::ios_shell!(Shimmering);
