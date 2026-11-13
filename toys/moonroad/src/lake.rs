//! The lake: a template of facets laid on the water, each a little patch of wave with its own tilt.
//! The cells are fixed to the water in rows like bricks: out, row `v = ln D / DV` (metres, `D` out
//! from the shore); across, each row's cells are `DU` times the row's own distance wide (metres, so
//! they stay put on the water however far you walk), so near cells are big on the screen and far ones
//! small, as a real surface is. A cell's tilt wanders (an Ornstein–Uhlenbeck walk) and the swell runs through it
//! in bands toward the shore. The moon is at infinity: a cell flashes when its tilt sends the moon's
//! light to the eye, the glitter path (Minnaert, *Light and Colour in the Outdoors*), and once
//! flashed, it keeps its paint.

/// The grid's steps (a row's cell width over its distance; natural log out).
pub const DU: f64 = 0.034;
pub const DV: f64 = 0.024;
/// The grid: cells across (a ring: the shore is long) and rows out.
pub const NU: usize = 48;
pub const NV: usize = 200;
/// Rows kept nearer than the bottom of the screen, so turning the phone never runs out of water.
const SPARE_ROWS: i64 = 40;
/// The eye over the water, metres (standing on the beach).
pub const EYE_M: f64 = 1.7;
/// The moon's elevation (radians, ~4°) and its azimuth straight ahead.
pub const MOON_EL: f64 = 0.07;
/// The calm night's spread of slopes (rms, each axis), and how fast a tilt forgets (seconds).
pub const CALM: f64 = 0.06;
const TAU_S: f64 = 0.9;
/// How near a cell's tilt must come to send the moon to the eye (rms, radians of slope): the moon's
/// half a degree, widened by the cell being many little facets.
const TOL: f64 = 0.018;
/// The swell: wavelength (m), period (s), its share of the slope.
const SWELL_M: f64 = 7.0;
const SWELL_S: f64 = 2.1;
const SWELL_K: f64 = 0.7;
/// How long a cell stays bright after the moon was last in it (seconds).
const LIT_S: f64 = 2.2;
/// A painted cell comes in over this long (seconds).
const PAINT_S: f64 = 0.45;

#[derive(Clone, Copy)]
pub struct Cell {
    /// the seed's place in its cell, 0..1 each way
    pub j: [f32; 2],
    /// paint number, 1..=3
    pub paint: u8,
    pub slope: [f64; 2],
    /// how much painted, 0..1; how bright the moon is in it now; and how lately it shone (the moon
    /// now, fading over a few seconds), so the live road stays the brightest thing on the water
    pub painted: f32,
    pub glint: f32,
    pub lit: f32,
    /// the turn of the ring this cell is now (it is a new piece of water when that changes)
    pub lap: i64,
}

pub struct Gust {
    /// where, in metres (along the shore, out), when, how strong
    pub at: [f64; 2],
    pub t: f64,
    pub power: f64,
}

pub struct Lake {
    pub cells: Vec<Cell>,
    /// the first row's index (v = (V0 + row) * DV)
    pub v0: i64,
    /// the eye along the shore, metres
    pub x_eye: f64,
    pub gusts: Vec<Gust>,
    /// cells newly painted this step: (metres along, out) for the sound
    pub fresh: Vec<[f64; 2]>,
    pub t: f64,
    rng: Rng,
    /// a slower walk when Reduce Motion is on
    pub slow: bool,
}

pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn f(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    /// a normal deviate
    pub fn n(&mut self) -> f64 {
        self.n2()[0]
    }
    /// two independent normal deviates (Box–Muller gives both for the price of one)
    pub fn n2(&mut self) -> [f64; 2] {
        let a = (-2.0 * self.f().max(1e-12).ln()).sqrt();
        let (s, c) = (std::f64::consts::TAU * self.f()).sin_cos();
        [a * c, a * s]
    }
}

pub fn hash(x: u64) -> f64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 11) as f64 / (1u64 << 53) as f64
}

impl Lake {
    /// A lake whose nearest row is `near_m` out (the bottom of the screen).
    pub fn new(near_m: f64, seed: u64) -> Lake {
        let v0 = (near_m.ln() / DV).floor() as i64 - SPARE_ROWS;
        let mut rng = Rng(seed | 1);
        let mut cells = Vec::with_capacity(NU * NV);
        for i in 0..NU * NV {
            let h = |k: u64| hash(seed ^ (i as u64).wrapping_mul(0x1000_0001) ^ k << 48);
            cells.push(Cell {
                j: [0.1 + 0.8 * h(1) as f32, 0.1 + 0.8 * h(2) as f32],
                paint: 1 + (h(3) * 3.0) as u8,
                slope: {
                    let n = rng.n2();
                    [n[0] * CALM, n[1] * CALM]
                },
                painted: 0.0,
                glint: 0.0,
                lit: 0.0,
                lap: 0,
            });
        }
        Lake {
            cells,
            v0,
            x_eye: 0.0,
            gusts: Vec::new(),
            fresh: Vec::new(),
            t: 0.0,
            rng,
            slow: false,
        }
    }

    /// Metres out to the middle of row `row`.
    pub fn row_m(&self, row: usize, jv: f64) -> f64 {
        ((self.v0 as f64 + row as f64 + jv) * DV).exp()
    }

    /// A row's cells' width along the shore, metres.
    pub fn width_m(&self, row: usize) -> f64 {
        DU * self.row_m(row, 0.5)
    }

    /// The extra spread of slope a gust gives at (along, out) now.
    fn gusty(&self, x: f64, d: f64) -> f64 {
        let mut s = 0.0;
        for g in &self.gusts {
            let age = self.t - g.t;
            if age < 0.0 {
                continue;
            }
            // a cat's paw: a ring of ruffled water spreading out from where it touched, sized to its
            // distance (a near gust is small, a far one covers a field)
            let scale = g.at[1] * 0.18;
            let r = scale * (0.4 + 1.6 * age);
            let w = scale * (0.35 + 0.4 * age);
            let dx = x - g.at[0];
            let dd = (d - g.at[1]) * 0.8;
            let dist = (dx * dx + dd * dd).sqrt();
            let ring = (-((dist - r) / w).powi(2)).exp() + 0.5 * (-(dist / w).powi(2)).exp();
            s += g.power * 0.16 * ring * (-age / 2.4).exp();
        }
        s
    }

    pub fn gust(&mut self, at: [f64; 2], power: f64) {
        self.gusts.push(Gust {
            at,
            t: self.t,
            power,
        });
        if self.gusts.len() > 24 {
            self.gusts.remove(0);
        }
    }

    /// A new page: every cell back to the template.
    pub fn clear(&mut self) {
        for c in &mut self.cells {
            c.painted = 0.0;
        }
    }

    pub fn step(&mut self, dt: f64) {
        self.t += dt;
        let t = self.t;
        self.gusts.retain(|g| t - g.t < 9.0);
        self.fresh.clear();
        let tau = if self.slow { TAU_S * 3.0 } else { TAU_S };
        let decay = (-dt / tau).exp();
        let fade = (-dt / LIT_S).exp() as f32;
        let kick = (1.0 - decay * decay).sqrt();
        let (sel, cel) = MOON_EL.sin_cos();
        let omega = std::f64::consts::TAU / SWELL_S;
        let k = std::f64::consts::TAU / SWELL_M;
        for row in 0..NV {
            let w = self.width_m(row);
            // the turn of the ring nearest the eye, for this row
            let ue = self.x_eye / w;
            for col in 0..NU {
                let i = row * NU + col;
                let c = self.cells[i];
                let lap = ((ue - col as f64) / NU as f64).round() as i64;
                let mut cell = c;
                if lap != c.lap {
                    // a fresh piece of water comes into the ring: unpainted
                    cell.lap = lap;
                    cell.painted = 0.0;
                    cell.lit = 0.0;
                }
                let u = col as f64 + (lap * NU as i64) as f64 + cell.j[0] as f64;
                let d = self.row_m(row, cell.j[1] as f64);
                let x = u * w;
                let sigma = CALM + self.gusty(x, d);
                let n = self.rng.n2();
                cell.slope[0] = cell.slope[0] * decay + sigma * kick * n[0];
                cell.slope[1] = cell.slope[1] * decay + sigma * kick * n[1];
                // the swell's bands, rolling in toward the shore
                let phase = k * d + omega * t + 0.6 * (x * 0.21).sin();
                let swell = SWELL_K * sigma * phase.sin();
                let s = [cell.slope[0], cell.slope[1] + swell];
                // the tilt that sends moonlight to the eye: the facet's normal halves the moon and the eye
                let (vx, vy, vz) = (-(x - self.x_eye), -d, EYE_M);
                let vl = (vx * vx + vy * vy + vz * vz).sqrt();
                let (nx, ny, nz) = (vx / vl, cel + vy / vl, sel + vz / vl);
                let need = [nx / nz, ny / nz];
                // (a facet's slope is minus its normal's lean)
                let e = [(-s[0]) - need[0], (-s[1]) - need[1]];
                let g = (-(e[0] * e[0] + e[1] * e[1]) / (2.0 * TOL * TOL)).exp() as f32;
                cell.glint = g;
                cell.lit = (cell.lit * fade).max(g);
                if g > 0.5 && cell.painted < 0.01 {
                    self.fresh.push([x - self.x_eye, d]);
                }
                if g > 0.5 || cell.painted > 0.0 {
                    cell.painted = (cell.painted + (dt / PAINT_S) as f32).min(1.0);
                }
                self.cells[i] = cell;
            }
        }
    }

    /// The field's four numbers a cell: painted, how lately lit, the seed across, the seed out plus two for
    /// each paint past the first.
    pub fn texels(&self, out: &mut Vec<[f32; 4]>) {
        out.clear();
        out.extend(self.cells.iter().map(|c| {
            [
                c.painted,
                c.lit,
                c.j[0],
                c.j[1] + 2.0 * (c.paint - 1) as f32,
            ]
        }));
    }

    /// How much of the water the moon could reach is painted (0..1): for the plans and the log.
    pub fn painted_share(&self) -> f32 {
        let n = self.cells.iter().filter(|c| c.painted > 0.5).count();
        n as f32 / self.cells.len() as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_road_lies_under_the_moon() {
        let mut lake = Lake::new(2.5, 7);
        for _ in 0..200 {
            lake.step(1.0 / 30.0);
        }
        // every painted cell is near the line to the moon (straight ahead)
        let mut on = 0;
        let mut off = 0;
        for row in 0..NV {
            let d = lake.row_m(row, 0.5);
            for col in 0..NU {
                let c = &lake.cells[row * NU + col];
                if c.painted > 0.0 {
                    let u = col as f64 + (c.lap * NU as i64) as f64;
                    let az = (u * lake.width_m(row) - lake.x_eye) / d;
                    if az.abs() < 0.12 {
                        on += 1;
                    } else {
                        off += 1;
                    }
                }
            }
        }
        assert!(on > 20, "the road painted {on} cells");
        assert!(
            off * 10 < on,
            "{off} painted off the road against {on} on it"
        );
    }

    #[test]
    fn a_gust_widens_the_road() {
        let run = |gust: bool| {
            let mut lake = Lake::new(2.5, 9);
            if gust {
                lake.gust([0.0, 8.0], 1.0);
            }
            for _ in 0..90 {
                lake.step(1.0 / 30.0);
            }
            lake.painted_share()
        };
        assert!(run(true) > run(false) * 1.3);
    }

    #[test]
    fn a_long_walk_keeps_the_water_whole() {
        // walk far along the shore: every row's cells stay the ones around the eye
        let mut lake = Lake::new(2.5, 11);
        for _ in 0..60 {
            lake.step(1.0 / 30.0);
        }
        assert!(lake.painted_share() > 0.0);
        for k in 0..400 {
            lake.x_eye = (k as f64) * 25.0;
            lake.step(1.0 / 30.0);
        }
        lake.x_eye = 10_000.0;
        lake.step(1.0 / 30.0);
        // every row's laps put the eye within half a ring of the middle
        for row in 0..NV {
            let ue = lake.x_eye / lake.width_m(row);
            for col in 0..NU {
                let c = &lake.cells[row * NU + col];
                let u = col as f64 + (c.lap * NU as i64) as f64;
                assert!(
                    (u - ue).abs() <= NU as f64 * 0.5 + 1.0,
                    "row {row} col {col}"
                );
            }
        }
    }
}
