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
const MONTH_START: [f64; 12] = [
    0.0, 31.0, 59.0, 90.0, 120.0, 151.0, 181.0, 212.0, 243.0, 273.0, 304.0, 334.0,
];

#[test]
fn the_same_draws_give_the_same_path() {
    let r = air();
    let from = LonLat {
        lon: -40.0,
        lat: 45.0,
    };
    let a = r.path(from, JULY, JULY + 10.0, 0.25, 7, 3);
    let b = r.path(from, JULY, JULY + 10.0, 0.25, 7, 3);
    assert_eq!(a, b);
    let c = r.path(from, JULY, JULY + 10.0, 0.25, 7, 4);
    assert_ne!(
        a.last().unwrap().at,
        c.last().unwrap().at,
        "another world, another path"
    );
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
    let r = AirRules {
        fall: 0.0,
        washout: 0.0,
        ..air()
    };
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
                let path = r.path(
                    LonLat { lon, lat },
                    day0,
                    day0 + days,
                    0.25,
                    1,
                    k + (d0 as u32) * 100,
                );
                ends.push(displacement(&path));
            }
        }
        let n = ends.len() as f64;
        let (me, mn) = (
            ends.iter().map(|e| e.0).sum::<f64>() / n,
            ends.iter().map(|e| e.1).sum::<f64>() / n,
        );
        let spread = (ends
            .iter()
            .map(|e| (e.0 - me).powi(2) + (e.1 - mn).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();
        ratios.push(spread / real);
        let (re, rn) = (
            p["east_km"].as_f64().unwrap(),
            p["north_km"].as_f64().unwrap(),
        );
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
    assert!(
        q(&ratios, 0.5) > 0.8 && q(&ratios, 0.5) < 1.25,
        "the median start spreads as the real paths do"
    );
    assert!(
        q(&ratios, 0.1) > 0.5 && q(&ratios, 0.9) < 2.0,
        "nine in ten starts within a factor of two"
    );
    assert!(
        q(&drift_err, 0.5) < 0.5,
        "the cloud's middle goes where the real paths' middle goes"
    );
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
    let r = AirRules {
        fall: 0.0,
        washout: 0.0,
        ..air()
    };
    let a = LonLat {
        lon: -30.0,
        lat: 45.0,
    };
    let b = LonLat {
        lon: -29.5,
        lat: 45.0,
    };
    let (mut same, mut apart) = (0.0, 0.0);
    for k in 1..=50 {
        let pa = r.path(a, JULY, JULY + 3.0, 0.25, 1, k).pop().unwrap().at;
        let pb = r.path(b, JULY, JULY + 3.0, 0.25, 2, k).pop().unwrap().at;
        let pc = r
            .path(b, JULY, JULY + 3.0, 0.25, 2, k + 1000)
            .pop()
            .unwrap()
            .at;
        same += super::drifters::distance_km(
            pa.lon as f32,
            pa.lat as f32,
            pb.lon as f32,
            pb.lat as f32,
        ) as f64;
        apart += super::drifters::distance_km(
            pa.lon as f32,
            pa.lat as f32,
            pc.lon as f32,
            pc.lat as f32,
        ) as f64;
    }
    eprintln!(
        "after 3 days, 40 km apart at the lift: {:.0} km in one world, {:.0} in two",
        same / 50.0,
        apart / 50.0
    );
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
    let sahara = life(LonLat {
        lon: 10.0,
        lat: 23.0,
    });
    let pool = life(LonLat {
        lon: 140.0,
        lat: 8.0,
    });
    eprintln!("a typical July's mean flight: {sahara:.2} days from the Sahara, {pool:.2} from the western Pacific's warm pool");
    assert!(
        sahara > 0.8 && sahara < 1.4,
        "dry: the settling's own 1 / 0.86 days, less a step's rounding"
    );
    assert!(pool < sahara * 0.6, "rain washes things out");
}

#[test]
fn a_thing_down_stays_down_and_says_where() {
    let r = AirRules {
        fall: 50.0,
        ..air()
    };
    let a = r.step(
        r.rise(
            LonLat {
                lon: 2.0,
                lat: 47.0,
            },
            JULY,
            1,
        ),
        JULY,
        0.25,
        1,
        1,
    );
    assert_eq!(a.aloft, Aloft::Land, "France is land");
    assert_eq!(r.step(a, JULY + 0.25, 0.25, 1, 1), a);
    let s = r.step(
        r.rise(
            LonLat {
                lon: -30.0,
                lat: 30.0,
            },
            JULY,
            1,
        ),
        JULY,
        0.25,
        1,
        1,
    );
    assert_eq!(s.aloft, Aloft::Sea);
}

#[test]
fn the_gpu_steps_the_wind_as_the_cpu_does() {
    use crate::gpu::{Gpu, Textures};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
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
            starts.push(LonLat {
                lon: lon as f64 + 0.3,
                lat: lat as f64 + 0.3,
            });
        }
    }
    let n = starts.len() as u32;
    let maybes = super::AirMaybes::new(&gpu, &assets, &textures, n).unwrap();
    for (dt, day0) in [(0.25, JULY), (0.5, 20.37)] {
        let mut cpu: Vec<Air> = starts
            .iter()
            .enumerate()
            .map(|(i, p)| r.rise(*p, day0, i as u32 + 1))
            .collect();
        maybes.seed(
            &gpu,
            &cpu.iter().map(super::wind::gpu_path).collect::<Vec<_>>(),
        );
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
                let d = super::drifters::distance_km(a.at.lon as f32, a.at.lat as f32, p.lon, p.lat)
                    as f64;
                worst = worst.max(d);
            }
            // keep the two in step where a draw sat on its threshold
            let fixed: Vec<_> = cpu.iter().map(super::wind::gpu_path).collect();
            maybes.seed(&gpu, &fixed);
        }
        eprintln!("dt {dt}: worst gap {worst:.4} km over 12 steps of {n}; {differ} fates differ");
        assert!(
            worst < 0.5,
            "the GPU's step lands within half a kilometre of the CPU's"
        );
        assert!(
            differ <= (n as usize * 12) / 1000,
            "a fate differs only where a draw sits on its threshold"
        );
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
    let Some(out) = std::env::var_os("ADRIFT_WIND_OUT") else {
        return;
    };
    let env = |k: &str, d: f64| {
        std::env::var(k)
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(d)
    };
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
            let from = LonLat {
                lon: lon as f64 + 20.0 * (i % 2) as f64 + 7.0,
                lat: lat as f64 + 3.0,
            };
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
    assert!(
        (lon.abs() - 180.0).abs() < 0.01 && (lat - 88.30).abs() < 0.01,
        "{lon} {lat}"
    );
    // and the plain step it hands over to at 75 degrees agrees with the great circle to metres
    let (a, b) = (
        great_circle(10.0, 74.999, 20_000.0, 10_000.0),
        great_circle(10.0, 75.001, 20_000.0, 10_000.0),
    );
    assert!(
        (a.0 - b.0).abs() < 0.01 && (a.1 - b.1 - (-0.002)).abs() < 0.01,
        "{a:?} {b:?}"
    );
}

/// How long a seed stays up (R21, the preset's lifetime): from a grid of places over land and sea
/// in May and November, the median and the quartiles of the day it comes down, at the measured fall
/// and washout and at the preset's (`AirRules::preset`), and the seconds that is at the wind's pace.
/// `cargo test --release -p adrift-core a_seeds_life -- --ignored --nocapture`.
#[test]
#[ignore]
fn a_seeds_life() {
    let base = air();
    for (name, r) in [("measured", air()), ("preset", base.preset())] {
        for day0 in [130.0, 315.0] {
            let mut lives = Vec::new();
            for lat in (-50..=60).step_by(10) {
                for lon in (-180..180).step_by(20) {
                    let from = LonLat {
                        lon: lon as f64 + 0.5,
                        lat: lat as f64 + 0.5,
                    };
                    for k in 1..=8 {
                        let p = r.path(from, day0, day0 + 40.0, 0.25, 11, k);
                        let i = p
                            .iter()
                            .position(|a| a.aloft != Aloft::Flying)
                            .unwrap_or(p.len());
                        lives.push(i as f64 * 0.25);
                    }
                }
            }
            lives.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let q = |f: f64| lives[((lives.len() - 1) as f64 * f) as usize];
            eprintln!(
                "{name:8} day {day0}: life median {:.2} d (quartiles {:.2}–{:.2}, 90% {:.2}); at {} d/s: {:.1} s median",
                q(0.5), q(0.25), q(0.75), q(0.9), crate::app::AIR_RATE, q(0.5) / crate::app::AIR_RATE
            );
        }
    }
}

/// The wind's tempo (R21), as `conduct::tests::the_tempo` measured the sea's (R11), and scale-free so
/// the two media compare whatever their framing: the worlds' spread in a stroke against the cloud's
/// own size (its RMS radius about its centre), with the cloud framed 200 pt across in RMS (a phone's
/// take). For the sea, a lamp off Hatteras 15.5 days out (the R11 table's); for the wind, seeds lifted
/// over Indiana and off Virginia in May, 1.5 days out, at the preset's lifetime.
/// `cargo test --release -p adrift-core the_winds_tempo -- --ignored --nocapture`.
#[test]
#[ignore]
fn the_winds_tempo() {
    use crate::conduct::{Worlds, WORLDS};
    use crate::model::{ConductGesture, Leg, Medium};
    use crate::sea::{MediumRules, Rules};
    let sea = Rules::load(&DirAssets(assets())).unwrap();
    let wind = air().preset();
    let cases: [(&str, &dyn MediumRules, Leg, f64, &[f64]); 4] = [
        (
            "sea, Hatteras",
            &sea,
            Leg::new(
                LonLat {
                    lon: -72.0,
                    lat: 35.0,
                },
                160.0,
                7,
            ),
            15.5,
            &[2.0, 3.0, 4.0],
        ),
        (
            "wind, Indiana",
            &wind,
            Leg::new(
                LonLat {
                    lon: -86.0,
                    lat: 40.0,
                },
                130.0,
                7,
            )
            .on(Medium::Wind),
            1.5,
            &[0.3, 0.45, 0.6, 0.8, 1.0, 1.2],
        ),
        (
            "wind, Virginia",
            &wind,
            Leg::new(
                LonLat {
                    lon: -74.0,
                    lat: 37.0,
                },
                130.0,
                7,
            )
            .on(Medium::Wind),
            1.5,
            &[0.3, 0.45, 0.6, 0.8, 1.0, 1.2],
        ),
        (
            "wind, Indiana 3 d",
            &wind,
            Leg::new(
                LonLat {
                    lon: -86.0,
                    lat: 40.0,
                },
                130.0,
                7,
            )
            .on(Medium::Wind),
            3.0,
            &[0.6, 0.8, 1.0],
        ),
    ];
    for (name, rules, leg, age, rates) in cases {
        let mut w = Worlds::new(rules, leg, 0);
        let day = leg.day + age;
        w.step_to(rules, day);
        let up: Vec<u32> = (1..=WORLDS)
            .filter(|&k| w.place(k, day).held == crate::model::Held::No)
            .collect();
        // a flat projection about the cloud's centre, scaled so the cloud's RMS radius is 100 pt
        let pts: Vec<LonLat> = up.iter().map(|&k| w.place(k, day).at).collect();
        let n = pts.len() as f64;
        let (clon, clat) = (
            pts.iter().map(|p| p.lon).sum::<f64>() / n,
            pts.iter().map(|p| p.lat).sum::<f64>() / n,
        );
        let c = clat.to_radians().cos();
        let rms = (pts
            .iter()
            .map(|p| ((p.lon - clon) * c).powi(2) + (p.lat - clat).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();
        let scale = 100.0 / rms;
        let flat = move |p: LonLat| Some([(p.lon - clon) * c * scale, -(p.lat - clat) * scale]);
        eprintln!("{name}: {:.0}% of the worlds up at {age} days; the cloud's RMS radius {:.0} km ({:.1} pt a degree)", n / WORLDS as f64 * 100.0, rms * 111.2, scale);
        eprintln!("  rate  secs  marks  drift pt  spread pt  sweep 0.95 pt  sweep matched  turned >= 1/4 (1 s)");
        for &rate in rates {
            for secs in [0.5, 1.0] {
                let m =
                    ((secs * rate * 4.0).round() as usize).clamp(1, crate::conduct::HISTORY - 1);
                let d: Vec<[f64; 2]> = up
                    .iter()
                    .map(|&k| {
                        let (a, b) = (
                            flat(w.place(k, day).at).unwrap(),
                            flat(w.place(k, day - m as f64 * 0.25).at).unwrap(),
                        );
                        [a[0] - b[0], a[1] - b[1]]
                    })
                    .collect();
                let mean = [
                    d.iter().map(|x| x[0]).sum::<f64>() / n,
                    d.iter().map(|x| x[1]).sum::<f64>() / n,
                ];
                let spread = (d
                    .iter()
                    .map(|x| (x[0] - mean[0]).powi(2) + (x[1] - mean[1]).powi(2))
                    .sum::<f64>()
                    / n)
                    .sqrt();
                let mut north: Vec<f64> = d.iter().map(|x| -(x[1] - mean[1])).collect();
                north.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let q95 = north[(0.95 * (n - 1.0)) as usize];
                let sweep = w
                    .choose(
                        rules,
                        ConductGesture::Sweep {
                            dx: 0.0,
                            dy: -160.0,
                            secs,
                        },
                        day,
                        rate,
                        &flat,
                    )
                    .unwrap();
                let turned = if secs == 1.0 {
                    let c = w
                        .choose(
                            rules,
                            ConductGesture::Circle { turn: 5.0, secs },
                            day,
                            rate,
                            &flat,
                        )
                        .unwrap();
                    format!("{:.0}%", c.matches as f64 / n * 100.0)
                } else {
                    String::new()
                };
                eprintln!(
                    "  {rate:4}  {secs:4}  {m:5}  {:8.0}  {spread:9.0}  {q95:13.0}  {:12.0}%  {turned}",
                    mean[0].hypot(mean[1]),
                    sweep.matches as f64 / n * 100.0
                );
            }
        }
    }
}

/// Conducting on the wind (R21): the chooser's tap, sweep and circle at the wind's tempo, seeds
/// lifted over Indiana and off Virginia in May at the preset's lifetime, 1.5 days out, framed as
/// `the_winds_tempo` frames them (the cloud's RMS radius 100 pt). No pick is a seed come down (the
/// first measure picked one for a sweep against the drift, "matching" 49%: the down seeds stood
/// still), a tap reaches the finger, a sweep's pick moved the stroke's way, and the shares matched
/// are the sea's (tap a handful to a third, sweep 14–19%, circle 13–16% there). A circle's pick:
/// how far it turns the circle's way in the day after the cue, against the median world's.
#[test]
fn conducting_on_the_wind() {
    use crate::conduct::{looping, Worlds, AIR_TEMPO, TAP_NEAR, WORLDS};
    use crate::model::{ConductGesture, Held, Leg, Medium};
    let wind = air().preset();
    for (name, at) in [
        (
            "Indiana",
            LonLat {
                lon: -86.0,
                lat: 40.0,
            },
        ),
        (
            "off Virginia",
            LonLat {
                lon: -74.0,
                lat: 37.0,
            },
        ),
    ] {
        let leg = Leg::new(at, 130.0, 7).on(Medium::Wind);
        let mut w = Worlds::new(&wind, leg, 0);
        let day = 131.5;
        w.step_to(&wind, day);
        let up = |k: u32| w.place(k, day).held == Held::No;
        let ups: Vec<u32> = (1..=WORLDS).filter(|&k| up(k)).collect();
        let pts: Vec<LonLat> = ups.iter().map(|&k| w.place(k, day).at).collect();
        let n = pts.len() as f64;
        let (clon, clat) = (
            pts.iter().map(|p| p.lon).sum::<f64>() / n,
            pts.iter().map(|p| p.lat).sum::<f64>() / n,
        );
        let c = clat.to_radians().cos();
        let rms = (pts
            .iter()
            .map(|p| ((p.lon - clon) * c).powi(2) + (p.lat - clat).powi(2))
            .sum::<f64>()
            / n)
            .sqrt();
        let scale = 100.0 / rms;
        let flat = move |p: LonLat| Some([(p.lon - clon) * c * scale, -(p.lat - clat) * scale]);
        let share = |m: u32| m as f64 / n * 100.0;
        let mut report = format!("{name}: {:.0}% up;", n / WORLDS as f64 * 100.0);
        for &k in [ups[3], ups[ups.len() / 2], ups[ups.len() - 2]].iter() {
            let [x, y] = flat(w.place(k, day).at).unwrap();
            let ch = w
                .choose(&wind, ConductGesture::Tap { x, y }, day, AIR_TEMPO, &flat)
                .unwrap();
            assert!(up(ch.k), "a tap picks a seed still up");
            let [px, py] = flat(w.place(ch.k, day).at).unwrap();
            assert!(
                (px - x).hypot(py - y) < TAP_NEAR,
                "the tap's pick is under the finger"
            );
            report += &format!(" tap {} matched,", ch.matches);
        }
        for (dx, dy) in [(0.0, -160.0), (160.0, 0.0), (0.0, 160.0), (-160.0, 0.0)] {
            let ch = w
                .choose(
                    &wind,
                    ConductGesture::Sweep { dx, dy, secs: 0.6 },
                    day,
                    AIR_TEMPO,
                    &flat,
                )
                .unwrap();
            assert!(up(ch.k), "a sweep picks a seed still up");
            let f = share(ch.matches);
            report += &format!(" sweep ({dx},{dy}) {f:.0}%,");
            assert!(
                f > 3.0 && f < 40.0,
                "a sweep matches the sea's share, not the down seeds: {f:.0}%"
            );
        }
        for turn in [5.0, -5.0] {
            let ch = w
                .choose(
                    &wind,
                    ConductGesture::Circle { turn, secs: 1.0 },
                    day,
                    AIR_TEMPO,
                    &flat,
                )
                .unwrap();
            assert!(up(ch.k), "a circle picks a seed still up");
            let ahead = w.ahead(&wind, 4);
            let turned = |k: u32| {
                let mut p = vec![flat(w.place(k, day).at).unwrap()];
                p.extend(
                    ahead
                        .iter()
                        .take_while(|m| m[k as usize - 1].held == Held::No)
                        .map(|m| flat(m[k as usize - 1].at).unwrap()),
                );
                looping(&p) * turn.signum()
            };
            let mut all: Vec<f64> = ups.iter().map(|&k| turned(k)).collect();
            all.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let stays = ahead
                .iter()
                .filter(|m| m[ch.k as usize - 1].held == Held::No)
                .count();
            report += &format!(" circle {turn:+} {:.0}% (the pick turns {:+.2} rad its way in the day after, the median world {:+.2}; up {stays}/4 marks),", share(ch.matches), turned(ch.k), all[all.len() / 2]);
        }
        eprintln!("{report}");
    }
}

/// A seed's cloud in the app's buffer (R21): the wind's pipeline over the sea's maybes' paths
/// (`Maybes.air`), a cloud at a base past a lamp's, stepped as `App::step_clouds` steps it (the
/// quarter-day grid from an off-grid put, frame-sized dates, park and glide), lands where the CPU's
/// `MediumRules` (`Media::of(Medium::Wind)`) puts path k at every mark; the glide arrives where the
/// mark's step does; the lamp's cloud beside it is untouched.
#[test]
fn a_seeds_cloud_is_its_cpu_paths() {
    use super::gpu::{GpuPath, Maybes};
    use super::medium::Media;
    use super::wind::{glide, grid_steps};
    use crate::gpu::{Gpu, Textures};
    use crate::model::{Held, Medium};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let gpu = match block(Gpu::new(&instance, None)) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("no GPU here ({e}); the cloud is not checked");
            return;
        }
    };
    let assets = DirAssets(assets());
    let textures = Textures::load(&gpu, &assets, 6).unwrap();
    let sea = super::Rules::load(&assets).unwrap();
    let air = AirRules::sharing(&sea, &assets).unwrap();
    let media = Media {
        sea: &sea,
        air: &air,
    };
    let rules = media.of(Medium::Wind);
    let n = 512u32;
    let maybes = Maybes::new(&gpu, &assets, &textures, 2 * n).unwrap();
    let lamp: Vec<GpuPath> = (0..n)
        .map(|i| GpuPath {
            lon: -40.0 + i as f32 * 0.01,
            lat: 30.0,
            measured: 1.0,
            ..Default::default()
        })
        .collect();
    maybes.seed_at(&gpu, 0, &lamp);
    let (at, day0, seed, base) = (
        LonLat {
            lon: -86.0,
            lat: 40.0,
        },
        130.37,
        7u64,
        n,
    );
    // the CPU's paths on the grid, as App::extend_path steps a leg's samples
    let mut cpu: Vec<(super::Drift, Held)> = (1..=n)
        .map(|k| (rules.put(at, day0, seed, k), Held::No))
        .collect();
    let paths: Vec<GpuPath> = cpu
        .iter()
        .map(|(d, _)| GpuPath {
            lon: at.lon as f32,
            lat: at.lat as f32,
            held: 0,
            measured: 0.0,
            eddy_u: d.eddy.0,
            eddy_v: d.eddy.1,
        })
        .collect();
    maybes.air.seed_at(&gpu, base, &paths);
    let (mut reached, mut cpu_day) = (day0, day0);
    let (mut worst, mut fates, mut glide_gap, mut marks) = (0.0f64, 0, 0.0f64, 0);
    let mut date = day0;
    for frame in 0..400 {
        // a frame's date: 0.3 days a second at 60 frames, with a jump now and then (a dropped frame)
        date += if frame % 37 == 36 { 0.4 } else { 0.005 };
        if frame > 0 {
            maybes.air.unpark(&gpu, base, n);
        }
        let steps = grid_steps(&air, reached, date, seed, n, base, 24);
        let batch: Vec<_> = steps.iter().map(|s| s.0).collect();
        maybes.air.step_many(&gpu, &batch);
        if let Some(&(_, to)) = steps.last() {
            reached = to;
        }
        maybes.air.park(&gpu, base, n);
        let glided = glide(&air, reached, date, seed, n, base);
        if let Some(g) = glided {
            maybes.air.step(&gpu, g);
        }
        if steps.is_empty() {
            continue;
        }
        // the CPU to the same mark, and the state at the mark (unparked) compared
        while cpu_day < reached - 1e-9 {
            let next = crate::app::grid_next(cpu_day);
            for (i, (d, h)) in cpu.iter_mut().enumerate() {
                let f = rules.flight(0, *d, *h, cpu_day, next - cpu_day, seed, i as u32 + 1);
                *d = f.drift;
                *h = f.held;
            }
            cpu_day = next;
        }
        maybes.air.unpark(&gpu, base, n);
        let got = maybes.air.read_at(&gpu, base, n);
        marks += 1;
        for ((d, h), g) in cpu.iter().zip(&got) {
            let code = match h {
                Held::Landed => 1,
                Held::Ditched => 2,
                _ => 0,
            };
            if code != g.held {
                fates += 1;
                continue;
            }
            worst = worst.max(super::drifters::distance_km(
                d.at.lon as f32,
                d.at.lat as f32,
                g.lon,
                g.lat,
            ) as f64);
        }
        // the glide to the next mark's day lands where the mark's step does, for the seeds that stay up
        let next = crate::app::grid_next(reached);
        let mut g = glide(&air, reached, next - 1e-7, seed, n, base).unwrap();
        g.dt = (next - reached) as f32;
        maybes.air.step(&gpu, g);
        let gl = maybes.air.read_at(&gpu, base, n);
        maybes.air.unpark(&gpu, base, n);
        maybes.air.step(
            &gpu,
            super::AirParams::new(&air, reached, next - reached, seed, n).at_base(base),
        );
        let full = maybes.air.read_at(&gpu, base, n);
        for (a, b) in gl.iter().zip(&full) {
            if a.held == 0 && b.held == 0 {
                glide_gap =
                    glide_gap.max(super::drifters::distance_km(a.lon, a.lat, b.lon, b.lat) as f64);
            }
        }
        maybes.air.unpark(&gpu, base, n);
        if let Some(g) = glided {
            maybes.air.step(&gpu, g);
        }
    }
    let up = cpu.iter().filter(|(_, h)| *h == Held::No).count();
    eprintln!("a seed's cloud over {marks} frames with marks to day {reached:.2}: worst gap {worst:.4} km, {fates} fates differ of {}, the glide's end {glide_gap:.6} km from the mark's step; {up} of {n} still up", n as usize * marks);
    assert!(reached > day0 + 5.0, "the cloud kept up with the date");
    assert!(worst < 1.0, "the GPU's cloud is the CPU's paths");
    assert!(
        fates <= n as usize * marks / 500,
        "a fate differs only where a draw sits on its threshold"
    );
    assert!(
        glide_gap < 0.01,
        "the glide arrives where the mark's step puts a seed"
    );
    let lamp_after = maybes.read_at(&gpu, 0, n);
    assert!(
        lamp_after
            .iter()
            .zip(&lamp)
            .all(|(a, b)| a.lon == b.lon && a.lat == b.lat && a.held == b.held),
        "the lamp's cloud beside it is untouched"
    );
}
