//! Adrift headless: renders a scene, or plays a plan of pointer events over time, to PNG, on Linux's
//! software Vulkan (tools/adrift/linux-setup.sh). Every craft judges its work in these.
//!
//!     ADRIFT_ASSETS=<dir> cargo run -p adrift-check -- render <scene.json> <out.png> [--size WxH] [--ppp 3]
//!     ADRIFT_ASSETS=<dir> cargo run -p adrift-check -- play <plan.json> <out-dir>
//!     ADRIFT_ASSETS=<dir> cargo run -p adrift-check -- play-all <plan-or-dir>... <out-dir> [-j N]
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
use std::io::Write;
use std::path::{Path, PathBuf};

fn assets_dir() -> PathBuf {
    let dir = match std::env::var("ADRIFT_ASSETS") {
        Ok(d) => PathBuf::from(d),
        Err(_) => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/assets"),
    };
    ensure_assets(&dir);
    dir
}

/// Assembles the assets directory when the manifest names a file it lacks (a craft's new sheet),
/// so a check never fails for a missing link: `tools/adrift/assets.py <dir> --link`.
fn ensure_assets(dir: &Path) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let Ok(text) = std::fs::read_to_string(root.join("adrift/data/manifest.json")) else {
        return;
    };
    let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&text) else {
        return;
    };
    let Some(files) = manifest.get("files").and_then(|f| f.as_object()) else {
        return;
    };
    let missing: Vec<&String> = files
        .keys()
        .filter(|name| !dir.join(name).exists())
        .collect();
    if missing.is_empty() {
        return;
    }
    eprintln!(
        "assets: {} file(s) the manifest names are not in {}: linking",
        missing.len(),
        dir.display()
    );
    let status = std::process::Command::new("python3")
        .arg(root.join("tools/adrift/assets.py"))
        .arg(dir)
        .arg("--link")
        .status();
    if !matches!(status, Ok(s) if s.success()) {
        eprintln!("assets: tools/adrift/assets.py failed; set ADRIFT_ASSETS or run it by hand");
    }
}

/// Waits for the GPU's queue, so a frame's time includes its drawing (the software driver's).
fn gpu_wait(app: &App) {
    let _ = app.gpu.device.poll(wgpu::PollType::wait_indefinitely());
}

fn gpu() -> Result<(wgpu::Instance, Gpu)> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let gpu = pollster::block_on(Gpu::new(&instance, None))?;
    Ok((instance, gpu))
}

fn save(path: &Path, w: u32, h: u32, rgba: &[u8]) -> Result<()> {
    if let Some(p) = path.parent() {
        std::fs::create_dir_all(p)?;
    }
    image::save_buffer(path, rgba, w, h, image::ColorType::Rgba8)
        .with_context(|| path.display().to_string())
}

fn arg(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1).cloned())
}

fn render(args: &[String]) -> Result<()> {
    let scene: Scene =
        serde_json::from_slice(&std::fs::read(&args[0]).with_context(|| args[0].clone())?)?;
    let out = PathBuf::from(&args[1]);
    let size = arg(args, "--size").unwrap_or_else(|| "1179x2556".into());
    let (w, h) = size.split_once('x').context("--size WxH")?;
    let (w, h): (u32, u32) = (w.parse()?, h.parse()?);
    let ppp: f64 = arg(args, "--ppp")
        .map(|s| s.parse())
        .transpose()?
        .unwrap_or(3.0);
    // `--clip`: the scene drawn as an exported clip's frame (R10): 9:16 at its own size, chrome out
    let clip = args.iter().any(|a| a == "--clip");
    let (_instance, gpu) = gpu()?;
    let target = if clip {
        Offscreen::new(
            &gpu,
            adrift_core::studio::CLIP.0,
            adrift_core::studio::CLIP.1,
        )
    } else {
        Offscreen::new(&gpu, w, h)
    };
    let t0 = std::time::Instant::now();
    let mut app = App::new(gpu, Box::new(DirAssets(assets_dir())), w, h, ppp)?;
    app.budget.locked = true;
    app.budget.scale = 1.0;
    eprintln!(
        "loaded in {:?} ({})",
        t0.elapsed(),
        app.gpu.adapter_info.name
    );
    app.set_scene(&scene);
    let t1 = std::time::Instant::now();
    if clip {
        app.render_clip(&target.view);
    } else {
        app.render(&target.view);
    }
    let px = target.read(&app.gpu);
    eprintln!(
        "drawn in {:?}; clouds behind the date by {:.2} days",
        t1.elapsed(),
        app.behind()
    );
    save(&out, target.width, target.height, &px)?;
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
    /// The page of credits open from the start.
    #[serde(default)]
    credits_open: bool,
    /// What the plan's floats are and what its puts make (R21): "Sea" (lamps) or "Wind" (seeds).
    #[serde(default)]
    medium: adrift_core::Medium,
    /// The panel's rows held fixed, `[[id, x, y, w, h], ...]` in points, whatever the look draws
    /// (Motion's D11, R6).
    panel: Option<Vec<[f64; 5]>>,
    /// Draw every advanced frame, not only the shots (Motion's D11, R6): the clouds step only when a
    /// frame is drawn, so a plan that reads `behind()` needs it; slower on the software driver.
    #[serde(default)]
    frames: bool,
    /// The shell's frame rate, frames a second (default 60): a plan at 20 shows a take recorded on a
    /// struggling phone (R10, Motion).
    #[serde(default = "default_fps")]
    fps: f64,
    /// A song the takes are conducted to (R12): a path to an audio file, or "tone" (a test tone);
    /// an export lays it under the clip from the take's first moment, as the device does.
    #[serde(default)]
    song: Option<String>,
    steps: Vec<Step>,
}
fn default_size() -> [f64; 2] {
    [1179.0, 2556.0]
}
fn default_ppp() -> f64 {
    3.0
}
fn default_fps() -> f64 {
    60.0
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
    /// A take begins (true) or ends (false) (R9).
    record: Option<bool>,
    /// The newest take exported to `<out>/<name>.mp4` at 30 frames a second (ffmpeg), its first and
    /// last frames as PNGs, and the replay's drift from the take's end checked (R9).
    export: Option<String>,
    /// A measured scrub (R5): `[days, seconds]`, the date dragged by `days` over `seconds` at 60
    /// frames a second, every frame drawn, the CPU cost of the state and of the frame logged.
    scrub: Option<[f64; 2]>,
    expect: Option<serde_json::Value>,
}

/// A finger's id and screen point: given, or its place projected through the camera.
fn point(
    app: &App,
    w: &Where,
    id: u64,
    at: Option<[f64; 2]>,
    xy: Option<[f64; 2]>,
) -> (u64, f64, f64) {
    match w {
        Where::Point(p) => (id, p[0], p[1]),
        Where::Finger(f) => {
            if let Some(p) = xy {
                return (*f, p[0], p[1]);
            }
            let at = at.expect("a finger step needs `at: [lon, lat]` or `xy`");
            let (p, _) = app
                .disc()
                .project(&app.camera.camera, adrift_core::LonLat::new(at[0], at[1]));
            (*f, p[0], p[1])
        }
    }
}

fn play(args: &[String]) -> Result<()> {
    let plan: Plan =
        serde_json::from_slice(&std::fs::read(&args[0]).with_context(|| args[0].clone())?)?;
    let out = PathBuf::from(&args[1]);
    std::fs::create_dir_all(&out)?;
    let (_instance, gpu) = gpu()?;
    play_plan(
        plan,
        &out,
        gpu,
        Box::new(DirAssets(assets_dir())),
        &mut std::io::stdout(),
    )
}

/// Plays one plan into `out`, its lines (the expectations, the shots, the scrubs, the exports) to
/// `say` as they happen: `play`'s stdout, or the plan's log in `play-all`.
fn play_plan(
    plan: Plan,
    out: &Path,
    gpu: Gpu,
    assets: Box<dyn adrift_core::Assets>,
    say: &mut dyn std::io::Write,
) -> Result<()> {
    let points = plan.scene.is_none();
    let (w, h) = if points {
        (
            (plan.size[0] * plan.ppp) as u32,
            (plan.size[1] * plan.ppp) as u32,
        )
    } else {
        (plan.size[0] as u32, plan.size[1] as u32)
    };
    let target = Offscreen::new(&gpu, w, h);
    let mut app = App::new(gpu, assets, w, h, plan.ppp)?;
    app.budget.locked = true;
    app.budget.scale = 1.0;
    app.reduce_motion = plan.reduce_motion;
    let scene = plan.scene.clone().unwrap_or_else(|| Scene {
        camera: plan
            .camera
            .unwrap_or(adrift_core::Camera::new(-40.0, 22.0, 1.0)),
        day: plan.day,
        running: true,
        floats: plan.floats.clone(),
        hold: None,
        choose: Vec::new(),
        follow: Vec::new(),
        keep: Vec::new(),
        seconds: 0.0,
        night: plan.night,
        credits_open: plan.credits_open,
        medium: plan.medium,
    });
    app.set_scene(&scene);
    if let Some(song) = &plan.song {
        app.set_song(Some(song.clone()));
    }
    if let Some(rows) = &plan.panel {
        app.panel_stand_in = Some(
            rows.iter()
                .map(|r| {
                    (
                        r[0] as u32,
                        [r[1] as f32, r[2] as f32, r[3] as f32, r[4] as f32],
                    )
                })
                .collect(),
        );
    }
    let mut t = 0.0;
    let fps = plan.fps;
    app.frame(&target.view, t);
    let mut log = String::new();
    let mut failed = 0;
    let every_frame = plan.frames;
    let advance = |app: &mut App, t: &mut f64, secs: f64| {
        let n = (secs * fps).ceil().max(1.0) as usize;
        for _ in 0..n {
            *t += secs / n as f64;
            if every_frame {
                app.frame(&target.view, *t);
            } else {
                app.advance(*t);
            }
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
        for (wh, phase) in [
            (&s.down, Phase::Down),
            (&s.move_, Phase::Move),
            (&s.up, Phase::Up),
            (&s.cancel, Phase::Cancel),
        ] {
            if let Some(wh) = wh {
                let (id, x, y) = point(&app, wh, s.id, s.at, s.xy);
                app.pointer(PointerEvent { id, phase, x, y, t });
            }
        }
        if let Some(on) = s.record {
            app.command(adrift_core::Intent::Record { on });
        }
        if let Some(intent) = s.intent {
            app.command(intent);
        }
        if let Some([days, secs]) = s.scrub {
            let n = (secs * fps).ceil().max(1.0) as usize;
            let (mut a_sum, mut a_worst, mut f_sum, mut f_worst) = (0.0f64, 0.0f64, 0.0f64, 0.0f64);
            for _ in 0..n {
                t += secs / n as f64;
                app.command(adrift_core::Intent::Scrub {
                    days: days / n as f64,
                });
                let t0 = std::time::Instant::now();
                app.advance(t);
                let a = t0.elapsed().as_secs_f64() * 1000.0;
                let t1 = std::time::Instant::now();
                app.frame(&target.view, t);
                gpu_wait(&app);
                let f = t1.elapsed().as_secs_f64() * 1000.0;
                a_sum += a;
                a_worst = a_worst.max(a);
                f_sum += f;
                f_worst = f_worst.max(f);
            }
            let gpu_name = format!(
                "{} {:?}",
                app.gpu.adapter_info.name, app.gpu.adapter_info.device_type
            );
            let line = format!(
                "{i:3} t={t:8.2}s scrub {days:+.0} days over {secs:.1}s: {n} frames; state {:.2} ms mean {:.2} worst; frame {:.1} ms mean {:.1} worst ({}); {}",
                a_sum / n as f64,
                a_worst,
                f_sum / n as f64,
                f_worst,
                gpu_name,
                app.stats()
            );
            writeln!(say, "{line}")?;
            log.push_str(&line);
            log.push('\n');
        }
        if let Some(k) = &s.key {
            app.key(match k.to_ascii_lowercase().as_str() {
                "redo" => adrift_core::touch::Key::Redo,
                "home" => adrift_core::touch::Key::Home,
                "clear" => adrift_core::touch::Key::ClearAll,
                _ => adrift_core::touch::Key::Undo,
            });
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
                        if let (Some(lon), Some(lat)) = (
                            f.get("lon").and_then(|v| v.as_f64()),
                            f.get("lat").and_then(|v| v.as_f64()),
                        ) {
                            let tol = f.get("tol").and_then(|v| v.as_f64()).unwrap_or(1.5);
                            if (x.at.lon - lon).abs() > tol || (x.at.lat - lat).abs() > tol {
                                bad.push(format!(
                                    "float {id} at {:.2},{:.2} not near {lon},{lat}",
                                    x.at.lon, x.at.lat
                                ));
                            }
                        }
                    }
                    None => bad.push(format!("no float {id}")),
                }
            }
            if let Some(moved) = e.get("camera_moved").and_then(|v| v.as_bool()) {
                let did = (cam.lon - camera0.lon).abs() > 0.01
                    || (cam.lat - camera0.lat).abs() > 0.01
                    || (cam.zoom - camera0.zoom).abs() > 0.001;
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
                let held = app
                    .floats
                    .iter()
                    .find(|f| f.hand != adrift_core::Hand::Free)
                    .map(|f| f.id as u64);
                if held != hv.as_u64() {
                    bad.push(format!("held {held:?} not {:?}", hv.as_u64()));
                }
            }
            if let Some(n) = e.get("born").and_then(|v| v.as_u64()) {
                // the floats that exist on the day (R5: none before its put)
                let born = app.floats.iter().filter(|f| f.born(app.clock.day)).count() as u64;
                if born != n {
                    bad.push(format!("{born} floats born, not {n}"));
                }
            }
            if let Some(c) = e.get("chosen") {
                // a float's id, or true/false for any or none (R5)
                // (R6: several may be chosen; an id means it is the front one, the last chosen; an array
                // is the whole set in order)
                let ok = match (c.as_bool(), c.as_u64(), c.as_array()) {
                    (Some(b), _, _) => !app.chosen.is_empty() == b,
                    (_, Some(id), _) => app.chosen.last() == Some(&(id as u32)),
                    (_, _, Some(ids)) => {
                        ids.iter()
                            .map(|v| v.as_u64().map(|x| x as u32))
                            .collect::<Option<Vec<u32>>>()
                            == Some(app.chosen.clone())
                    }
                    _ => c.is_null() && app.chosen.is_empty(),
                };
                if !ok {
                    bad.push(format!("chosen {:?} not {c}", app.chosen));
                }
            }
            if let Some(c) = e.get("followed") {
                // the floats chosen as themselves, in order (R8), as an array of ids
                let want: Option<Vec<u32>> = c
                    .as_array()
                    .and_then(|a| a.iter().map(|v| v.as_u64().map(|x| x as u32)).collect());
                if want.as_ref() != Some(&app.followed) {
                    bad.push(format!("followed {:?} not {c}", app.followed));
                }
            }
            if let Some(n) = e.get("clouds").and_then(|v| v.as_u64()) {
                // how many clouds are out (R8: a lamp chosen as itself shows none, ridden or not)
                let out = app.shown_ids().len() as u64;
                if out != n {
                    bad.push(format!("{out} clouds out, not {n}"));
                }
            }
            if let Some(n) = e.get("framed").and_then(|v| v.as_u64()) {
                // every place `frame_points` names is on the globe's face (R8: the camera frames the chosen)
                let disc = app.disc();
                let (w, h) = (
                    app.view().width as f64 / app.view().ppp,
                    app.view().height as f64 / app.view().ppp,
                );
                let inside = app
                    .frame_points()
                    .iter()
                    .filter(|p| {
                        let (q, facing) = disc.project(&app.camera.camera, **p);
                        facing > 0.0 && q[0] >= 0.0 && q[0] <= w && q[1] >= 0.0 && q[1] <= h
                    })
                    .count();
                let all = app.frame_points().len();
                if inside < all || (all as u64) < n {
                    bad.push(format!(
                        "{inside} of {all} framed points in view, wanted all of at least {n}"
                    ));
                }
            }
            if let Some(r) = e.get("riding") {
                // a float's id, or true/false for riding any float or none
                let riding = app.riding().map(|id| id as u64);
                let ok = match (r.as_bool(), r.as_u64()) {
                    (Some(b), _) => riding.is_some() == b,
                    (_, Some(id)) => riding == Some(id),
                    _ => r.is_null() && riding.is_none(),
                };
                if !ok {
                    bad.push(format!("riding {riding:?} not {r}"));
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
                if let (Some(lon), Some(lat)) = (
                    c.get("lon").and_then(|v| v.as_f64()),
                    c.get("lat").and_then(|v| v.as_f64()),
                ) {
                    if (cam.lon - lon).abs() > 2.0 || (cam.lat - lat).abs() > 2.0 {
                        bad.push(format!(
                            "camera {:.2},{:.2} not near {lon},{lat}",
                            cam.lon, cam.lat
                        ));
                    }
                }
            }
            if e.get("camera_same_as_last").and_then(|v| v.as_bool()) == Some(true) {
                if let Some(last) = last_camera {
                    if (cam.lon - last.lon).abs() > 0.01
                        || (cam.lat - last.lat).abs() > 0.01
                        || (cam.zoom - last.zoom).abs() > 0.001
                    {
                        bad.push("camera moved since the last expectation".into());
                    }
                }
            }
            // R10: whether a take is being recorded, and how many are kept
            if let Some(want) = e.get("recording").and_then(|v| v.as_bool()) {
                if app.studio.recording.is_some() != want {
                    bad.push(format!(
                        "recording {} (wanted {want})",
                        app.studio.recording.is_some()
                    ));
                }
            }
            if let Some(want) = e.get("takes").and_then(|v| v.as_u64()) {
                if app.studio.takes.len() as u64 != want {
                    bad.push(format!("{} takes (wanted {want})", app.studio.takes.len()));
                }
            }
            if e.get("camera_near_float").is_some() {
                if let Some(f) = app.floats.first() {
                    if (cam.lon - f.at.lon).abs() > 3.0 || (cam.lat - f.at.lat).abs() > 3.0 {
                        bad.push(format!(
                            "camera {:.1},{:.1} not near the float {:.1},{:.1}",
                            cam.lon, cam.lat, f.at.lon, f.at.lat
                        ));
                    }
                }
            }
            last_camera = Some(cam);
            // `saw` and `not` speak of the intents since the last check (touch/VOCABULARY.md › Plans)
            app.seen.clear();
            let line = if bad.is_empty() {
                format!("{i:3} t={t:7.2}s expect ok\n")
            } else {
                failed += 1;
                format!("{i:3} t={t:7.2}s EXPECT FAILED: {}\n", bad.join("; "))
            };
            write!(say, "{line}")?;
            log.push_str(&line);
        }
        if let Some(name) = &s.export {
            let frames = app.export_begin(None, 30.0);
            anyhow::ensure!(frames > 0, "export {name}: no take");
            let (tw, th) = app.export_target_size().unwrap();
            let (ew, eh) = app.export_size().unwrap();
            let shot = Offscreen::new(&app.gpu, tw, th);
            let mp4 = out.join(format!("{name}.mp4"));
            let mut ff = std::process::Command::new("ffmpeg")
                .args([
                    "-loglevel",
                    "error",
                    "-y",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "bgra",
                    "-s",
                    &format!("{ew}x{eh}"),
                    "-r",
                    "30",
                    "-i",
                    "-",
                    "-c:v",
                    "libx265",
                    "-b:v",
                    "1200k",
                    "-tag:v",
                    "hvc1",
                    "-x265-params",
                    "log-level=error",
                    "-pix_fmt",
                    "yuv420p",
                ])
                .arg(&mp4)
                .stdin(std::process::Stdio::piped())
                .spawn()
                .context("ffmpeg")?;
            let t_export = std::time::Instant::now();
            let mut n = 0u32;
            let mut last = Vec::new();
            let mut trace = String::from("frame,clip_lon,clip_lat,clip_zoom,clip_roll,take_lon,take_lat,take_zoom,take_roll,lamps\n");
            while let Some(px) = app.export_frame(&shot) {
                if let Some((c, k)) = app.export_camera() {
                    // each lamp out: id:lon:lat, space-separated (R12: lamps in frame becomes a number)
                    let lamps: Vec<String> = app
                        .floats
                        .iter()
                        .filter(|f| f.born(app.clock.day))
                        .map(|f| format!("{}:{:.4}:{:.4}", f.id, f.at.lon, f.at.lat))
                        .collect();
                    trace.push_str(&format!(
                        "{n},{:.5},{:.5},{:.5},{:.3},{:.5},{:.5},{:.5},{:.3},{}\n",
                        c.lon,
                        c.lat,
                        c.zoom,
                        c.roll,
                        k.lon,
                        k.lat,
                        k.zoom,
                        k.roll,
                        lamps.join(" ")
                    ));
                }
                ff.stdin.as_mut().unwrap().write_all(&px)?;
                if n == 0 || n + 1 == frames {
                    let rgba: Vec<u8> = px
                        .chunks_exact(4)
                        .flat_map(|p| [p[2], p[1], p[0], p[3]])
                        .collect();
                    save(
                        &out.join(format!(
                            "{name}-{}.png",
                            if n == 0 { "first" } else { "last" }
                        )),
                        ew,
                        eh,
                        &rgba,
                    )?;
                }
                last = px;
                n += 1;
            }
            drop(last);
            // the clip's camera, frame by frame, beside the take's (R10)
            std::fs::write(out.join(format!("{name}-camera.csv")), trace)?;
            drop(ff.stdin.take());
            ff.wait()?;
            // the song under the clip, from the take's first moment, fading over its last second (R12)
            if let (Some(song), Some((start, len))) = (&plan.song, app.export_song()) {
                let silent = out.join(format!("{name}-silent.mp4"));
                std::fs::rename(&mp4, &silent)?;
                let mut cmd = std::process::Command::new("ffmpeg");
                cmd.args(["-loglevel", "error", "-y", "-i"]).arg(&silent);
                if song == "tone" {
                    cmd.args([
                        "-f",
                        "lavfi",
                        "-i",
                        &format!("sine=frequency=330:duration={len}"),
                    ]);
                } else {
                    cmd.arg("-i").arg(song);
                }
                let ms = (start * 1000.0).round() as u64;
                let fade = (len - 1.0).max(0.0);
                cmd.args([
                    "-filter_complex",
                    &format!(
                        "[1:a]atrim=0:{len},adelay={ms}|{ms},afade=t=out:st={:.3}:d=1[a]",
                        start + fade
                    ),
                    "-map",
                    "0:v",
                    "-map",
                    "[a]",
                    "-c:v",
                    "copy",
                    "-c:a",
                    "aac",
                    "-t",
                    &format!("{:.3}", start + len),
                ]);
                let ok = cmd.arg(&mp4).status().context("ffmpeg (song)")?.success();
                anyhow::ensure!(
                    ok,
                    "export {name}: the song could not be laid under the clip"
                );
                let line = format!(
                    "{i:3} t={t:7.2}s export {name}: the song from {start:.2} s for {len:.2} s\n"
                );
                write!(say, "{line}")?;
                log.push_str(&line);
            }
            let drift = app.export_drift();
            app.export_end();
            let line = format!("{i:3} t={t:7.2}s export {name}: {n} frames {ew}x{eh} in {:.1}s, the replay ends {} from the take\n", t_export.elapsed().as_secs_f64(), drift.map_or("(no end recorded)".to_string(), |d| format!("{d:.3} km")));
            write!(say, "{line}")?;
            log.push_str(&line);
            if drift.map_or(true, |d| d > 0.5) {
                failed += 1;
                let line =
                    format!("{i:3} t={t:7.2}s EXPECT FAILED: the replay drifted from the take\n");
                write!(say, "{line}")?;
                log.push_str(&line);
            }
        }
        if let Some(name) = &s.shot {
            app.frame(&target.view, t);
            let px = target.read(&app.gpu);
            let path = out.join(format!("{name}.png"));
            save(&path, w, h, &px)?;
            let cam = app.camera.camera;
            let floats: Vec<String> = app
                .floats
                .iter()
                .map(|f| {
                    format!(
                        "#{} {:.3},{:.3} {:?} {:?} wake {}",
                        f.id,
                        f.at.lon,
                        f.at.lat,
                        f.held,
                        f.hand,
                        f.wake.len()
                    )
                })
                .collect();
            let line = format!(
                "{i:3} t={t:7.2}s day={:8.2} cam={:.2},{:.2} z{:.2} {} {} floats [{}]\n",
                app.clock.day,
                cam.lon,
                cam.lat,
                cam.zoom,
                app.gesture_state(),
                path.display(),
                floats.join("; ")
            );
            write!(say, "{line}")?;
            log.push_str(&line);
        }
    }
    std::fs::write(out.join("play.log"), log)?;
    if failed > 0 {
        anyhow::bail!("{failed} expectation(s) failed");
    }
    Ok(())
}

/// Every plan in one process: `play-all <plan-or-dir>... <out-dir> [-j N]`, the GPU and the data
/// loaded once instead of a launch a plan. One device and the assets' bytes are shared; each plan
/// gets its own App (its own textures, passes and state), so it plays exactly as `play` would
/// (tools/adrift/play_all_same.sh proves it). A directory gives its `*.json` in name order; a plan
/// without `"steps"` is skipped (a scene, as the GPU runner skips it). Each plan's lines go to
/// `<out>/<name>.log` (what `play` prints, and its error as `play` ends with it) and its outputs to
/// `<out>/<name>/`; one line a plan, in the order given: `ok   <name>`, or `FAIL <name>` and its
/// first failing lines indented. Exits non-zero if any failed. A name two plans share takes its
/// module's name before it (`look-credits`, `touch-credits`). N defaults to the cores, at most 4.
fn play_all(args: &[String]) -> Result<()> {
    let mut jobs: Option<usize> = None;
    let mut paths = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "-j" {
            jobs = Some(it.next().context("-j N")?.parse().context("-j N")?);
        } else if let Some(n) = a.strip_prefix("-j").filter(|n| !n.is_empty()) {
            jobs = Some(n.parse().context("-j N")?);
        } else {
            paths.push(PathBuf::from(a));
        }
    }
    let out = paths.pop().context("play-all <plan-or-dir>... <out-dir>")?;
    anyhow::ensure!(
        !paths.is_empty(),
        "play-all <plan-or-dir>... <out-dir>: no plans given"
    );
    let mut plans = Vec::new();
    for p in &paths {
        if p.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(p)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|f| f.extension().is_some_and(|x| x == "json"))
                .collect();
            found.sort();
            plans.extend(found);
        } else {
            plans.push(p.clone());
        }
    }
    plans.retain(|p| std::fs::read_to_string(p).map_or(true, |t| t.contains("\"steps\"")));
    let stem = |p: &Path| {
        p.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default()
    };
    let names: Vec<String> = plans
        .iter()
        .map(|p| {
            let name = stem(p);
            if plans.iter().filter(|q| stem(q) == name).count() > 1 {
                // the module the plan belongs to: touch/plans/credits.json is touch-credits
                let module = p
                    .parent()
                    .filter(|d| d.file_name().is_some_and(|n| n == "plans"))
                    .and_then(|d| d.parent())
                    .map(stem)
                    .unwrap_or_default();
                format!("{module}-{name}")
            } else {
                name
            }
        })
        .collect();
    std::fs::create_dir_all(&out)?;
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    let jobs = jobs.unwrap_or(cores.min(4)).clamp(1, plans.len().max(1));
    // once for all: the device, and every asset's bytes (each App still decodes its own textures)
    let (_instance, gpu0) = gpu()?;
    let dir = assets_dir();
    let mut bytes = std::collections::HashMap::new();
    for name in adrift_core::assets::FILES {
        if let Ok(b) = std::fs::read(dir.join(name)) {
            bytes.insert(name.to_string(), b);
        }
    }
    let shared = std::sync::Arc::new(bytes);
    // a plan that panics: its log says so (a release build, panic = "abort", then stops here)
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(log) = PLAYING.with(|p| p.borrow().clone()) {
            if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&log) {
                let _ = writeln!(f, "{info}");
            }
            eprintln!("play-all: the plan logged in {} panicked", log.display());
        }
        default_hook(info);
    }));
    let next = std::sync::atomic::AtomicUsize::new(0);
    let (tx, rx) = std::sync::mpsc::channel::<(usize, bool)>();
    let mut failed = 0;
    std::thread::scope(|s| {
        for _ in 0..jobs {
            let tx = tx.clone();
            let (gpu0, shared, plans, names, out, dir, next) =
                (&gpu0, &shared, &plans, &names, &out, &dir, &next);
            // one device for all (measured: a device per worker is no faster on the software driver)
            s.spawn(move || loop {
                let k = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if k >= plans.len() {
                    break;
                }
                let gpu = Gpu {
                    device: gpu0.device.clone(),
                    queue: gpu0.queue.clone(),
                    format: gpu0.format,
                    adapter_info: gpu0.adapter_info.clone(),
                };
                let assets = Box::new(Shared {
                    bytes: shared.clone(),
                    dir: DirAssets(dir.clone()),
                });
                let ok = play_one(
                    &plans[k],
                    &out.join(&names[k]),
                    &out.join(format!("{}.log", names[k])),
                    gpu,
                    assets,
                );
                let _ = tx.send((k, ok));
            });
        }
        drop(tx);
        // the lines in the order the plans were given, each as soon as those before it are done
        let mut done = std::collections::BTreeMap::new();
        let mut shown = 0;
        for (k, ok) in rx {
            done.insert(k, ok);
            while let Some(ok) = done.remove(&shown) {
                let name = &names[shown];
                if ok {
                    println!("ok   {name}");
                } else {
                    failed += 1;
                    println!("FAIL {name}");
                    let log = std::fs::read_to_string(out.join(format!("{name}.log")))
                        .unwrap_or_default();
                    for l in log
                        .lines()
                        .filter(|l| l.contains("EXPECT FAILED") || l.contains("rror"))
                        .take(5)
                    {
                        println!("     {l}");
                    }
                }
                shown += 1;
            }
        }
    });
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

thread_local! {
    /// The log of the plan this thread plays (for the panic hook).
    static PLAYING: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// One plan of `play-all`, as `play <plan> <dir>` with its output to `log`; true if it passed.
fn play_one(
    plan: &Path,
    out: &Path,
    log: &Path,
    gpu: Gpu,
    assets: Box<dyn adrift_core::Assets>,
) -> bool {
    let Ok(mut f) = std::fs::File::create(log) else {
        return false;
    };
    PLAYING.with(|p| *p.borrow_mut() = Some(log.to_path_buf()));
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<()> {
        let parsed: Plan = serde_json::from_slice(
            &std::fs::read(plan).with_context(|| plan.display().to_string())?,
        )?;
        std::fs::create_dir_all(out)?;
        play_plan(parsed, out, gpu, assets, &mut f)
    }));
    PLAYING.with(|p| *p.borrow_mut() = None);
    match run {
        Ok(Ok(())) => true,
        Ok(Err(e)) => {
            // as `play` ends: main's error, on the stderr the scripts' logs took in
            let _ = writeln!(f, "Error: {e:?}");
            false
        }
        Err(_) => false,
    }
}

/// The assets' bytes read once and shared by every plan's App; a name not read falls through to the
/// directory, so a missing file fails as it does for `play`.
struct Shared {
    bytes: std::sync::Arc<std::collections::HashMap<String, Vec<u8>>>,
    dir: DirAssets,
}

impl adrift_core::Assets for Shared {
    fn read(&self, name: &str) -> Result<Vec<u8>> {
        match self.bytes.get(name) {
            Some(b) => Ok(b.clone()),
            None => self.dir.read(name),
        }
    }
}

/// A journey clip of copied lamps (R13): `journey <lamps.txt> <seconds> <out.mp4>` pastes the text
/// the app's "Copy these lamps" makes (or a phone's whole kept state, `adrift.json`, whose lamps are
/// all taken) and exports their journey as the phone would, with first and last frames beside it.
fn journey(args: &[String]) -> Result<()> {
    let text = std::fs::read_to_string(&args[0]).with_context(|| args[0].clone())?;
    let seconds: f64 = args[1].parse().context("seconds")?;
    let out = PathBuf::from(&args[2]);
    if let Some(dir) = out.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let (_instance, gpu) = gpu()?;
    let (w, h, ppp) = (1179u32, 2556u32, 3.0);
    let mut app = App::new(gpu, Box::new(DirAssets(assets_dir())), w, h, ppp)?;
    // (R21) `--night`: the night palette, as a phone in dark appearance exports it
    app.night = args.iter().any(|a| a == "--night");
    let n = if text
        .trim_start()
        .starts_with(adrift_core::app::LAMPS_HEADER)
    {
        app.paste_lamps(&text, true)
    } else {
        anyhow::ensure!(
            app.restore(&text),
            "not lamps copied from Adrift, nor its kept state"
        );
        app.floats.len()
    };
    anyhow::ensure!(n > 0, "no lamps in {}", args[0]);
    // (R22) the edit's days and pace (`--from-day`, `--to-day`, `--pace`; days since 2000 and days a
    // second), the export's rate (`--fps`, 60 as the phone) and the master's bitrate (`--bitrate`, as
    // ffmpeg takes it; 12M as the phone)
    let num = |name: &str| arg(args, name).and_then(|v| v.parse::<f64>().ok());
    let edit = adrift_core::app::JourneyEdit {
        seconds,
        from: num("--from-day"),
        to: num("--to-day"),
        pace: num("--pace"),
    };
    let fps = num("--fps").unwrap_or(60.0);
    let bitrate = arg(args, "--bitrate").unwrap_or_else(|| "12M".to_string());
    // (R22, Motion) `--camera-only`: the replay and the clip's camera stepped, nothing drawn or encoded,
    // the trace (`<out>.csv`) written as ever: the framing measured in seconds, not minutes
    let dry = args.iter().any(|a| a == "--camera-only");
    app.export_dry = dry;
    let frames = app.journey_begin_edit(&edit, fps);
    anyhow::ensure!(frames > 0, "no journey to make");
    let day0 = app.clock.day;
    let (ew, eh) = app.export_size().unwrap();
    let shot = Offscreen::new(&app.gpu, ew, eh);
    let mut ff = std::process::Command::new(if dry { "true" } else { "ffmpeg" })
        .args([
            "-loglevel",
            "error",
            "-y",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "bgra",
            "-s",
            &format!("{ew}x{eh}"),
            "-r",
            &format!("{fps}"),
            "-i",
            "-",
            "-c:v",
            "libx265",
            "-b:v",
            &bitrate,
            "-tag:v",
            "hvc1",
            "-x265-params",
            "log-level=error",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&out)
        .stdin(std::process::Stdio::piped())
        .spawn()
        .context("ffmpeg")?;
    let t0 = std::time::Instant::now();
    let mut i = 0u32;
    // the clip's camera and each lamp, a row a frame, beside the mp4 (R14: as `play`'s export writes)
    let mut trace = String::from(
        "frame,clip_lon,clip_lat,clip_zoom,clip_roll,take_lon,take_lat,take_zoom,take_roll,lamps\n",
    );
    // (R24) the phone's pipeline: two frames in flight, each read back straight into the encoder's
    // rows (here one reused buffer), so the GPU draws the next while the last is copied out
    let mut rb = adrift_core::gpu::Readback::new(&app.gpu, &shot, 2);
    let mut px = vec![0u8; (ew * eh * 4) as usize];
    let (mut rendered, mut t_draw, mut t_take, mut t_pipe) = (0u32, 0.0f64, 0.0f64, 0.0f64);
    loop {
        while rb.has_room() {
            let t = std::time::Instant::now();
            let Some(drawn) = app.export_render(&shot) else {
                break;
            };
            if let Some((c, k)) = app.export_camera() {
                let lamps: Vec<String> = app
                    .floats
                    .iter()
                    .filter(|f| f.born(app.clock.day))
                    .map(|f| format!("{}:{:.4}:{:.4}", f.id, f.at.lon, f.at.lat))
                    .collect();
                trace.push_str(&format!(
                    "{rendered},{:.5},{:.5},{:.5},{:.3},{:.5},{:.5},{:.5},{:.3},{}\n",
                    c.lon,
                    c.lat,
                    c.zoom,
                    c.roll,
                    k.lon,
                    k.lat,
                    k.zoom,
                    k.roll,
                    lamps.join(" ")
                ));
            }
            rendered += 1;
            if drawn {
                rb.submit(&app.gpu, &shot);
            }
            t_draw += t.elapsed().as_secs_f64();
        }
        let t = std::time::Instant::now();
        if !rb.take(&app.gpu, &mut px, (ew * 4) as usize, true) {
            break;
        }
        t_take += t.elapsed().as_secs_f64();
        let t = std::time::Instant::now();
        ff.stdin.as_mut().unwrap().write_all(&px)?;
        if i == 0 || i + 1 == frames || i % 900 == 0 {
            let rgba: Vec<u8> = px
                .chunks_exact(4)
                .flat_map(|p| [p[2], p[1], p[0], p[3]])
                .collect();
            save(&out.with_extension(format!("{i:05}.png")), ew, eh, &rgba)?;
        }
        t_pipe += t.elapsed().as_secs_f64();
        i += 1;
        if i == 120 && adrift_core::look::profile::on() {
            eprintln!(
                "(ADRIFT_PROFILE) the GPU's passes, a frame, the first 120 frames:\n{}",
                adrift_core::look::profile::report()
            );
        }
    }
    if dry {
        i = rendered;
    }
    let n_f = i.max(1) as f64;
    eprintln!("export: {i} frames; a frame {:.1} ms drawing (the replay and the GPU's commands), {:.1} ms waiting for and copying its pixels, {:.1} ms to the encoder ({})", t_draw / n_f * 1e3, t_take / n_f * 1e3, t_pipe / n_f * 1e3, app.gpu.adapter_info.name);
    if adrift_core::look::profile::on() {
        eprintln!(
            "(ADRIFT_PROFILE) the GPU's passes, a frame, from frame 120:\n{}",
            adrift_core::look::profile::report()
        );
    }
    drop(ff.stdin.take());
    ff.wait()?;
    // (R21) the days the journey covered and the file it made: a longer clip is a longer journey
    let bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    eprintln!("journey: day {day0:.1} -> {:.1} ({:.1} days in {seconds} s, {:.2} days a second); {:.1} MB ({:.1} MB a minute)", app.clock.day, app.clock.day - day0, (app.clock.day - day0) / seconds, bytes as f64 / 1e6, bytes as f64 / 1e6 / seconds * 60.0);
    std::fs::write(out.with_extension("csv"), trace)?;
    app.export_end();
    println!(
        "{}: {n} lamps, {i} frames {ew}x{eh} in {:.1}s",
        out.display(),
        t0.elapsed().as_secs_f64()
    );
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("render") if args.len() >= 3 => render(&args[1..]),
        Some("play") if args.len() >= 3 => play(&args[1..]),
        Some("play-all") if args.len() >= 3 => play_all(&args[1..]),
        Some("journey") if args.len() >= 4 => journey(&args[1..]),
        Some("info") => {
            // (R21) the adapter wgpu chose (not the first the system lists), on one line the GPU
            // runner's summary carries: `adapter: <name> (<type>, <backend>, <driver> <driver info>)`
            let (_i, gpu) = gpu()?;
            let a = &gpu.adapter_info;
            println!(
                "adapter: {} ({:?}, {:?}, {} {})",
                a.name, a.device_type, a.backend, a.driver, a.driver_info
            );
            Ok(())
        }
        _ => {
            eprintln!("usage: adrift-check render <scene.json> <out.png> [--size WxH] [--ppp 3] [--clip] | play <plan.json> <out-dir> | play-all <plan-or-dir>... <out-dir> [-j N] | journey <lamps.txt> <seconds> <out.mp4> [--night] [--fps 60] [--bitrate 12M] [--from-day D] [--to-day D] [--pace P] [--camera-only] | info");
            std::process::exit(2);
        }
    }
}
