//! The data sheets as the rules read them, decoded once from the assets (`crate::assets`) and
//! sampled bilinearly, wrapping east-west and clamping north-south. Terra's own sheets are read as
//! they are (`currents.png`: OceanCurrents.png's layout, tools/ocean/build.py; `geo.png`'s coast
//! distances; `ice-MM.png`); Adrift's (`wind.png`, `eddy.png`, `eddy-time.png`) are laid out by
//! `tools/adrift/data/sheets.py`: a 4 x 3 grid of month tiles, tile (m % 4, m / 4) month m, each
//! with a one-cell border repeated from the opposite side east-west and the edge north-south.
//!
//! Every value is decoded exactly as `adrift/data/manifest.json` says, in f32: the same arithmetic
//! `rules.wgsl` does on the GPU with `textureLoad` and its own bilinear, so the two agree to the
//! last rounding of a multiply (not to the bit: the GPU may fuse a multiply and an add).

use crate::assets::Assets;
use crate::gpu::textures::decode;

/// A decoded sheet: 8-bit channels, rows north to south.
pub struct Sheet {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub bytes: Vec<u8>,
}

impl Sheet {
    /// Decoded with four channels (grey and RGB widened), or one when `grey`.
    pub fn load(assets: &dyn Assets, name: &str, grey: bool) -> anyhow::Result<Sheet> {
        let p = decode(&assets.read(name)?, if grey { 1 } else { 4 })?;
        Ok(Sheet { width: p.width as usize, height: p.height as usize, channels: p.channels as usize, bytes: p.data })
    }

    #[inline]
    pub fn at(&self, x: usize, y: usize, c: usize) -> f32 {
        self.bytes[(y * self.width + x) * self.channels + c] as f32
    }
}

/// A sheet of month tiles: `cells` across and down a tile, a one-cell border round each, laid
/// `tiles` across and down.
pub struct Tiled {
    pub sheet: Sheet,
    pub cells: (usize, usize),
    pub tiles: (usize, usize),
}

impl Tiled {
    pub fn new(sheet: Sheet, cells: (usize, usize), tiles: (usize, usize)) -> anyhow::Result<Tiled> {
        anyhow::ensure!(
            sheet.width == (cells.0 + 2) * tiles.0 && sheet.height == (cells.1 + 2) * tiles.1,
            "a sheet of {} x {} is not {} tiles of {} cells",
            sheet.width,
            sheet.height,
            tiles.0 * tiles.1,
            cells.0
        );
        Ok(Tiled { sheet, cells, tiles })
    }

    /// Channel `c` of month `m` at (lon, lat), bilinear, as the raw byte value 0..255; the tile's
    /// border does the wrap and the clamp.
    #[inline]
    pub fn sample(&self, m: usize, c: usize, lon: f32, lat: f32) -> f32 {
        let (w, h) = (self.cells.0 as f32, self.cells.1 as f32);
        let fx = (lon + 180.0).rem_euclid(360.0) / 360.0 * w - 0.5;
        let fy = ((90.0 - lat) / 180.0 * h - 0.5).clamp(-1.0, h);
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = fx - x0;
        let ty = fy - y0;
        let ox = (m % self.tiles.0) * (self.cells.0 + 2) + 1;
        let oy = (m / self.tiles.0) * (self.cells.1 + 2) + 1;
        let xa = (ox as isize + x0 as isize) as usize;
        let ya = (oy as isize + y0 as isize) as usize;
        let s = &self.sheet;
        let top = s.at(xa, ya, c) * (1.0 - tx) + s.at(xa + 1, ya, c) * tx;
        let bottom = s.at(xa, ya + 1, c) * (1.0 - tx) + s.at(xa + 1, ya + 1, c) * tx;
        top * (1.0 - ty) + bottom * ty
    }
}

/// Where in a typical year a day falls: the two months to mix and how far between them. Month
/// m's mean sits at its middle, so January 16 is wholly January and February 1 is half and half
/// (the model's `Clock::months`, in f32). `day` is days since 2000-01-01 00:00 UTC; the year is
/// 365.25 days: a climatology has no leap days, only a cycle.
#[inline]
pub fn months(day: f32) -> (usize, usize, f32) {
    let m = (day.rem_euclid(365.25) - 15.21875) / 30.4375;
    let m0 = m.floor();
    let a = (m0 as i32).rem_euclid(12) as usize;
    (a, (a + 1) % 12, m - m0)
}

/// The current sheet: R, G the velocity (128 + 127 s sqrt(|v| / 2 m/s)), B how measured.
pub struct Currents(pub Tiled);

impl Currents {
    pub const VMAX: f32 = 2.0;

    pub fn load(assets: &dyn Assets) -> anyhow::Result<Currents> {
        Ok(Currents(Tiled::new(Sheet::load(assets, "currents.png", false)?, (720, 360), (4, 3))?))
    }

    #[inline]
    fn decode(b: f32) -> f32 {
        let s = (b - 128.0) / 127.0;
        s.signum() * s * s * Self::VMAX
    }

    /// (u east, v north) in m/s and how measured 0..1, at the day's mix of months. Each month is
    /// decoded, then mixed: the encoding is not linear.
    pub fn at(&self, day: f32, lon: f32, lat: f32) -> (f32, f32, f32) {
        let (a, b, t) = months(day);
        let mix = |c| self.0.sample(a, c, lon, lat) * (1.0 - t) + self.0.sample(b, c, lon, lat) * t;
        let u = Self::decode(self.0.sample(a, 0, lon, lat)) * (1.0 - t) + Self::decode(self.0.sample(b, 0, lon, lat)) * t;
        let v = Self::decode(self.0.sample(a, 1, lon, lat)) * (1.0 - t) + Self::decode(self.0.sample(b, 1, lon, lat)) * t;
        (u, v, mix(2) / 255.0)
    }
}

/// The wind sheet: R, G the 10 m wind, 128 + 127 w / 16 m/s.
pub struct Wind(pub Tiled);

impl Wind {
    pub const WMAX: f32 = 16.0;

    pub fn load(assets: &dyn Assets) -> anyhow::Result<Wind> {
        Ok(Wind(Tiled::new(Sheet::load(assets, "wind.png", false)?, (360, 180), (4, 3))?))
    }

    /// (u east, v north) in m/s at the day's mix of months.
    pub fn at(&self, day: f32, lon: f32, lat: f32) -> (f32, f32) {
        let (a, b, t) = months(day);
        let mix = |c| self.0.sample(a, c, lon, lat) * (1.0 - t) + self.0.sample(b, c, lon, lat) * t;
        ((mix(0) - 128.0) / 127.0 * Self::WMAX, (mix(1) - 128.0) / 127.0 * Self::WMAX)
    }
}

/// The eddy sheet (one tile for the year): R, G the diffusivity, 255 sqrt(K / 40000 m^2/s); B how
/// measured.
pub struct Eddy(pub Tiled);

impl Eddy {
    pub const KMAX: f32 = 40000.0;

    pub fn load(assets: &dyn Assets) -> anyhow::Result<Eddy> {
        Ok(Eddy(Tiled::new(Sheet::load(assets, "eddy.png", false)?, (720, 360), (1, 1))?))
    }

    /// (K zonal, K meridional) in m^2/s and how measured 0..1.
    pub fn at(&self, lon: f32, lat: f32) -> (f32, f32, f32) {
        let k = |c| {
            let s = self.0.sample(0, c, lon, lat) / 255.0;
            s * s * Self::KMAX
        };
        (k(0), k(1), self.0.sample(0, 2, lon, lat) / 255.0)
    }
}

/// The eddy time sheet (one tile): R, G the Lagrangian integral time scale, 255 T / 10 days.
pub struct EddyTime(pub Tiled);

impl EddyTime {
    pub const TMAX: f32 = 10.0;

    pub fn load(assets: &dyn Assets) -> anyhow::Result<EddyTime> {
        Ok(EddyTime(Tiled::new(Sheet::load(assets, "eddy-time.png", false)?, (360, 180), (1, 1))?))
    }

    /// (T zonal, T meridional) in days.
    pub fn at(&self, lon: f32, lat: f32) -> (f32, f32) {
        (self.0.sample(0, 0, lon, lat) / 255.0 * Self::TMAX, self.0.sample(0, 1, lon, lat) / 255.0 * Self::TMAX)
    }
}

/// The twelve ice sheets, Terra's `ice-MM.png`: grey, 255 all ice, 1440 x 720 each, no border.
pub struct Ice(pub Vec<Sheet>);

impl Ice {
    pub fn load(assets: &dyn Assets) -> anyhow::Result<Ice> {
        let mut months = Vec::with_capacity(12);
        for m in 1..=12 {
            let s = Sheet::load(assets, &format!("ice-{m:02}.png"), true)?;
            anyhow::ensure!(s.width == 1440 && s.height == 720, "ice-{m:02}.png is {} x {}", s.width, s.height);
            months.push(s);
        }
        Ok(Ice(months))
    }

    fn month(&self, m: usize, lon: f32, lat: f32) -> f32 {
        let s = &self.0[m];
        let (w, h) = (s.width as f32, s.height as f32);
        let fx = (lon + 180.0).rem_euclid(360.0) / 360.0 * w - 0.5;
        let fy = ((90.0 - lat) / 180.0 * h - 0.5).clamp(0.0, h - 1.0);
        let x0f = fx.floor();
        let y0f = fy.floor();
        let (tx, ty) = (fx - x0f, fy - y0f);
        let x0 = (x0f as isize).rem_euclid(s.width as isize) as usize;
        let x1 = (x0 + 1) % s.width;
        let y0 = y0f as usize;
        let y1 = (y0 + 1).min(s.height - 1);
        let top = s.at(x0, y0, 0) * (1.0 - tx) + s.at(x1, y0, 0) * tx;
        let bottom = s.at(x0, y1, 0) * (1.0 - tx) + s.at(x1, y1, 0) * tx;
        top * (1.0 - ty) + bottom * ty
    }

    /// The concentration 0..1 at the day's mix of months.
    pub fn at(&self, day: f32, lon: f32, lat: f32) -> f32 {
        let (a, b, t) = months(day);
        (self.month(a, lon, lat) * (1.0 - t) + self.month(b, lon, lat) * t) / 255.0
    }
}

/// The coast, Terra's `geo.png`: R the fine signed distance (128 + 8 d, |d| <= 16 px), G the
/// coarse (128 + d / 2), land positive, in pixels of 360 / 4096 degrees; no border.
pub struct Coast(pub Sheet);

impl Coast {
    pub const DEGREES_PER_PIXEL: f32 = 360.0 / 4096.0;

    pub fn load(assets: &dyn Assets) -> anyhow::Result<Coast> {
        let s = Sheet::load(assets, "geo.png", false)?;
        anyhow::ensure!(s.width == 4096 && s.height == 2048, "geo.png is {} x {}", s.width, s.height);
        Ok(Coast(s))
    }

    /// The signed distance to the shore in pixels at (lon, lat), bilinear, wrapping east-west;
    /// the fine channel within its range, the coarse beyond.
    pub fn distance(&self, lon: f32, lat: f32) -> f32 {
        let s = &self.0;
        let (w, h) = (s.width as f32, s.height as f32);
        let fx = (lon + 180.0).rem_euclid(360.0) / 360.0 * w - 0.5;
        let fy = ((90.0 - lat) / 180.0 * h - 0.5).clamp(0.0, h - 1.0);
        let x0f = fx.floor();
        let y0f = fy.floor();
        let (tx, ty) = (fx - x0f, fy - y0f);
        let x0 = (x0f as isize).rem_euclid(s.width as isize) as usize;
        let x1 = (x0 + 1) % s.width;
        let y0 = y0f as usize;
        let y1 = (y0 + 1).min(s.height - 1);
        let read = |c: usize| {
            let top = s.at(x0, y0, c) * (1.0 - tx) + s.at(x1, y0, c) * tx;
            let bottom = s.at(x0, y1, c) * (1.0 - tx) + s.at(x1, y1, c) * tx;
            top * (1.0 - ty) + bottom * ty
        };
        let fine = (read(0) - 128.0) / 8.0;
        if fine.abs() < 15.0 {
            fine
        } else {
            (read(1) - 128.0) * 2.0
        }
    }

    pub fn is_sea(&self, lon: f32, lat: f32) -> bool {
        self.distance(lon, lat) < 0.0
    }

    /// Whether a place is land (the app's hit test).
    pub fn is_land(&self, at: crate::model::LonLat) -> bool {
        self.distance(at.lon as f32, at.lat as f32) > 0.0
    }
}
