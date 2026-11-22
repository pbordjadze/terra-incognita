//! The crowd on the promenade, come to watch the tide (at Yanguan they come in their tens of thousands
//! at the autumn springs): people along the rail, thinning back from it and away down the wall, each a
//! few strokes of ink. Where the bore bursts on the wall beside them, the nearest step back from the
//! spray, and drift back to the rail once it has fallen.

use crate::view::{Camera, WALL_TOP};
use toy_kit::paint::Painter;

pub struct Person {
    /// where they like to stand, and where they are (m)
    home: [f64; 2],
    at: [f64; 2],
    tall: f64,
    umbrella: bool,
    /// when they last stepped back from the spray (s)
    fled: f64,
    sway: f64,
}

pub struct Crowd {
    pub people: Vec<Person>,
}

struct Rng(u64);
impl Rng {
    fn f(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

impl Crowd {
    pub fn new() -> Crowd {
        let mut r = Rng(0xB0E5_C0DE);
        let mut people = Vec::new();
        for _ in 0..420 {
            // along the wall, most near; packed at the rail, thinning back from it
            let z = 40.0 + 700.0 * r.f().powf(1.8);
            let x = -0.7 - 5.0 * r.f().powf(1.6);
            people.push(Person {
                home: [x, z],
                at: [x, z],
                tall: 1.55 + 0.25 * r.f(),
                umbrella: r.f() < 0.14,
                fled: -100.0,
                sway: r.f() * std::f64::consts::TAU,
            });
        }
        Crowd { people }
    }

    /// The spray bursting at `z` along the wall this strong (0..1): those near it step back.
    pub fn update(&mut self, t: f64, dt: f64, burst: Option<(f64, f64)>) {
        for p in &mut self.people {
            if let Some((z, k)) = burst {
                if k > 0.25 && (p.home[1] - z).abs() < 6.0 + 30.0 * k && p.home[0] > -6.0 {
                    p.fled = t;
                }
            }
            let fleeing = t - p.fled < 4.0;
            let want = if fleeing {
                [
                    p.home[0] - 4.0 - 3.0 * (p.sway * 0.3).sin().abs(),
                    p.home[1] + 1.5 * p.sway.cos(),
                ]
            } else {
                p.home
            };
            let v = if fleeing { 3.2 } else { 0.9 };
            for i in 0..2 {
                let d = want[i] - p.at[i];
                p.at[i] += d.clamp(-v * dt, v * dt);
            }
        }
    }

    /// Each person as strokes, painted from the farthest in; `wet` darkens nothing here (it's the
    /// promenade's), `fade` takes them back to the paper.
    pub fn draw(
        &self,
        cam: &Camera,
        size: [f64; 2],
        ppp: f64,
        t: f64,
        fade: f32,
        out: &mut Vec<Stroke>,
    ) {
        for p in &self.people {
            let feet = [p.at[0], WALL_TOP, p.at[1]];
            let Some((a, zc)) = cam.project(feet, size) else {
                continue;
            };
            let k = cam.scale(zc, size) * ppp;
            if k < 0.6 || a[0] < -40.0 || a[0] > size[0] + 40.0 {
                continue;
            }
            let moving = (p.at[0] - p.home[0]).abs() > 0.3;
            let lean = if moving {
                0.18
            } else {
                0.03 * (t * 0.7 + p.sway).sin()
            };
            let a = [a[0] * ppp, a[1] * ppp];
            let h = p.tall * k;
            let ink = 0.92 * (1.0 - fade) * (zc as f32 / 900.0).min(1.0).mul_add(-0.45, 1.0);
            let col = [0.075, 0.075, 0.085, ink];
            let hip = [a[0] + lean * h * 0.3, a[1] - h * 0.47];
            let neck = [a[0] + lean * h, a[1] - h * 0.84];
            let stride = if moving {
                (t * 9.0 + p.sway).sin() * 0.12 * h
            } else {
                0.0
            };
            let w = (0.075 * h).max(0.5);
            out.push(Stroke::line(
                zc,
                [a[0] - 0.05 * h + stride, a[1]],
                hip,
                w * 0.7,
                col,
            ));
            out.push(Stroke::line(
                zc,
                [a[0] + 0.05 * h - stride, a[1]],
                hip,
                w * 0.7,
                col,
            ));
            out.push(Stroke::line(zc, hip, neck, w * 1.7, col));
            out.push(Stroke::disc(
                zc,
                [neck[0], neck[1] - 0.09 * h],
                0.075 * h,
                col,
            ));
            if p.umbrella {
                let top = [neck[0] + 0.05 * h, neck[1] - 0.38 * h];
                out.push(Stroke::line(
                    zc,
                    [top[0] - 0.42 * h, top[1] + 0.1 * h],
                    top,
                    w * 0.9,
                    col,
                ));
                out.push(Stroke::line(
                    zc,
                    top,
                    [top[0] + 0.42 * h, top[1] + 0.1 * h],
                    w * 0.9,
                    col,
                ));
                out.push(Stroke::line(
                    zc,
                    top,
                    [top[0], top[1] + 0.3 * h],
                    w * 0.3,
                    col,
                ));
            }
        }
    }
}

impl Default for Crowd {
    fn default() -> Self {
        Self::new()
    }
}

/// One thing to paint over the scene, with how far it is, so the near ones go over the far.
pub struct Stroke {
    pub depth: f64,
    a: [f32; 2],
    b: [f32; 2],
    r: f32,
    soft: f32,
    col: [f32; 4],
}

impl Stroke {
    pub fn line(depth: f64, a: [f64; 2], b: [f64; 2], r: f64, col: [f32; 4]) -> Stroke {
        Stroke {
            depth,
            a: [a[0] as f32, a[1] as f32],
            b: [b[0] as f32, b[1] as f32],
            r: r as f32,
            soft: (r as f32 * 0.25).clamp(0.4, 1.2),
            col,
        }
    }
    pub fn disc(depth: f64, c: [f64; 2], r: f64, col: [f32; 4]) -> Stroke {
        Stroke::line(depth, c, c, r, col)
    }
    pub fn soft(mut self, soft: f32) -> Stroke {
        self.soft = soft;
        self
    }
    pub fn paint(&self, p: &mut Painter) {
        if self.a == self.b {
            p.disc(self.a, self.r, self.soft, self.col);
        } else {
            p.line(self.a, self.b, self.r, self.soft, self.col);
        }
    }
}
