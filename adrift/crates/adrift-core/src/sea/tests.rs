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
        let (ca, cb, ct) = Clock { day, running: false, rate: 0.0 }.months();
        assert_eq!((a, b), (ca as usize, cb as usize), "day {day}");
        assert!((t - ct as f32).abs() < 2e-3, "day {day}: {t} against {ct}");
    }
}

const JULY: f64 = 15.21875 + 6.0 * 30.4375;   // the middle of a typical July

#[test]
fn gulf_stream_runs_north_east_at_the_measured_speed() {
    let mut r = rules();
    r.wander = 0.0; r.leeway = 0.0;
    let from = LonLat { lon: -74.5, lat: 35.8 };            // off Cape Hatteras
    let (u, v, measured) = r.currents.at(JULY as f32, from.lon as f32, from.lat as f32);
    assert!(measured > 0.99, "the Gulf Stream is measured");
    let speed = (u * u + v * v).sqrt();
    assert!(speed > 0.5 && u > 0.0 && v > 0.0, "the Gulf Stream here runs NE at over 0.5 m/s: ({u}, {v})");
    let s = r.step(from, JULY, 1.0, 1, 0);
    assert_eq!(s.held, Held::No);
    let km = distance_km(from.lon as f32, from.lat as f32, s.at.lon as f32, s.at.lat as f32);
    let expect = speed * 86400.0 / 1000.0;
    assert!((km - expect).abs() < 0.02 * expect, "a day's run {km} km against {expect}");
    assert!(s.at.lon > from.lon && s.at.lat > from.lat, "north-east");
}

#[test]
fn a_coast_holds() {
    let r = rules();
    // Cape Hatteras's beach: a point just inland, from one just offshore
    let coast = &r.coast;
    assert!(coast.is_sea(-75.2, 35.4) && !coast.is_sea(-77.5, 35.4), "the sheet knows the shore");
    let (lon, lat, held) = r.ashore(-75.2, 35.4, -77.5, 35.4);
    assert_eq!(held, Held::Coast);
    assert!(coast.is_sea(lon, lat), "put back on the water at ({lon}, {lat})");
    assert!(coast.distance(lon, lat) <= -r.shore * 0.5);
    // and a float in the sea with no land near takes its step
    assert_eq!(r.ashore(-40.0, 30.0, -40.1, 30.1).2, Held::No);
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
    assert_eq!(free.held, Held::No, "the ice has retreated in September");
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
    let (ku, kv, measured) = r.eddy.at(from.lon as f32, from.lat as f32);
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
    assert!(ends.iter().all(|s| s.held == Held::No));
}

#[test]
fn the_nearest_real_drifter_is_usually_near() {
    let d = Drifters::load(&assets().join("drifters.bin")).unwrap();
    assert!(d.runs.len() > 1000 && d.points() > 500_000);
    let r = rules();
    // a grid of sea points: how far is the nearest drifter point
    let mut far = 0; let mut total = 0; let mut sum = 0.0;
    for lat in (-60..=60).step_by(5) {
        for lon in (-180..180).step_by(5) {
            let (lon, lat) = (lon as f32 + 0.5, lat as f32 + 0.5);
            if !r.coast.is_sea(lon, lat) || r.coast.distance(lon, lat) > -10.0 { continue; }
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

#[test]
fn the_flight_spreads_as_the_diffusivity_says_and_keeps_its_way() {
    let mut r = rules();
    r.leeway = 0.0;
    let from = LonLat { lon: -40.0, lat: 30.0 };
    let (ku, kv, _) = r.eddy.at(from.lon as f32, from.lat as f32);
    let (tu, tv) = r.eddy_time.at(from.lon as f32, from.lat as f32);
    assert!(tu > 1.0 && tu < 6.0 && tv > 1.0 && tv < 6.0, "time scales of days: {tu}, {tv}");
    let days = 30.0f32;
    let ends = r.flights(Drift::put(from), JULY, JULY + days as f64, 0.25, 5, 3000);
    let coslat = (from.lat as f32).to_radians().cos();
    let m = 6_371_000.0f32;
    let xs: Vec<f32> = ends.iter().map(|f| ((f.drift.at.lon - from.lon) as f32).to_radians() * m * coslat).collect();
    let ys: Vec<f32> = ends.iter().map(|f| ((f.drift.at.lat - from.lat) as f32).to_radians() * m).collect();
    let var = |v: &[f32]| {
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        v.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / v.len() as f32
    };
    // Taylor (1921): the variance is 2 K (t - T (1 - exp(-t / T)))
    let expect = |k: f32, t: f32| 2.0 * k * 86400.0 * (days - t * (1.0 - (-days / t).exp()));
    let (vx, vy) = (var(&xs), var(&ys));
    let (ex, ey) = (expect(ku, tu), expect(kv, tv));
    assert!((vx / ex - 1.0).abs() < 0.2, "zonal spread {} km against {} (K {ku}, T {tu})", vx.sqrt() / 1000.0, ex.sqrt() / 1000.0);
    assert!((vy / ey - 1.0).abs() < 0.2, "meridional spread {} km against {} (K {kv}, T {tv})", vy.sqrt() / 1000.0, ey.sqrt() / 1000.0);
    // the carried velocity's variance is K / T
    let vu = var(&ends.iter().map(|f| f.drift.eddy.0).collect::<Vec<_>>());
    let eu = ku / (tu * 86400.0);
    assert!((vu / eu - 1.0).abs() < 0.2, "eddy velocity variance {vu} against K/T {eu}");
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
    let from = LonLat { lon: -74.5, lat: 35.8 };
    let summary = |lons: &mut Vec<f64>, coast: usize, name: &str| {
        lons.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let east = lons.iter().filter(|l| **l >= -65.0).count();
        eprintln!("{name}: {} of {} east of 65 W, {} on a coast, median {:.1} W (the record: 9 of 18, none, 64.8)", east, lons.len(), coast, -lons[lons.len() / 2]);
        (east as f64 / lons.len() as f64, lons[lons.len() / 2])
    };
    let walk = r.maybes(from, JULY, JULY + 30.0, 0.25, 11, 500);
    summary(&mut walk.iter().map(|s| s.at.lon).collect(), walk.iter().filter(|s| s.held == Held::Coast).count(), "the walk");
    let flight = r.flights(Drift::put(from), JULY, JULY + 30.0, 0.25, 11, 500);
    let (east, median) = summary(&mut flight.iter().map(|f| f.drift.at.lon).collect(), flight.iter().filter(|f| f.held == Held::Coast).count(), "the flight");
    assert!((median + 64.8).abs() < 4.0, "the flight's median {median} is far from the record's 64.8 W");
    assert!((east - 0.5).abs() < 0.2, "the flight's share east of 65 W, {east}, is far from the record's half");
}

/// The GPU mirror held to the CPU: the same paths stepped by `rules.wgsl` and by `Rules::step`
/// (and `flight`) land within a few metres after a step and within a kilometre after eight (the
/// sheets are read the same way; only the rounding of a fused multiply differs).
#[test]
fn the_gpu_steps_as_the_cpu_does() {
    use crate::gpu::{Gpu, Textures};
    use super::gpu::{GpuPath, Maybes, Params};
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL, ..Default::default() });
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
    for flight in [false, true] {
        let paths: Vec<GpuPath> = starts.iter().map(|p| GpuPath { lon: p.lon as f32, lat: p.lat as f32, ..Default::default() }).collect();
        maybes.seed(&gpu, &paths);
        let mut cpu: Vec<Drift> = starts.iter().map(|p| Drift::put(*p)).collect();
        let mut day = JULY;
        let dt = 0.25;
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
                assert!(worst < 0.01, "after one step the GPU and the CPU differ by {worst} km ({})", if flight { "flight" } else { "walk" });
            }
        }
        eprintln!("{}: {} paths, worst gap after 8 quarter-days {:.4} km", if flight { "flight" } else { "walk" }, n, worst);
        assert!(worst < 1.0, "after eight steps the GPU and the CPU differ by {worst} km");
    }
}

fn pollster_block<F: std::future::Future>(f: F) -> F::Output {
    use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
    fn noop(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker { RawWaker::new(p, &VTABLE) }
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
    let from = LonLat { lon: -40.0, lat: 30.0 };
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
    assert!((su / n as f64 / eu as f64 - 1.0).abs() < 0.15, "zonal first velocity variance {} against K/T {eu}", su / n as f64);
    assert!((sv / n as f64 / ev as f64 - 1.0).abs() < 0.15, "meridional {} against {ev}", sv / n as f64);
    assert_eq!(r.put(from, JULY, 5, 3).eddy, r.put(from, JULY, 5, 3).eddy);
    assert_ne!(r.put(from, JULY, 5, 3).eddy, r.put(from, JULY, 5, 4).eddy);
    // the calendar is the real one
    assert_eq!(super::drifters::civil(0.0), (2000, 1, 1));
    assert_eq!(super::drifters::civil(59.0), (2000, 2, 29));
    assert_eq!(super::drifters::civil(60.0), (2000, 3, 1));
    assert_eq!(super::drifters::civil(9000.0), (2024, 8, 22));
    let d = Drifters::load(&assets().join("drifters.bin")).unwrap();
    let t = d.track(LonLat { lon: -74.5, lat: 35.8 }, 500.0).expect("a drifter passed Hatteras");
    assert!(t.km < 100.0 && t.xyz.len() == t.run.points.len() && t.xyz.len() >= 2);
    let v = t.xyz[t.point];
    assert!((v[0] * v[0] + v[1] * v[1] + v[2] * v[2] - 1.0).abs() < 1e-4, "unit vectors");
    let label = t.label();
    let w = t.window(30);
    assert!(w.contains(&t.point) && w.len() <= 61 && w.len() >= 31);
    let short = t.label_window(w.clone());
    eprintln!("{label} ({:.0} km off); the month about it: {short}, {} points", t.km, w.len());
    assert!(short.starts_with("Drifter ") && short.contains(" to "));
    assert!(label.starts_with("Drifter ") && label.contains(" to ") && label.contains("201") || label.contains("202"));
}

/// No maybe is ever on land: the held float of `gulf-stream-close.json`, its paths flown twenty
/// days on the CPU and on the GPU, every one on the water at the end and after every step.
#[test]
fn no_maybe_lies_on_land() {
    use super::gpu::{GpuPath, Maybes, Params};
    use crate::gpu::{Gpu, Textures};
    let r = rules();
    let from = LonLat { lon: -75.5, lat: 33.0 };
    let (day0, days, dt, seed, n) = (180.0, 20.0, 0.25, 7u64, 2000u32);
    let ends = r.flights(Drift::put(from), day0, day0 + days, dt, seed, n);
    let ashore: Vec<_> = ends.iter().filter(|f| r.coast.is_land(f.drift.at)).collect();
    assert!(ashore.is_empty(), "{} of {n} CPU maybes end on land, e.g. {:?}", ashore.len(), ashore.first().map(|f| f.drift.at));
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL, ..Default::default() });
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
            GpuPath { lon: from.lon as f32, lat: from.lat as f32, held: 0, measured: 1.0, eddy_u: d.eddy.0, eddy_v: d.eddy.1 }
        })
        .collect();
    maybes.seed(&gpu, &paths);
    let mut day = day0;
    let mut worst = 0usize;
    while day < day0 + days - 1e-9 {
        maybes.step(&gpu, Params::new(&r, day, dt, seed, n), true);
        day += dt;
        let got = maybes.read(&gpu, n);
        let on_land = got.iter().filter(|p| r.coast.is_land(LonLat { lon: p.lon as f64, lat: p.lat as f64 })).count();
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
            if a[i] <= b[j] { i += 1 } else { j += 1 }
            worst = worst.max((i as f64 / a.len() as f64 - j as f64 / b.len() as f64).abs());
        }
        worst
    };
    let std = |v: &[f64]| { let m = v.iter().sum::<f64>() / v.len() as f64; (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / v.len() as f64).sqrt() };
    for (name, from) in [("Hatteras", LonLat { lon: -74.5, lat: 35.8 }), ("Sargasso", LonLat { lon: -40.0, lat: 30.0 })] {
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
                let (sx, sy) = (std(&lon) * 111.0 * from.lat.to_radians().cos(), std(&lat) * 111.0);
                eprintln!("  {n:5}  {dt:4.2}   {:.3}   {:.3}   {east:.2}  {sx:4.0} x {sy:4.0}   {ashore}", ks(&mut lon, &mut rlon.clone()), ks(&mut lat, &mut rlat.clone()));
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
    assert_eq!(put(34.0, 43.0), Put::Sea, "the Black Sea: the programme's drifters went there");
    assert_eq!(put(19.5, 58.5), Put::Sea, "the Baltic");
    assert_eq!(put(18.0, 36.0), Put::Sea, "the Ionian Sea");
    assert_eq!(put(-74.5, 35.8), Put::Sea, "Cape Hatteras");
    assert_eq!(put(0.0, -65.0), Put::Sea, "the ice's edge at 65 S in a typical year's record");
    // the record's reach is a number any look can read
    assert!(r.record(LonLat { lon: 51.0, lat: 42.0 }) == 0.0 && r.record(LonLat { lon: -40.0, lat: 30.0 }) > 0.99);
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
    let d = Drift::put(LonLat { lon: -74.5, lat: 35.8 });
    let (u, v) = r.velocity(d, JULY);
    assert!((u * u + v * v).sqrt() > 0.5, "the Gulf Stream runs: {u}, {v}");
    let glided = r.glide(d, JULY, 0.25);
    let stepped = r.flight(d, JULY, 0.25, 1, 0).drift.at;
    let km = distance_km(glided.lon as f32, glided.lat as f32, stepped.lon as f32, stepped.lat as f32);
    assert!(km < 2.0, "the glide and the step differ by {km} km over a quarter day");
    assert_eq!(r.velocity(Drift::put(LonLat { lon: -150.0, lat: 74.0 }), 15.0 + 2.0 * 30.4375), (0.0, 0.0), "held in ice: still");
}

/// Where no drifter went, the float rides no smoothed-in current: the wind's leeway and the
/// wander alone, a lake's drift; where one measured, the whole current.
#[test]
fn unrecorded_water_carries_a_float_on_the_wind_alone() {
    let r = rules();
    let caspian = Drift::put(LonLat { lon: 51.0, lat: 42.0 });
    let (u, v) = r.velocity(caspian, JULY);
    let (wu, wv) = r.wind.at(JULY as f32, 51.0, 42.0);
    assert!((u - r.leeway * wu).abs() < 1e-5 && (v - r.leeway * wv).abs() < 1e-5, "the Caspian: leeway only ({u}, {v})");
    let hatteras = Drift::put(LonLat { lon: -74.5, lat: 35.8 });
    let (u, _) = r.velocity(hatteras, JULY);
    let (cu, _, m) = r.currents.at(JULY as f32, -74.5, 35.8);
    assert!(m > 0.99 && (u - cu).abs() < 0.1, "the Gulf Stream: the whole current");
    assert_eq!(super::rules::reach(0.0), 0.0);
    assert_eq!(super::rules::reach(0.25), 0.5);
    assert_eq!(super::rules::reach(0.9), 1.0);
}
