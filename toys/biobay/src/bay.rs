//! The bay's water, from a kayak looking straight down: a grid of cells (`CELL` cm) each holding how
//! stirred the water is, how much light its dinoflagellates are giving now, how much they have left
//! to give, and whether a mangrove root stands there. The light comes only from stirring (shear on the
//! cells' walls), so a finger drawn through it, a fish darting or a raindrop lights it, and still water
//! is dark. The mangroves' roots are drawn by the shader (bay.wgsl). A flash spends some of a cell's light, which comes back slowly. Pyrodinium bahamense in
//! Puerto Mosquito, Vieques (SOURCES.md).

/// One cell is this many centimetres across.
pub const CELL: f32 = 0.8;

/// The rules as switches (the sheet's), in order: the moon is down; a flash tires them; fish swim through.
pub const RULES: [&str; 3] = ["The moon has set", "Flashes tire them", "Fish swim through"];

/// Stirring fades in this long (seconds): the water settles.
const SETTLE_S: f32 = 1.3;
/// A cell's light dies in this long (seconds) once the stirring stops.
const GLOW_S: f32 = 0.45;
/// A spent cell has its light back in this long (seconds, a time constant).
const RECHARGE_S: f32 = 22.0;
/// How fast lighting spends a cell's light.
const SPEND: f32 = 0.55;
/// How far stirring spreads into the water around it (cm² a second).
const SPREAD: f32 = 1.6;
/// A finger in the water stirs this wide (cm).
const FINGER_CM: f32 = 2.2;
/// A rain shower lasts this long (seconds), this many drops a second.
const SHOWER_S: f32 = 9.0;
const DROPS_PER_S: f32 = 30.0;
/// Fry darting about, this many a second over the whole screen.
const FRY_PER_S: f32 = 12.0;

pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// 0..1
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// -1..1
    pub fn s(&mut self) -> f32 {
        self.f() * 2.0 - 1.0
    }
}

/// A small fish under the surface (a needlefish or a mullet): seen only by the light it stirs.
pub struct Fish {
    pub p: [f32; 2],
    pub heading: f32,
    pub speed: f32,
    /// cruising speed (cm/s) and body length (cm)
    pub cruise: f32,
    pub len: f32,
    /// seconds left of a dart (fleeing a finger)
    pub dart: f32,
    turn: f32,
}

pub struct Bay {
    pub w: f32,
    pub h: f32,
    pub gw: usize,
    pub gh: usize,
    pub stir: Vec<f32>,
    pub light: Vec<f32>,
    pub charge: Vec<f32>,
    pub fish: Vec<Fish>,
    /// the rules (`RULES`), each on or off
    pub on: [bool; 3],
    /// seconds left of a rain shower
    pub shower: f32,
    /// raindrops that fell this step
    pub drops: Vec<[f32; 2]>,
    /// fish that darted this step
    pub darted: Vec<[f32; 2]>,
    pub t: f32,
    rng: Rng,
    scratch: Vec<f32>,
}

impl Bay {
    pub fn new(w: f32, h: f32, seed: u64) -> Bay {
        let gw = (w / CELL).ceil() as usize + 1;
        let gh = (h / CELL).ceil() as usize + 1;
        let n = gw * gh;
        let mut b = Bay {
            w,
            h,
            gw,
            gh,
            stir: vec![0.0; n],
            light: vec![0.0; n],
            charge: vec![1.0; n],
            fish: Vec::new(),
            on: [true; 3],
            shower: 0.0,
            drops: Vec::new(),
            darted: Vec::new(),
            t: 0.0,
            rng: Rng(seed | 1),
            scratch: vec![0.0; n],
        };
        for _ in 0..6 {
            let f = b.new_fish(true);
            b.fish.push(f);
        }
        b
    }

    fn new_fish(&mut self, anywhere: bool) -> Fish {
        let r = &mut self.rng;
        let len = 4.0 + 14.0 * r.f() * r.f();
        let (p, heading) = if anywhere {
            (
                [r.f() * self.w, self.h * (0.25 + 0.7 * r.f())],
                r.f() * std::f32::consts::TAU,
            )
        } else {
            // in from a side, heading across
            let left = r.f() < 0.5;
            let y = self.h * (0.25 + 0.7 * r.f());
            let x = if left { -15.0 } else { self.w + 15.0 };
            let a = if left { 0.0 } else { std::f32::consts::PI } + r.s() * 0.5;
            ([x, y], a)
        };
        let cruise = 14.0 + 16.0 * r.f();
        Fish {
            p,
            heading,
            speed: cruise,
            cruise,
            len,
            dart: 0.0,
            turn: 0.0,
        }
    }

    /// The cells within `r` of `c`: x0..x1, y0..y1.
    fn cells_near(&self, c: [f32; 2], r: f32) -> (usize, usize, usize, usize) {
        let lo = |v: f32| ((v - r) / CELL).floor().max(0.0) as usize;
        let hi = |v: f32, n: usize| (((v + r) / CELL).ceil() as usize + 1).min(n);
        (lo(c[0]), hi(c[0], self.gw), lo(c[1]), hi(c[1], self.gh))
    }

    /// Stir the water around `c`: `amount` at the middle, falling off over `r` cm.
    pub fn stir_at(&mut self, c: [f32; 2], r: f32, amount: f32) {
        let (x0, x1, y0, y1) = self.cells_near(c, r * 2.5);
        for gy in y0..y1 {
            for gx in x0..x1 {
                let d = dist([gx as f32 * CELL, gy as f32 * CELL], c);
                let i = gy * self.gw + gx;
                let k = (-(d * d) / (r * r)).exp();
                self.stir[i] = (self.stir[i] + amount * k).min(4.0);
            }
        }
    }

    /// A finger drawn through the water from `a` to `b` in `dt` seconds: the faster, the brighter.
    pub fn drag(&mut self, a: [f32; 2], b: [f32; 2], dt: f32) {
        let len = dist(a, b);
        let speed = len / dt.max(1.0 / 120.0);
        let amount = (speed / 40.0).clamp(0.0, 2.2);
        let n = (len / (CELL * 0.7)).ceil().max(1.0) as usize;
        for i in 0..n {
            let u = (i as f32 + 0.5) / n as f32;
            let p = [a[0] + (b[0] - a[0]) * u, a[1] + (b[1] - a[1]) * u];
            self.stir_at(p, FINGER_CM, amount * 0.45);
        }
        self.scare(b, 14.0);
    }

    /// A finger put into the water: a splash, and the fish near it dart away.
    pub fn touch(&mut self, c: [f32; 2]) {
        self.stir_at(c, FINGER_CM * 1.2, 1.2);
        for k in 0..16 {
            let a = k as f32 / 16.0 * std::f32::consts::TAU;
            self.stir_at([c[0] + a.cos() * 3.5, c[1] + a.sin() * 3.5], 1.0, 0.5);
        }
        self.scare(c, 20.0);
    }

    fn scare(&mut self, c: [f32; 2], r: f32) {
        for f in &mut self.fish {
            if f.dart <= 0.0 && dist(f.p, c) < r + f.len {
                f.heading = (f.p[1] - c[1]).atan2(f.p[0] - c[0]) + self.rng.s() * 0.4;
                f.dart = 0.5 + 0.3 * self.rng.f();
                self.darted.push(f.p);
            }
        }
    }

    pub fn rain(&mut self) {
        self.shower = SHOWER_S;
    }

    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        self.drops.clear();
        if self.on[2] {
            self.swim(dt);
            self.fry(dt);
        }
        self.fall(dt);
        self.settle(dt);
        self.shine(dt);
    }

    fn swim(&mut self, dt: f32) {
        let (w, h) = (self.w, self.h);
        let mut gone = Vec::new();
        let mut trails = Vec::new();
        for (k, f) in self.fish.iter_mut().enumerate() {
            f.turn += self.rng.s() * 5.0 * dt;
            f.turn *= (-dt * 0.8).exp();
            let target = if f.dart > 0.0 {
                f.cruise * 7.0
            } else {
                f.cruise
            };
            f.speed += (target - f.speed) * (dt * if f.dart > 0.0 { 12.0 } else { 1.5 }).min(1.0);
            f.heading += f.turn * dt * if f.dart > 0.0 { 0.2 } else { 1.0 };
            f.dart -= dt;
            let (c, s) = (f.heading.cos(), f.heading.sin());
            f.p = [f.p[0] + c * f.speed * dt, f.p[1] + s * f.speed * dt];
            // keep under the open water, away from the roots
            if f.p[1] < h * 0.22 {
                f.turn += 1.5 * dt * if c > 0.0 { 1.0 } else { -1.0 };
            }
            if f.p[0] < -30.0 || f.p[0] > w + 30.0 || f.p[1] < -30.0 || f.p[1] > h + 30.0 {
                gone.push(k);
            }
            trails.push((f.p, f.speed, f.len, [c, s]));
        }
        for (b, speed, len, dir) in trails {
            // the body stirs along its length, the tail most
            let amount = (speed / 60.0).clamp(0.08, 1.6) * dt * 5.5;
            let tail = [b[0] - dir[0] * len, b[1] - dir[1] * len];
            let n = 4;
            for i in 0..n {
                let u = i as f32 / n as f32;
                let p = [b[0] + (tail[0] - b[0]) * u, b[1] + (tail[1] - b[1]) * u];
                self.stir_at(p, 0.7 + 0.5 * u, amount * (0.4 + u));
            }
        }
        for k in gone.into_iter().rev() {
            self.fish[k] = self.new_fish(false);
        }
    }

    /// The fry and the shrimp too small to see, each nudge a spark or two here and there.
    fn fry(&mut self, dt: f32) {
        let mut n = FRY_PER_S * dt;
        while n > 0.0 {
            if self.rng.f() < n {
                let p = [self.rng.f() * self.w, self.h * (0.12 + 0.88 * self.rng.f())];
                let q = [p[0] + self.rng.s() * 2.0, p[1] + self.rng.s() * 2.0];
                for i in 0..4 {
                    let u = i as f32 / 3.0;
                    self.stir_at(
                        [p[0] + (q[0] - p[0]) * u, p[1] + (q[1] - p[1]) * u],
                        0.45,
                        0.5,
                    );
                }
            }
            n -= 1.0;
        }
    }

    fn fall(&mut self, dt: f32) {
        if self.shower <= 0.0 {
            return;
        }
        self.shower -= dt;
        // the shower swells and passes
        let u = 1.0 - self.shower / SHOWER_S;
        let rate = DROPS_PER_S * (u * std::f32::consts::PI).sin().max(0.15);
        let mut n = rate * dt;
        while n > 0.0 {
            if self.rng.f() < n {
                let p = [self.rng.f() * self.w, self.rng.f() * self.h];
                self.stir_at(p, 0.6, 1.1);
                for k in 0..10 {
                    let a = k as f32 / 10.0 * std::f32::consts::TAU;
                    self.stir_at([p[0] + a.cos() * 1.6, p[1] + a.sin() * 1.6], 0.5, 0.35);
                }
                self.drops.push(p);
            }
            n -= 1.0;
        }
    }

    /// The stirring spreads a little and settles.
    fn settle(&mut self, dt: f32) {
        let (gw, gh) = (self.gw, self.gh);
        let k = (SPREAD * dt / (CELL * CELL)).min(0.24);
        let fade = (-dt / SETTLE_S).exp();
        self.scratch.copy_from_slice(&self.stir);
        let s = &self.scratch;
        for gy in 0..gh {
            let up = gy.saturating_sub(1) * gw;
            let down = (gy + 1).min(gh - 1) * gw;
            let row = gy * gw;
            for gx in 0..gw {
                let l = row + gx.saturating_sub(1);
                let r = row + (gx + 1).min(gw - 1);
                let i = row + gx;
                let lap = s[l] + s[r] + s[up + gx] + s[down + gx] - 4.0 * s[i];
                self.stir[i] = (s[i] + k * lap) * fade;
            }
        }
    }

    /// Each stirred cell lights, by what it has left; lighting spends it; it comes back slowly.
    fn shine(&mut self, dt: f32) {
        let fade = (-dt / GLOW_S).exp();
        let back = 1.0 - (-dt / RECHARGE_S).exp();
        let tire = self.on[1];
        for i in 0..self.stir.len() {
            let make = self.stir[i] * self.charge[i];
            self.light[i] = self.light[i] * fade + make * dt * 6.0;
            if tire {
                self.charge[i] -= (make * SPEND * dt).min(self.charge[i]);
            }
            self.charge[i] += (1.0 - self.charge[i]) * back;
        }
    }

    /// The grid as the field's texels: (light, charge, stir, -).
    pub fn texels(&self, out: &mut Vec<[f32; 4]>) {
        out.clear();
        out.extend(
            (0..self.stir.len())
                .map(|i| [self.light[i].min(60.0), self.charge[i], self.stir[i], 0.0]),
        );
    }
}

pub fn dist(a: [f32; 2], b: [f32; 2]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn total_light(b: &Bay) -> f32 {
        b.light.iter().sum()
    }

    #[test]
    fn still_water_stays_dark() {
        let mut b = Bay::new(100.0, 200.0, 7);
        b.on[2] = false;
        for _ in 0..120 {
            b.step(1.0 / 30.0);
        }
        assert!(total_light(&b) < 1e-3);
    }

    #[test]
    fn a_stroke_lights_and_then_fades() {
        let mut b = Bay::new(100.0, 200.0, 7);
        b.on[2] = false;
        b.drag([20.0, 120.0], [60.0, 120.0], 0.3);
        b.step(1.0 / 30.0);
        let lit = total_light(&b);
        assert!(lit > 1.0, "lit {lit}");
        for _ in 0..240 {
            b.step(1.0 / 30.0);
        }
        assert!(total_light(&b) < lit * 0.05);
    }

    #[test]
    fn a_second_stroke_is_dimmer_when_they_tire() {
        let peak = |tire: bool| {
            let mut b = Bay::new(100.0, 200.0, 7);
            b.on = [true, tire, false];
            let mut best = [0.0f32; 2];
            for best in best.iter_mut() {
                b.drag([20.0, 120.0], [60.0, 120.0], 0.3);
                for _ in 0..90 {
                    b.step(1.0 / 30.0);
                    *best = best.max(total_light(&b));
                }
            }
            best[1] / best[0]
        };
        let tired = peak(true);
        let fresh = peak(false);
        assert!(tired < 0.7, "tired {tired}");
        assert!(fresh > 0.95, "fresh {fresh}");
    }

    #[test]
    fn fish_light_the_water_and_switch_off() {
        let mut b = Bay::new(100.0, 200.0, 7);
        b.on[1] = false;
        for _ in 0..60 {
            b.step(1.0 / 30.0);
        }
        let swimming = total_light(&b);
        assert!(swimming > 0.5, "swimming {swimming}");
        b.on[2] = false;
        for _ in 0..300 {
            b.step(1.0 / 30.0);
        }
        let still = total_light(&b);
        assert!(still < swimming * 0.02, "still {still} of {swimming}");
    }
}
