//! A journey clip's pace (R14, Engineering's seam; the measure of what is worth seeing is Truth's
//! this round). A journey clip is a take with no hand in it (`App::journey_take`): its picked lamps
//! from the day the last was put, for a length the reader picks. Its date need not run evenly: where
//! the lamps do something worth seeing (come near each other, turn, loop) it slows, and where they
//! only travel it hurries, so the length is filled with what makes the edit. `pace` gives the day
//! each frame shows; the take carries it as a pace before each frame, so it replays exactly.

use crate::model::Float;
use crate::sea::Rules;

/// The day each of `frames` frames (at `fps`) shows, from `from` (the frame before the first) to about
/// `to`, never going back. The first cut: even.
pub fn pace(_rules: &Rules, _floats: &[Float], from: f64, to: f64, frames: usize, _fps: f64) -> Vec<f64> {
    let n = frames.max(1) as f64;
    (1..=frames).map(|i| from + (to - from) * i as f64 / n).collect()
}

/// A lamp's own path (its legs, each under its own law and world), every quarter-day mark from its
/// first leg to `to`: (day, place). The lamp's path as the App computes it (`App::extend_path`).
pub fn path(rules: &Rules, f: &Float, to: f64) -> Vec<(f64, crate::model::LonLat)> {
    use crate::sea::Drift;
    let legs = f.legs();
    let mut out = Vec::new();
    for (i, leg) in legs.iter().enumerate() {
        let until = legs.get(i + 1).map_or(to, |l| l.day);
        let e = leg.eddy.map(|e| (e[0], e[1])).unwrap_or_else(|| rules.put(leg.at, leg.day, leg.seed, leg.world).eddy);
        let mut d = Drift { at: leg.at, eddy: e };
        let mut day = leg.day;
        out.push((day, d.at));
        while day < until - 1e-9 {
            let next = crate::app::grid_next(day);
            d = rules.flight_under(leg.law, d, day, next - day, leg.seed, leg.world).drift;
            day = next;
            out.push((day, d.at));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::DirAssets;
    use crate::model::{Leg, LonLat};
    use crate::sea::drifters::distance_km;
    use std::path::{Path, PathBuf};

    fn assets() -> PathBuf {
        std::env::var_os("ADRIFT_ASSETS").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/assets"))
    }

    pub(super) fn rules() -> Rules {
        Rules::load(&DirAssets(assets())).unwrap()
    }

    /// A lamp as a fresh put makes it (the current law).
    pub(super) fn lamp(id: u32, at: LonLat, day: f64, seed: u64) -> Float {
        serde_json::from_value(serde_json::json!({
            "id": id, "at": at, "put_day": day, "seed": seed, "held": "No", "hand": "Free", "wake": [], "lit_for": 10.0,
            "legs": [Leg::new(at, day, seed)], "kept": true
        }))
        .unwrap()
    }

    /// How a pair danced over their paths: the line between them wound (radians, signed), the share of
    /// marks they were 15-150 km apart, and their closest.
    pub(super) fn dance(a: &[(f64, LonLat)], b: &[(f64, LonLat)]) -> (f64, f64, f64) {
        let mut wind = 0.0;
        let mut last: Option<f64> = None;
        let (mut near, mut closest) = (0usize, f64::INFINITY);
        let n = a.len().min(b.len());
        for i in 0..n {
            let (p, q) = (a[i].1, b[i].1);
            let d = distance_km(p.lon as f32, p.lat as f32, q.lon as f32, q.lat as f32) as f64;
            closest = closest.min(d);
            if (15.0..150.0).contains(&d) {
                near += 1;
            }
            let dx = ((q.lon - p.lon + 540.0).rem_euclid(360.0) - 180.0) * p.lat.to_radians().cos();
            let ang = (q.lat - p.lat).atan2(dx);
            if let Some(l) = last {
                wind += (ang - l + 3.0 * std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI;
            }
            last = Some(ang);
        }
        (wind, near as f64 / n as f64, closest)
    }

    /// The search for a dancing pair (R14): pairs of lamps put 25-70 km apart over the eddying seas,
    /// at month starts, followed 60 days; the best by how far the line between them wound while they
    /// stayed 15-150 km apart. `--ignored --nocapture` prints the best; the chosen pair is
    /// `adrift-check/lamps/dance.txt`.
    #[test]
    #[ignore]
    fn find_a_dancing_pair() {
        let r = rules();
        let regions: [(&str, f64, f64, f64, f64); 6] = [
            ("off Taiwan", 120.5, 126.0, 20.0, 27.0),
            ("Kuroshio extension", 140.0, 155.0, 32.0, 38.0),
            ("Gulf Stream", -72.0, -55.0, 34.0, 41.0),
            ("Agulhas", 15.0, 35.0, -42.0, -34.0),
            ("Brazil-Malvinas", -58.0, -45.0, -44.0, -36.0),
            ("East Australian", 150.0, 158.0, -38.0, -28.0),
        ];
        let mut best: Vec<(f64, String, LonLat, LonLat, f64)> = Vec::new();
        for (name, lo0, lo1, la0, la1) in regions {
            for month in [0.0, 1.0, 2.0, 5.0, 8.0, 11.0] {
                let day0 = 9131.0 + month * 30.44 + 1.0; // a typical year from 2025-01-01, a month's first
                let mut lon = lo0;
                while lon < lo1 {
                    let mut lat = la0;
                    while lat < la1 {
                        let a = LonLat { lon, lat };
                        if !r.coast.is_sea(a.lon as f32, a.lat as f32) {
                            lat += 0.75;
                            continue;
                        }
                        for (dx, dy) in [(0.4, 0.0), (0.0, 0.4), (0.3, 0.3), (0.6, -0.2)] {
                            let b = LonLat { lon: lon + dx, lat: lat + dy };
                            if !r.coast.is_sea(b.lon as f32, b.lat as f32) {
                                continue;
                            }
                            let (fa, fb) = (lamp(1, a, day0, 1), lamp(2, b, day0, 2));
                            let (pa, pb) = (path(&r, &fa, day0 + 60.0), path(&r, &fb, day0 + 60.0));
                            let (w, near, closest) = dance(&pa, &pb);
                            let score = w.abs() * near;
                            best.push((score, format!("{name} {w:+.1} rad, near {:.0}%, closest {closest:.0} km", near * 100.0), a, b, day0));
                        }
                        lat += 0.75;
                    }
                    lon += 0.75;
                }
            }
        }
        best.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        for b in best.iter().take(12) {
            eprintln!("{:6.2}  {}  a {:.2},{:.2}  b {:.2},{:.2}  day {:.1}", b.0, b.1, b.2.lon, b.2.lat, b.3.lon, b.3.lat, b.4);
        }
    }

    /// The dancing pair (R14, `find_a_dancing_pair`'s best): two lamps put 40 km apart in the East
    /// Australian Current's eddies off New South Wales on a typical 1 December, whose line wound three
    /// and a half turns round them in 60 days, 15-150 km apart nine tenths of the time. The copied-lamps
    /// text of them, the date 60 days on: `adrift-check/lamps/dance.txt` (ADRIFT_DANCE_OUT writes it).
    pub(super) fn the_dance() -> (Vec<Float>, f64) {
        let day0 = 9131.0 + 11.0 * 30.44 + 1.0; // as the search reckoned it
        (vec![lamp(1, LonLat { lon: 156.75, lat: -29.0 }, day0, 1), lamp(2, LonLat { lon: 157.15, lat: -29.0 }, day0, 2)], day0 + 60.0)
    }

    #[test]
    fn the_dance_dances() {
        let r = rules();
        let (f, to) = the_dance();
        let (a, b) = (path(&r, &f[0], to), path(&r, &f[1], to));
        let (w, near, closest) = dance(&a, &b);
        eprintln!("the dance: the line between them wound {w:+.1} rad in 60 days, 15-150 km apart {:.0}% of the time, closest {closest:.0} km", near * 100.0);
        assert!(w.abs() > 4.0 * std::f64::consts::PI && near > 0.8);
        if let Some(out) = std::env::var_os("ADRIFT_DANCE_OUT") {
            let copy = crate::app::LampsCopy { version: 1, day: to, floats: f };
            std::fs::write(out, format!("{}\n{}\n", crate::app::LAMPS_HEADER, serde_json::to_string(&copy).unwrap())).unwrap();
        }
    }
}
