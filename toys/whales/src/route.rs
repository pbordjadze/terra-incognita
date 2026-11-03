//! A whale's year: the track tools/toys/whales/routes.py drew from its population's published
//! routes (four places a day, a year round), and how much it sings each month.

use serde::Deserialize;
use toy_kit::LonLat;

pub const YEAR: f64 = 365.0;

#[derive(Deserialize)]
struct File {
    per_day: f64,
    whales: std::collections::HashMap<String, Raw>,
}

#[derive(Deserialize)]
struct Raw {
    sings: Vec<f32>,
    track: Vec<[f64; 2]>,
}

pub struct Route {
    per_day: f64,
    track: Vec<[f64; 3]>,
    sings: Vec<f32>,
}

pub fn load(json: &[u8], names: [&str; 2]) -> anyhow::Result<[Route; 2]> {
    let f: File = serde_json::from_slice(json)?;
    let one = |n: &str| -> anyhow::Result<Route> {
        let r = f.whales.get(n).ok_or_else(|| anyhow::anyhow!("no route for {n}"))?;
        anyhow::ensure!(r.track.len() >= 2 && r.sings.len() == 12, "a short route for {n}");
        Ok(Route { per_day: f.per_day, track: r.track.iter().map(|p| LonLat::new(p[0], p[1]).to_xyz()).collect(), sings: r.sings.clone() })
    };
    Ok([one(names[0])?, one(names[1])?])
}

fn norm(v: [f64; 3]) -> [f64; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / n, v[1] / n, v[2] / n]
}

impl Route {
    /// Where it is on `day` (of the year, 0 = January 1st; any day, the year wraps), as a unit vector.
    pub fn at(&self, day: f64) -> [f64; 3] {
        let n = self.track.len();
        let x = (day.rem_euclid(YEAR) * self.per_day).rem_euclid(n as f64);
        let i = x.floor() as usize % n;
        let j = (i + 1) % n;
        // between two quarter-days, eased with the neighbours (a Catmull-Rom on the sphere's chords)
        let t = x - x.floor();
        let p0 = self.track[(i + n - 1) % n];
        let (p1, p2) = (self.track[i], self.track[j]);
        let p3 = self.track[(j + 1) % n];
        let (t2, t3) = (t * t, t * t * t);
        let c = |k: usize| 0.5 * (2.0 * p1[k] + (-p0[k] + p2[k]) * t + (2.0 * p0[k] - 5.0 * p1[k] + 4.0 * p2[k] - p3[k]) * t2 + (-p0[k] + 3.0 * p1[k] - 3.0 * p2[k] + p3[k]) * t3);
        norm([c(0), c(1), c(2)])
    }

    pub fn lonlat(&self, day: f64) -> LonLat {
        LonLat::from_xyz(self.at(day))
    }

    /// How much it sings on `day`, 0..1 (the month's measure, eased between months' middles).
    pub fn sings(&self, day: f64) -> f32 {
        let m = (day.rem_euclid(YEAR) / YEAR * 12.0 - 0.5).rem_euclid(12.0);
        let a = m.floor() as usize % 12;
        let k = (m - m.floor()) as f32;
        self.sings[a] * (1.0 - k) + self.sings[(a + 1) % 12] * k
    }
}
