//! Where the sea is, on the CPU: the globe's own coast (geo.png's signed distance, the same the
//! ground and the rings draw from), read at half its size. It answers whether a finger is on the
//! water, and whether the land stands between a whale and a place (sound in the sea does not cross a
//! peninsula): a march along the great circle, each step as long as the distance to the nearest coast.

use toy_kit::LonLat;

pub struct Sea {
    w: usize,
    h: usize,
    /// The signed distance to the coast, in pixels of the 4096-wide map (land positive).
    d: Vec<f32>,
}

/// One pixel of the 4096-wide map, in radians of longitude.
const PX: f64 = std::f64::consts::TAU / 4096.0;

impl Sea {
    pub fn new(geo_png: &[u8]) -> anyhow::Result<Sea> {
        let p = toy_kit::gpu::textures::decode(geo_png, 4)?;
        let (w, h) = (p.width as usize / 2, p.height as usize / 2);
        let mut d = Vec::with_capacity(w * h);
        for y in 0..h {
            for x in 0..w {
                let i = ((2 * y) * p.width as usize + 2 * x) * 4;
                let fine = (p.data[i] as f32 - 128.0) / 8.0;
                let coarse = (p.data[i + 1] as f32 - 128.0) * 2.0;
                d.push(if coarse.abs() > 14.0 { coarse } else { fine });
            }
        }
        Ok(Sea { w, h, d })
    }

    /// The distance to the coast at a place, in map pixels (land positive).
    pub fn coast(&self, at: LonLat) -> f64 {
        let x = ((at.lon / 360.0 + 0.5) * self.w as f64).floor().rem_euclid(self.w as f64) as usize;
        let y = (((0.5 - at.lat / 180.0) * self.h as f64).floor() as isize).clamp(0, self.h as isize - 1) as usize;
        self.d[y * self.w + x] as f64
    }

    pub fn is_sea(&self, at: [f64; 3]) -> bool {
        self.coast(LonLat::from_xyz(at)) < 0.0
    }

    /// Whether the way by sea from `a` to `b` is open: no land on the great circle between them.
    pub fn open(&self, a: [f64; 3], b: [f64; 3]) -> bool {
        let c = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
        let theta = c.acos();
        if theta < 1e-6 {
            return true;
        }
        let t = {
            let v = [b[0] - a[0] * c, b[1] - a[1] * c, b[2] - a[2] * c];
            let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
            [v[0] / n, v[1] / n, v[2] / n]
        };
        let mut s = 0.0;
        for _ in 0..200 {
            if s >= theta {
                return true;
            }
            let (sn, cs) = s.sin_cos();
            let q = [a[0] * cs + t[0] * sn, a[1] * cs + t[1] * sn, a[2] * cs + t[2] * sn];
            let at = LonLat::from_xyz(q);
            let d = self.coast(at);
            if d > 0.5 {
                return false;
            }
            s += (-d).max(1.0) * PX * at.lat.to_radians().cos().max(0.2) * 0.8;
        }
        true
    }
}
