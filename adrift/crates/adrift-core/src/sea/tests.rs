//! The rules held to the record (CLAUDE.md: a claim is a test or a PNG): the Gulf Stream at its
//! measured speed, a coast that holds, ice that holds till it retreats, the same seed the same
//! path, the draws standard, the maybes' spread the measured diffusivity, the nearest real
//! drifter usually near. `cargo test -p adrift-core sea::`.

use super::drifters::distance_km;
use super::rules::{gaussian, hash_seed, uniform};
use super::{Drifters, Held, Rules};
use crate::model::LonLat;
use std::path::{Path, PathBuf};

/// `adrift/data`, from the crate (adrift/crates/adrift-core), or wherever ADRIFT_DATA says.
fn data() -> PathBuf {
    std::env::var_os("ADRIFT_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"))
}

fn rules() -> Rules {
    Rules::load(&data()).unwrap()
}
const JULY: f64 = 15.2 + 6.0 * 30.4375;   // the middle of a typical July

#[test]
fn gulf_stream_runs_north_east_at_the_measured_speed() {
    let mut r = rules();
    r.wander = 0.0; r.leeway = 0.0;
    let from = LonLat { lon: -74.5, lat: 35.8 };            // off Cape Hatteras
    let (u, v, measured) = r.sea.currents.at(JULY as f32, from.lon as f32, from.lat as f32);
    assert!(measured > 0.99, "the Gulf Stream is measured");
    let speed = (u * u + v * v).sqrt();
    assert!(speed > 0.5 && u > 0.0 && v > 0.0, "the Gulf Stream here runs NE at over 0.5 m/s: ({u}, {v})");
    let s = r.step(from, JULY, 1.0, 1, 0);
    assert_eq!(s.held, Held::Free);
    let km = distance_km(from.lon as f32, from.lat as f32, s.at.lon as f32, s.at.lat as f32);
    let expect = speed * 86400.0 / 1000.0;
    assert!((km - expect).abs() < 0.02 * expect, "a day's run {km} km against {expect}");
    assert!(s.at.lon > from.lon && s.at.lat > from.lat, "north-east");
}

#[test]
fn a_coast_holds() {
    let r = rules();
    // Cape Hatteras's beach: a point just inland, from one just offshore
    let coast = &r.sea.coast;
    assert!(coast.is_sea(-75.2, 35.4) && !coast.is_sea(-77.5, 35.4), "the sheet knows the shore");
    let (lon, lat, held) = r.ashore(-75.2, 35.4, -77.5, 35.4);
    assert_eq!(held, Held::Coast);
    assert!(coast.is_sea(lon, lat), "put back on the water at ({lon}, {lat})");
    assert!(coast.distance(lon, lat) <= -r.shore * 0.5);
    // and a float in the sea with no land near takes its step
    assert_eq!(r.ashore(-40.0, 30.0, -40.1, 30.1).2, Held::Free);
}

#[test]
fn ice_holds_till_it_retreats() {
    let mut r = rules();
    r.wander = 0.0;
    let beaufort = LonLat { lon: -150.0, lat: 74.0 };
    let march = 15.2 + 2.0 * 30.4375;
    let september = 15.2 + 8.0 * 30.4375;
    let held = r.step(beaufort, march, 1.0, 1, 0);
    assert_eq!(held.held, Held::Ice);
    assert_eq!(held.at, beaufort, "held where it is");
    let free = r.step(beaufort, september, 1.0, 1, 0);
    assert_eq!(free.held, Held::Free, "the ice has retreated in September");
    assert_ne!(free.at, beaufort);
}

#[test]
fn the_same_seed_gives_the_same_path_and_another_k_another() {
    let r = rules();
    let from = LonLat { lon: -40.0, lat: 30.0 };
    let a = r.step(from, 100.0, 1.0, 42, 3);
    let b = r.step(from, 100.0, 1.0, 42, 3);
    assert_eq!(a.at, b.at);
    let c = r.step(from, 100.0, 1.0, 42, 4);
    assert_ne!(a.at, c.at);
    let d = r.step(from, 100.0, 1.0, 43, 3);
    assert_ne!(a.at, d.at);
}

#[test]
fn the_draws_are_uniform_and_the_gaussian_standard() {
    let n = 200_000;
    let (mut su, mut sg, mut sg2) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let h = hash_seed(9, i, 0);
        su += uniform(h, 0) as f64;
        let g = gaussian(h, 0) as f64;
        sg += g; sg2 += g * g;
    }
    let n = n as f64;
    assert!((su / n - 0.5).abs() < 0.005, "uniform mean {}", su / n);
    assert!((sg / n).abs() < 0.01, "gaussian mean {}", sg / n);
    assert!((sg2 / n - 1.0).abs() < 0.02, "gaussian variance {}", sg2 / n);
}

#[test]
fn the_maybes_spread_as_the_diffusivity_says() {
    let mut r = rules();
    r.leeway = 0.0;
    let from = LonLat { lon: -40.0, lat: 30.0 };            // the Sargasso Sea
    let (ku, kv, measured) = r.sea.eddy.at(from.lon as f32, from.lat as f32);
    assert!(measured > 0.99);
    let days = 30.0;
    let n = 3000;
    let ends = r.maybes(from, JULY, JULY + days, 0.25, 5, n);
    let coslat = (from.lat as f32).to_radians().cos();
    let m = 6_371_000.0f32;
    let xs: Vec<f32> = ends.iter().map(|s| ((s.at.lon - from.lon) as f32).to_radians() * m * coslat).collect();
    let ys: Vec<f32> = ends.iter().map(|s| ((s.at.lat - from.lat) as f32).to_radians() * m).collect();
    let var = |v: &[f32]| { let mean = v.iter().sum::<f32>() / v.len() as f32; v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32 };
    let (vx, vy) = (var(&xs), var(&ys));
    let (ex, ey) = (2.0 * ku * days as f32 * 86400.0, 2.0 * kv * days as f32 * 86400.0);
    // K varies over the cloud (about 100 km across), so within a fifth
    assert!((vx / ex - 1.0).abs() < 0.2, "zonal spread {} against 2Kt {} (K {ku})", vx.sqrt() / 1000.0, ex.sqrt() / 1000.0);
    assert!((vy / ey - 1.0).abs() < 0.2, "meridional spread {} against 2Kt {} (K {kv})", vy.sqrt() / 1000.0, ey.sqrt() / 1000.0);
    assert!(ends.iter().all(|s| s.held == Held::Free));
}

#[test]
fn the_nearest_real_drifter_is_usually_near() {
    let d = Drifters::load(&data().join("drifters.bin")).unwrap();
    assert!(d.runs.len() > 1000 && d.points() > 500_000);
    let r = rules();
    // a grid of sea points: how far is the nearest drifter point
    let mut far = 0; let mut total = 0; let mut sum = 0.0;
    for lat in (-60..=60).step_by(5) {
        for lon in (-180..180).step_by(5) {
            let (lon, lat) = (lon as f32 + 0.5, lat as f32 + 0.5);
            if !r.sea.coast.is_sea(lon, lat) || r.sea.coast.distance(lon, lat) > -10.0 { continue; }
            total += 1;
            match d.nearest(LonLat { lon: lon as f64, lat: lat as f64 }, 2000.0) {
                Some(n) => { sum += n.km; if n.km > 300.0 { far += 1; } }
                None => far += 1,
            }
        }
    }
    let mean = sum / total as f32;
    eprintln!("{total} open-sea points: mean nearest {mean:.0} km, {far} beyond 300 km");
    assert!(mean < 150.0, "mean nearest drifter point {mean} km");
    assert!((far as f32) < 0.1 * total as f32, "{far} of {total} points have no drifter within 300 km");
}
