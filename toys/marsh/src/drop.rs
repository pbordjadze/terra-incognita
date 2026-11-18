//! The drop and its bacteria. Each cell carries a chain of magnetite crystals fixed along its body, so the
//! field turns it as it would a compass needle, while the jostling of the water (rotational diffusion)
//! turns it at random; it swims forward along its body all the while. How well the field wins is the ratio
//! of the chain's magnetic energy to the water's thermal energy, mB/kT, about 5 to 7 in the Earth's field (Frankel
//! and Blakemore; a chain's moment is about 1e-15 A m²), so most cells head north, a few wander, and in a
//! magnet's field of hundreds of µT every one snaps round. The drop is thin, so only the field across the
//! glass steers them; at its edge they press on, gathering in a band on the side the field points to.
//!
//! Lengths are points on the screen; the drop is about 1.5 mm across, so a point is a few micrometres.

use std::f32::consts::{PI, TAU};

/// The field at which the chain's energy matches the water's (kT/m), µT: alignment goes as |B| / B_KT.
pub const B_KT: f32 = 3.0;
/// The water's turning of a cell (rotational diffusion), rad² a second.
pub const D_ROT: f32 = 0.18;
/// Swimming speeds, micrometres a second (magnetotactic cocci swim fast; spirilla slower).
pub const SPEED: [f32; 2] = [45.0, 150.0];
/// The share of cells that seek south instead (a few in every northern sample).
pub const SOUTH: f32 = 0.03;
/// The finger's magnet: a bar magnet's south pole held just over the glass, its height and its field
/// across the glass at its rim (µT at `POLE_H` points out).
pub const POLE_H: f32 = 30.0;
pub const POLE_B: f32 = 900.0;

/// The rules as the sheet's switches name them, in `Drop::on`'s order.
pub const RULES: [&str; 2] = ["Magnetite turns them", "They swim"];

pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        // xorshift64*
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    /// Uniform in 0..1.
    pub fn f(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }
    /// A standard normal (Box–Muller).
    pub fn n(&mut self) -> f32 {
        let u = self.f().max(1e-7);
        let v = self.f();
        (-2.0 * u.ln()).sqrt() * (TAU * v).cos()
    }
}

#[derive(Clone, Copy)]
pub struct Cell {
    pub p: [f32; 2],
    /// Heading, radians (screen: x right, y down).
    pub a: f32,
    /// Speed, points a second.
    pub v: f32,
    /// Seeks south.
    pub south: bool,
    /// The phase of its corkscrew wobble.
    pub phase: f32,
    /// How bright it scatters (size).
    pub glint: f32,
    /// A spirillum (a long corkscrew, slower) rather than a coccus.
    pub spiral: bool,
    /// How far from the plane of focus (0 sharp, 1 a soft blur).
    pub blur: f32,
}

/// A larger, unmagnetic protist (a ciliate) wandering the drop at its own will, for scale.
#[derive(Clone, Copy)]
pub struct Drifter {
    pub p: [f32; 2],
    pub a: f32,
    /// Its length, points.
    pub len: f32,
    pub v: f32,
}

pub struct Drop {
    /// The drop's centre and its half-width and half-height (pressed under the cover glass, it spreads
    /// into an oval), points.
    pub c: [f32; 2],
    pub r: f32,
    pub ry: f32,
    /// Micrometres a point.
    pub um: f32,
    pub cells: Vec<Cell>,
    pub drifters: Vec<Drifter>,
    /// Magnetite turns them; they swim.
    pub on: [bool; 2],
    pub rng: Rng,
}

/// The field across the glass at `p` (µT, screen directions) from the phone's field `b` and the finger
/// magnets' poles `poles`.
pub fn field_at(p: [f32; 2], b: [f32; 2], poles: &[[f32; 2]]) -> [f32; 2] {
    let mut f = b;
    for q in poles {
        let d = [q[0] - p[0], q[1] - p[1]];
        let r2 = d[0] * d[0] + d[1] * d[1];
        // a pole at height h: the field across the glass goes as r / (r² + h²)^1.5, toward a south pole
        let h2 = POLE_H * POLE_H;
        let k = POLE_B * (2.0 * h2).powf(1.5) / POLE_H / (r2 + h2).powf(1.5);
        f[0] += d[0] * k;
        f[1] += d[1] * k;
    }
    f
}

impl Drop {
    /// `n` cells in a drop at `c` of radius `r` points, already steered by a field toward `north` (a
    /// direction on the screen): most of them streaming that way, a band gathered at that edge.
    pub fn new(c: [f32; 2], r: f32, ry: f32, n: usize, north: [f32; 2], seed: u64) -> Drop {
        let mut rng = Rng(seed | 1);
        let um = 1500.0 / (2.0 * r);
        let oval = |rad: f32, ang: f32| [c[0] + rad * ang.cos() * r, c[1] + rad * ang.sin() * ry];
        let na = north[1].atan2(north[0]);
        let mut cells = Vec::with_capacity(n);
        for _ in 0..n {
            let south = rng.f() < SOUTH;
            // a third gathered at the north edge, the rest anywhere, thinning away from north
            let (rad, ang) = if rng.f() < 0.35 {
                (0.9 + 0.09 * rng.f(), na + 0.7 * rng.n())
            } else {
                (rng.f().sqrt() * 0.97, rng.f() * TAU)
            };
            let p = oval(rad, ang);
            let spiral = rng.f() < 0.15;
            let toward = if south { na + PI } else { na };
            cells.push(Cell {
                p,
                a: toward + 0.6 * rng.n(),
                v: (SPEED[0] + (SPEED[1] - SPEED[0]) * rng.f().powi(2)) / um
                    * if spiral { 0.5 } else { 1.0 },
                south,
                phase: rng.f() * TAU,
                glint: 0.35 + 0.65 * rng.f().powf(1.5),
                spiral,
                blur: rng.f().powi(7),
            });
        }
        let drifters = (0..5)
            .map(|_| Drifter {
                p: oval(rng.f().sqrt() * 0.8, rng.f() * TAU),
                a: rng.f() * TAU,
                len: (60.0 + 80.0 * rng.f()) / um,
                v: 25.0 / um,
            })
            .collect();
        Drop {
            c,
            r,
            ry,
            um,
            cells,
            drifters,
            on: [true, true],
            rng,
        }
    }

    /// One step of `dt` seconds in the phone's field `b` (µT across the glass) with magnet poles at `poles`.
    pub fn step(&mut self, dt: f32, b: [f32; 2], poles: &[[f32; 2]]) {
        let [turns, swim] = self.on;
        let noise = (2.0 * D_ROT * dt).sqrt();
        for cell in &mut self.cells {
            let f = field_at(cell.p, b, poles);
            let m = (f[0] * f[0] + f[1] * f[1]).sqrt();
            if turns && m > 1e-3 {
                let mut want = f[1].atan2(f[0]);
                if cell.south {
                    want += PI;
                }
                // the torque's turn, kept stable at a strong magnet's rates (never past the field)
                let k = (D_ROT * m / B_KT * dt).min(0.5);
                cell.a += k * (want - cell.a).sin();
            }
            cell.a += noise * self.rng.n();
            cell.phase += dt * 9.0;
            if swim {
                let wob = 0.35 * cell.phase.sin();
                let a = cell.a + wob;
                cell.p[0] += a.cos() * cell.v * dt;
                cell.p[1] += a.sin() * cell.v * dt;
            } else {
                // Brownian jiggle alone: a micrometre or so a second
                let j = (2.0 * 0.4 * dt).sqrt() / self.um;
                cell.p[0] += j * self.rng.n();
                cell.p[1] += j * self.rng.n();
            }
            cell.p = inside(self.c, self.r, self.ry, cell.p, 2.0);
        }
        // the protists wander, slowly turning, unmoved by the field
        for d in &mut self.drifters {
            d.a += (2.0 * 0.3 * dt).sqrt() * self.rng.n();
            d.p[0] += d.a.cos() * d.v * dt;
            d.p[1] += d.a.sin() * d.v * dt;
        }
        for i in 0..self.drifters.len() {
            let d = self.drifters[i];
            self.drifters[i].p = inside(self.c, self.r, self.ry, d.p, d.len * 0.6);
        }
    }
}

/// `p` kept `margin` points inside the oval drop at `c` (half-width `r`, half-height `ry`): a cell pressing
/// on its edge slides along it.
pub fn inside(c: [f32; 2], r: f32, ry: f32, p: [f32; 2], margin: f32) -> [f32; 2] {
    let (rx, ry) = (r - margin, ry - margin);
    let q = [(p[0] - c[0]) / rx, (p[1] - c[1]) / ry];
    let m = (q[0] * q[0] + q[1] * q[1]).sqrt();
    if m <= 1.0 {
        return p;
    }
    [c[0] + q[0] / m * rx, c[1] + q[1] / m * ry]
}
