//! touch-replay: plays a plan of pointer events through `Gestures` over a stand-in sea and checks
//! its expectations (README.md). The plan is the JSON `adrift-check -- play` takes, with `expect`
//! steps this replay checks and `land`/`drift` stand-ins it alone reads.
//!
//!     cargo run --manifest-path tools/adrift/touch-standalone/Cargo.toml -- [-v] plan.json...

use serde::Deserialize;
use std::collections::BTreeMap;
use touch_standalone::model::{Camera, FloatId, Hand, Intent, LonLat, Phase, PointerEvent};
use touch_standalone::touch::{motion, Frame, Gestures, Hit, Key, Motion, Probe};

#[derive(Deserialize)]
struct Plan {
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
    down: Option<u32>,
    #[serde(rename = "move")]
    move_: Option<u32>,
    up: Option<u32>,
    cancel: Option<u32>,
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
    riding: Option<Option<FloatId>>,
    held: Option<Option<FloatId>>,
    playing: Option<bool>,
    day: Option<f64>,
    float: Option<FloatAt>,
    /// The camera's centre is within this many degrees of the float.
    camera_near_float: Option<(FloatId, f64)>,
    camera_moved: Option<bool>,
    /// The camera has not moved since the last `expect`.
    camera_same_as_last: Option<bool>,
}
#[derive(Deserialize)]
struct FloatAt {
    id: FloatId,
    lon: f64,
    lat: f64,
}

/// The stand-in state: floats that only drift as the plan says, the one in hand, undo.
struct Sea {
    frame: Frame,
    camera: Camera,
    floats: BTreeMap<FloatId, LonLat>,
    next_id: FloatId,
    held: Option<(FloatId, Hand)>,
    land: Vec<[f64; 3]>,
    day: f64,
    playing: bool,
    undo: Vec<BTreeMap<FloatId, LonLat>>,
    redo: Vec<BTreeMap<FloatId, LonLat>>,
    riding: Option<FloatId>,
    lit: Option<FloatId>,
}

impl Sea {
    fn is_land(&self, p: LonLat) -> bool {
        self.land.iter().any(|c| {
            let d = motion::lon_delta(c[0], p.lon).hypot(c[1] - p.lat);
            d <= c[2]
        })
    }
    fn apply(&mut self, i: &Intent, g: &Gestures) {
        match *i {
            Intent::Put { id, at } => {
                self.undo.push(self.floats.clone());
                self.redo.clear();
                self.floats.insert(id, at);
                self.next_id = id + 1;
                self.held = Some((id, Hand::Sea(at)));
                self.lit = Some(id);
            }
            Intent::Hold { id } => {
                self.held = self.floats.get(&id).map(|p| (id, Hand::Sea(*p)));
                self.lit = Some(id);
            }
            Intent::Carry { id, to } => self.held = Some((id, to)),
            Intent::Release { id, at } => {
                if let Some(p) = at {
                    if self.floats.get(&id) != Some(&p) {
                        self.undo.push(self.floats.clone());
                        self.redo.clear();
                        self.floats.insert(id, p);
                    }
                }
                self.held = None;
            }
            Intent::TakeOut { id } => {
                self.undo.push(self.floats.clone());
                self.redo.clear();
                self.floats.remove(&id);
                self.held = None;
            }
            Intent::Ride { id } => {
                self.riding = Some(id);
                self.lit = Some(id);
            }
            Intent::Leave => self.riding = None,
            Intent::Camera(c) => self.camera = c,
            Intent::Scrub { day } => self.day = day,
            Intent::Play(p) => self.playing = p,
            Intent::Undo => {
                if let Some(s) = self.undo.pop() {
                    self.redo.push(std::mem::replace(&mut self.floats, s));
                }
            }
            Intent::Redo => {
                if let Some(s) = self.redo.pop() {
                    self.undo.push(std::mem::replace(&mut self.floats, s));
                }
            }
            Intent::Ring { .. } | Intent::Press { .. } | Intent::Unpress => {}
        }
        let _ = g;
    }
}

struct SeaProbe<'a> {
    sea: &'a Sea,
    /// The camera as the state mirrored it (a copy: the rig is being moved meanwhile).
    rig: touch_standalone::touch::CameraRig,
}

impl Probe for SeaProbe<'_> {
    fn hit(&self, x: f64, y: f64) -> Hit {
        let f = &self.sea.frame;
        let reach = motion::REACH * f.reading;
        let mut best: Option<(f64, FloatId, LonLat)> = None;
        for (&id, &p) in &self.sea.floats {
            if self.sea.held.map(|h| h.0) == Some(id) {
                continue;
            }
            let (px, py, facing) = self.rig.project(p, f);
            if facing <= 0.0 {
                continue;
            }
            let d = (px - x).hypot(py - y);
            if d <= reach && best.map_or(true, |b| d < b.0) {
                best = Some((d, id, p));
            }
        }
        if let Some((_, id, p)) = best {
            return Hit::Float(id, p);
        }
        // the date's stand-in bounds: a band 44 points high at the foot, the middle half of the width
        if y >= f.height - 60.0 && y <= f.height - 16.0 && x >= f.width * 0.25 && x <= f.width * 0.75 {
            return Hit::Date;
        }
        match self.rig.unproject(x, y, f) {
            None => Hit::Off,
            Some(p) if self.sea.is_land(p) => Hit::Land(p),
            Some(p) => Hit::Sea(p),
        }
    }
    fn float_at(&self, id: FloatId) -> Option<LonLat> {
        self.sea.floats.get(&id).copied()
    }
    fn any_floats(&self) -> bool {
        !self.sea.floats.is_empty()
    }
    fn playing(&self) -> bool {
        self.sea.playing
    }
    fn day(&self) -> f64 {
        self.sea.day
    }
    fn next_float_id(&self) -> FloatId {
        self.sea.next_id
    }
}

fn name(i: &Intent) -> &'static str {
    match i {
        Intent::Ring { .. } => "Ring",
        Intent::Press { .. } => "Press",
        Intent::Unpress => "Unpress",
        Intent::Put { .. } => "Put",
        Intent::Hold { .. } => "Hold",
        Intent::Carry { to: Hand::Sea(_), .. } => "Carry",
        Intent::Carry { to: Hand::Land(_), .. } => "CarryLand",
        Intent::Carry { to: Hand::Off, .. } => "CarryOff",
        Intent::Release { at: Some(_), .. } => "Release",
        Intent::Release { at: None, .. } => "ReleaseBack",
        Intent::TakeOut { .. } => "TakeOut",
        Intent::Ride { .. } => "Ride",
        Intent::Leave => "Leave",
        Intent::Camera(_) => "Camera",
        Intent::Scrub { .. } => "Scrub",
        Intent::Play(true) => "Play",
        Intent::Play(false) => "Pause",
        Intent::Undo => "Undo",
        Intent::Redo => "Redo",
    }
}

fn run(path: &str, verbose: bool) -> Result<usize, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
    let plan: Plan = serde_json::from_str(&text).map_err(|e| format!("{path}: {e}"))?;
    let frame = Frame::simple(plan.size[0], plan.size[1], plan.camera.zoom);
    let motion = Motion { reduce: plan.reduce_motion };
    let mut g = Gestures::new(plan.camera, motion, frame, 0.0);
    let mut sea = Sea {
        frame,
        camera: plan.camera,
        floats: plan.floats.iter().enumerate().map(|(i, p)| (i as FloatId, *p)).collect(),
        next_id: plan.floats.len() as FloatId,
        held: None,
        land: plan.land.clone(),
        day: plan.day,
        playing: plan.playing,
        undo: vec![],
        redo: vec![],
        riding: None,
        lit: None,
    };
    let mut now = 0.0;
    let mut seen: Vec<&'static str> = vec![];
    let mut failures = 0;
    let mut out = vec![];
    let camera0 = plan.camera;
    let mut camera_last = plan.camera;
    let step_dt = 1.0 / 120.0;

    let flush = |out: &mut Vec<Intent>, sea: &mut Sea, g: &Gestures, seen: &mut Vec<&'static str>, t: f64| {
        for i in out.drain(..) {
            if verbose && !matches!(i, Intent::Camera(_)) {
                println!("  {t:7.3}  {i:?}");
            }
            seen.push(name(&i));
            sea.apply(&i, g);
        }
    };

    for (n, step) in plan.steps.iter().enumerate() {
        // ticks up to the step's time, the floats drifting as the plan says
        while now + step_dt <= step.t + 1e-9 {
            now += step_dt;
            if sea.playing {
                for p in sea.floats.values_mut() {
                    p.lon = motion::wrap_lon(p.lon + plan.drift[0] * step_dt);
                    p.lat += plan.drift[1] * step_dt;
                }
            }
            let probe = SeaProbe { sea: &sea, rig: g.camera.clone() };
            g.tick(now, step_dt, &probe, &mut out);
            // the disc follows the zoom, as the view would
            let z = g.camera.camera().zoom;
            let f = Frame::simple(plan.size[0], plan.size[1], z);
            g.set_frame(f);
            sea.frame = f;
            flush(&mut out, &mut sea, &g, &mut seen, now);
        }
        let t = step.t;
        let point = |g: &Gestures, sea: &Sea| -> Result<(f64, f64), String> {
            if let Some(xy) = step.xy {
                return Ok((xy[0], xy[1]));
            }
            if let Some(at) = step.at {
                let (x, y, facing) = g.camera.project(LonLat { lon: at[0], lat: at[1] }, &sea.frame);
                if facing <= 0.0 {
                    return Err(format!("step {n} at t={t}: ({}, {}) is behind the globe", at[0], at[1]));
                }
                return Ok((x, y));
            }
            Err(format!("step {n} at t={t}: a pointer step needs xy or at"))
        };
        let mut pointer = |id: u32, phase: Phase, g: &mut Gestures, sea: &mut Sea| -> Result<(), String> {
            let (x, y) = point(g, sea)?;
            let probe = SeaProbe { sea, rig: g.camera.clone() };
            let mut o = vec![];
            g.event(&PointerEvent { id, phase, x, y, t }, &probe, &mut o);
            if verbose {
                println!("{t:7.3} {phase:?} {id} at ({x:.1}, {y:.1})  → {}", g.describe());
            }
            flush(&mut o, sea, g, &mut seen, t);
            Ok(())
        };
        if let Some(id) = step.down {
            pointer(id, Phase::Down, &mut g, &mut sea)?;
        }
        if let Some(id) = step.move_ {
            pointer(id, Phase::Move, &mut g, &mut sea)?;
        }
        if let Some(id) = step.up {
            pointer(id, Phase::Up, &mut g, &mut sea)?;
        }
        if let Some(id) = step.cancel {
            pointer(id, Phase::Cancel, &mut g, &mut sea)?;
        }
        if step.shake == Some(true) {
            g.shaken(&mut out);
            flush(&mut out, &mut sea, &g, &mut seen, t);
        }
        if let Some(k) = &step.key {
            let key = match k.as_str() {
                "undo" => Key::Undo,
                "redo" => Key::Redo,
                other => return Err(format!("step {n}: unknown key {other}")),
            };
            g.key(key, &mut out);
            flush(&mut out, &mut sea, &g, &mut seen, t);
        }
        if let Some(shot) = &step.shot {
            if verbose {
                println!("{t:7.3} shot {shot}: {} floats, camera {:?}, hand {}", sea.floats.len(), sea.camera, g.describe());
            }
        }
        if let Some(e) = &step.expect {
            let mut fail = |what: String| {
                failures += 1;
                println!("FAIL {path} step {n} (t={t}): {what}");
            };
            if let Some(k) = e.floats {
                if sea.floats.len() != k {
                    fail(format!("{} floats, expected {k}", sea.floats.len()));
                }
            }
            if let Some(s) = &e.state {
                if g.describe() != s {
                    fail(format!("hand is {}, expected {s}", g.describe()));
                }
            }
            for s in &e.saw {
                if !seen.iter().any(|x| x == s) {
                    fail(format!("never saw {s}; saw {:?}", dedup(&seen)));
                }
            }
            for s in &e.not {
                if seen.iter().any(|x| x == s) {
                    fail(format!("saw {s}, which was not to happen; saw {:?}", dedup(&seen)));
                }
            }
            if let Some(c) = e.camera {
                let got = sea.camera;
                if motion::lon_delta(got.lon, c.lon).abs() > 0.01 || (got.lat - c.lat).abs() > 0.01 || (got.zoom - c.zoom).abs() > 0.01 {
                    fail(format!("camera {got:?}, expected {c:?}"));
                }
            }
            if let Some(z) = e.zoom_gt {
                if !(sea.camera.zoom > z) {
                    fail(format!("zoom {} not > {z}", sea.camera.zoom));
                }
            }
            if let Some(z) = e.zoom_lt {
                if !(sea.camera.zoom < z) {
                    fail(format!("zoom {} not < {z}", sea.camera.zoom));
                }
            }
            if let Some(r) = e.riding {
                if g.camera.riding() != r {
                    fail(format!("riding {:?}, expected {r:?}", g.camera.riding()));
                }
            }
            if let Some(h) = e.held {
                if sea.held.map(|x| x.0) != h {
                    fail(format!("held {:?}, expected {h:?}", sea.held.map(|x| x.0)));
                }
            }
            if let Some(p) = e.playing {
                if sea.playing != p {
                    fail(format!("playing {}, expected {p}", sea.playing));
                }
            }
            if let Some(d) = e.day {
                if (sea.day - d).abs() > 0.01 {
                    fail(format!("day {}, expected {d}", sea.day));
                }
            }
            if let Some(f) = &e.float {
                match sea.floats.get(&f.id) {
                    Some(p) if motion::lon_delta(p.lon, f.lon).abs() <= 0.05 && (p.lat - f.lat).abs() <= 0.05 => {}
                    other => fail(format!("float {} at {:?}, expected ({}, {})", f.id, other, f.lon, f.lat)),
                }
            }
            if let Some((id, within)) = e.camera_near_float {
                match sea.floats.get(&id) {
                    Some(p) => {
                        let d = motion::lon_delta(sea.camera.lon, p.lon).hypot(sea.camera.lat - p.lat);
                        if d > within {
                            fail(format!("camera {:?} is {d:.2}° from float {id} at {p:?}, expected within {within}", sea.camera));
                        }
                    }
                    None => fail(format!("no float {id}")),
                }
            }
            if let Some(m) = e.camera_moved {
                let moved = motion::lon_delta(sea.camera.lon, camera0.lon).abs() > 0.01 || (sea.camera.lat - camera0.lat).abs() > 0.01 || (sea.camera.zoom - camera0.zoom).abs() > 0.01;
                if moved != m {
                    fail(format!("camera moved: {moved} ({:?} from {camera0:?}), expected {m}", sea.camera));
                }
            }
            if let Some(same) = e.camera_same_as_last {
                let c = sea.camera;
                let is_same = motion::lon_delta(c.lon, camera_last.lon).abs() < 1e-6 && (c.lat - camera_last.lat).abs() < 1e-6 && (c.zoom - camera_last.zoom).abs() < 1e-6;
                if is_same != same {
                    fail(format!("camera same as last check: {is_same} ({c:?} from {camera_last:?}), expected {same}"));
                }
            }
            camera_last = sea.camera;
            seen.clear();
        }
    }
    Ok(failures)
}

fn dedup(v: &[&'static str]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = vec![];
    for s in v {
        if out.last() != Some(s) {
            out.push(s);
        }
    }
    out
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let verbose = args.iter().any(|a| a == "-v");
    let plans: Vec<&String> = args.iter().filter(|a| *a != "-v").collect();
    if plans.is_empty() {
        eprintln!("usage: touch-replay [-v] plan.json...");
        std::process::exit(2);
    }
    let mut failed = 0;
    for p in plans {
        match run(p, verbose) {
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
