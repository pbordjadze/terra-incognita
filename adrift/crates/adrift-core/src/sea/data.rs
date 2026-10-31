//! The data sheets decoded for the CPU: the currents (OceanCurrents.png's layout, tools/ocean/build.py),
//! the coast (Globe/geo.png's distance field) and the month's ice.

use crate::assets::Assets;
use crate::gpu::textures::decode;
use crate::model::LonLat;

/// Twelve months of half-degree near-surface currents, as the sheet encodes them.
pub struct Currents {
    /// 12 months × 360 rows (north down) × 720 columns (from 180° W): east, north, as the sheet's
    /// bytes (128 + 127 s sqrt(|v| / 2)), and how measured (0..255).
    cells: Vec<[u8; 3]>,
}

pub const W: usize = 720;
pub const H: usize = 360;
pub const VMAX: f64 = 2.0;

impl Currents {
    pub fn load(assets: &dyn Assets) -> anyhow::Result<Currents> {
        let p = decode(&assets.read("currents.png")?, 4)?;
        anyhow::ensure!(p.width == 2888 && p.height == 1086, "currents sheet is {}x{}", p.width, p.height);
        let mut cells = vec![[128u8, 128, 0]; 12 * W * H];
        for m in 0..12 {
            let (ox, oy) = ((m % 4) * 722 + 1, (m / 4) * 362 + 1);
            for y in 0..H {
                for x in 0..W {
                    let i = ((oy + y) * p.width as usize + ox + x) * 4;
                    cells[(m * H + y) * W + x] = [p.data[i], p.data[i + 1], p.data[i + 2]];
                }
            }
        }
        Ok(Currents { cells })
    }

    fn cell(&self, m: usize, x: isize, y: isize) -> [f64; 3] {
        let x = x.rem_euclid(W as isize) as usize;
        let y = y.clamp(0, H as isize - 1) as usize;
        let c = self.cells[(m * H + y) * W + x];
        let dec = |b: u8| {
            let s = (b as f64 - 128.0) / 127.0;
            s.signum() * s * s * VMAX
        };
        [dec(c[0]), dec(c[1]), c[2] as f64 / 255.0]
    }

    /// The month's east and north current (m/s) and how measured (0..1) at a place, bilinear.
    pub fn at_month(&self, m: u32, at: LonLat) -> [f64; 3] {
        let fx = (at.lon + 180.0).rem_euclid(360.0) / 360.0 * W as f64 - 0.5;
        let fy = (90.0 - at.lat) / 180.0 * H as f64 - 0.5;
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let (x0, y0) = (x0 as isize, y0 as isize);
        let m = m as usize % 12;
        let mut out = [0.0; 3];
        for (dx, dy, w) in [(0, 0, (1.0 - tx) * (1.0 - ty)), (1, 0, tx * (1.0 - ty)), (0, 1, (1.0 - tx) * ty), (1, 1, tx * ty)] {
            let c = self.cell(m, x0 + dx, y0 + dy);
            for k in 0..3 {
                out[k] += w * c[k];
            }
        }
        out
    }

    /// Between two months.
    pub fn at(&self, months: (u32, u32, f64), at: LonLat) -> [f64; 3] {
        let a = self.at_month(months.0, at);
        let b = self.at_month(months.1, at);
        let t = months.2;
        [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t]
    }
}

/// The coast: the globe's signed distance to it, land positive, in the map's pixels at 4096 across
/// (Globe/geo.png's R fine within 16 px, G coarse within 254 px).
pub struct Coast {
    width: usize,
    height: usize,
    fine: Vec<u8>,
    coarse: Vec<u8>,
}

impl Coast {
    pub fn load(assets: &dyn Assets) -> anyhow::Result<Coast> {
        let p = decode(&assets.read("geo.png")?, 4)?;
        let n = (p.width * p.height) as usize;
        let mut fine = vec![0u8; n];
        let mut coarse = vec![0u8; n];
        for i in 0..n {
            fine[i] = p.data[i * 4];
            coarse[i] = p.data[i * 4 + 1];
        }
        Ok(Coast { width: p.width as usize, height: p.height as usize, fine, coarse })
    }

    /// Signed distance to the coast in map pixels (about 9.8 km each at the equator), land positive.
    pub fn distance(&self, at: LonLat) -> f64 {
        let x = (((at.lon + 180.0).rem_euclid(360.0) / 360.0 * self.width as f64) as usize).min(self.width - 1);
        let y = (((90.0 - at.lat) / 180.0 * self.height as f64).max(0.0) as usize).min(self.height - 1);
        let i = y * self.width + x;
        let coarse = (self.coarse[i] as f64 - 128.0) * 2.0;
        if coarse.abs() > 14.0 {
            coarse
        } else {
            (self.fine[i] as f64 - 128.0) / 8.0
        }
    }

    pub fn is_land(&self, at: LonLat) -> bool {
        self.distance(at) > 0.0
    }
}

/// The month's usual sea-ice concentration (0..1), 1440 × 720 a month.
pub struct Ice {
    width: usize,
    height: usize,
    months: Vec<Vec<u8>>,
}

impl Ice {
    pub fn load(assets: &dyn Assets) -> anyhow::Result<Ice> {
        let mut months = Vec::with_capacity(12);
        let (mut width, mut height) = (0, 0);
        for m in 1..=12 {
            let p = decode(&assets.read(&format!("ice-{m:02}.png"))?, 1)?;
            width = p.width as usize;
            height = p.height as usize;
            months.push(p.data);
        }
        Ok(Ice { width, height, months })
    }

    pub fn at(&self, months: (u32, u32, f64), at: LonLat) -> f64 {
        let x = (((at.lon + 180.0).rem_euclid(360.0) / 360.0 * self.width as f64) as usize).min(self.width - 1);
        let y = (((90.0 - at.lat) / 180.0 * self.height as f64).max(0.0) as usize).min(self.height - 1);
        let i = y * self.width + x;
        let a = self.months[months.0 as usize % 12][i] as f64 / 255.0;
        let b = self.months[months.1 as usize % 12][i] as f64 / 255.0;
        a + (b - a) * months.2
    }
}
