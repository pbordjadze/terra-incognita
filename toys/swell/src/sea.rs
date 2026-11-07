//! The sea: the linear long-wave (shallow-water) equation on the real ocean's depth, on the globe's own
//! half-degree grid. A wave runs at sqrt(g h): about 200 m/s over the abyss, 30 m/s over a shelf, so
//! it slows over ridges, bends round seamounts and shoals, grows as it climbs (Green's law falls out
//! of the equation) and reflects from coasts. Nothing else is in it: the floor is never drawn, only
//! felt in how the waves move.
//!
//! eta'' = div(g h grad eta), leapfrogged in its second-order form on the sphere (finite volumes;
//! a face's g h is the shallower side's, 0 at land, so coasts are walls), past 78 degrees a wall too.

pub const W: usize = 720;
pub const H: usize = 360;
const R: f64 = 6_371_000.0;
const G: f64 = 9.81;
/// The wave's seconds of sea a step (stable below the grid's limit at 78 degrees with the deepest cap).
pub const STEP: f64 = 20.0;
const MAX_DEPTH: f64 = 7000.0;
const EDGE_LAT: f64 = 78.0;

pub struct Sea {
    pub eta: Vec<f32>,
    prev: Vec<f32>,
    /// the east face of a cell (between i and i+1), times dt² over the cell's area terms
    ke: Vec<f32>,
    /// the north face of a cell (between row j and j-1, j counting down from the north)
    kn: Vec<f32>,
    inv_cos: Vec<f32>,
    damp: Vec<f32>,
    pub wet: Vec<bool>,
    depth: Vec<f32>,
    /// sea cells beside land: where waves arrive
    pub coast: Vec<u32>,
    steps: u64,
    scratch: Vec<f32>,
}

/// f - δ⁴f / 16 along each axis where the five cells are sea: grid-scale waves (2 cells) removed,
/// 4 cells kept three-quarters, 10 cells 99%.
fn shapiro(f: &mut [f32], wet: &[bool], tmp: &mut Vec<f32>) {
    tmp.clear();
    tmp.extend_from_slice(f);
    for j in 2..H - 2 {
        let row = j * W;
        for i in 0..W {
            let k = row + i;
            if !wet[k] {
                continue;
            }
            let at = |d: i64| row + (i as i64 + d).rem_euclid(W as i64) as usize;
            let (a, b, d, e) = (at(-2), at(-1), at(1), at(2));
            if wet[a] && wet[b] && wet[d] && wet[e] {
                f[k] =
                    tmp[k] - (tmp[a] - 4.0 * tmp[b] + 6.0 * tmp[k] - 4.0 * tmp[d] + tmp[e]) / 16.0;
            }
        }
    }
    tmp.clear();
    tmp.extend_from_slice(f);
    for j in 2..H - 2 {
        for i in 0..W {
            let k = j * W + i;
            if !wet[k] {
                continue;
            }
            let (a, b, d, e) = (k - 2 * W, k - W, k + W, k + 2 * W);
            if wet[a] && wet[b] && wet[d] && wet[e] {
                f[k] =
                    tmp[k] - (tmp[a] - 4.0 * tmp[b] + 6.0 * tmp[k] - 4.0 * tmp[d] + tmp[e]) / 16.0;
            }
        }
    }
}

pub fn lat_of(j: usize) -> f64 {
    90.0 - (j as f64 + 0.5) * 180.0 / H as f64
}

pub fn lon_of(i: usize) -> f64 {
    -180.0 + (i as f64 + 0.5) * 360.0 / W as f64
}

impl Sea {
    /// `ice`: the month's sea-ice concentration (1440 x 720 grey, the globe's grid); sea under half ice
    /// or more is taken as shore (the ice damps the long waves out).
    pub fn new(depth: &[u8], ice: Option<(&[u8], usize, usize)>) -> Sea {
        let n = W * H;
        let mut h = vec![0f64; n];
        for k in 0..n.min(depth.len() / 2) {
            h[k] = u16::from_le_bytes([depth[2 * k], depth[2 * k + 1]]) as f64;
        }
        for j in 0..H {
            if lat_of(j).abs() > EDGE_LAT {
                for i in 0..W {
                    h[j * W + i] = 0.0;
                }
            }
        }
        if let Some((px, iw, ih)) = ice {
            for j in 0..H {
                for i in 0..W {
                    let (x, y) = (i * iw / W, j * ih / H);
                    if px.get(y * iw + x).copied().unwrap_or(0) >= 128 {
                        h[j * W + i] = 0.0;
                    }
                }
            }
        }
        let wet: Vec<bool> = h.iter().map(|&d| d > 0.0).collect();
        let dl = 2.0 * std::f64::consts::PI / W as f64;
        let dp = std::f64::consts::PI / H as f64;
        let dt2 = STEP * STEP;
        let mut ke = vec![0f32; n];
        let mut kn = vec![0f32; n];
        let mut inv_cos = vec![0f32; H];
        for j in 0..H {
            let phi = lat_of(j).to_radians();
            let c = phi.cos();
            inv_cos[j] = (1.0 / c) as f32;
            // the face north of row j, at its latitude
            let cf = (phi + dp / 2.0).cos();
            for i in 0..W {
                let k = j * W + i;
                let e = j * W + (i + 1) % W;
                let he = h[k].min(h[e]).min(MAX_DEPTH);
                ke[k] = (dt2 * G * he / (R * R * c * c * dl * dl)) as f32;
                if j > 0 {
                    let nn = (j - 1) * W + i;
                    let hn = h[k].min(h[nn]).min(MAX_DEPTH);
                    kn[k] = (dt2 * G * hn * cf / (R * R * dp * dp)) as f32;
                }
            }
        }
        let mut coast = Vec::new();
        let mut damp = vec![0f32; n];
        for j in 1..H - 1 {
            for i in 0..W {
                let k = j * W + i;
                if !wet[k] {
                    continue;
                }
                let near_land = [(i + W - 1) % W + j * W, (i + 1) % W + j * W, k - W, k + W]
                    .iter()
                    .any(|&q| !wet[q]);
                if near_land {
                    coast.push(k as u32);
                }
                // a little loss everywhere, more at the shore (the coast takes some of each wave)
                damp[k] = if near_land { 0.008 } else { 0.0004 };
            }
        }
        Sea {
            depth: h.iter().map(|&d| d as f32).collect(),
            eta: vec![0.0; n],
            prev: vec![0.0; n],
            ke,
            kn,
            inv_cos,
            damp,
            wet,
            coast,
            steps: 0,
            scratch: Vec::with_capacity(n),
        }
    }

    /// One step of `STEP` seconds of sea, its rows in bands across the cores.
    pub fn step(&mut self) {
        let bands = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
            .clamp(1, 4);
        let rows = (H + bands - 1) / bands;
        let mut prev = std::mem::take(&mut self.prev);
        let (eta, ke, kn, ic, damp) = (&self.eta, &self.ke, &self.kn, &self.inv_cos, &self.damp);
        std::thread::scope(|s| {
            for (b, chunk) in prev.chunks_mut(rows * W).enumerate() {
                let j0 = b * rows;
                let mut work = move || {
                    for (jj, out) in chunk.chunks_mut(W).enumerate() {
                        let j = j0 + jj;
                        if j == 0 || j >= H - 1 {
                            continue;
                        }
                        let icj = ic[j];
                        let row = j * W;
                        for i in 0..W {
                            let k = row + i;
                            let w = row + (i + W - 1) % W;
                            let (kee, kw, knn, ks) = (ke[k], ke[w], kn[k], kn[k + W]);
                            if kee == 0.0 && kw == 0.0 && knn == 0.0 && ks == 0.0 {
                                out[i] = 0.0;
                                continue;
                            }
                            let e = row + (i + 1) % W;
                            let c = eta[k];
                            let lap = kee * (eta[e] - c) - kw * (c - eta[w])
                                + icj * (knn * (eta[k - W] - c) - ks * (c - eta[k + W]));
                            out[i] = c + (1.0 - damp[k]) * (c - out[i]) + lap;
                        }
                    }
                };
                if b + 1 == bands {
                    work()
                } else {
                    s.spawn(work);
                }
            }
        });
        self.prev = std::mem::replace(&mut self.eta, prev);
        self.steps += 1;
        if self.steps % 16 == 0 {
            // the grid's own ripples (a few cells long, which no real long wave has) taken out: a
            // second-order Shapiro filter, alike on both levels so only the shortest waves lose
            let wet = &self.wet;
            shapiro(&mut self.eta, wet, &mut self.scratch);
            shapiro(&mut self.prev, wet, &mut self.scratch);
        }
    }

    /// Raises the sea by a hump `height` high and `radius_km` wide about a place (a drop, a source's beat).
    pub fn raise(&mut self, lon: f64, lat: f64, height: f64, radius_km: f64) {
        let p = toy_kit::LonLat::new(lon, lat);
        let reach = (radius_km * 3.0 / 111.0).ceil() as i64 + 1;
        let jc = ((90.0 - lat) / 180.0 * H as f64) as i64;
        let ic = ((lon + 180.0) / 360.0 * W as f64) as i64;
        let wrap_i = (reach as f64 / lat.to_radians().cos().max(0.1)).ceil() as i64;
        for dj in -2 * reach..=2 * reach {
            let j = jc + dj;
            if j < 1 || j >= H as i64 - 1 {
                continue;
            }
            for di in -2 * wrap_i..=2 * wrap_i {
                let i = (ic + di).rem_euclid(W as i64) as usize;
                let k = j as usize * W + i;
                if !self.wet[k] {
                    continue;
                }
                let d = p.distance_m(toy_kit::LonLat::new(lon_of(i), lat_of(j as usize))) / 1000.0;
                let a = (height * (-(d / radius_km).powi(2)).exp()) as f32;
                // raised at rest: the hump falls and spreads as a ring
                self.eta[k] += a;
                self.prev[k] += a;
            }
        }
    }

    pub fn depth_at(&self, lon: f64, lat: f64) -> f64 {
        let j = (((90.0 - lat) / 180.0 * H as f64) as usize).min(H - 1);
        let i = (((lon + 180.0) / 360.0 * W as f64) as usize).min(W - 1);
        self.depth[j * W + i] as f64
    }

    pub fn is_wet(&self, lon: f64, lat: f64) -> bool {
        let j = (((90.0 - lat) / 180.0 * H as f64) as usize).min(H - 1);
        let i = (((lon + 180.0) / 360.0 * W as f64) as usize).min(W - 1);
        self.wet[j * W + i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// The sea's cost and its sanity: a drop in the open Pacific spreads, stays bounded, and reaches Hawaii's
    /// latitude band in about the real time (a day's crossing at ~200 m/s).
    #[test]
    fn a_drop_spreads_and_stays_bounded() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/data/depth.bin");
        let mut sea = Sea::new(&std::fs::read(path).unwrap(), None);
        sea.raise(143.2, 38.1, 1.0, 170.0);
        let t = std::time::Instant::now();
        for _ in 0..1080 {
            sea.step(); // six hours of sea
        }
        let per = t.elapsed().as_secs_f64() / 1080.0 * 1000.0;
        eprintln!("a step: {per:.2} ms");
        let max = sea.eta.iter().fold(0f32, |m, &v| m.max(v.abs()));
        assert!(max.is_finite() && max < 1.0, "max {max}");
        // six hours on, the front has crossed ~4000 km: the sea near Midway (177 W, 28 N) has moved
        let j = ((90.0 - 28.0) / 0.5) as usize;
        let i = ((-177.0 + 180.0) / 0.5) as usize;
        let moved = (j - 6..j + 6)
            .flat_map(|jj| (i - 6..i + 6).map(move |ii| (jj, ii)))
            .any(|(jj, ii)| sea.eta[jj * W + ii].abs() > 1e-3);
        assert!(moved);
    }
}
