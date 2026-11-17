//! The grassland as a field of grass and soil water (Klausmeier's model on flat ground, as Meron and
//! Getzin use for the fairy circles): rain falls everywhere; grass takes up water faster the more grass
//! there is (its roots and shade), and dies back at a steady rate; soil water spreads far, because the
//! grasses' roots reach out and draw it in from nearby, while grass spreads only slowly. That difference
//! of reach (scale-dependent feedback) is what breaks a uniform sward into a lattice of bare discs, each
//! ringed by grass that drinks from it. More rain closes the discs; less widens them, until the grass
//! breaks into spots.
//!
//! Units are the model's own (length in `H` per cell, time in the model's): `w` water, `b` grass.
//! A torus, so the screen's edges wrap without a seam.

/// The grass's mortality (Klausmeier's m).
pub const M: f32 = 0.45;
/// The rain when it is scarce (the circles' regime, just under where uniform grass is stable) and when it
/// is not (the scarce rule switched off: the sward closes).
pub const RAIN_SCARCE: f32 = 1.3;
pub const RAIN_PLENTY: f32 = 1.9;
/// How far water spreads (the roots drawing it from nearby) relative to the grass; off, it spreads no
/// farther than the grass does, and nothing makes the discs.
pub const D_ROOTS: f32 = 100.0;
pub const D_NEAR: f32 = 1.0;
/// One cell, in the model's lengths.
pub const H: f32 = 1.5;
/// The step of the grass (the water's spreading takes smaller steps within it).
pub const DT: f32 = 0.025;
/// The opening lattice: one disc to `SPACING` cells, a whole number of them round the torus.
pub const SPACING: f32 = 12.0;

/// The rules as the sheet's switches name them, in `Veld::on`'s order.
pub const RULES: [&str; 2] = ["Roots draw water", "Rain is scarce"];

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
}

pub struct Veld {
    pub w: usize,
    pub h: usize,
    /// grass (biomass) and soil water, a cell each
    pub b: Vec<f32>,
    pub water: Vec<f32>,
    /// extra rain on a cell (the finger's shower), decaying
    pub shower: Vec<f32>,
    tmp: Vec<f32>,
    /// 0..1: how deep a drought is on (a shake), easing back
    pub drought: f32,
    pub on: [bool; 2],
    pub rng: Rng,
}

/// The uniform sward's grass at rain `a` (the vegetated steady state).
pub fn sward(a: f32) -> f32 {
    let a = a.max(2.0 * M + 1e-3);
    (a + (a * a - 4.0 * M * M).sqrt()) / (2.0 * M)
}

impl Veld {
    /// A sward with a jittered hexagonal lattice of bare discs already in it (where the rule would
    /// take it after a long run), so the first frame shows the circles.
    pub fn new(w: usize, h: usize, seed: u64) -> Veld {
        let mut rng = Rng(seed | 1);
        let bs = sward(RAIN_SCARCE);
        let mut b = vec![bs; w * h];
        let row = SPACING * 3f32.sqrt() / 2.0;
        let rows = (h as f32 / row).round() as usize;
        let cols = (w as f32 / SPACING).round() as usize;
        for j in 0..rows {
            for i in 0..cols {
                let cx = (i as f32 + 0.5 * (j % 2) as f32) * SPACING + (rng.f() - 0.5) * 1.6;
                let cy = j as f32 * row + (rng.f() - 0.5) * 1.6;
                let r = 2.0 + rng.f() * 0.8;
                dig(&mut b, w, h, [cx, cy], r);
            }
        }
        let water = b.iter().map(|&g| M / g.max(0.3)).collect();
        Veld {
            w,
            h,
            tmp: vec![0.0; w * h],
            shower: vec![0.0; w * h],
            b,
            water,
            drought: 0.0,
            on: [true, true],
            rng,
        }
    }

    /// The rain everywhere now (before the finger's shower).
    pub fn rain(&self) -> f32 {
        let base = if self.on[1] { RAIN_SCARCE } else { RAIN_PLENTY };
        base - 0.3 * self.drought
    }

    /// Rain on a disc of cells round `c` (cells), `amount` a second's worth.
    pub fn shower_at(&mut self, c: [f32; 2], r: f32, amount: f32) {
        let (w, h) = (self.w as i32, self.h as i32);
        let ri = r.ceil() as i32 + 1;
        let (x0, y0) = (c[0].floor() as i32, c[1].floor() as i32);
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                let (x, y) = (x0 + dx, y0 + dy);
                let fx = x as f32 + 0.5 - c[0];
                let fy = y as f32 + 0.5 - c[1];
                let k = (-(fx * fx + fy * fy) / (r * r)).exp();
                let i = (y.rem_euclid(h) * w + x.rem_euclid(w)) as usize;
                self.shower[i] = (self.shower[i] + amount * k).min(2.5);
            }
        }
    }

    /// Run the rule for `steps` steps of `DT`.
    pub fn run(&mut self, steps: usize) {
        for _ in 0..steps {
            self.spread_water();
            self.grow();
        }
    }

    /// The water spreads (explicitly, in as many small steps as its reach needs to stay stable).
    fn spread_water(&mut self) {
        let d = if self.on[0] { D_ROOTS } else { D_NEAR };
        let k_total = DT * d / (H * H);
        let n = (k_total / 0.22).ceil().max(1.0) as usize;
        let k = k_total / n as f32;
        for _ in 0..n {
            laplace(&self.water, self.w, self.h, &mut self.tmp);
            for (wv, l) in self.water.iter_mut().zip(&self.tmp) {
                *wv += k * l;
            }
        }
    }

    /// Rain, uptake and growth, a cell at a time; then the grass spreads a little.
    fn grow(&mut self) {
        let a0 = self.rain();
        let fade = (-DT / 40.0).exp();
        laplace(&self.b, self.w, self.h, &mut self.tmp);
        let kb = DT / (H * H);
        for i in 0..self.b.len() {
            let a = a0 + self.shower[i];
            self.shower[i] *= fade;
            let b = self.b[i];
            let b2 = b * b;
            // water: rain in, evaporation and uptake out (implicit in the water, so a thick sward is stable)
            let w = (self.water[i] + DT * a) / (1.0 + DT * (1.0 + b2));
            self.water[i] = w;
            let nb = b + DT * (w * b2 - M * b) + kb * self.tmp[i];
            self.b[i] = nb.clamp(0.0, 12.0);
        }
        // a seed now and then on bare ground, so a lost sward can come back
        for _ in 0..4 {
            let i = (self.rng.next_u64() % self.b.len() as u64) as usize;
            self.b[i] += 0.02 * self.rng.f();
        }
    }
}

/// A bare disc dug into the grass round `c` (cells, wrapping), radius `r` cells.
fn dig(b: &mut [f32], w: usize, h: usize, c: [f32; 2], r: f32) {
    let ri = (r * 3.0) as i32;
    let (x0, y0) = (c[0].round() as i32, c[1].round() as i32);
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let (x, y) = (x0 + dx, y0 + dy);
            let fx = x as f32 - c[0];
            let fy = y as f32 - c[1];
            let k = (-(fx * fx + fy * fy) / (2.0 * r * r)).exp();
            let i = (y.rem_euclid(h as i32) as usize) * w + x.rem_euclid(w as i32) as usize;
            b[i] *= 1.0 - 0.8 * k;
        }
    }
}

/// The five-point Laplacian (in cells) round the torus.
fn laplace(f: &[f32], w: usize, h: usize, out: &mut [f32]) {
    for y in 0..h {
        let up = ((y + h - 1) % h) * w;
        let dn = ((y + 1) % h) * w;
        let row = y * w;
        for x in 0..w {
            let l = if x == 0 { w - 1 } else { x - 1 };
            let r = if x + 1 == w { 0 } else { x + 1 };
            out[row + x] = f[row + l] + f[row + r] + f[up + x] + f[dn + x] - 4.0 * f[row + x];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lattice_holds_and_rain_closes_it() {
        let mut v = Veld::new(96, 208, 7);
        v.run(600);
        let bs = sward(RAIN_SCARCE);
        let bare = v.b.iter().filter(|&&g| g < 0.55 * bs).count();
        assert!(bare > 200, "the discs held: {bare}");
        let mut dry = Veld::new(96, 208, 7);
        dry.run(600);
        v.on[1] = false;
        v.run(1200);
        let bare2 = v.b.iter().filter(|&&g| g < 0.55 * bs).count();
        assert!(
            bare2 < bare / 4,
            "plenty of rain closed them: {bare} -> {bare2}"
        );
        dry.on[0] = false;
        dry.run(1200);
        let bare3 = dry.b.iter().filter(|&&g| g < 0.55 * bs).count();
        assert!(
            bare3 < bare / 4,
            "without the roots' reach they filled: {bare} -> {bare3}"
        );
    }
}
