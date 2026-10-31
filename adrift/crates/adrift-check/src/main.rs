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
    #[serde(default = "default_size")]
    size: [u32; 2],
    #[serde(default = "default_ppp")]
    ppp: f64,
    scene: Scene,
    steps: Vec<Step>,
}
fn default_size() -> [u32; 2] {
    [1179, 2556]
}
fn default_ppp() -> f64 {
    3.0
}

#[derive(Deserialize)]
struct Step {
    #[serde(default)]
    id: u64,
    down: Option<[f64; 2]>,
    #[serde(rename = "move")]
    move_: Option<[f64; 2]>,
    up: Option<[f64; 2]>,
    advance: Option<f64>,
    shot: Option<String>,
    intent: Option<adrift_core::Intent>,
}

fn play(args: &[String]) -> Result<()> {
    let plan: Plan = serde_json::from_slice(&std::fs::read(&args[0]).with_context(|| args[0].clone())?)?;
    let out = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&out)?;
    let (_instance, gpu) = gpu()?;
    let [w, h] = plan.size;
    let target = Offscreen::new(&gpu, w, h);
    let mut app = App::new(gpu, Box::new(DirAssets(assets_dir())), w, h, plan.ppp)?;
    app.budget.locked = true;
    app.budget.scale = 1.0;
    app.set_scene(&plan.scene);
    let mut t = 0.0;
    let fps = 30.0;
    app.frame(&target.view, t);
    let mut log = String::new();
    for (i, s) in plan.steps.iter().enumerate() {
        if let Some(p) = s.down {
            app.pointer(PointerEvent { id: s.id, phase: Phase::Down, x: p[0], y: p[1], t });
        }
        if let Some(p) = s.move_ {
            app.pointer(PointerEvent { id: s.id, phase: Phase::Move, x: p[0], y: p[1], t });
        }
        if let Some(p) = s.up {
            app.pointer(PointerEvent { id: s.id, phase: Phase::Up, x: p[0], y: p[1], t });
        }
        if let Some(intent) = s.intent {
            app.intend(intent);
        }
        if let Some(secs) = s.advance {
            let n = (secs * fps).ceil().max(1.0) as usize;
            for _ in 0..n {
                t += secs / n as f64;
                app.frame(&target.view, t);
            }
        }
        if let Some(name) = &s.shot {
            app.frame(&target.view, t);
            let px = target.read(&app.gpu);
            let path = out.join(format!("{name}.png"));
            save(&path, w, h, &px)?;
            let floats: Vec<String> = app.floats.iter().map(|f| format!("#{} {:.3},{:.3} {:?} {:?} wake {}", f.id, f.at.lon, f.at.lat, f.held, f.hand, f.wake.len())).collect();
            let line = format!("{i:3} t={t:7.2}s day={:8.2} cam={:.2},{:.2} z{:.2} {} floats [{}]\n", app.clock.day, app.camera.camera.lon, app.camera.camera.lat, app.camera.camera.zoom, path.display(), floats.join("; "));
            print!("{line}");
            log.push_str(&line);
        }
    }
    std::fs::write(out.join("play.log"), log)?;
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
