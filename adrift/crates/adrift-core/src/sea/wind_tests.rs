//! The wind's rules held to the record: the same draws the same path, the GPU where the CPU is,
//! the maybes' 10-day spread the spread of real paths through five real years of wind
//! (`adrift/data/weather.json`, tools/adrift/data/weather.py), things lifted together keeping
//! together in one world, and coming down sooner where it rains. `cargo test -p adrift-core wind`.

use super::wind::{Air, AirParams, AirRules, Aloft};
use crate::assets::DirAssets;
use crate::model::LonLat;
use std::path::{Path, PathBuf};

fn assets() -> PathBuf {
    std::env::var_os("ADRIFT_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/assets"))
}

fn air() -> AirRules {
    AirRules::load(&DirAssets(assets())).unwrap()
}

const WEATHER: &str = include_str!("../../../../data/weather.json");
const JULY: f64 = 15.21875 + 6.0 * 30.4375;
/// The first of each month in a typical (non-leap) year, days since January 1.
const MONTH_START: [f64; 12] = [0.0, 31.0, 59.0, 90.0, 120.0, 151.0, 181.0, 212.0, 243.0, 273.0, 304.0, 334.0];

#[test]
fn the_same_draws_give_the_same_path() {
    let r = air();
    let from = LonLat { lon: -40.0, lat: 45.0 };
    let a = r.path(from, JULY, JULY + 10.0, 0.25, 7, 3);
    let b = r.path(from, JULY, JULY + 10.0, 0.25, 7, 3);
    assert_eq!(a, b);
    let c = r.path(from, JULY, JULY + 10.0, 0.25, 7, 4);
    assert_ne!(a.last().unwrap().at, c.last().unwrap().at, "another world, another path");
}

#[test]
fn the_weather_is_the_reanalysis_numbers() {
    let r = air();
    // the North Atlantic's storm track in January gusts harder than the trade winds in July
    let storm = r.gusts.at(15.0, -40.0, 50.0);
    let trades = r.gusts.at(JULY as f32, -40.0, 15.0);
    eprintln!("storm track {storm:?}, trades {trades:?}");
    assert!(storm.sigma.0 > 5.0 && storm.sigma.0 < 10.0, "{storm:?}");
    assert!(trades.sigma.0 < 3.0, "{trades:?}");
    assert!(storm.time > 0.5 && storm.time < 3.0, "{storm:?}");
    // the Sahara in July has no rain to speak of; the warm pool of the western Pacific has
    let sahara = r.gusts.at(JULY as f32, 10.0, 23.0);
    let pool = r.gusts.at(JULY as f32, 140.0, 8.0);
    assert!(sahara.rain < 0.5 && pool.rain > 5.0, "{sahara:?} {pool:?}");
}

#[test]
fn the_maybes_spread_as_real_paths_do() {
    let r = AirRules { fall: 0.0, washout: 0.0, ..air() };
    let w: serde_json::Value = serde_json::from_str(WEATHER).unwrap();
    let days = w["days"].as_f64().unwrap();
    let mut ratios = Vec::new();
    let mut drift_err = Vec::new();
    for p in w["paths"].as_array().unwrap() {
        let month = p["month"].as_u64().unwrap() as usize;
        if month % 3 != 1 {
            continue; // January, April, July, October
        }
        let (lon, lat) = (p["lon"].as_f64().unwrap(), p["lat"].as_f64().unwrap());
        let real = p["spread_km"].as_f64().unwrap();
        let mut ends = Vec::new();
        for d0 in [0.0, 14.0] {
            let day0 = MONTH_START[month - 1] + d0;
            for k in 1..=40 {
                let path = r.path(LonLat { lon, lat }, day0, day0 + days, 0.25, 1, k + (d0 as u32) * 100);
                ends.push(displacement(&path));
            }
        }
        let n = ends.len() as f64;
        let (me, mn) = (ends.iter().map(|e| e.0).sum::<f64>() / n, ends.iter().map(|e| e.1).sum::<f64>() / n);
        let spread = (ends.iter().map(|e| (e.0 - me).powi(2) + (e.1 - mn).powi(2)).sum::<f64>() / n).sqrt();
        ratios.push(spread / real);
        let (re, rn) = (p["east_km"].as_f64().unwrap(), p["north_km"].as_f64().unwrap());
        drift_err.push(((me - re).powi(2) + (mn - rn).powi(2)).sqrt() / real.max(1.0));
    }
    ratios.sort_by(|a, b| a.partial_cmp(b).unwrap());
    drift_err.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |v: &Vec<f64>, f: f64| v[((v.len() - 1) as f64 * f) as usize];
    eprintln!(
        "{} starts: spread / real spread, 10th {:.2}, median {:.2}, 90th {:.2}; mean drift's miss / real spread, median {:.2}",
        ratios.len(),
        q(&ratios, 0.1),
        q(&ratios, 0.5),
        q(&ratios, 0.9),
        q(&drift_err, 0.5)
    );
    assert!(q(&ratios, 0.5) > 0.8 && q(&ratios, 0.5) < 1.25, "the median start spreads as the real paths do");
    assert!(q(&ratios, 0.1) > 0.5 && q(&ratios, 0.9) < 2.0, "nine in ten starts within a factor of two");
    assert!(q(&drift_err, 0.5) < 0.5, "the cloud's middle goes where the real paths' middle goes");
}

/// The path's displacement, km east and north, summed step by step (as weather.py sums it).
fn displacement(path: &[Air]) -> (f64, f64) {
    let (mut e, mut n) = (0.0, 0.0);
    for w in path.windows(2) {
        let (a, b) = (w[0].at, w[1].at);
        let dlon = (b.lon - a.lon + 540.0).rem_euclid(360.0) - 180.0;
        let mid = ((a.lat + b.lat) / 2.0).to_radians();
        e += dlon.to_radians() * mid.cos() * 6371.0;
        n += (b.lat - a.lat).to_radians() * 6371.0;
    }
    (e, n)
}

#[test]
fn things_lifted_together_fly_together_in_one_world() {
    let r = AirRules { fall: 0.0, washout: 0.0, ..air() };
    let a = LonLat { lon: -30.0, lat: 45.0 };
    let b = LonLat { lon: -29.5, lat: 45.0 };
    let (mut same, mut apart) = (0.0, 0.0);
    for k in 1..=50 {
        let pa = r.path(a, JULY, JULY + 3.0, 0.25, 1, k).pop().unwrap().at;
        let pb = r.path(b, JULY, JULY + 3.0, 0.25, 2, k).pop().unwrap().at;
        let pc = r.path(b, JULY, JULY + 3.0, 0.25, 2, k + 1000).pop().unwrap().at;
        same += super::drifters::distance_km(pa.lon as f32, pa.lat as f32, pb.lon as f32, pb.lat as f32) as f64;
        apart += super::drifters::distance_km(pa.lon as f32, pa.lat as f32, pc.lon as f32, pc.lat as f32) as f64;
    }
    eprintln!("after 3 days, 40 km apart at the lift: {:.0} km in one world, {:.0} in two", same / 50.0, apart / 50.0);
    assert!(same < apart * 0.3);
}

#[test]
fn things_come_down_sooner_where_it_rains() {
    let r = air();
    let life = |from: LonLat| {
        let mut total = 0.0;
        for k in 1..=400 {
            let path = r.path(from, JULY, JULY + 30.0, 0.25, 9, k);
            let up = path.iter().take_while(|a| a.aloft == Aloft::Flying).count();
            total += (up - 1) as f64 * 0.25;
        }
        total / 400.0
    };
    let sahara = life(LonLat { lon: 10.0, lat: 23.0 });
    let pool = life(LonLat { lon: 140.0, lat: 8.0 });
    eprintln!("a typical July's mean flight: {sahara:.2} days from the Sahara, {pool:.2} from the western Pacific's warm pool");
    assert!(sahara > 0.8 && sahara < 1.4, "dry: the settling's own 1 / 0.86 days, less a step's rounding");
    assert!(pool < sahara * 0.6, "rain washes things out");
}

#[test]
fn a_thing_down_stays_down_and_says_where() {
    let r = AirRules { fall: 50.0, ..air() };
    let a = r.step(r.rise(LonLat { lon: 2.0, lat: 47.0 }, JULY, 1), JULY, 0.25, 1, 1);
    assert_eq!(a.aloft, Aloft::Land, "France is land");
    assert_eq!(r.step(a, JULY + 0.25, 0.25, 1, 1), a);
    let s = r.step(r.rise(LonLat { lon: -30.0, lat: 30.0 }, JULY, 1), JULY, 0.25, 1, 1);
    assert_eq!(s.aloft, Aloft::Sea);
}

#[test]
fn the_gpu_steps_the_wind_as_the_cpu_does() {
    use crate::gpu::{Gpu, Textures};
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL, ..Default::default() });
    let gpu = match block(Gpu::new(&instance, None)) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("no GPU here ({e}); the mirror is not checked");
            return;
        }
    };
    let assets = DirAssets(assets());
    let textures = Textures::load(&gpu, &assets, 6).unwrap();
    let r = air();
    let mut starts = Vec::new();
    for lat in (-70..=70).step_by(10) {
        for lon in (-180..180).step_by(12) {
            starts.push(LonLat { lon: lon as f64 + 0.3, lat: lat as f64 + 0.3 });
        }
    }
    let n = starts.len() as u32;
    let maybes = super::AirMaybes::new(&gpu, &assets, &textures, n).unwrap();
    for (dt, day0) in [(0.25, JULY), (0.5, 20.37)] {
        let mut cpu: Vec<Air> = starts.iter().enumerate().map(|(i, p)| r.rise(*p, day0, i as u32 + 1)).collect();
        maybes.seed(&gpu, &cpu.iter().map(super::wind::gpu_path).collect::<Vec<_>>());
        let mut day = day0;
        let (mut worst, mut differ) = (0.0f64, 0);
        for _ in 0..12 {
            maybes.step(&gpu, AirParams::new(&r, day, dt, 5, n));
            for (i, a) in cpu.iter_mut().enumerate() {
                *a = r.step(*a, day, dt, 5, i as u32 + 1);
            }
            day += dt;
            let g = maybes.read(&gpu, n);
            for (a, p) in cpu.iter().zip(&g) {
                if a.aloft.code() != p.held {
                    differ += 1;
                    continue;
                }
                let d = super::drifters::distance_km(a.at.lon as f32, a.at.lat as f32, p.lon, p.lat) as f64;
                worst = worst.max(d);
            }
            // keep the two in step where a draw sat on its threshold
            let fixed: Vec<_> = cpu.iter().map(super::wind::gpu_path).collect();
            maybes.seed(&gpu, &fixed);
        }
        eprintln!("dt {dt}: worst gap {worst:.4} km over 12 steps of {n}; {differ} fates differ");
        assert!(worst < 0.5, "the GPU's step lands within half a kilometre of the CPU's");
        assert!(differ <= (n as usize * 12) / 1000, "a fate differs only where a draw sits on its threshold");
    }
}

fn block<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
    let mut cx = Context::from_waker(&waker);
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// The wind's paths for a picture (`tools/adrift/data/wind_paths.py` runs this and draws them):
/// a few dozen things lifted over the globe on the 1st of a typical month, each world k = 0
/// (its path) and k = 1..n (its maybes), every quarter day for `days`, written as JSON to
/// ADRIFT_WIND_OUT. ADRIFT_WIND_MONTH (1-12, default 7), ADRIFT_WIND_MAYBES (default 12),
/// ADRIFT_WIND_DAYS (default 30), ADRIFT_WIND_ALOFT=1 keeps every thing up (no settling, no rain).
#[test]
#[ignore]
fn wind_paths_for_the_picture() {
    let Some(out) = std::env::var_os("ADRIFT_WIND_OUT") else { return };
    let env = |k: &str, d: f64| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let month = env("ADRIFT_WIND_MONTH", 7.0) as usize;
    let n = env("ADRIFT_WIND_MAYBES", 12.0) as u32;
    let days = env("ADRIFT_WIND_DAYS", 30.0);
    let mut r = air();
    if env("ADRIFT_WIND_ALOFT", 0.0) > 0.0 {
        r.fall = 0.0;
        r.washout = 0.0;
    }
    let day0 = MONTH_START[month.clamp(1, 12) - 1];
    let mut things = Vec::new();
    for (i, lat) in (-50..=70).step_by(20).enumerate() {
        for lon in (-180..180).step_by(40) {
            let from = LonLat { lon: lon as f64 + 20.0 * (i % 2) as f64 + 7.0, lat: lat as f64 + 3.0 };
            let id = things.len() as u64;
            let paths: Vec<serde_json::Value> = (0..=n)
                .map(|k| {
                    let p = r.path(from, day0, day0 + days, 0.25, id, k);
                    let up = p.iter().take_while(|a| a.aloft == Aloft::Flying).count();
                    let end = p[(up).min(p.len() - 1)];
                    serde_json::json!({
                        "k": k,
                        "points": p[..(up + 1).min(p.len())].iter().map(|a| [(a.at.lon * 100.0).round() / 100.0, (a.at.lat * 100.0).round() / 100.0]).collect::<Vec<_>>(),
                        "down": match end.aloft { Aloft::Flying => "aloft", Aloft::Land => "land", Aloft::Sea => "sea" },
                    })
                })
                .collect();
            things.push(serde_json::json!({ "from": [from.lon, from.lat], "paths": paths }));
        }
    }
    let doc = serde_json::json!({ "month": month, "days": days, "dt": 0.25, "things": things });
    std::fs::write(out, serde_json::to_string(&doc).unwrap()).unwrap();
}

#[test]
fn the_air_crosses_the_pole() {
    use super::wind::great_circle;
    // 300 km north from 89N on the prime meridian comes down the 180th meridian at about 88.3N
    let (lon, lat) = great_circle(0.0, 89.0, 0.0, 300_000.0);
    assert!((lon.abs() - 180.0).abs() < 0.01 && (lat - 88.30).abs() < 0.01, "{lon} {lat}");
    // and the plain step it hands over to at 75 degrees agrees with the great circle to metres
    let (a, b) = (great_circle(10.0, 74.999, 20_000.0, 10_000.0), great_circle(10.0, 75.001, 20_000.0, 10_000.0));
    assert!((a.0 - b.0).abs() < 0.01 && (a.1 - b.1 - (-0.002)).abs() < 0.01, "{a:?} {b:?}");
}
