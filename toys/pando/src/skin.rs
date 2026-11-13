//! The leaf's skin under the lens: the backdrop draws its interlocking cells (scene.wgsl); here are
//! the pores between them, the stomata. Each is a slit between two bean-shaped guard cells, the only
//! cells of the skin that still hold green chloroplasts. In light the guard cells swell and bow
//! apart and the pore opens to let the leaf breathe; in shade they slacken and it closes. A finger
//! held over them is shade; the leaf twisting its pale back to the sun dims them a little too.

use crate::wind::hash;
use crate::Cam;
use std::f32::consts::{PI, TAU};
use toy_kit::paint::Painter;

/// How quickly a pore follows its light (s): closing is quicker than opening, as in the real leaf,
/// though both are hurried here from minutes to seconds.
const OPEN_S: f32 = 3.0;
const CLOSE_S: f32 = 1.4;

pub struct Pore {
    /// Where (screen widths), which way it lies (radians), its size (screen widths, half its length).
    pub at: [f32; 2],
    pub turn: f32,
    pub size: f32,
    /// How open, 0 shut to 1 wide.
    pub open: f32,
    pub seed: u32,
}

pub struct Skin {
    pub pores: Vec<Pore>,
    /// The finger's shade: where (screen widths), how deep now (0..1), and whether a finger is down.
    pub shade_at: [f32; 2],
    pub shade: f32,
    pub held: bool,
    /// Water vapour leaving the open pores: where (screen widths), age (s).
    vapour: Vec<([f32; 2], f32, u32)>,
    next_vapour: f32,
    puffs: u32,
}

impl Skin {
    pub fn new(seed: u32, aspect: f32) -> Skin {
        let mut pores = Vec::new();
        let mut k = seed;
        let mut rnd = || {
            k = k.wrapping_add(0x9e37_79b9);
            hash(k)
        };
        let step = 0.27;
        // beyond the picture too, so the skin still reaches its edges as it shrinks away on the way out
        let mut y = -0.7 * aspect;
        let mut row = 0;
        while y < 1.7 * aspect {
            let mut x = if row % 2 == 0 { -0.75 } else { -0.62 };
            while x < 1.8 {
                pores.push(Pore {
                    at: [x + (rnd() - 0.5) * 0.12, y + (rnd() - 0.5) * 0.12],
                    turn: (rnd() - 0.5) * PI,
                    size: 0.055 + 0.015 * rnd(),
                    open: 0.9,
                    seed: (rnd() * 1.0e9) as u32,
                });
                x += step;
            }
            y += step * 0.8;
            row += 1;
        }
        Skin {
            pores,
            shade_at: [0.5, 0.5 * aspect],
            shade: 0.0,
            held: false,
            vapour: Vec::new(),
            next_vapour: 0.0,
            puffs: 0,
        }
    }

    /// The light at a point: the sun, less where the finger shades it, less with the leaf's back to
    /// the sun (`facing`, 0..1).
    pub fn light(&self, p: [f32; 2], facing: f32) -> f32 {
        let (dx, dy) = (p[0] - self.shade_at[0], p[1] - self.shade_at[1]);
        let shade = self.shade * (-(dx * dx + dy * dy) / (0.33f32 * 0.33)).exp();
        (0.55 + 0.45 * facing) * (1.0 - 0.95 * shade)
    }

    pub fn step(&mut self, facing: f32, dt: f32) {
        let target = if self.held { 1.0 } else { 0.0 };
        self.shade += (target - self.shade) * (1.0 - (-dt / 0.25).exp());
        for i in 0..self.pores.len() {
            let want = ((self.light(self.pores[i].at, facing) - 0.25) / 0.6).clamp(0.0, 1.0);
            let p = &mut self.pores[i];
            let tau = if want > p.open { OPEN_S } else { CLOSE_S };
            p.open += (want - p.open) * (1.0 - (-dt / tau).exp());
        }
        for v in &mut self.vapour {
            v.1 += dt;
        }
        self.vapour.retain(|v| v.1 < 2.5);
        // the open pores breathe out: a faint wisp now and then from one
        self.next_vapour -= dt;
        if self.next_vapour <= 0.0 && !self.pores.is_empty() {
            let n = self.pores.len();
            self.puffs = self.puffs.wrapping_add(1);
            let s = self.puffs.wrapping_mul(2_654_435_761);
            let i = (hash(s) * n as f32) as usize % n;
            if self.pores[i].open > 0.5 {
                self.vapour.push((self.pores[i].at, 0.0, s));
            }
            self.next_vapour = 0.12;
        }
    }

    pub fn draw(&self, p: &mut Painter, cam: &Cam, w: f32, fade: f32) {
        let px = |q: [f32; 2]| cam.pt([q[0] * w, q[1] * w]);
        let len = |r: f32| cam.len(r * w);
        for (at, age, s) in &self.vapour {
            let u = age / 2.5;
            let drift = [at[0] + 0.05 * u * (hash(*s) - 0.5), at[1] - 0.04 * u];
            let a = 0.10 * (1.0 - u) * (u * 5.0).min(1.0);
            p.glow(
                px(drift),
                px(drift),
                len(0.02 + 0.05 * u),
                len(0.04),
                [1.0, 1.0, 0.95, a * fade],
            );
        }
        for pore in &self.pores {
            draw_pore(p, pore, &px, &len, fade);
        }
    }
}

fn draw_pore(
    p: &mut Painter,
    pore: &Pore,
    px: &dyn Fn([f32; 2]) -> [f32; 2],
    len: &dyn Fn(f32) -> f32,
    fade: f32,
) {
    let (c, s) = (pore.turn.cos(), pore.turn.sin());
    let at = |along: f32, across: f32| {
        px([
            pore.at[0] + along * c - across * s,
            pore.at[1] + along * s + across * c,
        ])
    };
    let l = pore.size;
    let gap = l * (0.03 + 0.22 * pore.open);
    let thick = l * 0.26;
    // a faint pale rim of the neighbouring cells pressed round the pair
    p.disc(
        at(0.0, 0.0),
        len(l * 1.18),
        len(l * 0.2),
        [1.0, 0.93, 0.70, 0.35 * fade],
    );
    // the pore, dark, between them
    let n = 10;
    for i in 0..n {
        let u0 = -1.0 + 2.0 * i as f32 / n as f32;
        let u1 = u0 + 2.0 / n as f32;
        let o0 = gap * (1.0 - u0 * u0).max(0.0).sqrt();
        let o1 = gap * (1.0 - u1 * u1).max(0.0).sqrt();
        let m = 0.5 * (o0 + o1);
        p.line(
            at(u0 * l * 0.86, 0.0),
            at(u1 * l * 0.86, 0.0),
            len(m.max(l * 0.015)),
            len(l * 0.02),
            [0.10, 0.07, 0.04, fade],
        );
    }
    // the two guard cells: bowed apart as the pore opens
    for side in [-1.0f32, 1.0] {
        let mut last = None;
        for i in 0..=12 {
            let u = -1.0 + 2.0 * i as f32 / 12.0;
            let bow = (1.0 - u * u).max(0.0).sqrt();
            let q = at(u * l, side * (gap + thick * 0.95) * bow.max(0.25));
            let r = thick * (0.55 + 0.45 * bow);
            if let Some(a) = last {
                p.line(a, q, len(r), len(l * 0.05), [0.52, 0.62, 0.22, fade]);
            }
            last = Some(q);
        }
        // their chloroplasts
        for j in 0..7 {
            let hs = pore
                .seed
                .wrapping_add(j * 31 + if side > 0.0 { 7 } else { 0 });
            let u = (hash(hs) - 0.5) * 1.5;
            let bow = (1.0 - u * u).max(0.0).sqrt();
            let across = side * (gap + thick * (0.5 + 0.8 * hash(hs ^ 9)) * bow.max(0.25));
            let wob = 0.02 * l * (hash(hs ^ 3) * TAU).sin();
            p.disc(
                at(u * l, across + wob),
                len(l * 0.075),
                len(l * 0.03),
                [0.22, 0.42, 0.12, 0.9 * fade],
            );
        }
    }
}
