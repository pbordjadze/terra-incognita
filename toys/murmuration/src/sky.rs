//! The real sky: the light of the sun, wherever it really is, scattered once by the air's molecules
//! (Rayleigh) and its haze (Mie) and filtered by its ozone, on a round Earth whose own shadow rises in
//! the east after sunset. Measured constants (Bruneton's precomputed atmospheric scattering, 2017,
//! after Bucholtz 1995 and Penndorf 1957; ozone after the US Standard Atmosphere); drawn: one bounce
//! instead of all of them, the sun's colour white above the air, and the eye's adaptation.
//!
//! The sky is symmetric about the sun's vertical, so it is a table of two angles: the view's
//! elevation and its azimuth from the sun. The table is worked out on the CPU a few rows a frame
//! (`Sky::step`), double-buffered, and handed to the GPU when it is whole, so the sun can move with
//! no frame waiting for it.

pub const AZ: usize = 64;
pub const EL: usize = 128;
/// The lowest elevation in the table, degrees (a little under the horizon, so it blends).
pub const EL_MIN: f64 = -2.0;

const R_GROUND: f64 = 6_360_000.0;
const R_TOP: f64 = 6_420_000.0;
const OBSERVER: f64 = 40.0;
const RAYLEIGH: [f64; 3] = [5.802e-6, 13.558e-6, 33.1e-6];
const RAYLEIGH_H: f64 = 8000.0;
const MIE_SCATTER: f64 = 3.996e-6;
const MIE_EXTINCT: f64 = 4.44e-6;
const MIE_H: f64 = 1200.0;
const MIE_G: f64 = 0.8;
const OZONE: [f64; 3] = [0.650e-6, 1.881e-6, 0.085e-6];
const VIEW_STEPS: usize = 40;
const SUN_STEPS: usize = 16;

/// The view elevation (degrees) of row `j`: rows are packed toward the horizon.
pub fn row_elevation(j: usize) -> f64 {
    let v = (j as f64 + 0.5) / EL as f64;
    EL_MIN + (90.0 - EL_MIN) * v * v
}

fn densities(h: f64) -> (f64, f64, f64) {
    let h = h.max(0.0);
    let ozone = (1.0 - (h - 25_000.0).abs() / 15_000.0).max(0.0);
    ((-h / RAYLEIGH_H).exp(), (-h / MIE_H).exp(), ozone)
}

fn extinction(d: (f64, f64, f64)) -> [f64; 3] {
    let mut e = [0.0; 3];
    for c in 0..3 {
        e[c] = RAYLEIGH[c] * d.0 + MIE_EXTINCT * d.1 + OZONE[c] * d.2;
    }
    e
}

/// Distance along `dir` (unit) from `p` to the sphere of radius `r`, outward (None if missed).
fn to_sphere(p: [f64; 3], dir: [f64; 3], r: f64) -> Option<(f64, f64)> {
    let b = p[0] * dir[0] + p[1] * dir[1] + p[2] * dir[2];
    let c = p[0] * p[0] + p[1] * p[1] + p[2] * p[2] - r * r;
    let disc = b * b - c;
    if disc < 0.0 {
        return None;
    }
    let s = disc.sqrt();
    Some((-b - s, -b + s))
}

fn add(p: [f64; 3], d: [f64; 3], t: f64) -> [f64; 3] {
    [p[0] + d[0] * t, p[1] + d[1] * t, p[2] + d[2] * t]
}

fn len(p: [f64; 3]) -> f64 {
    (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt()
}

/// Optical depth from `p` toward the sun, or None in the Earth's shadow.
fn sun_depth(p: [f64; 3], sun: [f64; 3]) -> Option<[f64; 3]> {
    if let Some((t0, _)) = to_sphere(p, sun, R_GROUND) {
        if t0 > 0.0 {
            return None;
        }
    }
    let (_, t1) = to_sphere(p, sun, R_TOP)?;
    let dt = t1 / SUN_STEPS as f64;
    let mut tau = [0.0; 3];
    for i in 0..SUN_STEPS {
        let q = add(p, sun, (i as f64 + 0.5) * dt);
        let e = extinction(densities(len(q) - R_GROUND));
        for c in 0..3 {
            tau[c] += e[c] * dt;
        }
    }
    Some(tau)
}

/// The radiance (linear, the sun's irradiance above the air taken as 1) seen along `view` with the
/// sun along `sun`, both unit vectors with y up, from the observer.
pub fn radiance(view: [f64; 3], sun: [f64; 3], ambient: [f64; 3]) -> [f64; 3] {
    let o = [0.0, R_GROUND + OBSERVER, 0.0];
    let (_, mut t_end) = match to_sphere(o, view, R_TOP) {
        Some(t) => t,
        None => return [0.0; 3],
    };
    if let Some((g0, _)) = to_sphere(o, view, R_GROUND) {
        if g0 > 0.0 {
            t_end = t_end.min(g0);
        }
    }
    let mu = view[0] * sun[0] + view[1] * sun[1] + view[2] * sun[2];
    let phase_r = 3.0 / (16.0 * std::f64::consts::PI) * (1.0 + mu * mu);
    let g = MIE_G;
    let phase_m = 3.0 / (8.0 * std::f64::consts::PI) * ((1.0 - g * g) * (1.0 + mu * mu))
        / ((2.0 + g * g) * (1.0 + g * g - 2.0 * g * mu).powf(1.5));
    let mut tau_view = [0.0; 3];
    let mut sum = [0.0; 3];
    let mut t_prev = 0.0;
    for i in 0..VIEW_STEPS {
        // Quadratic spacing: fine near the eye, where the haze is.
        let k = (i as f64 + 1.0) / VIEW_STEPS as f64;
        let t = t_end * k * k;
        let dt = t - t_prev;
        let tm = 0.5 * (t + t_prev);
        t_prev = t;
        let p = add(o, view, tm);
        let d = densities(len(p) - R_GROUND);
        let e = extinction(d);
        for c in 0..3 {
            tau_view[c] += e[c] * dt * 0.5;
        }
        // The light of the rest of the sky, scattered once more (the bounces a single scattering
        // leaves out, taken as the sky's mean light from every side; it is what blues the twilight).
        for c in 0..3 {
            let tr = (-tau_view[c]).exp();
            sum[c] += tr * (RAYLEIGH[c] * d.0 + MIE_SCATTER * d.1) * 0.5 * ambient[c] * dt;
        }
        if let Some(ts) = sun_depth(p, sun) {
            for c in 0..3 {
                let tr = (-(tau_view[c] + ts[c])).exp();
                sum[c] += tr * (RAYLEIGH[c] * d.0 * phase_r + MIE_SCATTER * d.1 * phase_m) * dt;
            }
        }
        for c in 0..3 {
            tau_view[c] += e[c] * dt * 0.5;
        }
    }
    sum
}

/// The transmittance of the whole air along `dir` from the observer (the sun's disc's colour).
pub fn transmittance(dir: [f64; 3]) -> [f64; 3] {
    let o = [0.0, R_GROUND + OBSERVER, 0.0];
    match sun_depth(o, dir) {
        Some(t) => [(-t[0]).exp(), (-t[1]).exp(), (-t[2]).exp()],
        None => [0.0; 3],
    }
}

/// The table, worked out a few rows at a time.
pub struct Sky {
    /// The sun's elevation (degrees) the finished table is for, and the table (rgb, linear).
    pub ready_for: Option<f64>,
    pub table: Vec<[f32; 4]>,
    pub changed: bool,
    work: Vec<[f32; 4]>,
    work_for: f64,
    work_ambient: [f64; 3],
    next_row: usize,
}

impl Sky {
    pub fn new() -> Sky {
        Sky {
            ready_for: None,
            table: vec![[0.0; 4]; AZ * EL],
            changed: false,
            work: vec![[0.0; 4]; AZ * EL],
            work_for: 0.0,
            work_ambient: [0.0; 3],
            next_row: EL,
        }
    }

    fn row(work: &mut [[f32; 4]], j: usize, sun_el: f64, ambient: [f64; 3]) {
        let el = row_elevation(j).to_radians();
        let se = sun_el.to_radians();
        let sun = [0.0, se.sin(), se.cos()];
        for i in 0..AZ {
            let az = (i as f64 + 0.5) / AZ as f64 * std::f64::consts::PI;
            let view = [az.sin() * el.cos(), el.sin(), az.cos() * el.cos()];
            let r = radiance(view, sun, ambient);
            work[j * AZ + i] = [r[0] as f32, r[1] as f32, r[2] as f32, 1.0];
        }
    }

    /// Works out the whole table now (at the start).
    pub fn compute_now(&mut self, sun_el: f64) {
        let mut ambient = [0.0; 3];
        for _ in 0..3 {
            for j in 0..EL {
                Self::row(&mut self.work, j, sun_el, ambient);
            }
            std::mem::swap(&mut self.work, &mut self.table);
            ambient = self.ambient();
        }
        self.ready_for = Some(sun_el);
        self.changed = true;
        self.next_row = EL;
    }

    /// Works on `rows` rows toward the sun at `sun_el`, starting a new table when the sun has moved
    /// far enough from the one shown.
    pub fn step(&mut self, sun_el: f64, rows: usize) {
        if self.next_row >= EL {
            match self.ready_for {
                Some(e) if (e - sun_el).abs() < 0.05 => return,
                _ => {}
            }
            self.work_for = sun_el;
            self.work_ambient = self.ambient();
            self.next_row = 0;
        }
        let end = (self.next_row + rows).min(EL);
        for j in self.next_row..end {
            Self::row(&mut self.work, j, self.work_for, self.work_ambient);
        }
        self.next_row = end;
        if end == EL {
            std::mem::swap(&mut self.work, &mut self.table);
            self.ready_for = Some(self.work_for);
            self.changed = true;
        }
    }

    /// The sky's mean radiance over the upper hemisphere (rows weighted by their solid angle).
    pub fn ambient(&self) -> [f64; 3] {
        let mut s = [0.0f64; 3];
        let mut n = 0.0f64;
        for j in 0..EL {
            let e = row_elevation(j);
            if e < 0.0 {
                continue;
            }
            let v0 = (j as f64) / EL as f64;
            let v1 = (j as f64 + 1.0) / EL as f64;
            let dh = (EL_MIN + (90.0 - EL_MIN) * v1 * v1 - (EL_MIN + (90.0 - EL_MIN) * v0 * v0))
                .to_radians();
            let w = e.to_radians().cos() * dh;
            for i in 0..AZ {
                let c = self.table[j * AZ + i];
                for k in 0..3 {
                    s[k] += c[k] as f64 * w;
                }
                n += w;
            }
        }
        [s[0] / n, s[1] / n, s[2] / n]
    }

    /// The table's mean luminance over the middle sky (5° to 40° up, all round): what the eye adapts to.
    pub fn adaptation(&self) -> f64 {
        let (mut s, mut n) = (0.0f64, 0.0f64);
        for j in 0..EL {
            let e = row_elevation(j);
            if (5.0..40.0).contains(&e) {
                for i in 0..AZ {
                    let c = self.table[j * AZ + i];
                    s += (0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2]) as f64;
                    n += 1.0;
                }
            }
        }
        s / n.max(1.0)
    }

    /// The radiance toward a view elevation (degrees) and azimuth from the sun (degrees, 0..180),
    /// read from the table as the shader reads it (nearest row, linear across).
    pub fn lookup(&self, el: f64, daz: f64) -> [f32; 4] {
        let v = ((el - EL_MIN) / (90.0 - EL_MIN)).clamp(0.0, 1.0).sqrt();
        let j = ((v * EL as f64) as usize).min(EL - 1);
        let i = ((daz.abs() / 180.0 * AZ as f64) as usize).min(AZ - 1);
        self.table[j * AZ + i]
    }
}

impl Default for Sky {
    fn default() -> Self {
        Self::new()
    }
}

/// f32 to IEEE half (round to nearest; no NaNs here).
pub fn half(x: f32) -> u16 {
    let b = x.to_bits();
    let sign = ((b >> 16) & 0x8000) as u16;
    let exp = ((b >> 23) & 0xff) as i32 - 127 + 15;
    let man = b & 0x7f_ffff;
    if exp <= 0 {
        if exp < -10 {
            return sign;
        }
        let m = (man | 0x80_0000) >> (1 - exp);
        return sign | ((m + 0x1000) >> 13) as u16;
    }
    if exp >= 31 {
        return sign | 0x7c00;
    }
    sign | (((exp as u32) << 10) + ((man + 0x1000) >> 13)) as u16
}

#[cfg(test)]
mod tests {
    #[test]
    fn dusk_sky_is_blue_above_and_warm_at_the_sun() {
        let mut s = super::Sky::new();
        s.compute_now(2.0);
        let z = s.lookup(89.0, 0.0);
        let h = s.lookup(1.0, 0.0);
        assert!(z[2] > z[0] * 1.3, "zenith {z:?}");
        assert!(h[0] > h[2] * 5.0, "horizon at the sun {h:?}");
        s.compute_now(-4.0);
        assert!(s.adaptation() < 0.001, "twilight is dim");
    }
}
