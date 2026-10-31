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
