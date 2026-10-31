//! The real drifters a float is measured against: `adrift/data/drifters.bin`, runs of daily
//! positions of the NOAA Global Drifter Program's buoys (built by `tools/adrift/data/drifters.py`,
//! whose docstring is the layout), and the nearest point of any run to a place.

use crate::model::LonLat;
use std::path::Path;

/// One run of whole days of one buoy.
pub struct Run {
    /// The GDP buoy identifier.
    pub id: u64,
    /// The first point's day, days since 2000-01-01 00:00 UTC; point i is day0 + i.
    pub day0: f32,
    /// Whether the drogue was on at the first point.
    pub drogued: bool,
    /// (lon, lat) in degrees, one a day.
    pub points: Vec<(f32, f32)>,
}

pub struct Drifters {
    pub runs: Vec<Run>,
    // a 2-degree grid of (run, point) for the search
    cells: Vec<Vec<(u32, u32)>>,
}

/// The nearest real point to a place.
#[derive(Clone, Copy, Debug)]
pub struct Nearest {
    pub run: usize,
    pub point: usize,
    pub km: f32,
}

const CELL: f32 = 2.0;
const NX: usize = 180;
const NY: usize = 90;
const SCALE: f32 = 180.0;

impl Drifters {
    pub fn parse(bytes: &[u8]) -> Result<Drifters, String> {
        if bytes.len() < 12 || &bytes[..4] != b"ADRF" {
            return Err("not a drifters file".into());
        }
        let u32_at = |p: usize| u32::from_le_bytes(bytes[p..p + 4].try_into().unwrap());
        let version = u32_at(4);
        if version != 1 {
            return Err(format!("drifters version {} is not 1", version));
        }
        let count = u32_at(8) as usize;
        let mut runs = Vec::with_capacity(count);
        let mut p = 12;
        for _ in 0..count {
            if p + 16 > bytes.len() {
                return Err("drifters file cut short".into());
            }
            let id = u64::from_le_bytes(bytes[p..p + 8].try_into().unwrap());
            let day0 = f32::from_le_bytes(bytes[p + 8..p + 12].try_into().unwrap());
            let n = u16::from_le_bytes(bytes[p + 12..p + 14].try_into().unwrap()) as usize;
            let flags = u16::from_le_bytes(bytes[p + 14..p + 16].try_into().unwrap());
            p += 16;
            if p + 4 * n > bytes.len() {
                return Err("drifters file cut short".into());
            }
            let points = (0..n)
                .map(|i| {
                    let q = p + 4 * i;
                    let x = i16::from_le_bytes(bytes[q..q + 2].try_into().unwrap()) as f32 / SCALE;
                    let y = i16::from_le_bytes(bytes[q + 2..q + 4].try_into().unwrap()) as f32 / SCALE;
                    (x, y)
                })
                .collect();
            p += 4 * n;
            runs.push(Run { id, day0, drogued: flags & 1 != 0, points });
        }
        let mut cells = vec![Vec::new(); NX * NY];
        for (r, run) in runs.iter().enumerate() {
            for (i, &(lon, lat)) in run.points.iter().enumerate() {
                cells[cell(lon, lat)].push((r as u32, i as u32));
            }
        }
        Ok(Drifters { runs, cells })
    }

    pub fn load(path: &Path) -> Result<Drifters, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("{}: {}", path.display(), e))?;
        Drifters::parse(&bytes)
    }

    pub fn points(&self) -> usize {
        self.runs.iter().map(|r| r.points.len()).sum()
    }

    /// The nearest point of any run to `at`, searching outward up to `within_km`.
    pub fn nearest(&self, at: LonLat, within_km: f32) -> Option<Nearest> {
        let (lon, lat) = (at.lon as f32, at.lat as f32);
        let cx = (((lon + 180.0).rem_euclid(360.0)) / CELL) as isize;
        let cy = (((90.0 - lat) / CELL) as isize).clamp(0, NY as isize - 1);
        let km_per_cell = CELL.to_radians() * 6371.0;
        let rings = ((within_km / km_per_cell).ceil() as isize + 1).min(NY as isize);
        let mut best: Option<Nearest> = None;
        for ring in 0..=rings {
            // nothing beyond this ring can beat what we have
            if let Some(b) = best {
                if (ring as f32 - 1.0) * km_per_cell > b.km {
                    break;
                }
            }
            for dy in -ring..=ring {
                let y = cy + dy;
                if y < 0 || y >= NY as isize {
                    continue;
                }
                for dx in -ring..=ring {
                    if dy.abs() != ring && dx.abs() != ring {
                        continue;
                    }
                    let x = (cx + dx).rem_euclid(NX as isize);
                    for &(r, i) in &self.cells[y as usize * NX + x as usize] {
                        let (plon, plat) = self.runs[r as usize].points[i as usize];
                        let km = distance_km(lon, lat, plon, plat);
                        if km <= within_km && best.map_or(true, |b| km < b.km) {
                            best = Some(Nearest { run: r as usize, point: i as usize, km });
                        }
                    }
                }
            }
        }
        best
    }
}

/// A real run to draw beside a held float: the nearest point of any run and the run itself,
/// as unit vectors in the globe's frame (`LonLat::to_xyz`), with what to say of it.
pub struct Track<'a> {
    pub run: &'a Run,
    /// The run's point nearest the float, and how far.
    pub point: usize,
    pub km: f32,
    /// Every point of the run, in order, as unit vectors.
    pub xyz: Vec<[f32; 3]>,
}

impl Track<'_> {
    /// The buoy and its days, for the screen: a real buoy, a real year, never the float's day.
    /// "Drifter 300234061198840, 12 May 2019 to 3 Feb 2020".
    pub fn label(&self) -> String {
        let n = self.run.points.len() as f64;
        let (y0, m0, d0) = civil(self.run.day0 as f64);
        let (y1, m1, d1) = civil(self.run.day0 as f64 + (n - 1.0));
        format!("Drifter {}, {} {} {} to {} {} {}", self.run.id, d0, MONTHS[m0 - 1], y0, d1, MONTHS[m1 - 1], y1)
    }

    /// Every point of the run, in order, as places (for a look that wants degrees).
    pub fn lonlat(&self) -> Vec<LonLat> {
        self.run.points.iter().map(|&(lon, lat)| LonLat { lon: lon as f64, lat: lat as f64 }).collect()
    }

    /// The run's points within `days` of where it passed nearest the float, as a range of
    /// indices into `xyz` and `run.points`: a whole run can cross a frame and a year, while what
    /// the float asks is where this buoy went from here. Thirty days each side shows a month's
    /// way in and out.
    pub fn window(&self, days: usize) -> std::ops::Range<usize> {
        let start = self.point.saturating_sub(days);
        let end = (self.point + days + 1).min(self.run.points.len());
        start..end
    }

    /// The label for a window: the buoy and the real dates of those days.
    pub fn label_window(&self, w: std::ops::Range<usize>) -> String {
        let (y0, m0, d0) = civil(self.run.day0 as f64 + w.start as f64);
        let (y1, m1, d1) = civil(self.run.day0 as f64 + (w.end.max(1) - 1) as f64);
        if (y0, m0) == (y1, m1) {
            format!("Drifter {}, {} to {} {} {}", self.run.id, d0, d1, MONTHS[m0 - 1], y0)
        } else if y0 == y1 {
            format!("Drifter {}, {} {} to {} {} {}", self.run.id, d0, MONTHS[m0 - 1], d1, MONTHS[m1 - 1], y0)
        } else {
            format!("Drifter {}, {} {} {} to {} {} {}", self.run.id, d0, MONTHS[m0 - 1], y0, d1, MONTHS[m1 - 1], y1)
        }
    }

    /// The day (days since 2000-01-01) of the point nearest the float.
    pub fn day(&self) -> f64 {
        self.run.day0 as f64 + self.point as f64
    }
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// The calendar date of a day since 2000-01-01 00:00 UTC (the real calendar, with its leap
/// days: a run's dates are real dates, unlike the climatology's typical year).
pub fn civil(day: f64) -> (i64, usize, u32) {
    // days since 1970-01-01, then Howard Hinnant's civil-from-days
    let z = day.floor() as i64 + 10957 + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as usize;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

impl Drifters {
    /// The run to draw for a float at `at`: the nearest point of any run within `within_km`.
    pub fn track(&self, at: LonLat, within_km: f32) -> Option<Track<'_>> {
        let n = self.nearest(at, within_km)?;
        let run = &self.runs[n.run];
        let xyz = run
            .points
            .iter()
            .map(|&(lon, lat)| {
                let v = LonLat { lon: lon as f64, lat: lat as f64 }.to_xyz();
                [v[0] as f32, v[1] as f32, v[2] as f32]
            })
            .collect();
        Some(Track { run, point: n.point, km: n.km, xyz })
    }
}

#[inline]
fn cell(lon: f32, lat: f32) -> usize {
    let x = (((lon + 180.0).rem_euclid(360.0)) / CELL) as usize % NX;
    let y = (((90.0 - lat) / CELL) as usize).min(NY - 1);
    y * NX + x
}

/// The great-circle distance in km (haversine).
pub fn distance_km(lon1: f32, lat1: f32, lon2: f32, lat2: f32) -> f32 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = p2 - p1;
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * 6371.0 * a.sqrt().asin()
}
