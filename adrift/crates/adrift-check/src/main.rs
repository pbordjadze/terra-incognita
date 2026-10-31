//! Adrift headless: renders a scene, or plays a plan of pointer events over time, to PNG, on Linux's
//! software Vulkan (tools/adrift/linux-setup.sh). Every craft judges its work in these.
//!
//!     ADRIFT_ASSETS=<dir> cargo run -p adrift-check -- render <scene.json> <out.png> [--size WxH] [--ppp 3]
//!     ADRIFT_ASSETS=<dir> cargo run -p adrift-check -- play <plan.json> <out-dir>
//!     cargo run -p adrift-check -- info
//!
//! ADRIFT_ASSETS defaults to adrift/build/assets (python3 tools/adrift/assets.py adrift/build/assets).
//! A scene (adrift-core's `Scene`): camera {lon, lat, zoom}, day, running, floats [{lon, lat,
//! days_ago}], hold, seconds. A plan: {"size": [w, h], "ppp": 3, "scene": {...}, "steps": [...]} where
//! each step is {"at": seconds, ...} and one of {"down"|"move"|"up": [x, y], "id": n} (points),
//! {"advance": seconds} (time passes, frames drawn), {"shot": "name"} (a PNG of that moment),
//! {"intent": {...}} (an Intent, as the model serialises it).

use adrift_core::{App, DirAssets, Gpu, Offscreen, Phase, PointerEvent, Scene};
use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::{Path, PathBuf};

fn assets_dir() -> PathBuf {
    if let Ok(d) = std::env::var("ADRIFT_ASSETS") {
        return d.into();
    }
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.join("../../build/assets")
}

fn gpu() -> Result<(wgpu::Instance, Gpu)> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL, ..Default::default() });
    let gpu = pollster::block_on(Gpu::new(&instance, None))?;
    Ok((instance, gpu))
}

fn save(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    image::save_buffer(path, rgba, w, h, image::ColorType::Rgba8).with_context(|| path.display().to_string())
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1).cloned())
}

fn render(args: &[String]) -> Result<()> {
    let scene: Scene = serde_json::from_slice(&std::fs::read(&args[0]).with_context(|| args[0].clone())?)?;
    let out = PathBuf::from(&args[1]);
    let size = arg(args, "--size").unwrap_or_else(|| "1179x2556".into());
    let (w, h) = size.split_once('x').context("--size WxH")?;
    let (w, h): (u32, u32) = (w.parse()?, h.parse()?);
    let ppp: f64 = arg(args, "--ppp").map(|s| s.parse()).transpose()?.unwrap_or(3.0);
    let (_instance, gpu) = gpu()?;
    let target = Offscreen::new(&gpu, w, h);
    let t0 = std::time::Instant::now();
    let mut app = App::new(gpu, Box::new(DirAssets(assets_dir())), w, h, ppp)?;
    app.budget.locked = true;
    app.budget.scale = 1.0;
    eprintln!("loaded in {:?} ({})", t0.elapsed(), app.gpu.adapter_info.name);
    app.set_scene(&scene);
    let t1 = std::time::Instant::now();
    app.render(&target.view);
    let px = target.read(&app.gpu);
    eprintln!("drawn in {:?}", t1.elapsed());
    save(&out, w, h, &px)?;
    println!("{}", out.display());
    Ok(())
}

#[derive(Deserialize)]
struct Plan {
    /// Pixels (Engineering's plans) or points (Motion's: a plan with a `camera` and no `scene`).
    #[serde(default = "default_size")]
    size: [f64; 2],
    #[serde(default = "default_ppp")]
    ppp: f64,
    /// Engineering's form: a whole scene.
    scene: Option<Scene>,
    /// Motion's form (touch/VOCABULARY.md › Plans): a camera, steps at absolute times `t`, fingers
    /// by id at a place `at: [lon, lat]` or at screen points `xy`, and `expect`s.
    camera: Option<adrift_core::Camera>,
    #[serde(default)]
    floats: Vec<adrift_core::SceneFloat>,
    #[serde(default)]
    day: f64,
    /// The reader's Reduce Motion setting.
    #[serde(default)]
    reduce_motion: bool,
    /// The night palette.
    #[serde(default)]
    night: bool,
    steps: Vec<Step>,
}
fn default_size() -> [f64; 2] {
    [1179.0, 2556.0]
}
fn default_ppp() -> f64 {
    3.0
}

/// A finger's place: screen points `[x, y]` (Engineering's plans) or a finger id whose place is the
/// step's `at: [lon, lat]` or `xy` (Motion's).
#[derive(Deserialize)]
#[serde(untagged)]
enum Where {
    Point([f64; 2]),
    Finger(u64),
}

#[derive(Deserialize)]
struct Step {
    #[serde(default)]
    id: u64,
    t: Option<f64>,
    down: Option<Where>,
    #[serde(rename = "move")]
    move_: Option<Where>,
    up: Option<Where>,
    cancel: Option<Where>,
    at: Option<[f64; 2]>,
    #[serde(alias = "px")]
    xy: Option<[f64; 2]>,
    /// A key the shell forwards: "undo" or "redo".
    key: Option<String>,
    /// The device shaken.
    shake: Option<bool>,
    advance: Option<f64>,
    tick: Option<f64>,
    shot: Option<String>,
    intent: Option<adrift_core::Intent>,
    expect: Option<serde_json::Value>,
}

/// A finger's id and screen point: given, or its place projected through the camera.
fn point(app: &App, w: &Where, id: u64, at: Option<[f64; 2]>, xy: Option<[f64; 2]>) -> (u64, f64, f64) {
    match w {
        Where::Point(p) => (id, p[0], p[1]),
        Where::Finger(f) => {
            if let Some(p) = xy {
                return (*f, p[0], p[1]);
            }
            let at = at.expect("a finger step needs `at: [lon, lat]` or `xy`");
            let (p, _) = app.disc().project(&app.camera.camera, adrift_core::LonLat::new(at[0], at[1]));
            (*f, p[0], p[1])
        }
    }
}

fn play(args: &[String]) -> Result<()> {
    let plan: Plan = serde_json::from_slice(&std::fs::read(&args[0]).with_context(|| args[0].clone())?)?;
    let out = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&out)?;
    let (_instance, gpu) = gpu()?;
    let points = plan.scene.is_none();
    let (w, h) = if points { ((plan.size[0] * plan.ppp) as u32, (plan.size[1] * plan.ppp) as u32) } else { (plan.size[0] as u32, plan.size[1] as u32) };
    let target = Offscreen::new(&gpu, w, h);
    let mut app = App::new(gpu, Box::new(DirAssets(assets_dir())), w, h, plan.ppp)?;
    app.budget.locked = true;
    app.budget.scale = 1.0;
    app.reduce_motion = plan.reduce_motion;
    let scene = plan.scene.clone().unwrap_or_else(|| Scene {
        camera: plan.camera.unwrap_or(adrift_core::Camera { lon: -40.0, lat: 22.0, zoom: 1.0 }),
        day: plan.day,
        running: true,
        floats: plan.floats.clone(),
        hold: None,
        seconds: 0.0,
        night: plan.night,
    });
    app.set_scene(&scene);
    let mut t = 0.0;
    let fps = 60.0;
    app.frame(&target.view, t);
    let mut log = String::new();
    let mut failed = 0;
    let advance = |app: &mut App, t: &mut f64, secs: f64| {
        let n = (secs * fps).ceil().max(1.0) as usize;
        for _ in 0..n {
            *t += secs / n as f64;
            app.advance(*t);
        }
    };
    let camera0 = app.camera.camera;
    let mut last_camera: Option<adrift_core::Camera> = None;
    for (i, s) in plan.steps.iter().enumerate() {
        if let Some(at) = s.t {
            if at > t {
                let d = at - t;
                advance(&mut app, &mut t, d);
            }
        }
        if let Some(secs) = s.tick {
            advance(&mut app, &mut t, secs);
        }
        for (wh, phase) in [(&s.down, Phase::Down), (&s.move_, Phase::Move), (&s.up, Phase::Up), (&s.cancel, Phase::Cancel)] {
            if let Some(wh) = wh {
                let (id, x, y) = point(&app, wh, s.id, s.at, s.xy);
                app.pointer(PointerEvent { id, phase, x, y, t });
            }
        }
        if let Some(intent) = s.intent {
            app.intend(intent);
        }
        if let Some(k) = &s.key {
            app.key(if k.eq_ignore_ascii_case("redo") { adrift_core::touch::Key::Redo } else { adrift_core::touch::Key::Undo });
        }
        if s.shake == Some(true) {
            app.shaken();
        }
        if let Some(secs) = s.advance {
            advance(&mut app, &mut t, secs);
        }
        if let Some(e) = &s.expect {
            let cam = app.camera.camera;
            let mut bad = Vec::new();
            if let Some(n) = e.get("floats").and_then(|v| v.as_u64()) {
                if app.floats.len() as u64 != n {
                    bad.push(format!("floats {} not {n}", app.floats.len()));
                }
            }
            if let Some(st) = e.get("state").and_then(|v| v.as_str()) {
                if app.gesture_state() != st {
                    bad.push(format!("state {} not {st}", app.gesture_state()));
                }
            }
            if let Some(f) = e.get("float") {
                let id = f.get("id").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                match app.floats.iter().find(|x| x.id == id) {
                    Some(x) => {
                        if let (Some(lon), Some(lat)) = (f.get("lon").and_then(|v| v.as_f64()), f.get("lat").and_then(|v| v.as_f64())) {
                            if (x.at.lon - lon).abs() > 1.5 || (x.at.lat - lat).abs() > 1.5 {
                                bad.push(format!("float {id} at {:.2},{:.2} not near {lon},{lat}", x.at.lon, x.at.lat));
                            }
                        }
                    }
                    None => bad.push(format!("no float {id}")),
                }
            }
            if let Some(moved) = e.get("camera_moved").and_then(|v| v.as_bool()) {
                let did = (cam.lon - camera0.lon).abs() > 0.01 || (cam.lat - camera0.lat).abs() > 0.01 || (cam.zoom - camera0.zoom).abs() > 0.001;
                if did != moved {
                    bad.push(format!("camera moved {did}, expected {moved}"));
                }
            }
            if let Some(saw) = e.get("saw").and_then(|v| v.as_array()) {
                for want in saw.iter().filter_map(|v| v.as_str()) {
                    if !app.seen.iter().any(|s| s == want) {
                        bad.push(format!("never saw {want} (saw {:?})", app.seen));
                    }
                }
            }
            if let Some(not) = e.get("not").and_then(|v| v.as_array()) {
                for n in not.iter().filter_map(|v| v.as_str()) {
                    if app.seen.iter().any(|s| s == n) {
                        bad.push(format!("saw {n}, which it must not"));
                    }
                }
            }
            if let Some(hv) = e.get("held") {
                let held = app.floats.iter().find(|f| f.hand != adrift_core::Hand::Free).map(|f| f.id as u64);
                if held != hv.as_u64() {
                    bad.push(format!("held {held:?} not {:?}", hv.as_u64()));
                }
            }
            if let Some(r) = e.get("riding") {
                let riding = app.riding().map(|id| id as u64);
                if riding != r.as_u64() {
                    bad.push(format!("riding {riding:?} not {:?}", r.as_u64()));
                }
            }
            if let Some(p) = e.get("playing").and_then(|v| v.as_bool()) {
                if app.clock.running != p {
                    bad.push(format!("playing {} not {p}", app.clock.running));
                }
            }
            if let Some(d) = e.get("day").and_then(|v| v.as_f64()) {
                if (app.clock.day - d).abs() > 2.0 {
                    bad.push(format!("day {:.1} not near {d}", app.clock.day));
                }
            }
            if let Some(z) = e.get("zoom_gt").and_then(|v| v.as_f64()) {
                if !(cam.zoom > z) {
                    bad.push(format!("zoom {:.2} not > {z}", cam.zoom));
                }
            }
            if let Some(c) = e.get("camera") {
                if let (Some(lon), Some(lat)) = (c.get("lon").and_then(|v| v.as_f64()), c.get("lat").and_then(|v| v.as_f64())) {
                    if (cam.lon - lon).abs() > 2.0 || (cam.lat - lat).abs() > 2.0 {
                        bad.push(format!("camera {:.2},{:.2} not near {lon},{lat}", cam.lon, cam.lat));
                    }
                }
            }
            if e.get("camera_same_as_last").and_then(|v| v.as_bool()) == Some(true) {
                if let Some(last) = last_camera {
                    if (cam.lon - last.lon).abs() > 0.01 || (cam.lat - last.lat).abs() > 0.01 || (cam.zoom - last.zoom).abs() > 0.001 {
                        bad.push("camera moved since the last expectation".into());
                    }
                }
            }
            if e.get("camera_near_float").is_some() {
                if let Some(f) = app.floats.first() {
                    if (cam.lon - f.at.lon).abs() > 3.0 || (cam.lat - f.at.lat).abs() > 3.0 {
                        bad.push(format!("camera {:.1},{:.1} not near the float {:.1},{:.1}", cam.lon, cam.lat, f.at.lon, f.at.lat));
                    }
                }
            }
            last_camera = Some(cam);
            // `saw` and `not` speak of the intents since the last check (touch/VOCABULARY.md › Plans)
            app.seen.clear();
            let line = if bad.is_empty() { format!("{i:3} t={t:7.2}s expect ok\n") } else { failed += 1; format!("{i:3} t={t:7.2}s EXPECT FAILED: {}\n", bad.join("; ")) };
            print!("{line}");
            log.push_str(&line);
        }
        if let Some(name) = &s.shot {
            app.frame(&target.view, t);
            let px = target.read(&app.gpu);
            let path = out.join(format!("{name}.png"));
            save(&path, w, h, &px)?;
            let cam = app.camera.camera;
            let floats: Vec<String> = app.floats.iter().map(|f| format!("#{} {:.3},{:.3} {:?} {:?} wake {}", f.id, f.at.lon, f.at.lat, f.held, f.hand, f.wake.len())).collect();
            let line = format!("{i:3} t={t:7.2}s day={:8.2} cam={:.2},{:.2} z{:.2} {} {} floats [{}]\n", app.clock.day, cam.lon, cam.lat, cam.zoom, app.gesture_state(), path.display(), floats.join("; "));
            print!("{line}");
            log.push_str(&line);
        }
    }
    std::fs::write(out.join("play.log"), log)?;
    if failed > 0 {
        anyhow::bail!("{failed} expectation(s) failed");
    }
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("render") if args.len() >= 3 => render(&args[1..]),
        Some("play") if args.len() >= 3 => play(&args[1..]),
        Some("info") => {
            let (_i, gpu) = gpu()?;
            println!("{:?}", gpu.adapter_info);
            Ok(())
        }
        _ => {
            eprintln!("usage: adrift-check render <scene.json> <out.png> [--size WxH] [--ppp 3] | play <plan.json> <out-dir> | info");
            std::process::exit(2);
        }
    }
}
