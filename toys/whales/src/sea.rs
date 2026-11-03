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
    /// The same averaged over 4x4 map pixels (the rings shader's mip 2): the way the sound goes is
    /// asked of it, so a coast's wiggles and the islands blur away.
    blur: Vec<f32>,
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
        let (bw, bh) = (w / 2, h / 2);
        let mut blur = Vec::with_capacity(bw * bh);
        for y in 0..bh {
            for x in 0..bw {
                let i = 2 * y * w + 2 * x;
                blur.push((d[i] + d[i + 1] + d[i + w] + d[i + w + 1]) / 4.0);
            }
        }
        Ok(Sea { w, h, d, blur })
    }

    /// The distance to the coast at a place, in map pixels (land positive).
    pub fn coast(&self, at: LonLat) -> f64 {
        let x = ((at.lon / 360.0 + 0.5) * self.w as f64).floor().rem_euclid(self.w as f64) as usize;
        let y = (((0.5 - at.lat / 180.0) * self.h as f64).floor() as isize).clamp(0, self.h as isize - 1) as usize;
        self.d[y * self.w + x] as f64
    }

    fn coast_blurred(&self, at: LonLat) -> f64 {
        let (bw, bh) = (self.w / 2, self.h / 2);
        let x = ((at.lon / 360.0 + 0.5) * bw as f64).floor().rem_euclid(bw as f64) as usize;
        let y = (((0.5 - at.lat / 180.0) * bh as f64).floor() as isize).clamp(0, bh as isize - 1) as usize;
        self.blur[y * bw + x] as f64
    }

    pub fn is_sea(&self, at: [f64; 3]) -> bool {
        self.coast(LonLat::from_xyz(at)) < 0.0
    }

    /// Whether the way by sea from `a` to `b` is open: no peninsula or coast on the great circle
    /// between them (an island a few tens of kilometres across does not stop the sound; the rings
    /// shader's `open_sea` is the same march).
    pub fn open(&self, singer: [f64; 3], listener: [f64; 3]) -> bool {
        // marched from the listener toward the singer, as the rings shader marches from each point of
        // the sea to the singer: the singer's own bay is spared at the march's end, never the listener's
        let (a, b) = (listener, singer);
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
        // the singer's own bay casts no shadow (rings.wgsl's open_sea is the same)
        let end = theta - 5.0 * PX;
        for _ in 0..400 {
            if s >= end {
                return true;
            }
            let (sn, cs) = s.sin_cos();
            let q = [a[0] * cs + t[0] * sn, a[1] * cs + t[1] * sn, a[2] * cs + t[2] * sn];
            let at = LonLat::from_xyz(q);
            let d = self.coast_blurred(at);
            if d > 1.4 {
                return false;
            }
            s += d.abs().max(1.5) * PX * at.lat.to_radians().cos().max(0.2) * 0.8;
        }
        true
    }
}
