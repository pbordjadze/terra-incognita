//! The wind over the grove: a light steady breeze, and gusts that run across the crowns faster than
//! the breeze itself, as gusts over a canopy do (they ride down from the faster air above). A finger's
//! drag is one more gust, laid where it was drawn and blowing the way it went. Every scale reads the
//! same wind: the grove at each tree, one stem and one leaf at their tree.

/// The breeze's direction on the ground (x across the view, z away from it), unit length.
pub const DIR: [f32; 2] = [0.94, -0.34];
/// The breeze at the crowns, m/s: enough to set aspen leaves trembling, not to bend the trees.
pub const MEAN: f32 = 1.6;
/// How much faster than the breeze a gust's pattern runs over the crowns.
pub const CONVECT: f32 = 1.8;

/// A small hash: a number in 0..1 from a seed.
pub fn hash(mut x: u32) -> f32 {
    x ^= x >> 16;
    x = x.wrapping_mul(0x7feb_352d);
    x ^= x >> 15;
    x = x.wrapping_mul(0x846c_a68b);
    x ^= x >> 16;
    (x >> 8) as f32 / (1u32 << 24) as f32
}

/// One gust: where its middle is (m), how it moves, its size along and across its way, its strength
/// (m/s at its middle), its age and life (s).
#[derive(Clone, Copy)]
pub struct Gust {
    pub at: [f32; 2],
    pub vel: [f32; 2],
    pub along: f32,
    pub across: f32,
    pub strength: f32,
    pub age: f32,
    pub life: f32,
}

impl Gust {
    fn envelope(&self) -> f32 {
        let u = self.age / self.life;
        if u >= 1.0 {
            return 0.0;
        }
        // comes up quickly, dies away slowly
        (u * 6.0).min(1.0) * (1.0 - u) * (1.0 - u)
    }

    fn at(&self, p: [f32; 2]) -> f32 {
        let s = (self.vel[0] * self.vel[0] + self.vel[1] * self.vel[1])
            .sqrt()
            .max(1e-3);
        let (ux, uz) = (self.vel[0] / s, self.vel[1] / s);
        let (dx, dz) = (p[0] - self.at[0], p[1] - self.at[1]);
        let a = (dx * ux + dz * uz) / self.along;
        let c = (-dx * uz + dz * ux) / self.across;
        self.strength * self.envelope() * (-(a * a + c * c)).exp()
    }
}

pub struct Wind {
    pub gusts: Vec<Gust>,
    seed: u32,
    next: f32,
    pub t: f32,
}

impl Wind {
    pub fn new(seed: u32) -> Wind {
        let mut w = Wind {
            gusts: Vec::new(),
            seed,
            next: 0.0,
            t: 0.0,
        };
        // the grove is already moving when it opens: two gusts on their way
        for k in 0..2 {
            w.natural(40.0 + 50.0 * k as f32);
        }
        w
    }

    fn rnd(&mut self) -> f32 {
        self.seed = self.seed.wrapping_add(0x9e37_79b9);
        hash(self.seed)
    }

    /// A gust coming in over the grove from upwind, `ahead` metres along its way.
    fn natural(&mut self, ahead: f32) {
        let z = 15.0 + self.rnd() * 140.0;
        let speed = MEAN * CONVECT * (0.8 + 0.5 * self.rnd());
        let start = [-0.75 * z - 30.0 + ahead * DIR[0], z + ahead * DIR[1]];
        let strength = 3.0 + 3.0 * self.rnd();
        let along = 14.0 + 10.0 * self.rnd();
        let across = 45.0 + 45.0 * self.rnd();
        self.gusts.push(Gust {
            at: start,
            vel: [DIR[0] * speed, DIR[1] * speed],
            along,
            across,
            strength,
            age: 0.0,
            life: 40.0,
        });
    }

    /// A gust from a finger: at a place in the grove (m), blowing `vel` (m/s along the ground).
    pub fn push(&mut self, at: [f32; 2], vel: [f32; 2], size: f32) {
        let s = (vel[0] * vel[0] + vel[1] * vel[1]).sqrt();
        if s < 0.2 {
            return;
        }
        // a hand can't blow a gale: the strength saturates
        let strength = 5.0 * (1.0 - (-s / 6.0).exp());
        let speed = s.clamp(MEAN * CONVECT, 9.0);
        self.gusts.push(Gust {
            at,
            vel: [vel[0] / s * speed, vel[1] / s * speed],
            along: size,
            across: size * 1.6,
            strength,
            age: 0.0,
            life: 9.0,
        });
        if self.gusts.len() > 48 {
            self.gusts.remove(0);
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.t += dt;
        for g in &mut self.gusts {
            g.at[0] += g.vel[0] * dt;
            g.at[1] += g.vel[1] * dt;
            g.age += dt;
        }
        self.gusts.retain(|g| g.age < g.life);
        self.next -= dt;
        if self.next <= 0.0 {
            self.natural(0.0);
            self.next = 4.0 + 7.0 * self.rnd();
        }
    }

    /// The wind speed at a place in the grove (m/s), with the breeze's own small flutter.
    pub fn at(&self, p: [f32; 2]) -> f32 {
        let t = self.t;
        let ripple = 0.35 * ((t * 0.9 + p[0] * 0.05).sin() * (t * 1.7 - p[1] * 0.07).sin()) * MEAN;
        let mut w = MEAN + ripple;
        for g in &self.gusts {
            w += g.at(p);
        }
        w.max(0.0)
    }
}
