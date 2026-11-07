//! The rules held to the record (CLAUDE.md: a claim is a test or a PNG): the Gulf Stream at its
//! measured speed, a coast that holds, ice that holds till it retreats, the same seed the same
//! path, the draws standard, the maybes' spread the measured diffusivity, the nearest real
//! drifter usually near. `cargo test -p adrift-core sea::`.

use super::drifters::distance_km;
use super::rules::{gaussian, hash_seed, uniform};
use super::{Drift, Drifters, Put, Rules};
use crate::assets::DirAssets;
use crate::model::{Clock, Held, LonLat};
use std::path::{Path, PathBuf};

/// The assembled assets (`python3 tools/adrift/assets.py adrift/build/assets`), or wherever
/// ADRIFT_ASSETS says, as adrift-check reads them.
fn assets() -> PathBuf {
    std::env::var_os("ADRIFT_ASSETS")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/assets"))
}

fn rules() -> Rules {
    Rules::load(&DirAssets(assets())).unwrap()
}

#[test]
fn the_months_are_read_as_the_clock_reads_them() {
    for day in [0.0, 15.0, 16.0, 100.5, 182.0, 364.9, 400.0, 9000.25] {
        let (a, b, t) = super::sheets::months(day as f32);
        let (ca, cb, ct) = Clock {
            day,
            running: false,
            rate: 0.0,
        }
        .months();
        assert_eq!((a, b), (ca as usize, cb as usize), "day {day}");
        assert!((t - ct as f32).abs() < 2e-3, "day {day}: {t} against {ct}");
    }
}

const JULY: f64 = 15.21875 + 6.0 * 30.4375; // the middle of a typical July

#[test]
fn gulf_stream_runs_north_east_at_the_measured_speed() {
    let mut r = rules();
    r.wander = 0.0;
    r.leeway = 0.0;
    let from = LonLat {
        lon: -74.5,
        lat: 35.8,
    }; // off Cape Hatteras
    let (u, v, measured) = r.currents.at(JULY as f32, from.lon as f32, from.lat as f32);
    assert!(measured > 0.99, "the Gulf Stream is measured");
    let speed = (u * u + v * v).sqrt();
    assert!(
        speed > 0.5 && u > 0.0 && v > 0.0,
        "the Gulf Stream here runs NE at over 0.5 m/s: ({u}, {v})"
    );
    let s = r.step(from, JULY, 1.0, 1, 0);
    assert_eq!(s.held, Held::No);
    let km = distance_km(
        from.lon as f32,
        from.lat as f32,
        s.at.lon as f32,
        s.at.lat as f32,
    );
    let expect = speed * 86400.0 / 1000.0;
    assert!(
        (km - expect).abs() < 0.02 * expect,
        "a day's run {km} km against {expect}"
    );
    assert!(s.at.lon > from.lon && s.at.lat > from.lat, "north-east");
}

#[test]
fn a_coast_holds() {
    let r = rules();
    // Cape Hatteras's beach: a point just inland, from one just offshore
    let coast = &r.coast;
    assert!(
        coast.is_sea(-75.2, 35.4) && !coast.is_sea(-77.5, 35.4),
        "the sheet knows the shore"
    );
    let (lon, lat, held) = r.ashore(-75.2, 35.4, -77.5, 35.4);
    assert_eq!(held, Held::Coast);
    assert!(
        coast.is_sea(lon, lat),
        "put back on the water at ({lon}, {lat})"
    );
    assert!(coast.distance(lon, lat) <= -r.shore * 0.5);
    // and a float in the sea with no land near takes its step
    assert_eq!(r.ashore(-40.0, 30.0, -40.1, 30.1).2, Held::No);
}

#[test]
fn ice_holds_till_it_retreats() {
    let mut r = rules();
    r.wander = 0.0;
    let beaufort = LonLat {
        lon: -150.0,
        lat: 74.0,
    };
    let march = 15.2 + 2.0 * 30.4375;
    let september = 15.2 + 8.0 * 30.4375;
    let held = r.step(beaufort, march, 1.0, 1, 0);
    assert_eq!(held.held, Held::Ice);
    assert_eq!(held.at, beaufort, "held where it is");
    let free = r.step(beaufort, september, 1.0, 1, 0);
    assert_eq!(free.held, Held::No, "the ice has retreated in September");
    assert_ne!(free.at, beaufort);
}

#[test]
fn the_same_seed_gives_the_same_path_and_another_k_another() {
    let r = rules();
    let from = LonLat {
        lon: -40.0,
        lat: 30.0,
    };
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
        sg += g;
        sg2 += g * g;
    }
    let n = n as f64;
    assert!((su / n - 0.5).abs() < 0.005, "uniform mean {}", su / n);
    assert!((sg / n).abs() < 0.01, "gaussian mean {}", sg / n);
    assert!(
        (sg2 / n - 1.0).abs() < 0.02,
        "gaussian variance {}",
        sg2 / n
    );
}

#[test]
fn the_maybes_spread_as_the_diffusivity_says() {
    let mut r = rules();
    r.leeway = 0.0;
    let from = LonLat {
        lon: -40.0,
        lat: 30.0,
    }; // the Sargasso Sea
    let (ku, kv, measured) = r.eddy.at(from.lon as f32, from.lat as f32);
    assert!(measured > 0.99);
    let days = 30.0;
    let n = 3000;
    let ends = r.maybes(from, JULY, JULY + days, 0.25, 5, n);
    let coslat = (from.lat as f32).to_radians().cos();
    let m = 6_371_000.0f32;
    let xs: Vec<f32> = ends
        .iter()
        .map(|s| ((s.at.lon - from.lon) as f32).to_radians() * m * coslat)
        .collect();
    let ys: Vec<f32> = ends
        .iter()
        .map(|s| ((s.at.lat - from.lat) as f32).to_radians() * m)
        .collect();
    let var = |v: &[f32]| {
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32
    };
    let (vx, vy) = (var(&xs), var(&ys));
    let (ex, ey) = (
        2.0 * ku * days as f32 * 86400.0,
        2.0 * kv * days as f32 * 86400.0,
    );
    // K varies over the cloud (about 100 km across), so within a fifth
    assert!(
        (vx / ex - 1.0).abs() < 0.2,
        "zonal spread {} against 2Kt {} (K {ku})",
        vx.sqrt() / 1000.0,
        ex.sqrt() / 1000.0
    );
    assert!(
        (vy / ey - 1.0).abs() < 0.2,
        "meridional spread {} against 2Kt {} (K {kv})",
        vy.sqrt() / 1000.0,
        ey.sqrt() / 1000.0
    );
    assert!(ends.iter().all(|s| s.held == Held::No));
}

#[test]
fn the_nearest_real_drifter_is_usually_near() {
    let d = Drifters::load(&assets().join("drifters.bin")).unwrap();
    assert!(d.runs.len() > 1000 && d.points() > 500_000);
    let r = rules();
    // a grid of sea points: how far is the nearest drifter point
    let mut far = 0;
    let mut total = 0;
    let mut sum = 0.0;
    for lat in (-60..=60).step_by(5) {
        for lon in (-180..180).step_by(5) {
            let (lon, lat) = (lon as f32 + 0.5, lat as f32 + 0.5);
            if !r.coast.is_sea(lon, lat) || r.coast.distance(lon, lat) > -10.0 {
                continue;
            }
            total += 1;
            match d.nearest(
                LonLat {
                    lon: lon as f64,
                    lat: lat as f64,
                },
                2000.0,
            ) {
                Some(n) => {
                    sum += n.km;
                    if n.km > 300.0 {
                        far += 1;
                    }
                }
                None => far += 1,
            }
        }
    }
    let mean = sum / total as f32;
    eprintln!("{total} open-sea points: mean nearest {mean:.0} km, {far} beyond 300 km");
    assert!(mean < 150.0, "mean nearest drifter point {mean} km");
    assert!(
        (far as f32) < 0.1 * total as f32,
        "{far} of {total} points have no drifter within 300 km"
    );
}

#[test]
fn the_flight_spreads_as_the_diffusivity_says_and_keeps_its_way() {
    let mut r = rules();
    r.leeway = 0.0;
    let from = LonLat {
        lon: -40.0,
        lat: 30.0,
    };
    let (ku, kv, _) = r.eddy.at(from.lon as f32, from.lat as f32);
    let (tu, tv) = r.eddy_time.at(from.lon as f32, from.lat as f32);
    assert!(
        tu > 1.0 && tu < 6.0 && tv > 1.0 && tv < 6.0,
        "time scales of days: {tu}, {tv}"
    );
    let days = 30.0f32;
    let ends = r.flights(Drift::put(from), JULY, JULY + days as f64, 0.25, 5, 3000);
    let coslat = (from.lat as f32).to_radians().cos();
    let m = 6_371_000.0f32;
    let xs: Vec<f32> = ends
        .iter()
        .map(|f| ((f.drift.at.lon - from.lon) as f32).to_radians() * m * coslat)
        .collect();
    let ys: Vec<f32> = ends
        .iter()
        .map(|f| ((f.drift.at.lat - from.lat) as f32).to_radians() * m)
        .collect();
    let var = |v: &[f32]| {
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32
    };
    // Taylor (1921): the variance is 2 K (t - T (1 - exp(-t / T)))
    let expect = |k: f32, t: f32| 2.0 * k * 86400.0 * (days - t * (1.0 - (-days / t).exp()));
    let (vx, vy) = (var(&xs), var(&ys));
    let (ex, ey) = (expect(ku, tu), expect(kv, tv));
    assert!(
        (vx / ex - 1.0).abs() < 0.2,
        "zonal spread {} km against {} (K {ku}, T {tu})",
        vx.sqrt() / 1000.0,
        ex.sqrt() / 1000.0
    );
    assert!(
        (vy / ey - 1.0).abs() < 0.2,
        "meridional spread {} km against {} (K {kv}, T {tv})",
        vy.sqrt() / 1000.0,
        ey.sqrt() / 1000.0
    );
    // the carried velocity's variance is K / T
    let vu = var(&ends.iter().map(|f| f.drift.eddy.0).collect::<Vec<_>>());
    let eu = ku / (tu * 86400.0);
    assert!(
        (vu / eu - 1.0).abs() < 0.2,
        "eddy velocity variance {vu} against K/T {eu}"
    );
    // and a flight is deterministic
    let a = r.flight(Drift::put(from), JULY, 0.25, 5, 7);
    let b = r.flight(Drift::put(from), JULY, 0.25, 5, 7);
    assert_eq!(a.drift, b.drift);
}

/// The D1 observable (DEMANDS.md): where 500 maybes are thirty days after Cape Hatteras in a
/// typical July, beside where the bundled GDP runs that passed there were (18 runs: half east of
/// 65 W, median 64.8 W, none ashore). The walk is printed for comparison; the flight is held to
/// the record loosely here, exactly in the demand.
#[test]
fn the_flight_off_hatteras_keeps_to_the_record() {
    let r = rules();
    let from = LonLat {
        lon: -74.5,
        lat: 35.8,
    };
    let summary = |lons: &mut Vec<f64>, coast: usize, name: &str| {
        lons.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let east = lons.iter().filter(|l| **l >= -65.0).count();
        eprintln!("{name}: {} of {} east of 65 W, {} on a coast, median {:.1} W (the record: 9 of 18, none, 64.8)", east, lons.len(), coast, -lons[lons.len() / 2]);
        (east as f64 / lons.len() as f64, lons[lons.len() / 2])
    };
    let walk = r.maybes(from, JULY, JULY + 30.0, 0.25, 11, 500);
    summary(
        &mut walk.iter().map(|s| s.at.lon).collect(),
        walk.iter().filter(|s| s.held == Held::Coast).count(),
        "the walk",
    );
    let flight = r.flights(Drift::put(from), JULY, JULY + 30.0, 0.25, 11, 500);
    let (east, median) = summary(
        &mut flight.iter().map(|f| f.drift.at.lon).collect(),
        flight.iter().filter(|f| f.held == Held::Coast).count(),
        "the flight",
    );
    assert!(
        (median + 64.8).abs() < 4.0,
        "the flight's median {median} is far from the record's 64.8 W"
    );
    assert!(
        (east - 0.5).abs() < 0.2,
        "the flight's share east of 65 W, {east}, is far from the record's half"
    );
}

/// The GPU mirror held to the CPU: the same paths stepped by `rules.wgsl` and by `Rules::step`
/// (and `flight`) land within a few metres after a step and within a kilometre after eight (the
/// sheets are read the same way; only the rounding of a fused multiply differs).
#[test]
fn the_gpu_steps_as_the_cpu_does() {
    use super::gpu::{GpuPath, Maybes, Params};
    use crate::gpu::{Gpu, Textures};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let gpu = match pollster_block(Gpu::new(&instance, None)) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("no GPU here ({e}); the mirror is not checked");
            return;
        }
    };
    let assets = DirAssets(assets());
    let textures = Textures::load(&gpu, &assets, 6).unwrap();
    let r = rules();
    // a grid of sea points, a quarter day at a time
    let mut starts = Vec::new();
    for lat in (-60..=70).step_by(10) {
        for lon in (-180..180).step_by(15) {
            let (lon, lat) = (lon as f64 + 0.5, lat as f64 + 0.5);
            if r.coast.is_sea(lon as f32, lat as f32) {
                starts.push(LonLat { lon, lat });
            }
        }
    }
    let n = starts.len() as u32;
    let maybes = Maybes::new(&gpu, &assets, &textures, n).unwrap();
    // the walk and the flight a quarter day at a time from the grid; the flight half a day at a
    // time from a day off it (the maybes' steps: a world's kick spans two quarter days)
    for (flight, dt, day0) in [
        (false, 0.25, JULY),
        (true, 0.25, JULY),
        (true, 0.5, JULY + 0.37),
    ] {
        let paths: Vec<GpuPath> = starts
            .iter()
            .map(|p| GpuPath {
                lon: p.lon as f32,
                lat: p.lat as f32,
                ..Default::default()
            })
            .collect();
        maybes.seed(&gpu, &paths);
        let mut cpu: Vec<Drift> = starts.iter().map(|p| Drift::put(*p)).collect();
        let mut day = day0;
        let mut worst = 0.0f32;
        for step in 0..8 {
            maybes.step(&gpu, Params::new(&r, day, dt, 99, n), flight);
            for (i, d) in cpu.iter_mut().enumerate() {
                let k = i as u32 + 1;
                if flight {
                    *d = r.flight(*d, day, dt, 99, k).drift;
                } else {
                    d.at = r.step(d.at, day, dt, 99, k).at;
                }
            }
            day += dt;
            let got = maybes.read(&gpu, n);
            for (g, d) in got.iter().zip(&cpu) {
                let km = distance_km(g.lon, g.lat, d.at.lon as f32, d.at.lat as f32);
                worst = worst.max(km);
            }
            if step == 0 {
                assert!(
                    worst < 0.01,
                    "after one step the GPU and the CPU differ by {worst} km ({})",
                    if flight { "flight" } else { "walk" }
                );
            }
        }
        eprintln!(
            "{}: {} paths, worst gap after 8 steps of {dt} days {:.4} km",
            if flight { "flight" } else { "walk" },
            n,
            worst
        );
        assert!(
            worst < 1.0,
            "after eight steps the GPU and the CPU differ by {worst} km"
        );
    }
}

fn pollster_block<F: std::future::Future>(f: F) -> F::Output {
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

#[test]
fn a_put_float_starts_with_the_measured_velocity_and_a_track_has_real_dates() {
    let r = rules();
    let from = LonLat {
        lon: -40.0,
        lat: 30.0,
    };
    let (ku, kv, _) = r.eddy.at(-40.0, 30.0);
    let (tu, tv) = r.eddy_time.at(-40.0, 30.0);
    let n = 3000;
    let (mut su, mut sv) = (0.0f64, 0.0f64);
    for k in 0..n {
        let d = r.put(from, JULY, 5, k);
        su += (d.eddy.0 as f64).powi(2);
        sv += (d.eddy.1 as f64).powi(2);
    }
    let (eu, ev) = (ku / (tu * 86400.0), kv / (tv * 86400.0));
    assert!(
        (su / n as f64 / eu as f64 - 1.0).abs() < 0.15,
        "zonal first velocity variance {} against K/T {eu}",
        su / n as f64
    );
    assert!(
        (sv / n as f64 / ev as f64 - 1.0).abs() < 0.15,
        "meridional {} against {ev}",
        sv / n as f64
    );
    assert_eq!(r.put(from, JULY, 5, 3).eddy, r.put(from, JULY, 5, 3).eddy);
    assert_ne!(r.put(from, JULY, 5, 3).eddy, r.put(from, JULY, 5, 4).eddy);
    // the calendar is the real one
    assert_eq!(super::drifters::civil(0.0), (2000, 1, 1));
    assert_eq!(super::drifters::civil(59.0), (2000, 2, 29));
    assert_eq!(super::drifters::civil(60.0), (2000, 3, 1));
    assert_eq!(super::drifters::civil(9000.0), (2024, 8, 22));
    let d = Drifters::load(&assets().join("drifters.bin")).unwrap();
    let t = d
        .track(
            LonLat {
                lon: -74.5,
                lat: 35.8,
            },
            500.0,
        )
        .expect("a drifter passed Hatteras");
    assert!(t.km < 100.0 && t.xyz.len() == t.run.points.len() && t.xyz.len() >= 2);
    let v = t.xyz[t.point];
    assert!(
        (v[0] * v[0] + v[1] * v[1] + v[2] * v[2] - 1.0).abs() < 1e-4,
        "unit vectors"
    );
    let label = t.label();
    let w = t.window(30);
    assert!(w.contains(&t.point) && w.len() <= 61 && w.len() >= 31);
    let short = t.label_window(w.clone());
    eprintln!(
        "{label} ({:.0} km off); the month about it: {short}, {} points",
        t.km,
        w.len()
    );
    assert!(short.starts_with("Drifter ") && short.contains(" to "));
    assert!(
        label.starts_with("Drifter ") && label.contains(" to ") && label.contains("201")
            || label.contains("202")
    );
}

/// No maybe is ever on land: the held float of `gulf-stream-close.json`, its paths flown twenty
/// days on the CPU and on the GPU, every one on the water at the end and after every step.
#[test]
fn no_maybe_lies_on_land() {
    use super::gpu::{GpuPath, Maybes, Params};
    use crate::gpu::{Gpu, Textures};
    let r = rules();
    let from = LonLat {
        lon: -75.5,
        lat: 33.0,
    };
    let (day0, days, dt, seed, n) = (180.0, 20.0, 0.25, 7u64, 2000u32);
    let ends = r.flights(Drift::put(from), day0, day0 + days, dt, seed, n);
    let ashore: Vec<_> = ends
        .iter()
        .filter(|f| r.coast.is_land(f.drift.at))
        .collect();
    assert!(
        ashore.is_empty(),
        "{} of {n} CPU maybes end on land, e.g. {:?}",
        ashore.len(),
        ashore.first().map(|f| f.drift.at)
    );
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let gpu = match pollster_block(Gpu::new(&instance, None)) {
        Ok(g) => g,
        Err(_) => return,
    };
    let assets = DirAssets(assets());
    let textures = Textures::load(&gpu, &assets, 6).unwrap();
    let maybes = Maybes::new(&gpu, &assets, &textures, n).unwrap();
    let paths: Vec<GpuPath> = (1..=n)
        .map(|k| {
            let d = r.put(from, day0, seed, k);
            GpuPath {
                lon: from.lon as f32,
                lat: from.lat as f32,
                held: 0,
                measured: 1.0,
                eddy_u: d.eddy.0,
                eddy_v: d.eddy.1,
            }
        })
        .collect();
    maybes.seed(&gpu, &paths);
    let mut day = day0;
    let mut worst = 0usize;
    while day < day0 + days - 1e-9 {
        maybes.step(&gpu, Params::new(&r, day, dt, seed, n), true);
        day += dt;
        let got = maybes.read(&gpu, n);
        let on_land = got
            .iter()
            .filter(|p| {
                r.coast.is_land(LonLat {
                    lon: p.lon as f64,
                    lat: p.lat as f64,
                })
            })
            .count();
        worst = worst.max(on_land);
    }
    assert_eq!(worst, 0, "GPU maybes on land at some step");
}

/// A tool, not a check (ignored): what count and step the maybes' honesty needs, for the device's
/// budget (DECISIONS.md R3).
///
///     cargo test -p adrift-core maybes_budget -- --ignored --nocapture
///
/// Each (paths, step) cloud thirty days after a put off Cape Hatteras (a jet) and in the Sargasso
/// Sea (quiet) against a reference of 8192 paths at a quarter day: Kolmogorov's greatest
/// difference of the clouds' cumulative shares along longitude and latitude (0 the same shape,
/// 0.05 a difference an eye can't see in specks), the share east of 65 W, the spread, how many
/// ashore.
#[test]
#[ignore]
fn maybes_budget() {
    let r = rules();
    let ks = |a: &mut Vec<f64>, b: &mut Vec<f64>| {
        a.sort_by(|x, y| x.partial_cmp(y).unwrap());
        b.sort_by(|x, y| x.partial_cmp(y).unwrap());
        let (mut i, mut j, mut worst) = (0usize, 0usize, 0.0f64);
        while i < a.len() && j < b.len() {
            if a[i] <= b[j] {
                i += 1
            } else {
                j += 1
            }
            worst = worst.max((i as f64 / a.len() as f64 - j as f64 / b.len() as f64).abs());
        }
        worst
    };
    let std = |v: &[f64]| {
        let m = v.iter().sum::<f64>() / v.len() as f64;
        (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt()
    };
    for (name, from) in [
        (
            "Hatteras",
            LonLat {
                lon: -74.5,
                lat: 35.8,
            },
        ),
        (
            "Sargasso",
            LonLat {
                lon: -40.0,
                lat: 30.0,
            },
        ),
    ] {
        let reference = r.flights(Drift::put(from), JULY, JULY + 30.0, 0.25, 3, 8192);
        let mut rlon: Vec<f64> = reference.iter().map(|f| f.drift.at.lon).collect();
        let mut rlat: Vec<f64> = reference.iter().map(|f| f.drift.at.lat).collect();
        let reast = rlon.iter().filter(|l| **l >= -65.0).count() as f64 / rlon.len() as f64;
        eprintln!("{name}: reference 8192 paths at 1/4 day: spread {:.0} x {:.0} km, east of 65 W {:.2}, ashore {}",
            std(&rlon) * 111.0 * (from.lat.to_radians().cos()), std(&rlat) * 111.0, reast, reference.iter().filter(|f| f.held == Held::Coast).count());
        eprintln!("  paths  step   KS lon  KS lat  east  spread km   ashore");
        for n in [256u32, 512, 1024, 2048] {
            for dt in [0.25, 0.5, 1.0] {
                let c = r.flights(Drift::put(from), JULY, JULY + 30.0, dt, 3, n);
                let mut lon: Vec<f64> = c.iter().map(|f| f.drift.at.lon).collect();
                let mut lat: Vec<f64> = c.iter().map(|f| f.drift.at.lat).collect();
                let east = lon.iter().filter(|l| **l >= -65.0).count() as f64 / n as f64;
                let ashore = c.iter().filter(|f| f.held == Held::Coast).count();
                let (sx, sy) = (
                    std(&lon) * 111.0 * from.lat.to_radians().cos(),
                    std(&lat) * 111.0,
                );
                eprintln!(
                    "  {n:5}  {dt:4.2}   {:.3}   {:.3}   {east:.2}  {sx:4.0} x {sy:4.0}   {ashore}",
                    ks(&mut lon, &mut rlon.clone()),
                    ks(&mut lat, &mut rlat.clone())
                );
            }
        }
    }
}

/// Where a float may be put: the record's reach decides, and lakes and the seas no drifter
/// entered refuse it; the enclosed seas the programme measured take it.
#[test]
fn where_a_float_may_be_put() {
    let r = rules();
    let put = |lon, lat| r.may_put(LonLat { lon, lat });
    assert_eq!(put(-77.5, 35.4), Put::Land, "North Carolina");
    assert_eq!(put(51.0, 42.0), Put::NoRecord, "the Caspian");
    assert_eq!(put(-87.0, 44.0), Put::NoRecord, "Lake Michigan");
    assert_eq!(put(108.0, 53.0), Put::NoRecord, "Baikal");
    assert_eq!(put(-86.0, 58.5), Put::NoRecord, "Hudson Bay");
    assert_eq!(put(38.0, 20.0), Put::NoRecord, "the Red Sea");
    assert_eq!(put(52.0, 27.0), Put::NoRecord, "the Persian Gulf");
    assert_eq!(
        put(34.0, 43.0),
        Put::Sea,
        "the Black Sea: the programme's drifters went there"
    );
    assert_eq!(put(19.5, 58.5), Put::Sea, "the Baltic");
    assert_eq!(put(18.0, 36.0), Put::Sea, "the Ionian Sea");
    assert_eq!(put(-74.5, 35.8), Put::Sea, "Cape Hatteras");
    assert_eq!(
        put(0.0, -65.0),
        Put::Sea,
        "the ice's edge at 65 S in a typical year's record"
    );
    // the record's reach is a number any look can read
    assert!(
        r.record(LonLat {
            lon: 51.0,
            lat: 42.0
        }) == 0.0
            && r.record(LonLat {
                lon: -40.0,
                lat: 30.0
            }) > 0.99
    );
    // somewhere the record reaches only from a distance: a guess, carried and marked
    let mut guesses = 0;
    for lat in (-70..=80).step_by(2) {
        for lon in (-180..180).step_by(2) {
            if put(lon as f64 + 0.5, lat as f64 + 0.5) == Put::Guess {
                guesses += 1;
            }
        }
    }
    assert!(guesses > 20, "{guesses} guess cells on a 2-degree grid");
}

/// Between steps a float glides along its velocity: a quarter day of gliding lands within a
/// few kilometres of the step itself (the step's draw aside), so the drawn float never jumps.
#[test]
fn a_float_glides_between_steps() {
    let mut r = rules();
    r.wander = 0.0;
    let d = Drift::put(LonLat {
        lon: -74.5,
        lat: 35.8,
    });
    let (u, v) = r.velocity(d, JULY);
    assert!(
        (u * u + v * v).sqrt() > 0.5,
        "the Gulf Stream runs: {u}, {v}"
    );
    let glided = r.glide(d, JULY, 0.25);
    let stepped = r.flight(d, JULY, 0.25, 1, 0).drift.at;
    let km = distance_km(
        glided.lon as f32,
        glided.lat as f32,
        stepped.lon as f32,
        stepped.lat as f32,
    );
    assert!(
        km < 2.0,
        "the glide and the step differ by {km} km over a quarter day"
    );
    assert_eq!(
        r.velocity(
            Drift::put(LonLat {
                lon: -150.0,
                lat: 74.0
            }),
            15.0 + 2.0 * 30.4375
        ),
        (0.0, 0.0),
        "held in ice: still"
    );
}

/// Where no drifter went, the float rides no smoothed-in current: the wind's leeway and the
/// wander alone, a lake's drift; where one measured, the whole current.
#[test]
fn unrecorded_water_carries_a_float_on_the_wind_alone() {
    let r = rules();
    let caspian = Drift::put(LonLat {
        lon: 51.0,
        lat: 42.0,
    });
    let (u, v) = r.velocity(caspian, JULY);
    let (wu, wv) = r.wind.at(JULY as f32, 51.0, 42.0);
    assert!(
        (u - r.leeway * wu).abs() < 1e-5 && (v - r.leeway * wv).abs() < 1e-5,
        "the Caspian: leeway only ({u}, {v})"
    );
    let hatteras = Drift::put(LonLat {
        lon: -74.5,
        lat: 35.8,
    });
    let (u, _) = r.velocity(hatteras, JULY);
    let (cu, _, m) = r.currents.at(JULY as f32, -74.5, 35.8);
    assert!(
        m > 0.99 && (u - cu).abs() < 0.1,
        "the Gulf Stream: the whole current"
    );
    assert_eq!(super::rules::reach(0.0), 0.0);
    assert_eq!(super::rules::reach(0.25), 0.5);
    assert_eq!(super::rules::reach(0.9), 1.0);
}

/// Time is reversible because a step is a pure function of (state, day, dt, seed, k): the draws
/// hash the seed, the path and the step's start day (as f32 bits) and nothing else; no call
/// remembers the last. A quarter-day grid is exact in f32 for any day under 2^21 (the year
/// 7745), so the same mark always gives the same bits; a day off the grid (a put's first step)
/// is rounded to f32 once, the same way every time.
#[test]
fn a_step_is_a_pure_function_of_its_arguments() {
    let r = rules();
    let d = Drift {
        at: LonLat {
            lon: -74.5,
            lat: 35.8,
        },
        eddy: (0.1, -0.05),
    };
    let a = r.flight(d, 9000.25, 0.25, 42, 7);
    // other calls between, other arguments
    let _ = r.flight(
        Drift::put(LonLat {
            lon: -40.0,
            lat: 30.0,
        }),
        100.0,
        1.0,
        1,
        1,
    );
    let _ = r.step(d.at, 9000.5, 0.25, 42, 7);
    let b = r.flight(d, 9000.25, 0.25, 42, 7);
    assert_eq!((a.drift, a.held, a.measured), (b.drift, b.held, b.measured));
    // the start day's bits are what the draws hang on: a different quarter mark, a different draw
    let c = r.flight(d, 9000.5, 0.25, 42, 7);
    assert_ne!(a.drift.eddy, c.drift.eddy);
    // the quarter grid is exact in f32 far beyond the app's years
    for day in [9000.25f64, 9000.5, 9000.75, 16000.25, 100000.75, 2097151.75] {
        assert_eq!(day as f32 as f64, day, "{day} is exact in f32");
    }
    // a day off the grid rounds the same way every time
    let e = r.flight(d, 9000.137, 0.113, 42, 7);
    let f = r.flight(d, 9000.137, 0.113, 42, 7);
    assert_eq!(e.drift, f.drift);
}

/// A tool (ignored): what a step costs on this box, for the scrub's budget.
///
///     cargo test -p adrift-core step_cost -- --ignored --nocapture
#[test]
#[ignore]
fn step_cost() {
    let r = rules();
    let starts: Vec<LonLat> = (0..64)
        .map(|i| LonLat {
            lon: -74.5 + (i % 8) as f64 * 3.0,
            lat: 20.0 + (i / 8) as f64 * 2.5,
        })
        .collect();
    let n = 20_000;
    let t = std::time::Instant::now();
    let mut acc = 0.0f64;
    for i in 0..n {
        let s = starts[i % starts.len()];
        let f = r.flight(
            Drift::put(s),
            9000.25 + (i % 97) as f64 * 0.25,
            0.25,
            5,
            i as u32,
        );
        acc += f.drift.at.lon;
    }
    let per = t.elapsed().as_secs_f64() * 1e6 / n as f64;
    eprintln!(
        "flight: {per:.2} us a call ({acc:.0}); a scrub of 30 floats x 400 steps = {:.1} ms",
        per * 12000.0 / 1000.0
    );
    let t = std::time::Instant::now();
    for i in 0..n {
        let s = starts[i % starts.len()];
        acc += r.currents.at(9000.25, s.lon as f32, s.lat as f32).0 as f64;
    }
    eprintln!(
        "currents.at: {:.2} us",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
    let t = std::time::Instant::now();
    for i in 0..n {
        let s = starts[i % starts.len()];
        acc += r.coast.distance(s.lon as f32, s.lat as f32) as f64;
    }
    eprintln!(
        "coast.distance: {:.2} us ({acc:.0})",
        t.elapsed().as_secs_f64() * 1e6 / n as f64
    );
}

/// The sheets read all their channels at once give the same numbers as channel by channel.
#[test]
fn the_cheaper_sampling_changes_no_number() {
    let r = rules();
    let mut n = 0;
    for lat in (-88..=88).step_by(4) {
        for lon in (-180..180).step_by(5) {
            let (lon, lat) = (lon as f32 + 0.37, lat as f32 - 0.21);
            for (tiled, months) in [
                (&r.currents.0, 12usize),
                (&r.wind.0, 12),
                (&r.eddy.0, 1),
                (&r.eddy_time.0, 1),
            ] {
                for m in 0..months {
                    let all = tiled.sample_all(m, lon, lat);
                    for c in 0..3 {
                        assert_eq!(
                            all[c],
                            tiled.sample(m, c, lon, lat),
                            "month {m} channel {c} at ({lon}, {lat})"
                        );
                    }
                    n += 1;
                }
            }
        }
    }
    assert!(n > 10_000);
}

/// A rewind of the cloud is a restore and a few steps: a checkpoint kept at day D, stepped on to
/// D + 20, restored and stepped on again lands the same paths bit for bit; and many steps in one
/// submit are the steps one by one.
#[test]
fn a_kept_cloud_restores_and_steps_on_the_same() {
    use super::gpu::{GpuPath, Maybes, Params};
    use crate::gpu::{Gpu, Textures};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let gpu = match pollster_block(Gpu::new(&instance, None)) {
        Ok(g) => g,
        Err(_) => return,
    };
    let assets = DirAssets(assets());
    let textures = Textures::load(&gpu, &assets, 6).unwrap();
    let r = rules();
    let n = 512;
    let from = LonLat {
        lon: -74.5,
        lat: 35.8,
    };
    let maybes = Maybes::new(&gpu, &assets, &textures, n).unwrap();
    let seed = |k: u32| {
        let d = r.put(from, JULY, 9, k);
        GpuPath {
            lon: from.lon as f32,
            lat: from.lat as f32,
            held: 0,
            measured: 1.0,
            eddy_u: d.eddy.0,
            eddy_v: d.eddy.1,
        }
    };
    let paths: Vec<GpuPath> = (1..=n).map(seed).collect();
    let steps = |from_day: f64, days: usize| -> Vec<Params> {
        (0..days * 2)
            .map(|i| Params::new(&r, from_day + i as f64 * 0.5, 0.5, 9, n))
            .collect()
    };
    // one by one to day 30
    maybes.seed(&gpu, &paths);
    for p in steps(JULY, 30) {
        maybes.step(&gpu, p, true);
    }
    let one_by_one = maybes.read(&gpu, n);
    // in one submit to day 30, kept there, on to 50, restored, on to 50 again
    maybes.seed(&gpu, &paths);
    maybes.step_many(&gpu, &steps(JULY, 30), true);
    let many = maybes.read(&gpu, n);
    assert!(
        one_by_one
            .iter()
            .zip(&many)
            .all(|(a, b)| (a.lon, a.lat, a.eddy_u, a.eddy_v) == (b.lon, b.lat, b.eddy_u, b.eddy_v)),
        "many steps in one submit are the steps one by one"
    );
    let kept = maybes.keep(&gpu, n);
    maybes.step_many(&gpu, &steps(JULY + 30.0, 20), true);
    let first = maybes.read(&gpu, n);
    maybes.restore(&gpu, &kept);
    let back = maybes.read(&gpu, n);
    assert!(
        back.iter()
            .zip(&many)
            .all(|(a, b)| (a.lon, a.lat) == (b.lon, b.lat)),
        "restored to the kept day"
    );
    maybes.step_many(&gpu, &steps(JULY + 30.0, 20), true);
    let again = maybes.read(&gpu, n);
    assert!(
        first
            .iter()
            .zip(&again)
            .all(|(a, b)| (a.lon, a.lat, a.eddy_u, a.eddy_v) == (b.lon, b.lat, b.eddy_u, b.eddy_v)),
        "stepped on from the keep, the same cloud bit for bit"
    );
    let moved = first
        .iter()
        .zip(&many)
        .filter(|(a, b)| a.lon != b.lon)
        .count();
    assert!(
        moved > n as usize / 2,
        "the cloud moved on between day 30 and 50"
    );
}

/// A tool (ignored): is the lamp (k = 0, stepped as the app steps it: put, then the quarter-day
/// grid) statistically one of its cloud (k = 1..n, put afresh and stepped half a day at a time as
/// the GPU does)? For many seeds, the lamp's end is ranked within its own cloud along each axis;
/// honest means the ranks are uniform (a mean near 0.5, a tenth of them in each tenth).
///
///     cargo test --release -p adrift-core the_lamp_is_one_of_its_cloud -- --ignored --nocapture
#[test]
#[ignore]
fn the_lamp_is_one_of_its_cloud() {
    let r = rules();
    for (name, from, day0, days) in [
        (
            "Hatteras",
            LonLat {
                lon: -74.2,
                lat: 34.6,
            },
            160.37,
            60.0,
        ),
        (
            "Sargasso",
            LonLat {
                lon: -40.0,
                lat: 30.0,
            },
            160.37,
            60.0,
        ),
        (
            "Agulhas",
            LonLat {
                lon: 31.0,
                lat: -31.0,
            },
            40.6,
            45.0,
        ),
    ] {
        let seeds = 160u64;
        let n = 400u32;
        let (mut rx, mut ry) = (Vec::new(), Vec::new());
        let (mut lamp_lon, mut cloud_lon) = (0.0, 0.0);
        for s in 0..seeds {
            let seed = 0x5eed_0000 + s * 7919;
            let lamp = lamp_path(&r, from, day0, day0 + days, seed);
            let cloud = cloud_at(&r, from, day0, day0 + days, 0.5, seed, n);
            let below = |f: &dyn Fn(&LonLat) -> f64, v: f64| {
                cloud.iter().filter(|c| f(c) < v).count() as f64 / n as f64
            };
            rx.push(below(&|c| c.lon, lamp.lon));
            ry.push(below(&|c| c.lat, lamp.lat));
            lamp_lon += lamp.lon / seeds as f64;
            cloud_lon += cloud.iter().map(|c| c.lon).sum::<f64>() / n as f64 / seeds as f64;
        }
        let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let tenths = |v: &[f64]| {
            (0..10)
                .map(|t| {
                    v.iter()
                        .filter(|x| (**x * 10.0).floor() as i32 == t || (t == 9 && **x >= 1.0))
                        .count()
                })
                .collect::<Vec<_>>()
        };
        eprintln!("{name}: lamp's rank in its cloud: lon mean {:.2} {:?}, lat mean {:.2} {:?}; mean lon lamps {:.2} clouds {:.2}",
            mean(&rx), tenths(&rx), mean(&ry), tenths(&ry), lamp_lon, cloud_lon);
    }
}

/// The lamp as the app steps it (app.rs `ensure_path`): put with k = 0, then to the quarter grid.
fn lamp_path(r: &Rules, from: LonLat, day0: f64, until: f64, seed: u64) -> LonLat {
    let mut d = r.put(from, day0, seed, 0);
    let mut day = day0;
    while day < until - 1e-9 {
        let next = ((day / 0.25).floor() * 0.25 + 0.25).min(until);
        let next = if next - day < 1e-9 { next + 0.25 } else { next };
        d = r.flight(d, day, next - day, seed, 0).drift;
        day = next;
    }
    d.at
}

/// A cloud as the GPU steps it (app.rs `step_clouds`): each k put afresh, `dt` at a time.
fn cloud_at(
    r: &Rules,
    from: LonLat,
    day0: f64,
    until: f64,
    dt: f64,
    seed: u64,
    n: u32,
) -> Vec<LonLat> {
    r.flights(Drift::put(from), day0, until, dt, seed, n)
        .iter()
        .map(|f| f.drift.at)
        .collect()
}

/// A tool (ignored): two lamps put 50 km apart, their clouds 30, 60 and 90 days on (R6, the
/// worlds). For each place: each cloud's centroid and spread, the distance between the
/// centroids, and the k-th selves' separation (lamp A's k-th maybe from lamp B's k-th: the same
/// world) beside the separation of selves from different worlds (what every pair was before the
/// worlds: two independent draws).
///
///     cargo test --release -p adrift-core two_lamps_fifty_km_apart -- --ignored --nocapture
///
/// With ADRIFT_OUT=<dir> it also draws `two-lamps-<place>.png` there: the 30, 60 and 90 days side
/// by side, land grey, lamp A's maybes amber, B's blue, and a thread between every fourth pair of
/// k-th selves (the same world), so where the worlds keep them together and where they part shows.
#[test]
#[ignore]
fn two_lamps_fifty_km_apart() {
    let r = rules();
    let n = 1024u32;
    let km = |a: &LonLat, b: &LonLat| {
        distance_km(a.lon as f32, a.lat as f32, b.lon as f32, b.lat as f32) as f64
    };
    for (name, a, day0) in [
        (
            "jet (Gulf Stream off Hatteras)",
            LonLat {
                lon: -74.0,
                lat: 35.2,
            },
            JULY,
        ),
        (
            "gyre's middle (Sargasso)",
            LonLat {
                lon: -50.0,
                lat: 28.0,
            },
            JULY,
        ),
        (
            "front (Brazil-Malvinas confluence)",
            LonLat {
                lon: -53.5,
                lat: -38.5,
            },
            JULY,
        ),
        (
            "front (Kuroshio Extension)",
            LonLat {
                lon: 146.0,
                lat: 35.5,
            },
            JULY,
        ),
    ] {
        let b = LonLat {
            lon: a.lon + 50.0 / (111.2 * a.lat.to_radians().cos()),
            lat: a.lat,
        };
        eprintln!(
            "{name}: A {:.2},{:.2}  B {:.2},{:.2} ({:.0} km apart)",
            a.lon,
            a.lat,
            b.lon,
            b.lat,
            km(&a, &b)
        );
        eprintln!("  days  centroids km apart  spread A  spread B   same world: sep p10 / p50 / p90 km   other worlds: p10 / p50 / p90");
        let (mut ca, mut cb): (Vec<Drift>, Vec<Drift>) = (
            (1..=n).map(|k| r.put(a, day0, 1, k)).collect(),
            (1..=n).map(|k| r.put(b, day0, 2, k)).collect(),
        );
        let mut day = day0;
        let mut panels = Vec::new();
        for until in [30.0, 60.0, 90.0] {
            while day < day0 + until - 1e-9 {
                for (i, d) in ca.iter_mut().enumerate() {
                    *d = r.flight(*d, day, 0.5, 1, i as u32 + 1).drift;
                }
                for (i, d) in cb.iter_mut().enumerate() {
                    *d = r.flight(*d, day, 0.5, 2, i as u32 + 1).drift;
                }
                day += 0.5;
            }
            let (pa, pb): (Vec<LonLat>, Vec<LonLat>) = (
                ca.iter().map(|d| d.at).collect(),
                cb.iter().map(|d| d.at).collect(),
            );
            let (ma, sa) = centroid_spread(&pa);
            let (mb, sb) = centroid_spread(&pb);
            let pct = |mut v: Vec<f64>| {
                v.sort_by(|x, y| x.partial_cmp(y).unwrap());
                (v[v.len() / 10], v[v.len() / 2], v[v.len() * 9 / 10])
            };
            let same = pct(pa.iter().zip(&pb).map(|(x, y)| km(x, y)).collect());
            let other = pct(pa
                .iter()
                .zip(pb.iter().cycle().skip(n as usize / 2))
                .map(|(x, y)| km(x, y))
                .collect());
            eprintln!("  {until:4.0}  {:8.0}            {sa:6.0}    {sb:6.0}       {:5.0} / {:5.0} / {:5.0}                {:5.0} / {:5.0} / {:5.0}",
                km(&ma, &mb), same.0, same.1, same.2, other.0, other.1, other.2);
            panels.push((pa, pb));
        }
        if let Some(dir) = std::env::var_os("ADRIFT_OUT") {
            let key = name
                .split(' ')
                .next()
                .unwrap()
                .trim_end_matches("'s")
                .to_string();
            let path = Path::new(&dir).join(format!("two-lamps-{key}.png"));
            draw_pairs(&r, &panels, &path);
            eprintln!("  drawn: {}", path.display());
        }
    }
}

/// The panels of `two_lamps_fifty_km_apart`: each its own frame round both clouds.
fn draw_pairs(r: &Rules, panels: &[(Vec<LonLat>, Vec<LonLat>)], path: &Path) {
    let side = 600u32;
    let mut img = image::RgbImage::from_pixel(
        side * panels.len() as u32 + 8 * (panels.len() as u32 - 1),
        side,
        image::Rgb([250, 248, 242]),
    );
    for (pi, (pa, pb)) in panels.iter().enumerate() {
        let all: Vec<&LonLat> = pa.iter().chain(pb.iter()).collect();
        let q = |f: &dyn Fn(&LonLat) -> f64, at: f64| {
            let mut v: Vec<f64> = all.iter().map(|p| f(p)).collect();
            v.sort_by(|a, b| a.partial_cmp(b).unwrap());
            v[((v.len() - 1) as f64 * at) as usize]
        };
        let (lo_lon, hi_lon, lo_lat, hi_lat) = (
            q(&|p| p.lon, 0.01),
            q(&|p| p.lon, 0.99),
            q(&|p| p.lat, 0.01),
            q(&|p| p.lat, 0.99),
        );
        let mid_lat = (lo_lat + hi_lat) / 2.0;
        let c = mid_lat.to_radians().cos();
        let span = ((hi_lon - lo_lon) * c).max(hi_lat - lo_lat) * 1.15 + 0.5;
        let (cx, cy) = ((lo_lon + hi_lon) / 2.0, mid_lat);
        let ox = pi as u32 * (side + 8);
        let to_px = |p: &LonLat| -> (f64, f64) {
            (
                ((p.lon - cx) * c / span + 0.5) * side as f64,
                ((cy - p.lat) / span + 0.5) * side as f64,
            )
        };
        for y in 0..side {
            for x in 0..side {
                let lat = cy - (y as f64 / side as f64 - 0.5) * span;
                let lon = cx + (x as f64 / side as f64 - 0.5) * span / c;
                let land = r.coast.is_land(LonLat { lon, lat });
                img.put_pixel(
                    ox + x,
                    y,
                    if land {
                        image::Rgb([205, 200, 190])
                    } else {
                        image::Rgb([232, 240, 246])
                    },
                );
            }
        }
        let mut dot =
            |img: &mut image::RgbImage, (x, y): (f64, f64), rgb: [u8; 3], a: f32, rad: i32| {
                for dy in -rad..=rad {
                    for dx in -rad..=rad {
                        if dx * dx + dy * dy > rad * rad {
                            continue;
                        }
                        let (px, py) = (x as i32 + dx, y as i32 + dy);
                        if px < 0 || py < 0 || px >= side as i32 || py >= side as i32 {
                            continue;
                        }
                        let o = img.get_pixel_mut(ox + px as u32, py as u32);
                        for ch in 0..3 {
                            o.0[ch] = (o.0[ch] as f32 * (1.0 - a) + rgb[ch] as f32 * a) as u8;
                        }
                    }
                }
            };
        for (k, (a, b)) in pa.iter().zip(pb).enumerate() {
            if k % 4 != 0 {
                continue;
            }
            let (p0, p1) = (to_px(a), to_px(b));
            let n = ((p1.0 - p0.0).hypot(p1.1 - p0.1) as usize).max(1);
            for i in 0..=n {
                let t = i as f64 / n as f64;
                dot(
                    &mut img,
                    (p0.0 + (p1.0 - p0.0) * t, p0.1 + (p1.1 - p0.1) * t),
                    [90, 90, 100],
                    0.12,
                    0,
                );
            }
        }
        for a in pa {
            dot(&mut img, to_px(a), [214, 120, 30], 0.7, 2);
        }
        for b in pb {
            dot(&mut img, to_px(b), [30, 90, 200], 0.6, 2);
        }
    }
    img.save(path).unwrap();
}

/// A cloud's centroid (on the sphere's tangent, fine for clouds of a few thousand km) and its rms
/// spread from it in km.
fn centroid_spread(p: &[LonLat]) -> (LonLat, f64) {
    let lat = p.iter().map(|q| q.lat).sum::<f64>() / p.len() as f64;
    let lon0 = p[0].lon;
    let lon = lon0
        + p.iter()
            .map(|q| (q.lon - lon0 + 540.0).rem_euclid(360.0) - 180.0)
            .sum::<f64>()
            / p.len() as f64;
    let c = LonLat { lon, lat };
    let s = (p
        .iter()
        .map(|q| {
            (distance_km(q.lon as f32, q.lat as f32, c.lon as f32, c.lat as f32) as f64).powi(2)
        })
        .sum::<f64>()
        / p.len() as f64)
        .sqrt();
    (c, s)
}

/// A tool (ignored): what the maybes' step costs on this box's GPU, as the app runs it with eight
/// clouds out: 8 x 2048 paths, 24 half-day steps each, one submit a cloud, timed to the queue's
/// end (the median of a few frames). On a software GPU (llvmpipe) the number is the shader's ALU
/// and loads on the CPU: compare runs, not boxes.
///
///     cargo test --release -p adrift-core gpu_step_cost -- --ignored --nocapture
#[test]
#[ignore]
fn gpu_step_cost() {
    use super::gpu::{GpuPath, Maybes, Params};
    use crate::gpu::{Gpu, Textures};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let Ok(gpu) = pollster_block(Gpu::new(&instance, None)) else {
        return;
    };
    let assets = DirAssets(assets());
    let textures = Textures::load(&gpu, &assets, 6).unwrap();
    let r = rules();
    let per = 2048u32;
    let maybes = Maybes::new(&gpu, &assets, &textures, per * 8).unwrap();
    let starts = [
        (-74.0, 35.2),
        (-50.0, 28.0),
        (-53.5, -38.5),
        (146.0, 35.5),
        (-30.0, 45.0),
        (60.0, -20.0),
        (-140.0, 10.0),
        (5.0, -40.0),
    ];
    let mut frames = Vec::new();
    for frame in 0..6 {
        for (c, (lon, lat)) in starts.iter().enumerate() {
            let paths: Vec<GpuPath> = (1..=per)
                .map(|k| {
                    let e = r
                        .put(
                            LonLat {
                                lon: *lon,
                                lat: *lat,
                            },
                            JULY,
                            1,
                            k,
                        )
                        .eddy;
                    GpuPath {
                        lon: *lon as f32,
                        lat: *lat as f32,
                        eddy_u: e.0,
                        eddy_v: e.1,
                        measured: 1.0,
                        ..Default::default()
                    }
                })
                .collect();
            maybes.seed_at(&gpu, c as u32 * per, &paths);
        }
        let _ = maybes.read(&gpu, 1);
        let t = std::time::Instant::now();
        for c in 0..8u32 {
            let steps: Vec<Params> = (0..24)
                .map(|i| {
                    Params::new(&r, JULY + frame as f64 * 12.0 + i as f64 * 0.5, 0.5, 1, per)
                        .at_base(c * per)
                })
                .collect();
            maybes.step_many(&gpu, &steps, true);
        }
        let _ = maybes.read(&gpu, 1);
        frames.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    frames.sort_by(|a, b| a.partial_cmp(b).unwrap());
    eprintln!("8 clouds x {per} paths x 24 half-day steps: {:.1} ms a frame (median of {}; {:.1} ns a path-step) on {}", frames[frames.len() / 2], frames.len(), frames[frames.len() / 2] * 1e6 / (8.0 * per as f64 * 24.0), gpu.adapter_info.name);
}

/// Visual's question (R10): does a lamp's journey zigzag? The atlantic scene's Gulf Stream lamp
/// (put 34N 75W on May 2, followed 60 days), stepped a quarter day as the app steps it: how far its
/// direction turns between quarter days and between days, and where it was held. `--ignored
/// --nocapture` prints it.
#[test]
#[ignore]
fn a_lamps_line_at_a_quarter_day_and_a_day() {
    let r = rules();
    let day0 = 182.0 - 60.0;
    let mut d = r.put(
        LonLat {
            lon: -75.0,
            lat: 34.0,
        },
        day0,
        0,
        0,
    );
    let mut pts = vec![(d.at, Held::No)];
    for i in 0..240 {
        let f = r.flight(d, day0 + i as f64 * 0.25, 0.25, 0, 0);
        d = f.drift;
        pts.push((d.at, f.held));
    }
    let turn = |s: usize| {
        let mut t = Vec::new();
        let p: Vec<_> = pts.iter().step_by(s).collect();
        for w in p.windows(3) {
            let h = |a: LonLat, b: LonLat| {
                ((b.lat - a.lat).atan2((b.lon - a.lon) * a.lat.to_radians().cos())).to_degrees()
            };
            let (h1, h2) = (h(w[0].0, w[1].0), h(w[1].0, w[2].0));
            t.push(((h2 - h1 + 540.0).rem_euclid(360.0) - 180.0).abs());
        }
        t.sort_by(|a, b| a.partial_cmp(b).unwrap());
        (t[t.len() / 2], t[t.len() * 9 / 10])
    };
    let held = pts.iter().filter(|p| p.1 != Held::No).count();
    eprintln!("turn between quarter-day segments (median, 90th): {:?}; between daily: {:?}; held {held} of 240 steps", turn(1), turn(4));
    for (i, p) in pts.iter().enumerate().step_by(4) {
        eprintln!("day {:2}: {:7.2} {:6.2} {:?}", i / 4, p.0.lon, p.0.lat, p.1);
    }
}

/// The ice's fast path is exact: no month's sheet has ice nearer the equator than `Ice::ICE_FREE`
/// plus a pixel, so the skipped bilinear read would have been 0 (R11).
#[test]
fn no_ice_nearer_the_equator() {
    let r = rules();
    for (m, s) in r.ice.0.iter().enumerate() {
        for y in 0..s.height {
            let lat = 90.0 - (y as f32 + 0.5) / s.height as f32 * 180.0;
            if lat.abs() < super::Ice::ICE_FREE + 0.25 {
                assert!(
                    (0..s.width).all(|x| s.at(x, y, 0) == 0.0),
                    "ice in month {} at {lat}",
                    m + 1
                );
            }
        }
    }
}

/// The law-0 digest: every step of 28 paths (7 places x 4 worlds, 20 days a quarter day at a time)
/// under the flight as shipped up to build 49, computed by that code (commit 77a693b) and pinned.
const LAW_ZERO_GOLDEN: u64 = 0x6921_f0d5_a9ea_3ff6;

fn law_digest(r: &Rules, law: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| {
        h ^= x;
        h = h.wrapping_mul(0x100_0000_01b3);
    };
    for (i, (lon, lat)) in [
        (-72.0, 35.0),
        (-40.0, 38.0),
        (-20.0, -30.0),
        (60.0, -45.0),
        (150.0, 35.0),
        (-150.0, 20.0),
        (5.0, 75.0),
    ]
    .into_iter()
    .enumerate()
    {
        for k in [0u32, 1, 7, 300] {
            let mut d = r.put(LonLat { lon, lat }, 100.0, i as u64, k);
            let mut day = 100.0;
            for _ in 0..80 {
                d = r.flight_under(law, d, day, 0.25, i as u64, k).drift;
                day += 0.25;
                eat(d.at.lon.to_bits());
                eat(d.at.lat.to_bits());
                eat(d.eddy.0.to_bits() as u64);
                eat(d.eddy.1.to_bits() as u64);
            }
        }
    }
    h
}

/// R13: a lamp made under the old law keeps it, bit for bit.
#[test]
fn law_zero_is_the_old_flight() {
    assert_eq!(
        law_digest(&rules(), 0),
        LAW_ZERO_GOLDEN,
        "law 0 must step exactly as build 49 and before did"
    );
}

/// R13: the current law spins, and is what `flight` steps.
#[test]
fn the_current_law_spins() {
    let r = rules();
    assert_eq!(super::rules::LAW, 2);
    assert_ne!(
        law_digest(&r, 1),
        LAW_ZERO_GOLDEN,
        "law 1 has the spinning eddies"
    );
    let d = r.put(
        LonLat {
            lon: -40.0,
            lat: 38.0,
        },
        100.0,
        3,
        5,
    );
    for k in 0..64 {
        let (a, b) = (
            r.flight(d, 100.0, 0.25, 3, k).drift,
            r.flight_under(super::rules::LAW, d, 100.0, 0.25, 3, k)
                .drift,
        );
        assert_eq!(a, b);
    }
}

/// R13: a lamp as build 46 saved it (legs with only at, day, seed; or no legs at all) reads as law 0.
#[test]
fn a_build_46_snapshot_loads_under_the_old_law() {
    let leg: crate::model::Leg = serde_json::from_str(
        r#"{"at": {"lon": -72.0, "lat": 35.0}, "day": 150.25, "seed": 12345}"#,
    )
    .unwrap();
    assert_eq!((leg.law, leg.world, leg.eddy), (0, 0, None));
    let f: crate::model::Float = serde_json::from_str(
        r#"{"id": 1, "at": {"lon": -70.0, "lat": 36.0}, "put_day": 150.25, "seed": 12345, "held": "No", "hand": "Free", "wake": [], "lit_for": 0.0}"#,
    )
    .unwrap();
    assert_eq!(
        f.legs()[0].law,
        0,
        "a float from before legs is under the old law"
    );
    assert_eq!(
        crate::model::Leg::new(LonLat { lon: 0.0, lat: 0.0 }, 0.0, 0).law,
        super::rules::LAW,
        "a fresh put takes the current law"
    );
}

/// R13: the GPU's maybes honour a cloud's law (Params::under), as the CPU does.
#[test]
fn the_gpu_keeps_the_law() {
    use super::gpu::{GpuPath, Maybes, Params};
    use crate::gpu::{Gpu, Textures};
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let Ok(gpu) = pollster_block(Gpu::new(&instance, None)) else {
        return;
    };
    let assets = DirAssets(assets());
    let textures = Textures::load(&gpu, &assets, 6).unwrap();
    let r = rules();
    let n = 512u32;
    let maybes = Maybes::new(&gpu, &assets, &textures, n).unwrap();
    // the open sea, and beside the coast where the dance's lamp met it (law 2's slide)
    for (start, law) in [(-40.0, 38.0), (152.95, -32.0)]
        .into_iter()
        .flat_map(|(lon, lat)| [0u32, 1, 2].map(|l| (LonLat { lon, lat }, l)))
    {
        let mut cpu: Vec<Drift> = (1..=n).map(|k| r.put(start, JULY, 0, k)).collect();
        maybes.seed(
            &gpu,
            &cpu.iter()
                .map(|d| GpuPath {
                    lon: d.at.lon as f32,
                    lat: d.at.lat as f32,
                    eddy_u: d.eddy.0,
                    eddy_v: d.eddy.1,
                    ..Default::default()
                })
                .collect::<Vec<_>>(),
        );
        let mut day = JULY;
        for _ in 0..16 {
            maybes.step(&gpu, Params::new(&r, day, 0.25, 0, n).under(law), true);
            for (i, d) in cpu.iter_mut().enumerate() {
                *d = r.flight_under(law, *d, day, 0.25, 0, i as u32 + 1).drift;
            }
            day += 0.25;
        }
        let g = maybes.read(&gpu, n);
        let worst = cpu
            .iter()
            .zip(&g)
            .map(|(c, p)| distance_km(c.at.lon as f32, c.at.lat as f32, p.lon, p.lat))
            .fold(0.0f32, f32::max);
        eprintln!("law {law} from {start:?}: the GPU's maybes within {worst:.4} km of the CPU's after 4 days");
        assert!(worst < 0.5);
    }
}

/// R15: the lamps made under law 1 (builds 50 to 5x) keep it, bit for bit: pinned digests of the
/// flight and of paths along coasts, computed before law 2 was written.
#[test]
fn law_one_is_kept() {
    let r = rules();
    assert_eq!(law_digest(&r, 1), 0x4efb_5708_0c88_3b44);
    assert_eq!(coast_digest(&r, 1), 0xdbf7_a53c_9fc6_970d);
    assert_ne!(
        coast_digest(&r, 2),
        0xdbf7_a53c_9fc6_970d,
        "law 2's coast is another"
    );
}

/// A digest of paths along coasts (the dance's lamps and others put near shores), under `law`: every
/// mark of 60 days on the quarter-day grid. Pins the coast's handling for each law.
fn coast_digest(r: &Rules, law: u32) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut eat = |x: u64| {
        h ^= x;
        h = h.wrapping_mul(0x100_0000_01b3);
    };
    let day0 = 9131.0 + 11.0 * 30.44 + 1.0;
    for (lon, lat) in [
        (156.75, -29.0),
        (157.15, -29.0),
        (-75.3, 35.0),
        (-9.6, 38.6),
        (151.5, -33.5),
    ] {
        let mut d = r.put(LonLat { lon, lat }, day0, 1, 0);
        let mut day = day0;
        while day < day0 + 60.0 {
            let next = crate::app::grid_next(day);
            d = r.flight_under(law, d, day, next - day, 1, 0).drift;
            day = next;
            eat(d.at.lon.to_bits());
            eat(d.at.lat.to_bits());
        }
    }
    h
}
