//! The spray where the bore meets the wall: as its front sweeps up the river, the place where it strikes
//! the revetment runs along the wall toward us, and there the water runs up the stones and is thrown up,
//! higher the bigger and the more broken the bore. Drops fly and fall under gravity; drawn as paper left
//! bare with a little ink at their edges, and a mist that hangs and thins.

use crate::crowd::Stroke;
use crate::view::Camera;

pub struct Drop {
    p: [f64; 3],
    v: [f64; 3],
    age: f64,
    life: f64,
    size: f64,
    mist: bool,
}

#[derive(Default)]
pub struct Spray {
    pub drops: Vec<Drop>,
    seed: u64,
    owed: f64,
    owed_roll: f64,
}

impl Spray {
    fn f(&mut self) -> f64 {
        self.seed = self
            .seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (self.seed >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Throws drops up where the front meets the stones at (`x`, `level`, `z`): `dh` the bore's height (m),
    /// `broken` how far it is breaking (0..1), for `dt` seconds of the scene shown at `rate` times its own.
    pub fn burst(&mut self, at: [f64; 3], dh: f64, broken: f64, dt: f64, rate: f64) {
        let strength = (dh * (0.4 + broken)).clamp(0.0, 3.0);
        if strength < 0.15 {
            return;
        }
        self.owed += dt * 450.0 * strength;
        let n = self.owed as usize;
        self.owed -= n as f64;
        // drops leave faster when the scene runs fast, so they still read as thrown
        let pace = rate.sqrt().clamp(1.0, 4.0);
        for _ in 0..n.min(400) {
            let mist = self.f() < 0.08;
            // most of it climbs: a sheet thrown up the stones, a little falling short
            let up = (2.0 * 9.81 * (0.8 + 3.5 * strength) * (0.35 + 0.65 * self.f())).sqrt();
            let v = [
                -1.5 * self.f() * strength + 0.8 * (self.f() - 0.5),
                up,
                -2.0 * self.f() - 4.0 * self.f() * broken,
            ];
            let p = [
                at[0] + 2.5 * (self.f() - 0.3),
                at[1] + 0.3 * self.f(),
                at[2] + 6.0 * (self.f() - 0.5),
            ];
            let life = if mist { 2.5 + 2.0 * self.f() } else { 4.0 };
            let size = if mist {
                1.0 + 2.5 * self.f() * strength
            } else {
                0.03 + 0.35 * self.f().powi(2)
            };
            self.drops.push(Drop {
                p,
                v: [v[0] * pace, v[1] * pace, v[2] * pace],
                age: 0.0,
                life,
                size,
                mist,
            });
        }
    }

    /// The breaking crest's own spray, all along the front across the river: low puffs thrown up off
    /// the roller, so from afar the bore reads as one white line.
    pub fn roll(&mut self, zf: f64, level: f64, dh: f64, broken: f64, dt: f64) {
        let strength = dh * broken;
        if strength < 0.1 || zf > 3000.0 {
            return;
        }
        self.owed_roll += dt * 600.0 * strength.min(1.5);
        let n = self.owed_roll as usize;
        self.owed_roll -= n as f64;
        for _ in 0..n.min(200) {
            // more of them near us, where the front is in view (spread wider out across it far off)
            let x = 6.0 + (60.0 + zf.max(0.0) * 0.9) * self.f().powf(1.6);
            let z = zf + crate::view::bow(x, zf) - 1.0 * self.f();
            let up = (2.0 * 9.81 * (0.3 + 1.2 * dh) * self.f()).sqrt();
            let p = [x, level + 0.2, z];
            // the roller throws its water up and forward, toward us, off the breaking crest
            let mist = self.f() < 0.15;
            let size = if mist {
                0.25 + 0.6 * self.f() * dh
            } else {
                0.03 + 0.25 * self.f().powi(2)
            };
            let v = [
                0.0,
                up * if mist { 1.0 } else { 1.4 },
                -2.5 - 3.0 * self.f(),
            ];
            let life = 1.2 + self.f();
            self.drops.push(Drop {
                p,
                v,
                age: 0.0,
                life,
                size,
                mist,
            });
        }
    }

    pub fn update(&mut self, dt: f64, rate: f64, floor: impl Fn(f64, f64) -> f64) {
        let g = 9.81 * rate.sqrt().clamp(1.0, 4.0).powi(2);
        for d in &mut self.drops {
            d.age += dt;
            if d.mist {
                d.v[1] = d.v[1] * (1.0 - 2.5 * dt) + 0.3 * dt;
                d.v[0] *= 1.0 - 2.0 * dt;
                d.v[2] *= 1.0 - 2.0 * dt;
                d.size += dt * 0.8;
            } else {
                d.v[1] -= g * dt;
            }
            for i in 0..3 {
                d.p[i] += d.v[i] * dt;
            }
        }
        self.drops
            .retain(|d| d.age < d.life && (d.mist || d.p[1] > floor(d.p[0], d.p[2]) - 0.2));
        if self.drops.len() > 3000 {
            let extra = self.drops.len() - 3000;
            self.drops.drain(..extra);
        }
    }

    pub fn draw(&self, cam: &Camera, size: [f64; 2], ppp: f64, fade: f32, out: &mut Vec<Stroke>) {
        let paper = [0.935, 0.905, 0.835];
        for d in &self.drops {
            let Some((a, zc)) = cam.project(d.p, size) else {
                continue;
            };
            let k = cam.scale(zc, size) * ppp;
            let a = [a[0] * ppp, a[1] * ppp];
            let r = (d.size * k)
                .max(0.6)
                .min(if d.mist { 22.0 * ppp } else { 1.8 * ppp });
            let life = (1.0 - d.age / d.life).clamp(0.0, 1.0) as f32;
            if d.mist {
                let al = 0.1 * life * (1.0 - fade);
                out.push(
                    Stroke::disc(zc, a, r, [paper[0], paper[1], paper[2], al]).soft(r as f32 * 0.8),
                );
            } else {
                // a fleck of the brush along its flight, fat at its head and thinning behind, no two alike;
                // now and then a grain of ink thrown with it
                let tail = 0.03 + 0.12 * (d.size * 37.0).fract();
                let al = (0.9 * life.sqrt()) * (1.0 - fade);
                let mut head = a;
                for (j, w) in [1.0, 0.6, 0.3].into_iter().enumerate() {
                    let back = tail * (j + 1) as f64 / 3.0;
                    let q = [
                        d.p[0] - d.v[0] * back,
                        d.p[1] - d.v[1] * back,
                        d.p[2] - d.v[2] * back,
                    ];
                    let b = cam
                        .project(q, size)
                        .map_or(head, |(b, _)| [b[0] * ppp, b[1] * ppp]);
                    out.push(Stroke::line(
                        zc - 0.01,
                        head,
                        b,
                        r * w,
                        [paper[0], paper[1], paper[2], al * (1.0 - 0.2 * j as f32)],
                    ));
                    head = b;
                }
                if (d.size * 91.0).fract() < 0.12 {
                    let off = [a[0] + r * 2.5, a[1] + r * 1.5];
                    out.push(Stroke::disc(
                        zc,
                        off,
                        (r * 0.35).max(0.5 * ppp),
                        [0.1, 0.1, 0.11, al * 0.7],
                    ));
                }
            }
        }
    }
}
