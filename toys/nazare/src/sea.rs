//! The swell over Nazaré's floor: where its crests are and how tall, for one swell (a period and the
//! direction it comes from), as two trains of waves that cross off Praia do Norte.
//!
//! A swell's speed depends on the depth under it and on its period (the linear dispersion relation,
//! ω² = g k tanh(k h)): over deep water a 17 s swell runs at about 26 m/s whatever the depth, but once the
//! water is shallower than about half its wavelength it feels the bottom and slows, so a crest bends toward
//! the shallower water (refraction).
//!
//! The canyon's last kilometres are a wall to a long swell: where the shelf, 15 to 20 m deep, drops into the
//! canyon over a few tens of metres, a swell longer than about 7 s is mostly turned back rather than carried
//! across (Hope et al. 2026, measured from buoys over the canyon and cameras on the cliff). So the swell on the
//! shelf north of the canyon runs along its edge to the headland, and what meets the edge is thrown back
//! north toward the beach. The two trains, the swell arriving and the swell the edge turned back, cross off
//! Praia do Norte; where their crests meet they stand up into peaks, the pyramids the surfers ride.
//!
//! Here: the arriving swell's crests are lines of equal travel time (the eikonal |∇τ| = 1/c, solved by fast
//! sweeping), the canyon a wall to it when it reflects. Its energy follows rays that turn toward slower water
//! (Snell's law made continuous): where the rays crowd, the waves grow. The turned-back train leaves the
//! wall's face where the arriving crests meet it, at the times they meet it (Huygens: each point of the face a
//! source, their envelope the reflected crest), so its crests are a second eikonal from the face; its height
//! is the arriving wave's at the face, spreading as it goes. Both grow as they slow (shoaling, through the
//! group speed). The groups of bigger waves (sets) travel at the group speed.

mod solve;
pub use solve::Solve;

/// Gravity (m/s²).
const G: f64 = 9.81;
/// The swell's height in open water (m), crest to trough: a big winter swell (the buoys off Portugal see a
/// significant height near 10 m about yearly, Hope et al. 2026).
pub const H0: f64 = 7.0;
/// A wave breaks when its height passes this share of the depth. A sea crossed by two trains stands steeper
/// before it breaks than one train alone (McAllister et al. 2024, in Hope et al. 2026): the McCowan 0.78 of
/// a single train, raised.
pub const BREAK: f64 = 1.0;
/// The depth at the canyon's rim (m): the shelf's at its edge (Hope et al. 2026: 15 to 20 m).
const RIM_M: f64 = 20.0;
/// Inside the break, the broken waves' height over the depth.
const SPENT: f64 = 1.2;
/// How far the turned-back train spreads before its height halves (m): it leaves the edge as a fan.
const SPREAD_M: f64 = 1200.0;
/// The most the rays' crowding may raise a train (a caustic's height is bounded by diffraction).
pub const REFRACT_MAX: f64 = 2.6;
/// How far apart the rays start, in cells across the swell, and how far each steps.
const RAY: f64 = 0.5;
const STEP: f64 = 0.5;
/// The fast sweep's rounds, and the passes of the crests' smoothing (a 3 × 3 box each).
const ROUNDS: usize = 4;
const SMOOTH: usize = 3;
/// Swell this long or longer is turned back by the canyon's edge, shorter crosses it (s): the reflection's
/// cut-off is near 7 s at Nazaré (Hope et al. 2026), as at La Jolla's canyons (7 to 9 s). A clean step here.
pub const TURNS: f64 = 7.0;

/// The floor: the ground's height a cell (m, negative under the sea), w × h, a cell `cell_m` a side, and
/// which cells are the canyon's wall (the incised canyon within a few km of its head).
pub struct Floor {
    pub w: usize,
    pub h: usize,
    pub cell_m: f64,
    pub ground: Vec<f32>,
    pub canyon: Vec<bool>,
}

impl Floor {
    pub fn depth(&self, i: usize) -> f64 {
        (-self.ground[i] as f64).max(0.0)
    }
    pub fn sea(&self, i: usize) -> bool {
        self.ground[i] < -0.5
    }
}

/// One swell's field over the floor: a cell's travel times (s) for the arriving train and for the one the
/// canyon turned back, at the group speed for the sets, and each train's height (m, crest to trough).
pub struct Field {
    pub period: f64,
    /// where the swell comes from, degrees clockwise from north (as a surf forecast gives it)
    pub from_deg: f64,
    pub tau: Vec<f64>,
    pub tau_back: Vec<f64>,
    pub tau_group: Vec<f64>,
    pub height: Vec<f32>,
    pub height_back: Vec<f32>,
    /// which of `RULES` held when it was solved
    pub rules: [bool; 2],
}

/// The rules the sheet can switch off: the shallows slowing the swell (off, it runs at its deep-water speed
/// over every depth, so nothing bends or grows), and the canyon's edge turning the long swell back (off, it
/// crosses the canyon as a short swell does, and no second train meets the first).
pub const RULES: [&str; 2] = [
    "Shallow water slows the swell",
    "The canyon's edge turns it back",
];

/// The wavenumber (1/m) of a wave of angular frequency `omega` over depth `h`: Fenton and McKee's
/// explicit form of the dispersion relation (within about 1.5%).
fn wavenumber(omega: f64, h: f64) -> f64 {
    let k0 = omega * omega / G;
    let x = k0 * h.max(0.05);
    let kh = x / x.powf(0.75).tanh().powf(2.0 / 3.0);
    kh / h.max(0.05)
}

/// The wavenumber (1/m) of a swell of `period` over depth `h`.
pub fn wavenumber_of(period: f64, h: f64) -> f64 {
    wavenumber(std::f64::consts::TAU / period, h)
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

/// Whether the canyon's edge turns a swell of `period` back.
pub fn reflects(period: f64) -> bool {
    period >= TURNS
}

impl Field {
    pub fn new(floor: &Floor, period: f64, from_deg: f64) -> Field {
        Field::with_rules(floor, period, from_deg, [true; 2])
    }

    /// A swell's field with some of `RULES` switched off (`Solve`'s parts, all at once).
    pub fn with_rules(floor: &Floor, period: f64, from_deg: f64, rules: [bool; 2]) -> Field {
        let mut s = Solve::new(floor, period, from_deg, rules);
        while !s.part(floor) {}
        s.field()
    }

    /// The highest the two trains stand at a cell where their crests meet (m).
    pub fn crossed(&self, i: usize) -> f64 {
        (self.height[i] + self.height_back[i]) as f64
    }

    /// How near the waves at a cell are to breaking: their height together over what the depth allows.
    pub fn nearness(&self, floor: &Floor, i: usize) -> f64 {
        self.crossed(i) / (BREAK * floor.depth(i)).max(0.01)
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

/// The swell arriving over the entry edges as a plane wave (travel time p·d / c at open-sea speed), from
/// zero at the first crest in. Returns the time taken off, so the rays can keep the same clock.
fn seed(floor: &Floor, d: [f64; 2], slowness: f64, tau: &mut [f64]) -> f64 {
    for (cells, _) in edges(floor, d) {
        for i in cells {
            if floor.sea(i) {
                let (x, y) = ((i % floor.w) as f64, (i / floor.w) as f64);
                tau[i] = tau[i].min((x * d[0] + y * d[1]) * slowness);
            }
        }
    }
    let lo = tau.iter().copied().fold(f64::INFINITY, f64::min);
    if lo.is_finite() {
        tau.iter_mut()
            .filter(|t| t.is_finite())
            .for_each(|t| *t -= lo);
    }
    lo
}

/// Fast sweeping (Zhao 2005): Godunov's upwind update of |∇τ| = s, in the four diagonal orders, `rounds`
/// times. A cell of slowness 0 (land, or the canyon's wall) holds nothing.
fn sweep(floor: &Floor, slowness: &[f64], tau: &mut [f64], rounds: usize) {
    let (w, h) = (floor.w, floor.h);
    let update = |tau: &mut [f64], x: usize, y: usize| {
        let i = y * w + x;
        let s = slowness[i];
        if s <= 0.0 {
            tau[i] = f64::INFINITY;
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

/// Whether a cell has the wall beside it (its four neighbours).
fn on_face(wall: &[bool], w: usize, h: usize, i: usize) -> bool {
    let (x, y) = (i % w, i / w);
    (x > 0 && wall[i - 1])
        || (x + 1 < w && wall[i + 1])
        || (y > 0 && wall[i - w])
        || (y + 1 < h && wall[i + w])
}

/// A grid's value at a point (cells), bilinear; None off the grid or where any corner is 0 or less.
fn at(floor: &Floor, v: &[f64], p: [f64; 2]) -> Option<f64> {
    let (w, h) = (floor.w, floor.h);
    if p[0] < 0.0 || p[1] < 0.0 || p[0] >= (w - 1) as f64 || p[1] >= (h - 1) as f64 {
        return None;
    }
    let (x0, y0) = (p[0] as usize, p[1] as usize);
    let (fx, fy) = (p[0] - x0 as f64, p[1] - y0 as f64);
    let i = y0 * w + x0;
    let c = [v[i], v[i + 1], v[i + w], v[i + w + 1]];
    if c.iter().any(|&v| v <= 0.0) {
        return None;
    }
    Some((c[0] * (1.0 - fx) + c[1] * fx) * (1.0 - fy) + (c[2] * (1.0 - fx) + c[3] * fx) * fy)
}

/// One ray, from `p` heading `v`, until it leaves the sea or meets the wall.
fn trace(
    floor: &Floor,
    speed: &[f64],
    wall: &[bool],
    crowd: &mut [f64],
    mut p: [f64; 2],
    mut v: [f64; 2],
) {
    let w = floor.w;
    let e = 0.5;
    for _ in 0..(4 * (floor.w + floor.h)) {
        let Some(c) = at(floor, speed, p) else {
            return;
        };
        let (x0, y0) = (p[0] as usize, p[1] as usize);
        if wall[y0 * w + x0] {
            return;
        }
        let grad = |q: [f64; 2], r: [f64; 2]| {
            at(floor, speed, q)
                .zip(at(floor, speed, r))
                .map_or(0.0, |(a, b)| (a - b) / (2.0 * e))
        };
        let gx = grad([p[0] + e, p[1]], [p[0] - e, p[1]]);
        let gy = grad([p[0], p[1] + e], [p[0], p[1] - e]);
        let along = gx * v[0] + gy * v[1];
        v = [
            v[0] - (gx - along * v[0]) / c * STEP,
            v[1] - (gy - along * v[1]) / c * STEP,
        ];
        let m = v[0].hypot(v[1]);
        v = [v[0] / m, v[1] / m];
        let (fx, fy) = (p[0] - x0 as f64, p[1] - y0 as f64);
        for (j, wt) in [
            (y0 * w + x0, (1.0 - fx) * (1.0 - fy)),
            (y0 * w + x0 + 1, fx * (1.0 - fy)),
            ((y0 + 1) * w + x0, (1.0 - fx) * fy),
            ((y0 + 1) * w + x0 + 1, fx * fy),
        ] {
            crowd[j] += wt;
        }
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

    fn flat(depth: f32) -> Floor {
        Floor {
            w: 60,
            h: 60,
            cell_m: 50.0,
            ground: vec![-depth; 3600],
            canyon: vec![false; 3600],
        }
    }

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
        let floor = flat(500.0);
        let f = Field::new(&floor, 12.0, 270.0);
        let (c, _) = speeds(12.0, 500.0);
        // from the west: τ grows with x at 1/c a metre, the same down a column
        let t = |x: usize, y: usize| f.tau[y * 60 + x];
        assert!(((t(40, 30) - t(10, 30)) - 30.0 * 50.0 / c).abs() < 0.5);
        assert!((t(40, 10) - t(40, 50)).abs() < 0.5);
        // its height stays the open sea's, and nothing is turned back
        let hh = f.height[30 * 60 + 30] as f64;
        assert!((hh - H0).abs() < 0.8, "{hh}");
        assert!(f.height_back[30 * 60 + 30] < 0.1);
    }

    #[test]
    fn a_wall_turns_a_long_swell_back_and_lets_a_short_one_across() {
        // a canyon across the east half of the floor, running west to east, the swell from the north-west
        let mut floor = flat(20.0);
        for y in 34..44 {
            for x in 20..60 {
                floor.ground[y * 60 + x] = -150.0;
                floor.canyon[y * 60 + x] = true;
            }
        }
        let long = Field::new(&floor, 17.0, 315.0);
        let short = Field::new(&floor, 5.0, 315.0);
        let north = 28 * 60 + 40;
        assert!(long.height_back[north] > 1.0, "{}", long.height_back[north]);
        assert!(long.tau_back[north].is_finite());
        assert!(short.height_back[north] < 0.1);
        // the long swell does not cross: nothing arrives inside the canyon
        assert!(!long.tau[38 * 60 + 50].is_finite());
        assert!(short.tau[38 * 60 + 50].is_finite());
    }
}

#[cfg(test)]
mod probe {
    use super::*;

    /// The field near the headland, for looking at: `cargo test --release -p nazare -- --ignored --nocapture
    /// near_the_headland` (NAZARE_T, NAZARE_FROM set the swell, NAZARE_BACK=0 turns the canyon rule off;
    /// NAZARE_OUT a PPM of the two trains near the headland).
    #[test]
    #[ignore]
    fn near_the_headland() {
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
        let l = &b["places"]["headland"]["cell"];
        let light = [l[0].as_f64().unwrap(), l[1].as_f64().unwrap()];
        let floor = crate::floor::build(w, h, 50.0, ground, light);
        let var = |k: &str, d: f64| {
            std::env::var(k)
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(d)
        };
        let t0 = std::time::Instant::now();
        let back = var("NAZARE_BACK", 1.0) > 0.5;
        let f = Field::with_rules(
            &floor,
            var("NAZARE_T", 17.0),
            var("NAZARE_FROM", 295.0),
            [true, back],
        );
        println!(
            "solved in {:.0} ms; canyon cells {}",
            t0.elapsed().as_secs_f64() * 1000.0,
            floor.canyon.iter().filter(|&&c| c).count()
        );
        let (cx, cy) = (light[0] as usize, light[1] as usize);
        for y in (cy - 30..cy + 4).step_by(3) {
            let row: Vec<String> = (cx - 36..cx + 3)
                .step_by(3)
                .map(|x| {
                    let i = y * w + x;
                    if !floor.sea(i) {
                        "  ##".into()
                    } else if floor.canyon[i] {
                        "  ..".into()
                    } else {
                        format!("{:2.0}/{:1.0}", f.height[i], f.height_back[i].min(9.0))
                    }
                })
                .collect();
            println!("{y:4} {}", row.join(""));
        }
        let near: Vec<(usize, f64)> = (0..w * h)
            .filter(|&i| floor.sea(i) && !floor.canyon[i])
            .map(|i| (i, f.nearness(&floor, i)))
            .filter(|&(i, n)| n > 0.8 && f.height_back[i] > 1.0)
            .collect();
        println!("stacking cells {}", near.len());
        if let Ok(out) = std::env::var("NAZARE_OUT") {
            // red the arriving train, green the turned-back, blue the wall, grey the land (window 200 cells)
            let (x0, y0) = (cx.saturating_sub(140), cy.saturating_sub(100));
            let mut img = b"P6 200 200 255\n".to_vec();
            for y in y0..y0 + 200 {
                for x in x0..x0 + 200 {
                    let i = y.min(h - 1) * w + x.min(w - 1);
                    let px = if !floor.sea(i) {
                        [90, 90, 90]
                    } else {
                        let r = (f.height[i] * 12.0).min(255.0) as u8;
                        let g = (f.height_back[i] * 25.0).min(255.0) as u8;
                        [
                            r,
                            g,
                            if floor.canyon[i] {
                                200
                            } else {
                                (floor.depth(i) * 0.6).min(120.0) as u8
                            },
                        ]
                    };
                    img.extend_from_slice(&px);
                }
            }
            std::fs::write(out, img).unwrap();
        }
    }
}
