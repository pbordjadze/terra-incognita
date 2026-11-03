//! The Earth's free oscillations: what rings when it is struck (SOURCES.md says what is measured).
//!
//! The fundamental spheroidal modes 0S0 and 0S2..0S12 of PREM (Dziewonski & Anderson 1981): their
//! frequency and quality factor. A point strike at `s` excites each degree `l` as P_l(cos γ) about it
//! (the addition theorem summed over the 2l+1 singlets), so the surface's radial motion at `x` is
//! sum_l A_l P_l(x·s) cos(2π f_l t) e^(-π f_l t / Q_l). The ear hears that sum at the point under the
//! screen's centre, the singlets split by the Earth's turning (each m its own pitch, so the chord beats).

/// (degree l, frequency in mHz, Q). PREM.
pub const MODES: [(u32, f64, f64); 12] = [
    (0, 0.8147, 5327.0),
    (2, 0.3093, 509.6),
    (3, 0.4686, 417.6),
    (4, 0.6470, 373.4),
    (5, 0.8404, 355.9),
    (6, 1.0382, 347.4),
    (7, 1.2318, 341.4),
    (8, 1.4135, 335.5),
    (9, 1.5783, 328.6),
    (10, 1.7265, 320.8),
    (11, 1.8638, 312.6),
    (12, 1.9905, 304.6),
];

/// One factor takes every mode into hearing (0S2 to D4, 293.7 Hz, where a phone's speaker sings): the
/// chord's ratios are the Earth's.
pub const PITCH: f64 = 293.66 / 0.3093e-3;
/// The drawn motion runs this much slower than the heard (0S2 swells about once in two seconds).
pub const SLOW: f64 = 600.0;
/// Every ring lasts this many times as long as the Earth's quality factors give at that pitch (the
/// ratios of the decays are kept: the high degrees go first), the breathing mode held to `BREATH`.
pub const LINGER: f64 = 5.0;

/// A mode's frequency heard (Hz) and its decay (seconds to fall by e) on the device.
pub fn heard(i: usize) -> (f64, f64) {
    let (_, mhz, q) = MODES[i];
    let f = mhz * 1e-3 * PITCH;
    let tau = q / (std::f64::consts::PI * f) * LINGER;
    (f, if MODES[i].0 == 0 { tau.min(BREATH) } else { tau })
}

/// The breathing mode's decay at most (its Q of 5300 would hold it as a drone over all the rest).
pub const BREATH: f64 = 1.6;

/// How strongly a strike excites each degree (drawn: an even, mallet-like spectrum falling with l;
/// the breathing mode gently).
pub fn weight(i: usize) -> f64 {
    let l = MODES[i].0 as f64;
    if l == 0.0 {
        0.05
    } else {
        (2.0 * l + 1.0) / (4.0 * std::f64::consts::PI) * 2.6 / l.powf(1.25)
    }
}

/// The rotational splitting of degree l: Hz heard per step of m. 0S2's from PREM's computed singlets
/// (4.6 µHz apart); the others fall as 1/(l(l+1)), the Coriolis form (drawn).
pub fn split(i: usize) -> f64 {
    let l = MODES[i].0 as f64;
    if l == 0.0 {
        return 0.0;
    }
    4.6e-6 * 6.0 / (l * (l + 1.0)) * PITCH
}

/// Legendre polynomials P_0..P_lmax at x.
pub fn legendre(x: f64, out: &mut [f64]) {
    out[0] = 1.0;
    if out.len() > 1 {
        out[1] = x;
    }
    for l in 2..out.len() {
        let lf = l as f64;
        out[l] = ((2.0 * lf - 1.0) * x * out[l - 1] - (lf - 1.0) * out[l - 2]) / lf;
    }
}

/// Schmidt semi-normalised associated Legendre functions S_l^m(cos θ) for m = 0..=l, so that
/// P_l(cos γ) = sum_m S_l^m(cos θ_a) S_l^m(cos θ_b) cos(m Δφ) (the addition theorem).
pub fn schmidt(l: usize, x: f64) -> Vec<f64> {
    // unnormalised P_l^m by the usual recurrences, then the Schmidt factor
    let s = (1.0 - x * x).max(0.0).sqrt();
    let mut out = vec![0.0; l + 1];
    for m in 0..=l {
        // P_m^m
        let mut pmm = 1.0;
        for k in 1..=m {
            pmm *= (2 * k - 1) as f64 * s;
        }
        let val = if l == m {
            pmm
        } else {
            let mut p0 = pmm;
            let mut p1 = x * (2 * m + 1) as f64 * pmm;
            for ll in (m + 2)..=l {
                let p2 = ((2 * ll - 1) as f64 * x * p1 - (ll + m - 1) as f64 * p0) / (ll - m) as f64;
                p0 = p1;
                p1 = p2;
            }
            p1
        };
        // sqrt(2 (l-m)!/(l+m)!) for m > 0
        let mut f = 1.0f64;
        for k in (l - m + 1)..=(l + m) {
            f /= k as f64;
        }
        let norm = if m == 0 { 1.0 } else { (2.0 * f).sqrt() };
        out[m] = val * norm;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn addition_theorem() {
        let (ta, pa, tb, pb) = (0.7f64, 0.3f64, 2.1f64, -1.4f64);
        let ga = (ta.sin() * pa.cos(), ta.sin() * pa.sin(), ta.cos());
        let gb = (tb.sin() * pb.cos(), tb.sin() * pb.sin(), tb.cos());
        let c = ga.0 * gb.0 + ga.1 * gb.1 + ga.2 * gb.2;
        let mut p = [0.0; 13];
        legendre(c, &mut p);
        for l in [2usize, 5, 12] {
            let a = schmidt(l, ta.cos());
            let b = schmidt(l, tb.cos());
            let s: f64 = (0..=l).map(|m| a[m] * b[m] * (m as f64 * (pa - pb)).cos()).sum();
            assert!((s - p[l]).abs() < 1e-9, "l {l}: {s} vs {}", p[l]);
        }
    }
}
