//! The data sheets as the rules read them: `adrift/data/*.png`, decoded once and sampled
//! bilinearly, wrapping east-west and clamping north-south, with each month's tile found by the
//! layout in `tools/adrift/data/sheets.py` (a 4 x 3 grid of tiles, tile (m % 4, m / 4) month m,
//! each with a one-cell border repeated from the opposite side east-west and the edge north-south).
//!
//! Every value is decoded exactly as `adrift/data/manifest.json` says, in f32, the same arithmetic
//! `rules.wgsl` does on the GPU with `textureLoad` and its own bilinear, so the two agree to the
//! last rounding of a multiply (not to the bit: the GPU may fuse a multiply and an add).

use std::io::Read;
use std::path::Path;

/// A decoded PNG: 8-bit channels, rows north to south.
pub struct Sheet {
    pub width: usize,
    pub height: usize,
    pub channels: usize,
    pub bytes: Vec<u8>,
}

impl Sheet {
    pub fn decode(png: &[u8]) -> Result<Sheet, String> {
        let decoder = png::Decoder::new(png);
        let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).map_err(|e| e.to_string())?;
        if info.bit_depth != png::BitDepth::Eight {
            return Err("a sheet is eight bits a channel".into());
        }
        let channels = match info.color_type {
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            _ => return Err("a sheet has no palette".into()),
        };
        buf.truncate(info.line_size * info.height as usize);
        Ok(Sheet { width: info.width as usize, height: info.height as usize, channels, bytes: buf })
    }

    pub fn load(path: &Path) -> Result<Sheet, String> {
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .and_then(|mut f| f.read_to_end(&mut bytes))
            .map_err(|e| format!("{}: {}", path.display(), e))?;
        Sheet::decode(&bytes).map_err(|e| format!("{}: {}", path.display(), e))
    }

    #[inline]
    pub fn at(&self, x: usize, y: usize, c: usize) -> f32 {
        self.bytes[(y * self.width + x) * self.channels + c] as f32
    }
}

/// A sheet of month tiles: `cells` across and down a tile, `border` cells round each.
pub struct Tiled {
    pub sheet: Sheet,
    pub cells: (usize, usize),
    pub border: usize,
    pub tiles: (usize, usize),
}

impl Tiled {
    pub fn new(sheet: Sheet, cells: (usize, usize), border: usize, tiles: (usize, usize)) -> Tiled {
        debug_assert_eq!(sheet.width, (cells.0 + 2 * border) * tiles.0);
        debug_assert_eq!(sheet.height, (cells.1 + 2 * border) * tiles.1);
        Tiled { sheet, cells, border, tiles }
    }

    /// The four texels round (lon, lat) in month `m`'s tile and their bilinear weights:
    /// ((x0, y0), (x1, y1), tx, ty), the tile's border doing the wrap and the clamp.
    #[inline]
    fn corners(&self, m: usize, lon: f32, lat: f32) -> ((usize, usize), (usize, usize), f32, f32) {
        let (w, h) = (self.cells.0 as f32, self.cells.1 as f32);
        // cell centres at half-cells: the cell's fractional index, then the border's offset
        let fx = (lon + 180.0).rem_euclid(360.0) / 360.0 * w - 0.5;
        let fy = ((90.0 - lat) / 180.0 * h - 0.5).clamp(-1.0, h);
        let x0 = fx.floor();
        let y0 = fy.floor();
        let tx = fx - x0;
        let ty = fy - y0;
        let tw = self.cells.0 + 2 * self.border;
        let th = self.cells.1 + 2 * self.border;
        let ox = (m % self.tiles.0) * tw + self.border;
        let oy = (m / self.tiles.0) * th + self.border;
        // x0 may be -1 (the west border) and x0 + 1 may be w (the east border); y likewise
        let xa = (ox as isize + x0 as isize) as usize;
        let ya = (oy as isize + y0 as isize) as usize;
        ((xa, ya), (xa + 1, ya + 1), tx, ty)
    }

    /// Channel `c` of month `m` at (lon, lat), bilinear, as the raw byte value 0..255.
    #[inline]
    pub fn sample(&self, m: usize, c: usize, lon: f32, lat: f32) -> f32 {
        let ((x0, y0), (x1, y1), tx, ty) = self.corners(m, lon, lat);
        let s = &self.sheet;
        let top = s.at(x0, y0, c) * (1.0 - tx) + s.at(x1, y0, c) * tx;
        let bottom = s.at(x0, y1, c) * (1.0 - tx) + s.at(x1, y1, c) * tx;
        top * (1.0 - ty) + bottom * ty
    }
}

/// Where in a typical year a day falls: the two months to mix and how far between them. Month
/// m's mean sits at its middle, so January 16 is wholly January and February 1 is half and half.
/// `day` is days since 2000-01-01 00:00 UTC; the year is 365.25 days: a climatology has no leap
/// days, only a cycle.
#[inline]
pub fn months(day: f32) -> (usize, usize, f32) {
    let m = (day.rem_euclid(365.25) - 15.2) / 30.4375;
    let m0 = m.floor();
    let t = m - m0;
    let a = (m0 as i32).rem_euclid(12) as usize;
    (a, (a + 1) % 12, t)
}

/// The current sheet: R, G the velocity (128 + 127 s sqrt(|v| / 2 m/s)), B how measured.
pub struct Currents(pub Tiled);

impl Currents {
    pub const VMAX: f32 = 2.0;

    #[inline]
    fn decode(b: f32) -> f32 {
        let s = (b - 128.0) / 127.0;
        s.signum() * s * s * Self::VMAX
    }

    /// (u east, v north) in m/s and how measured 0..1, at the day's mix of months.
    pub fn at(&self, day: f32, lon: f32, lat: f32) -> (f32, f32, f32) {
        let (a, b, t) = months(day);
        let mix = |c| self.0.sample(a, c, lon, lat) * (1.0 - t) + self.0.sample(b, c, lon, lat) * t;
        // decode each month, then mix: the encoding is not linear
        let u = Self::decode(self.0.sample(a, 0, lon, lat)) * (1.0 - t) + Self::decode(self.0.sample(b, 0, lon, lat)) * t;
        let v = Self::decode(self.0.sample(a, 1, lon, lat)) * (1.0 - t) + Self::decode(self.0.sample(b, 1, lon, lat)) * t;
        (u, v, mix(2) / 255.0)
    }
}

/// The wind sheet: R, G the 10 m wind, 128 + 127 w / 16 m/s.
pub struct Wind(pub Tiled);

impl Wind {
    pub const WMAX: f32 = 16.0;

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

    /// (K zonal, K meridional) in m^2/s and how measured 0..1.
    pub fn at(&self, lon: f32, lat: f32) -> (f32, f32, f32) {
        let k = |c| {
            let s = self.0.sample(0, c, lon, lat) / 255.0;
            s * s * Self::KMAX
        };
        (k(0), k(1), self.0.sample(0, 2, lon, lat) / 255.0)
    }
}

/// The ice sheet: grey, 255 all ice.
pub struct Ice(pub Tiled);

impl Ice {
    /// The concentration 0..1 at the day's mix of months.
    pub fn at(&self, day: f32, lon: f32, lat: f32) -> f32 {
        let (a, b, t) = months(day);
        (self.0.sample(a, 0, lon, lat) * (1.0 - t) + self.0.sample(b, 0, lon, lat) * t) / 255.0
    }
}

/// The coast sheet: no tiles, no border; L the fine signed distance (128 + 8 d, |d| <= 16 px), A
/// the coarse (128 + d / 2), land positive, in pixels of 360 / 4096 degrees.
pub struct Coast(pub Sheet);

impl Coast {
    pub const DEGREES_PER_PIXEL: f32 = 360.0 / 4096.0;

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

    /// Whether (lon, lat) is sea.
    pub fn is_sea(&self, lon: f32, lat: f32) -> bool {
        self.distance(lon, lat) < 0.0
    }
}

/// Every sheet the rules read, loaded from `adrift/data`.
pub struct Sea {
    pub currents: Currents,
    pub wind: Wind,
    pub eddy: Eddy,
    pub ice: Ice,
    pub coast: Coast,
}

impl Sea {
    /// The sheets' layouts, as `tools/adrift/data/*.py` write them (and `manifest.json` records).
    pub fn load(dir: &Path) -> Result<Sea, String> {
        Ok(Sea {
            currents: Currents(Tiled::new(Sheet::load(&dir.join("currents.png"))?, (720, 360), 1, (4, 3))),
            wind: Wind(Tiled::new(Sheet::load(&dir.join("wind.png"))?, (360, 180), 1, (4, 3))),
            eddy: Eddy(Tiled::new(Sheet::load(&dir.join("eddy.png"))?, (720, 360), 1, (1, 1))),
            ice: Ice(Tiled::new(Sheet::load(&dir.join("ice.png"))?, (1440, 720), 1, (4, 3))),
            coast: Coast(Sheet::load(&dir.join("coast.png"))?),
        })
    }
}
