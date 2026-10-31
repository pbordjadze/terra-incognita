//! touch-replay: plays a plan of pointer events through `Gestures` over the stand-in `App` and
//! checks its expectations (README.md), in milliseconds; and exports the same plans in the form
//! `adrift-check -- play` takes, so they run on the real build.
//!
//!     cargo run --manifest-path tools/adrift/touch-standalone/Cargo.toml -- [-v] plan.json...
//!     cargo run --manifest-path tools/adrift/touch-standalone/Cargo.toml -- --export <dir> plan.json...

use serde::Deserialize;
use touch_standalone::app::App;
use touch_standalone::model::{Camera, Hand, Intent, LonLat, Phase, PointerEvent};
use touch_standalone::touch::motion;

#[derive(Deserialize)]
struct Plan {
    #[serde(default)]
    title: String,
    /// Points.
    #[serde(default = "default_size")]
    size: [f64; 2],
    #[serde(default = "default_camera")]
    camera: Camera,
    #[serde(default)]
    day: f64,
    #[serde(default = "yes")]
    playing: bool,
    #[serde(default)]
    floats: Vec<LonLat>,
    /// Stand-in land: circles (lon, lat, radius in degrees). adrift-check uses the real coast.
    #[serde(default)]
    land: Vec<[f64; 3]>,
    /// Stand-in drift of every float, degrees a second of play time (adrift-check uses the rules).
    #[serde(default)]
    drift: [f64; 2],
    #[serde(default)]
    reduce_motion: bool,
    steps: Vec<Step>,
}
fn default_size() -> [f64; 2] {
    [820.0, 1180.0]
}
fn default_camera() -> Camera {
    Camera { lon: -40.0, lat: 22.0, zoom: 1.0 }
}
fn yes() -> bool {
    true
}

#[derive(Deserialize)]
struct Step {
    t: f64,
    down: Option<u64>,
    #[serde(rename = "move")]
    move_: Option<u64>,
    up: Option<u64>,
    cancel: Option<u64>,
    /// The finger's point: on screen, or a place on the globe projected by the camera as it is.
    xy: Option<[f64; 2]>,
    at: Option<[f64; 2]>,
    shot: Option<String>,
    shake: Option<bool>,
    key: Option<String>,
    expect: Option<Expect>,
}

#[derive(Deserialize, Default)]
struct Expect {
    floats: Option<usize>,
    state: Option<String>,
    #[serde(default)]
    saw: Vec<String>,
    #[serde(default)]
    not: Vec<String>,
    camera: Option<Camera>,
    zoom_gt: Option<f64>,
    zoom_lt: Option<f64>,
    riding: Option<Option<u32>>,
    held: Option<Option<u32>>,
    playing: Option<bool>,
    day: Option<f64>,
    float: Option<FloatAt>,
    /// The camera's centre is within this many degrees of the float.
    camera_near_float: Option<(u32, f64)>,
    camera_moved: Option<bool>,
    /// The camera has not moved since the last `expect`.
    camera_same_as_last: Option<bool>,
}
#[derive(Deserialize)]
struct FloatAt {
    id: u32,
    lon: f64,
    lat: f64,
}

fn name(i: &Intent) -> &'static str {
    match i {
        Intent::Ring { .. } => "Ring",
        Intent::Put { .. } => "Put",
        Intent::Hold { .. } => "Hold",
        Intent::Carry { .. } => "Carry",
        Intent::Release { at: Some(_), .. } => "Release",
        Intent::Release { at: None, .. } => "TakeOut",
        Intent::Ride { .. } => "Ride",
        Intent::Undo => "Undo",
        Intent::Scrub { .. } => "Scrub",
        Intent::SetRunning { running: true } => "Play",
        Intent::SetRunning { running: false } => "Pause",
    }
}

fn lon_delta(a: f64, b: f64) -> f64 {
    motion::lon_delta(a, b)
}

/// A step of the exported plan (adrift-check's form).
#[derive(serde::Serialize)]
struct OutStep {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    down: Option<[f64; 2]>,
    #[serde(rename = "move", skip_serializing_if = "Option::is_none")]
    move_: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    up: Option<[f64; 2]>,
    /// The system took the touch (adrift-check has yet to take this step: such a plan is marked `needs`).
    #[serde(skip_serializing_if = "Option::is_none")]
    cancel: Option<[f64; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    advance: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    intent: Option<Intent>,
    /// For `check_play.py`: what the shot of this moment should show.
    #[serde(skip_serializing_if = "Option::is_none")]
    expect: Option<serde_json::Value>,
}
fn step() -> OutStep {
    OutStep { id: None, down: None, move_: None, up: None, cancel: None, advance: None, shot: None, intent: None, expect: None }
}

fn run(path: &str, verbose: bool, export: Option<&str>) -> Result<usize, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let plan: Plan = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    let raw: serde_json::Value = serde_json::from_str(&text).unwrap();
    let ppp = 3.0;
    let mut app = App::new((plan.size[0] * ppp) as u32, (plan.size[1] * ppp) as u32, ppp, plan.camera);
    app.clock.day = plan.day;
    app.clock.running = plan.playing;
    app.rules.coast.land = plan.land.clone();
    app.reduce_motion = plan.reduce_motion;
    for p in &plan.floats {
        let id = app.put(*p);
        app.floats.iter_mut().find(|f| f.id == id).unwrap().hand = Hand::Free;
    }
    let mut now = 0.0;
    let mut applied_seen = 0;
    let mut failures = 0;
    let camera0 = plan.camera;
    let mut camera_last = plan.camera;
    let step_dt = 1.0 / 120.0;
    let mut exported: Vec<OutStep> = vec![];
    let mut exported_t = 0.0;
    let mut expects = 0;
    let mut needs: Vec<&str> = vec![];
    if plan.reduce_motion {
        needs.push("reduce_motion");
    }

    for (n, s) in plan.steps.iter().enumerate() {
        // ticks up to the step's time, the floats drifting as the plan says
        while now + step_dt <= s.t + 1e-9 {
            now += step_dt;
            if app.clock.running {
                for f in app.floats.iter_mut().filter(|f| f.hand != Hand::Carried) {
                    f.at.lon = motion::wrap_lon(f.at.lon + plan.drift[0] * step_dt);
                    f.at.lat += plan.drift[1] * step_dt;
                }
            }
            app.tick(now, step_dt);
            if verbose {
                for i in &app.applied[applied_seen..] {
                    println!("  {now:7.3}  {i:?}");
                }
                applied_seen = app.applied.len();
            }
        }
        let t = s.t;
        if s.t - exported_t > 1e-9 {
            exported.push(OutStep { advance: Some(s.t - exported_t), ..step() });
            exported_t = s.t;
        }
        let point = |app: &App| -> Result<[f64; 2], String> {
            if let Some(xy) = s.xy {
                return Ok(xy);
            }
            if let Some(at) = s.at {
                let (p, facing) = app.disc().project(&app.camera.camera, LonLat::new(at[0], at[1]));
                if facing <= 0.0 {
                    return Err(format!("step {n} at t={t}: ({}, {}) is behind the globe", at[0], at[1]));
                }
                return Ok(p);
            }
            Err(format!("step {n} at t={t}: a pointer step needs xy or at"))
        };
        let mut pointer = |id: u64, phase: Phase, app: &mut App, exported: &mut Vec<OutStep>| -> Result<(), String> {
            let p = point(app)?;
            app.pointer(PointerEvent { id, phase, x: p[0], y: p[1], t });
            if verbose {
                println!("{t:7.3} {phase:?} {id} at ({:.1}, {:.1})  → {}", p[0], p[1], app.gestures.describe());
                for i in &app.applied[applied_seen..] {
                    println!("    {i:?}");
                }
                applied_seen = app.applied.len();
            }
            let mut o = OutStep { id: Some(id), ..step() };
            match phase {
                Phase::Down => o.down = Some(p),
                Phase::Move => o.move_ = Some(p),
                Phase::Up => o.up = Some(p),
                Phase::Cancel => o.cancel = Some(p),
            }
            exported.push(o);
            Ok(())
        };
        if let Some(id) = s.down {
            pointer(id, Phase::Down, &mut app, &mut exported)?;
        }
        if let Some(id) = s.move_ {
            pointer(id, Phase::Move, &mut app, &mut exported)?;
        }
        if let Some(id) = s.up {
            pointer(id, Phase::Up, &mut app, &mut exported)?;
        }
        if let Some(id) = s.cancel {
            pointer(id, Phase::Cancel, &mut app, &mut exported)?;
            if !needs.contains(&"cancel") {
                needs.push("cancel");
            }
        }
        if s.shake == Some(true) {
            let out = app.gestures.shaken();
            app.apply(out, t);
            exported.push(OutStep { intent: Some(Intent::Undo), ..step() });
        }
        if let Some(k) = &s.key {
            let key = match k.as_str() {
                "undo" => touch_standalone::touch::Key::Undo,
                "redo" => touch_standalone::touch::Key::Redo,
                other => return Err(format!("step {n}: unknown key {other}")),
            };
            let out = app.gestures.key(key);
            app.apply(out, t);
            if key == touch_standalone::touch::Key::Undo {
                exported.push(OutStep { intent: Some(Intent::Undo), ..step() });
            }
        }
        if let Some(shot) = &s.shot {
            if verbose {
                println!("{t:7.3} shot {shot}: {} floats, camera {:?}, hand {}", app.floats.len(), app.camera.camera, app.gestures.describe());
            }
            exported.push(OutStep { shot: Some(shot.clone()), ..step() });
        }
        if let Some(e) = &s.expect {
            expects += 1;
            exported.push(OutStep { shot: Some(format!("expect-{expects:02}")), expect: raw["steps"][n]["expect"].clone().into(), ..step() });
            let seen: Vec<&'static str> = app.applied.iter().map(name).collect();
            let mut fail = |what: String| {
                failures += 1;
                println!("FAIL {path} step {n} (t={t}): {what}");
            };
            let cam = app.camera.camera;
            if let Some(k) = e.floats {
                if app.floats.len() != k {
                    fail(format!("{} floats, expected {k}", app.floats.len()));
                }
            }
            if let Some(st) = &e.state {
                if app.gestures.describe() != st {
                    fail(format!("hand is {}, expected {st}", app.gestures.describe()));
                }
            }
            for x in &e.saw {
                if !seen.iter().any(|y| y == x) {
                    fail(format!("never saw {x}; saw {:?}", seen));
                }
            }
            for x in &e.not {
                if seen.iter().any(|y| y == x) {
                    fail(format!("saw {x}, which was not to happen; saw {:?}", seen));
                }
            }
            if let Some(c) = e.camera {
                if lon_delta(cam.lon, c.lon).abs() > 0.01 || (cam.lat - c.lat).abs() > 0.01 || (cam.zoom - c.zoom).abs() > 0.01 {
                    fail(format!("camera {cam:?}, expected {c:?}"));
                }
            }
            if let Some(z) = e.zoom_gt {
                if !(cam.zoom > z) {
                    fail(format!("zoom {} not > {z}", cam.zoom));
                }
            }
            if let Some(z) = e.zoom_lt {
                if !(cam.zoom < z) {
                    fail(format!("zoom {} not < {z}", cam.zoom));
                }
            }
            if let Some(r) = e.riding {
                if app.camera.riding != r {
                    fail(format!("riding {:?}, expected {r:?}", app.camera.riding));
                }
            }
            if let Some(h) = e.held {
                let held = app.floats.iter().find(|f| f.hand != Hand::Free).map(|f| f.id);
                if held != h {
                    fail(format!("held {held:?}, expected {h:?}"));
                }
            }
            if let Some(p) = e.playing {
                if app.clock.running != p {
                    fail(format!("playing {}, expected {p}", app.clock.running));
                }
            }
            if let Some(d) = e.day {
                if (app.clock.day - d).abs() > 0.01 {
                    fail(format!("day {}, expected {d}", app.clock.day));
                }
            }
            if let Some(f) = &e.float {
                match app.floats.iter().find(|x| x.id == f.id) {
                    Some(x) if lon_delta(x.at.lon, f.lon).abs() <= 0.05 && (x.at.lat - f.lat).abs() <= 0.05 => {}
                    other => fail(format!("float {} at {:?}, expected ({}, {})", f.id, other.map(|x| x.at), f.lon, f.lat)),
                }
            }
            if let Some((id, within)) = e.camera_near_float {
                match app.floats.iter().find(|x| x.id == id) {
                    Some(x) => {
                        let d = lon_delta(cam.lon, x.at.lon).hypot(cam.lat - x.at.lat);
                        if d > within {
                            fail(format!("camera {cam:?} is {d:.2}° from float {id} at {:?}, expected within {within}", x.at));
                        }
                    }
                    None => fail(format!("no float {id}")),
                }
            }
            if let Some(m) = e.camera_moved {
                let moved = lon_delta(cam.lon, camera0.lon).abs() > 0.01 || (cam.lat - camera0.lat).abs() > 0.01 || (cam.zoom - camera0.zoom).abs() > 0.01;
                if moved != m {
                    fail(format!("camera moved: {moved} ({cam:?} from {camera0:?}), expected {m}"));
                }
            }
            if let Some(same) = e.camera_same_as_last {
                let is_same = lon_delta(cam.lon, camera_last.lon).abs() < 1e-6 && (cam.lat - camera_last.lat).abs() < 1e-6 && (cam.zoom - camera_last.zoom).abs() < 1e-6;
                if is_same != same {
                    fail(format!("camera same as last check: {is_same} ({cam:?} from {camera_last:?}), expected {same}"));
                }
            }
            camera_last = cam;
            app.applied.clear();
            applied_seen = 0;
        }
    }
    if let Some(dir) = export {
        let scene = serde_json::json!({
            "camera": plan.camera, "day": plan.day, "running": plan.playing, "seconds": 5.0,
            "floats": plan.floats.iter().map(|p| serde_json::json!({"lon": p.lon, "lat": p.lat, "days_ago": 3.0})).collect::<Vec<_>>(),
        });
        let out = serde_json::json!({
            "title": plan.title,
            "needs": needs,
            "reduce_motion": plan.reduce_motion,
            "size": [(plan.size[0] * ppp) as u32, (plan.size[1] * ppp) as u32], "ppp": ppp,
            "scene": scene, "steps": exported,
        });
        let name = std::path::Path::new(path).file_name().unwrap().to_string_lossy().to_string();
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::write(format!("{dir}/{name}"), serde_json::to_string_pretty(&out).unwrap() + "\n").map_err(|e| e.to_string())?;
    }
    Ok(failures)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v");
    let export = args.iter().position(|a| a == "--export").and_then(|i| args.get(i + 1).cloned());
    let mut plans: Vec<&String> = vec![];
    let mut skip = false;
    for a in &args {
        if skip {
            skip = false;
            continue;
        }
        if a == "-v" {
            continue;
        }
        if a == "--export" {
            skip = true;
            continue;
        }
        plans.push(a);
    }
    if plans.is_empty() {
        eprintln!("usage: touch-replay [-v] [--export <dir>] plan.json...");
        std::process::exit(2);
    }
    let mut failed = 0;
    for p in plans {
        match run(p, verbose, export.as_deref()) {
            Ok(0) => println!("ok   {p}"),
            Ok(n) => {
                failed += n;
                println!("FAIL {p}: {n} expectation(s)");
            }
            Err(e) => {
                failed += 1;
                println!("FAIL {e}");
            }
        }
    }
    if failed > 0 {
        std::process::exit(1);
    }
}
