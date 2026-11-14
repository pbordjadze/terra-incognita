//! The swell over Nazaré's floor: where its crests are and how tall, for one swell (a period and the
//! direction it comes from).
//!
//! A swell's speed depends on the depth under it and on its period (the linear dispersion relation,
//! ω² = g k tanh(k h)): over deep water a 16 s swell runs at about 25 m/s whatever the depth, but once the
//! water is shallower than about half its wavelength it feels the bottom and slows. A long swell feels it
//! deeper, so it bends more. Over the canyon the water stays deep almost to the beach; on the shelf
//! beside it the swell slows. A crest bends toward the slow water (refraction), so it bends away from the
//! canyon on both sides, and the two halves converge north of the canyon's head, off Praia do Norte.
//!
//! Here: the crests are lines of equal travel time (the eikonal |∇τ| = 1/c, solved by fast sweeping); the
//! energy follows rays at right angles to them, and where rays crowd together the waves grow (refraction)
//! as they do where they slow (shoaling, Green's law through the group speed); a wave breaks once it is
//! 0.78 of the depth (the McCowan limit). The groups of bigger waves (sets) travel at the group speed.

/// Gravity (m/s²).
const G: f64 = 9.81;
/// The swell's height in open water (m): a big winter swell.
pub const H0: f64 = 6.0;
/// A wave breaks when its height passes this share of the depth (McCowan).
pub const BREAK: f64 = 0.78;
/// The most the rays' crowding may raise a wave (a caustic's height is bounded by diffraction).
pub const REFRACT_MAX: f64 = 2.5;
/// How far apart the rays start, in cells across the swell, and how far each steps.
const RAY: f64 = 1.0;
const STEP: f64 = 0.5;
/// The fast sweep's rounds, and the passes of the crests' smoothing (a 3 × 3 box each).
const ROUNDS: usize = 4;
const SMOOTH: usize = 3;

/// The floor: the ground's height a cell (m, negative under the sea), w × h, a cell `cell_m` a side.
pub struct Floor {
    pub w: usize,
    pub h: usize,
    pub cell_m: f64,
    pub ground: Vec<f32>,
}

impl Floor {
    pub fn depth(&self, i: usize) -> f64 {
        (-self.ground[i] as f64).max(0.0)
    }
    pub fn sea(&self, i: usize) -> bool {
        self.ground[i] < -0.5
    }
}

/// One swell's field over the floor: a cell's travel times (s) at the phase and group speeds, the waves'
/// height (m) and whether they break there.
pub struct Field {
    pub period: f64,
    /// where the swell comes from, degrees clockwise from north (as a surf forecast gives it)
    pub from_deg: f64,
    pub tau: Vec<f64>,
    pub tau_group: Vec<f64>,
    pub height: Vec<f32>,
    /// how much the rays' crowding has raised the waves there (1 in the open sea; up to REFRACT_MAX)
    pub focus: Vec<f32>,
}

/// The wavenumber (1/m) of a wave of angular frequency `omega` over depth `h`: Fenton and McKee's
/// explicit form of the dispersion relation (within about 1.5%).
fn wavenumber(omega: f64, h: f64) -> f64 {
    let k0 = omega * omega / G;
    let x = k0 * h.max(0.05);
    let kh = x / x.powf(0.75).tanh().powf(2.0 / 3.0);
    kh / h.max(0.05)
}

/// The phase and group speeds (m/s) of a swell of `period` over depth `h`.
pub fn speeds(period: f64, h: f64) -> (f64, f64) {
    let omega = std::f64::consts::TAU / period;
    let k = wavenumber(omega, h);
    let c = omega / k;
    let kh2 = (2.0 * k * h).min(60.0);
    let n = 0.5 * (1.0 + kh2 / kh2.sinh().max(1e-9));
    (c, c * n)
}

impl Field {
    pub fn new(floor: &Floor, period: f64, from_deg: f64) -> Field {
        let n = floor.w * floor.h;
        let mut phase = vec![f64::INFINITY; n];
        let mut group = vec![f64::INFINITY; n];
        let mut slow = vec![0.0f64; n];
        let mut slow_g = vec![0.0f64; n];
        let mut speed = vec![0.0f64; n];
        for i in 0..n {
            if floor.sea(i) {
                let (c, cg) = speeds(period, floor.depth(i));
                speed[i] = c / floor.cell_m;
                slow[i] = floor.cell_m / c;
                slow_g[i] = floor.cell_m / cg;
            }
        }
        let d = heading(from_deg);
        let (c0, cg0) = speeds(period, 4000.0);
        seed(floor, d, floor.cell_m / c0, &mut phase);
        seed(floor, d, floor.cell_m / cg0, &mut group);
        sweep(floor, &slow, &mut phase, ROUNDS);
        // the sets' timing needs less care than the crests'
        sweep(floor, &slow_g, &mut group, 1);
        // the first arrival kinks where a faster path overtakes (the canyon's head waves): a real crest
        // there is a blend of both arrivals, diffraction rounding the corner over about a wavelength
        smooth_sea(floor, &mut phase, SMOOTH);
        smooth_sea(floor, &mut group, SMOOTH);
        // the rays see the floor smoothed over about a wavelength: a swell does not feel a single rock
        let mut smooth = speed.clone();
        blur(&mut smooth, floor.w, floor.h);
        blur(&mut smooth, floor.w, floor.h);
        for (s, &c) in smooth.iter_mut().zip(&speed) {
            if c <= 0.0 {
                *s = 0.0;
            }
        }
        let crowd = rays(floor, d, &smooth);
        let mut height = vec![0.0f32; n];
        let mut focus = vec![0.0f32; n];
        for i in 0..n {
            if !floor.sea(i) || !phase[i].is_finite() {
                continue;
            }
            let h = floor.depth(i);
            let (_, cg) = speeds(period, h);
            let shoal = (cg0 / cg).sqrt();
            let refract = (crowd[i] * RAY * STEP).sqrt().min(REFRACT_MAX);
            height[i] = (H0 * shoal * refract).min(BREAK * h) as f32;
            focus[i] = refract as f32;
        }
        Field {
            period,
            from_deg,
            tau: phase,
            tau_group: group,
            height,
            focus,
        }
    }
}

/// The direction the swell travels in (x east, y south), from where it comes from.
fn heading(from_deg: f64) -> [f64; 2] {
    let a = from_deg.to_radians();
    // coming from the bearing a, it travels toward a + 180°: east is sin, south is cos
    [-a.sin(), a.cos()]
}

/// The edges the swell enters by, as (cells along it, its inward direction).
fn edges(floor: &Floor, d: [f64; 2]) -> Vec<(Vec<usize>, [f64; 2])> {
    let (w, h) = (floor.w, floor.h);
    let mut out = Vec::new();
    if d[0] > 0.05 {
        out.push(((0..h).map(|y| y * w).collect(), [1.0, 0.0]));
    }
    if d[0] < -0.05 {
        out.push(((0..h).map(|y| y * w + w - 1).collect(), [-1.0, 0.0]));
    }
    if d[1] > 0.05 {
        out.push(((0..w).collect(), [0.0, 1.0]));
    }
    if d[1] < -0.05 {
        out.push(((0..w).map(|x| (h - 1) * w + x).collect(), [0.0, -1.0]));
    }
    out
}

/// The swell arriving over the entry edges as a plane wave (travel time p·d / c at open-sea speed).
fn seed(floor: &Floor, d: [f64; 2], slowness: f64, tau: &mut [f64]) {
    for (cells, _) in edges(floor, d) {
        for i in cells {
            if floor.sea(i) {
                let (x, y) = ((i % floor.w) as f64, (i / floor.w) as f64);
                tau[i] = tau[i].min((x * d[0] + y * d[1]) * slowness);
            }
        }
    }
    // from zero at the first crest in
    let lo = tau.iter().copied().fold(f64::INFINITY, f64::min);
    if lo.is_finite() {
        tau.iter_mut()
            .filter(|t| t.is_finite())
            .for_each(|t| *t -= lo);
    }
}

/// Fast sweeping (Zhao 2005): Godunov's upwind update of |∇τ| = s, in the four diagonal orders, `rounds`
/// times.
fn sweep(floor: &Floor, slowness: &[f64], tau: &mut [f64], rounds: usize) {
    let (w, h) = (floor.w, floor.h);
    let update = |tau: &mut [f64], x: usize, y: usize| {
        let i = y * w + x;
        let s = slowness[i];
        if s <= 0.0 {
            return;
        }
        let a = (if x > 0 { tau[i - 1] } else { f64::INFINITY }).min(if x + 1 < w {
            tau[i + 1]
        } else {
            f64::INFINITY
        });
        let b = (if y > 0 { tau[i - w] } else { f64::INFINITY }).min(if y + 1 < h {
            tau[i + w]
        } else {
            f64::INFINITY
        });
        let lo = a.min(b);
        if !lo.is_finite() {
            return;
        }
        let t = if (a - b).abs() >= s {
            lo + s
        } else {
            0.5 * (a + b + (2.0 * s * s - (a - b) * (a - b)).sqrt())
        };
        if t < tau[i] {
            tau[i] = t;
        }
    };
    for _ in 0..rounds {
        for y in 0..h {
            for x in 0..w {
                update(tau, x, y);
            }
            for x in (0..w).rev() {
                update(tau, x, y);
            }
        }
        for y in (0..h).rev() {
            for x in 0..w {
                update(tau, x, y);
            }
            for x in (0..w).rev() {
                update(tau, x, y);
            }
        }
    }
}

/// The phase speed at a point (cells), bilinear; None off the grid or over land.
fn speed_at(floor: &Floor, speed: &[f64], p: [f64; 2]) -> Option<f64> {
    let (w, h) = (floor.w, floor.h);
    if p[0] < 0.0 || p[1] < 0.0 || p[0] >= (w - 1) as f64 || p[1] >= (h - 1) as f64 {
        return None;
    }
    let (x0, y0) = (p[0] as usize, p[1] as usize);
    let (fx, fy) = (p[0] - x0 as f64, p[1] - y0 as f64);
    let i = y0 * w + x0;
    let c = [speed[i], speed[i + 1], speed[i + w], speed[i + w + 1]];
    if c.iter().any(|&v| v <= 0.0) {
        return None;
    }
    Some((c[0] * (1.0 - fx) + c[1] * fx) * (1.0 - fy) + (c[2] * (1.0 - fx) + c[3] * fx) * fy)
}

/// How the rays crowd. Each ray is the swell's own: it starts in the open sea, RAY apart across the
/// swell, and turns toward the slower water as it goes (dv/ds = −(∇c − (∇c·v) v) / c, Snell's law made
/// continuous), depositing once a STEP, so open water holds 1 / (RAY · STEP) a cell; more where rays
/// converge. Blurred a little (the rays are few, and a real caustic is softened by diffraction).
fn rays(floor: &Floor, d: [f64; 2], speed: &[f64]) -> Vec<f64> {
    let (w, h) = (floor.w as f64, floor.h as f64);
    let mut crowd = vec![0.0f64; floor.w * floor.h];
    let across = [-d[1], d[0]];
    let centre = [0.5 * w, 0.5 * h];
    let reach = 0.5 * w.hypot(h) + 2.0;
    let mut s = -reach;
    while s < reach {
        // from a line across the swell, upstream of the floor, straight in until it is over the sea
        let mut p = [
            centre[0] + across[0] * s - d[0] * reach,
            centre[1] + across[1] * s - d[1] * reach,
        ];
        let mut entered = false;
        for _ in 0..(4.0 * reach / STEP) as usize {
            if speed_at(floor, speed, p).is_some() {
                entered = true;
                break;
            }
            p = [p[0] + d[0] * STEP, p[1] + d[1] * STEP];
        }
        if entered {
            trace(floor, speed, p, d, &mut crowd);
        }
        s += RAY;
    }
    blur(&mut crowd, floor.w, floor.h);
    blur(&mut crowd, floor.w, floor.h);
    crowd
}

fn trace(floor: &Floor, speed: &[f64], mut p: [f64; 2], mut v: [f64; 2], crowd: &mut [f64]) {
    let w = floor.w;
    for _ in 0..(4 * (floor.w + floor.h)) {
        let Some(c) = speed_at(floor, speed, p) else {
            return;
        };
        let e = 0.5;
        let gx = speed_at(floor, speed, [p[0] + e, p[1]])
            .zip(speed_at(floor, speed, [p[0] - e, p[1]]))
            .map_or(0.0, |(a, b)| (a - b) / (2.0 * e));
        let gy = speed_at(floor, speed, [p[0], p[1] + e])
            .zip(speed_at(floor, speed, [p[0], p[1] - e]))
            .map_or(0.0, |(a, b)| (a - b) / (2.0 * e));
        let along = gx * v[0] + gy * v[1];
        v = [
            v[0] - (gx - along * v[0]) / c * STEP,
            v[1] - (gy - along * v[1]) / c * STEP,
        ];
        let m = v[0].hypot(v[1]);
        v = [v[0] / m, v[1] / m];
        let (x0, y0) = (p[0] as usize, p[1] as usize);
        let (fx, fy) = (p[0] - x0 as f64, p[1] - y0 as f64);
        crowd[y0 * w + x0] += (1.0 - fx) * (1.0 - fy);
        crowd[y0 * w + x0 + 1] += fx * (1.0 - fy);
        crowd[(y0 + 1) * w + x0] += (1.0 - fx) * fy;
        crowd[(y0 + 1) * w + x0 + 1] += fx * fy;
        p = [p[0] + v[0] * STEP, p[1] + v[1] * STEP];
    }
}

/// Times smoothed over the sea alone (a box `passes` times, land cells left out of each mean): a ramp is
/// kept as it is, a kink rounded.
fn smooth_sea(floor: &Floor, tau: &mut [f64], passes: usize) {
    let (w, h) = (floor.w, floor.h);
    let ok = |t: f64| t.is_finite();
    for _ in 0..passes {
        let mut tmp = vec![f64::INFINITY; tau.len()];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !ok(tau[i]) {
                    continue;
                }
                let (mut s, mut n) = (0.0, 0.0);
                for xx in x.saturating_sub(1)..(x + 2).min(w) {
                    let t = tau[y * w + xx];
                    if ok(t) {
                        s += t;
                        n += 1.0;
                    }
                }
                tmp[i] = s / n;
            }
        }
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if !ok(tmp[i]) {
                    continue;
                }
                let (mut s, mut n) = (0.0, 0.0);
                for yy in y.saturating_sub(1)..(y + 2).min(h) {
                    let t = tmp[yy * w + x];
                    if ok(t) {
                        s += t;
                        n += 1.0;
                    }
                }
                tau[i] = s / n;
            }
        }
    }
}

/// A 5 × 5 box blur, edges clamped.
fn blur(v: &mut [f64], w: usize, h: usize) {
    let r = 2i64;
    let mut tmp = vec![0.0; v.len()];
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for dx in -r..=r {
                let xx = (x as i64 + dx).clamp(0, w as i64 - 1) as usize;
                s += v[y * w + xx];
            }
            tmp[y * w + x] = s / (2 * r + 1) as f64;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let mut s = 0.0;
            for dy in -r..=r {
                let yy = (y as i64 + dy).clamp(0, h as i64 - 1) as usize;
                s += tmp[yy * w + x];
            }
            v[y * w + x] = s / (2 * r + 1) as f64;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deep_and_shallow_speeds() {
        // deep water: c = gT/2π, cg = c/2
        let (c, cg) = speeds(16.0, 4000.0);
        assert!((c - G * 16.0 / std::f64::consts::TAU).abs() < 0.3, "{c}");
        assert!((cg - c / 2.0).abs() < 0.2, "{cg}");
        // shallow: both sqrt(g h)
        let (c, cg) = speeds(16.0, 2.0);
        assert!((c - (G * 2.0f64).sqrt()).abs() < 0.2, "{c}");
        assert!((cg - c).abs() < 0.2, "{cg}");
    }

    #[test]
    fn a_flat_sea_is_a_plane_wave() {
        let floor = Floor {
            w: 60,
            h: 60,
            cell_m: 50.0,
            ground: vec![-500.0; 3600],
        };
        let f = Field::new(&floor, 12.0, 270.0);
        let (c, _) = speeds(12.0, 500.0);
        // from the west: τ grows with x at 1/c a metre, the same down a column
        let t = |x: usize, y: usize| f.tau[y * 60 + x];
        assert!(((t(40, 30) - t(10, 30)) - 30.0 * 50.0 / c).abs() < 0.5);
        assert!((t(40, 10) - t(40, 50)).abs() < 0.5);
        // and its height stays the open sea's
        let hh = f.height[30 * 60 + 30] as f64;
        assert!((hh - H0).abs() < 0.6, "{hh}");
    }
}

#[cfg(test)]
mod probe {
    use super::*;

    /// The field's heights as a picture, for looking at: `cargo test --release -p nazare -- --ignored
    /// --nocapture heights` (writes NAZARE_OUT or /tmp/nazare-heights.pgm; NAZARE_FROM, NAZARE_T set the swell).
    #[test]
    #[ignore]
    fn heights() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/data/");
        let b: serde_json::Value =
            serde_json::from_slice(&std::fs::read(format!("{dir}floor.json")).unwrap()).unwrap();
        let (w, h) = (
            b["w"].as_u64().unwrap() as usize,
            b["h"].as_u64().unwrap() as usize,
        );
        let bytes = std::fs::read(format!("{dir}floor.bin")).unwrap();
        let ground = bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| i16::from_le_bytes(*c) as f32 / 10.0)
            .collect();
        let floor = Floor {
            w,
            h,
            cell_m: 50.0,
            ground,
        };
        let var = |k: &str, d: f64| {
            std::env::var(k)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(d)
        };
        let t0 = std::time::Instant::now();
        let f = Field::new(&floor, var("NAZARE_T", 17.0), var("NAZARE_FROM", 290.0));
        println!("solved in {:.0} ms", t0.elapsed().as_secs_f64() * 1000.0);
        let hmax = f.height.iter().copied().fold(0.0f32, f32::max);
        println!("highest {hmax:.1} m");
        let surf: Vec<f32> = (0..w * h)
            .filter(|&i| floor.sea(i) && f.height[i] as f64 >= 0.99 * BREAK * floor.depth(i))
            .map(|i| f.focus[i])
            .collect();
        let mean = surf.iter().sum::<f32>() / surf.len().max(1) as f32;
        println!("surf cells {} mean focus {mean:.2}", surf.len());
        // along the coast, north to south: the most focus within 10 cells of the shore, by 20-row bands
        for band in (300..700).step_by(20) {
            let mut best = (0.0f32, 0usize);
            for y in band..band + 20 {
                let row = y * w;
                let shore = (0..w).rev().find(|&x| floor.sea(row + x)).unwrap_or(0);
                for x in shore.saturating_sub(10)..=shore {
                    if f.focus[row + x] > best.0 {
                        best = (f.focus[row + x], x);
                    }
                }
            }
            println!("rows {band}: focus {:.2} at x {}", best.0, best.1);
        }
        let mut out = format!("P5 {w} {h} 255\n").into_bytes();
        out.extend(
            f.height
                .iter()
                .map(|&x| (x / 24.0 * 255.0).min(255.0) as u8),
        );
        let path = std::env::var("NAZARE_OUT").unwrap_or("/tmp/nazare-heights.pgm".into());
        std::fs::write(&path, out).unwrap();
        // and the crests: where each one is at one moment
        let mut out = format!("P5 {w} {h} 255\n").into_bytes();
        out.extend(f.tau.iter().map(|&t| {
            if t.is_finite() {
                (127.5 + 127.0 * (std::f64::consts::TAU * t / f.period).cos()) as u8
            } else {
                0
            }
        }));
        std::fs::write(format!("{path}.crests.pgm"), out).unwrap();
    }
}
