//! The headless check every toy has: a plan of fingers played on Linux (Mesa's software Vulkan) or a
//! Mac, its moments written as PNG, a film of frames, and the toy's sound as WAV. A toy's
//! `src/bin/check.rs` is one line: `fn main() { toy_kit::check::main::<my_toy::MyToy>("my-toy") }`
//! (`check::main_with` takes the arguments given, for a binary that checks several toys).
//!
//! ```text
//! <toy>-check play <plan.json> <out-dir> [--size WxH] [--ppp n] [--stretch] [--tap-check] [--strict-taps] [--no-tap-check]
//! <toy>-check play-all <plans-dir or plan.json>... <out-dir> [-j N] [same flags]
//! <toy>-check clip <plan.json> <out.mp4> [--size 1080x1920] [--ppp n]
//! ```
//!
//! (`cargo run -p <toy> --bin <toy>-check --release -- play …`.) A plan (JSON; times in seconds from
//! the start, places in points from the top left):
//!
//! ```json
//! { "size": [1179, 2556], "ppp": 3, "night": false, "reduce_motion": false, "seconds": 8, "fps": 30,
//!   "events": [
//!     { "t": 1.0, "tap": [200, 400] },
//!     { "t": 1.5, "tap": "whale" },
//!     { "t": 1.8, "tap": { "lonlat": [-30.0, 40.0] } },
//!     { "t": 2.0, "drag": [[100, 500], "whale"], "for": 0.6 },
//!     { "t": 3.0, "pinch": [[150, 400], [250, 400]], "to": 2.0, "for": 0.5 },
//!     { "t": 4.0, "hold": [200, 300], "for": 1.5 },
//!     { "t": 5.0, "down": [10, 10], "id": 7 }, { "t": 5.2, "move": [40, 10], "id": 7 }, { "t": 5.3, "up": [40, 10], "id": 7 },
//!     { "t": 5.6, "tap": [20, 20], "inert": true },
//!     { "t": 6.0, "shake": true },
//!     { "t": 0.2, "open": "build/screw/song.wav" }
//!   ],
//!   "shots": [0.5, { "t": 3.5, "name": "pinched" }],
//!   "film": { "from": 0, "to": 8, "every": 0.1, "fingers": true },
//!   "audio": true,
//!   "strict_taps": false }
//! ```
//!
//! `shots` are written as `<out>/<name or t>.png` (and, with `--stretch`, a contrast-stretched copy
//! `<name>.stretch.png` that shows faint panels and seams); `film` as `<out>/film/0000.png…`, with a
//! soft ring where each finger is down when `fingers` is set (the plan's point events only)
//! (tools/toys/film.sh makes an MP4 and a contact sheet of them); `audio` as `<out>/audio.wav` (48 kHz
//! stereo, the whole plan) and a line of its loudness (integrated LUFS, RMS and peak in dBFS). Each
//! shot prints a line: its time and what the frame cost on the CPU (the fingers' events handed to the toy and
//! its update), and the run ends with its worst frame.
//!
//! `open` hands the toy a WAV file (a path from where the check runs) as if the reader had chosen it
//! from Files (`Toy::opened`; on the device the shell decodes any format iOS plays).
//!
//! **Places.** Wherever a plan gives a point (`tap`, `hold`, `drag`'s ends, `pinch`'s fingers,
//! `down`/`move`/`up`) it may instead name a place, resolved at the moment the event starts: a name
//! the toy gives (`Toy::anchors`), or `{ "lonlat": [lon, lat] }` through the toy's globe
//! (`Toy::project`). A place that doesn't resolve (an unknown name, a toy without a projection, a
//! place behind the globe) is a plan error: the check fails.
//!
//! **Touches must do something.** After the plan is played, every finger event (`tap`, `hold`, `drag`,
//! `pinch`, `down` with its moves and up) is played again without it: a shadow run with the same clock
//! and every other event, compared with the real run 0.4 s after the gesture began (when it lasts
//! longer than that) and 0.4 s after it ended. A touch whose frames match its shadow's did nothing
//! that shows: the check prints `WARN: tap at t=… did nothing` and counts it on the summary line.
//! `"strict_taps": true` in the plan (or `KIT_STRICT_TAPS=1`, or `--strict-taps`) makes it
//! `EXPECT FAILED` and fails the check. An event that is meant to do nothing says `"inert": true`.
//! A control shadow with nothing left out shows whether the toy is deterministic (seeded randomness,
//! time from the plan, the sound pulled as the real run pulls it); one that isn't (GPU atomics
//! racing) is judged within 80 points of the finger, where the touch must change clearly more than
//! the control does (`KIT_TAP_VERBOSE=1` prints every touch's count). It costs a shadow run a touch
//! (a plan plays 1.5–3× longer), so it is opt-in: `KIT_TAP_CHECK=1` or `--tap-check` (CI's toys.yml
//! sets it), and always when taps are strict; `--no-tap-check` turns it off (the renders don't change
//! either way).
//!
//! **play-all** plays many plans in one process: the GPU made once, the plans shared out to N threads
//! (`-j`, default the CPUs), each plan's output in `<out>/<plan name>/` and its lines in
//! `<out>/<plan name>.log`, one line a plan (`ok`/`FAIL`), exit 1 if any failed.
//!
//! **clip** renders the plan's whole length at 1080×1920 (9:16; `--size` to change), every frame at
//! 30 fps, with its sound, and encodes it with `tools/kit/clip.sh` (H.264 high, yuv420p, AAC 48 kHz,
//! faststart: ready to post) into `out.mp4`. The plan's points keep their width (393 points for a
//! phone plan), so the clip shows what the phone shows, a little taller.

use crate::{block_on, Ctx, DirAssets, Gpu, Host, Offscreen, Phase, PointerEvent, Toy};
use image::ImageEncoder;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Plan {
    pub size: Option<[u32; 2]>,
    pub ppp: Option<f64>,
    pub night: bool,
    pub reduce_motion: bool,
    pub seconds: Option<f64>,
    pub fps: Option<f64>,
    pub events: Vec<Event>,
    pub shots: Vec<Shot>,
    pub film: Option<Film>,
    pub audio: bool,
    /// Text handed to the toy's `restore` before the first frame.
    pub restore: Option<String>,
    /// A touch that does nothing fails the check (otherwise it is a warning).
    pub strict_taps: bool,
}

/// One event of a plan. The point fields hold the places given as points; `places` holds every
/// place as written (points, names, `lonlat`), which the player resolves.
#[derive(Deserialize, Default, Clone)]
#[serde(from = "RawEvent")]
pub struct Event {
    pub t: f64,
    pub tap: Option<[f64; 2]>,
    pub drag: Option<[[f64; 2]; 2]>,
    pub pinch: Option<[[f64; 2]; 2]>,
    pub to: Option<f64>,
    pub hold: Option<[f64; 2]>,
    pub dur: Option<f64>,
    pub down: Option<[f64; 2]>,
    pub mv: Option<[f64; 2]>,
    pub up: Option<[f64; 2]>,
    pub id: Option<u64>,
    pub shake: bool,
    /// A sound file (WAV) handed to the toy as if the reader chose it from Files (`Toy::opened`).
    pub open: Option<String>,
    /// The event is meant to change nothing on screen (the touch check leaves it alone).
    pub inert: bool,
    pub places: Places,
}

/// A place in a plan: a point (points from the top left), a name the toy gives (`Toy::anchors`), or a
/// longitude and latitude on the toy's globe (`Toy::project`).
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(untagged)]
pub enum Place {
    At([f64; 2]),
    Named(String),
    LonLat { lonlat: [f64; 2] },
}

/// An event's places as the plan wrote them.
#[derive(Default, Clone, Debug)]
pub struct Places {
    pub tap: Option<Place>,
    pub hold: Option<Place>,
    pub drag: Option<[Place; 2]>,
    pub pinch: Option<[Place; 2]>,
    pub down: Option<Place>,
    pub mv: Option<Place>,
    pub up: Option<Place>,
}

impl Places {
    fn slots(&self) -> [Option<&Place>; 9] {
        fn two(p: &Option<[Place; 2]>, i: usize) -> Option<&Place> {
            p.as_ref().map(|p| &p[i])
        }
        [
            self.tap.as_ref(),
            self.hold.as_ref(),
            two(&self.drag, 0),
            two(&self.drag, 1),
            two(&self.pinch, 0),
            two(&self.pinch, 1),
            self.down.as_ref(),
            self.mv.as_ref(),
            self.up.as_ref(),
        ]
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawEvent {
    t: f64,
    tap: Option<Place>,
    drag: Option<[Place; 2]>,
    pinch: Option<[Place; 2]>,
    to: Option<f64>,
    hold: Option<Place>,
    #[serde(rename = "for")]
    dur: Option<f64>,
    down: Option<Place>,
    #[serde(rename = "move")]
    mv: Option<Place>,
    up: Option<Place>,
    id: Option<u64>,
    shake: bool,
    open: Option<String>,
    inert: bool,
}

impl From<RawEvent> for Event {
    fn from(r: RawEvent) -> Event {
        let pt = |p: &Option<Place>| match p {
            Some(Place::At(a)) => Some(*a),
            _ => None,
        };
        let two = |p: &Option<[Place; 2]>| match p {
            Some([Place::At(a), Place::At(b)]) => Some([*a, *b]),
            _ => None,
        };
        Event {
            t: r.t,
            tap: pt(&r.tap),
            drag: two(&r.drag),
            pinch: two(&r.pinch),
            to: r.to,
            hold: pt(&r.hold),
            dur: r.dur,
            down: pt(&r.down),
            mv: pt(&r.mv),
            up: pt(&r.up),
            id: r.id,
            shake: r.shake,
            open: r.open,
            inert: r.inert,
            places: Places {
                tap: r.tap,
                hold: r.hold,
                drag: r.drag,
                pinch: r.pinch,
                down: r.down,
                mv: r.mv,
                up: r.up,
            },
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]
pub enum Shot {
    At(f64),
    Named { t: f64, name: String },
}

#[derive(Deserialize, Clone)]
pub struct Film {
    #[serde(default)]
    pub from: f64,
    pub to: f64,
    pub every: f64,
    /// Draw a soft ring where each finger is down on the film's frames (not on shots), so a film of
    /// several fingers shows which is where.
    #[serde(default)]
    pub fingers: bool,
}

/// Where each finger is down at `t` (points), from a plan's point events: a finger is down from its
/// `down` (or a tap, hold or drag's start) to its `up`.
pub fn fingers_at(pointers: &[(f64, Option<PointerEvent>)], t: f64) -> Vec<[f64; 2]> {
    let mut down: BTreeMap<u64, [f64; 2]> = BTreeMap::new();
    for (at, e) in pointers {
        if *at > t + 1e-9 {
            break;
        }
        let Some(e) = e else { continue };
        match e.phase {
            Phase::Down | Phase::Move => {
                down.insert(e.id, [e.x, e.y]);
            }
            Phase::Up | Phase::Cancel => {
                down.remove(&e.id);
            }
        }
    }
    down.into_values().collect()
}

/// A soft white ring a fingertip wide (44 pt) at each place, drawn over an RGBA frame.
pub fn draw_fingers(px: &mut [u8], w: u32, h: u32, ppp: f64, at: &[[f64; 2]]) {
    let r = 22.0 * ppp;
    let lw = 2.5 * ppp;
    for p in at {
        let (cx, cy) = (p[0] * ppp, p[1] * ppp);
        let (x0, x1) = (
            ((cx - r - lw).floor().max(0.0)) as u32,
            ((cx + r + lw).ceil().min(w as f64 - 1.0)) as u32,
        );
        let (y0, y1) = (
            ((cy - r - lw).floor().max(0.0)) as u32,
            ((cy + r + lw).ceil().min(h as f64 - 1.0)) as u32,
        );
        for y in y0..=y1 {
            for x in x0..=x1 {
                let d = ((x as f64 + 0.5 - cx).powi(2) + (y as f64 + 0.5 - cy).powi(2)).sqrt();
                // the ring, and a faint fill inside it
                let ring = (1.0 - ((d - r).abs() / lw)).clamp(0.0, 1.0) * 0.85;
                let fill = if d < r { 0.12 } else { 0.0 };
                let a = ring.max(fill);
                if a <= 0.0 {
                    continue;
                }
                let i = ((y * w + x) * 4) as usize;
                for c in 0..3 {
                    px[i + c] = (px[i + c] as f64 * (1.0 - a) + 255.0 * a).round() as u8;
                }
            }
        }
    }
}

/// A plan's fingers as pointer events, in time order (the events whose places are all points; a
/// named place needs the toy, which the player asks as it plays).
pub fn pointers(events: &[Event]) -> Vec<(f64, Option<PointerEvent>)> {
    let mut out: Vec<(f64, Option<PointerEvent>)> = Vec::new();
    let mut next_id = 1000u64;
    let pe = |id, phase, p: [f64; 2], t| {
        (
            t,
            Some(PointerEvent {
                id,
                phase,
                x: p[0],
                y: p[1],
                t,
            }),
        )
    };
    for e in events {
        let t = e.t;
        if let Some(p) = e.tap {
            next_id += 1;
            out.push(pe(next_id, Phase::Down, p, t));
            out.push(pe(next_id, Phase::Up, p, t + 0.08));
        }
        if let Some(p) = e.hold {
            next_id += 1;
            out.push(pe(next_id, Phase::Down, p, t));
            out.push(pe(next_id, Phase::Up, p, t + e.dur.unwrap_or(1.0)));
        }
        if let Some([a, b]) = e.drag {
            next_id += 1;
            let d = e.dur.unwrap_or(0.5);
            let n = (d * 60.0).ceil().max(2.0) as usize;
            out.push(pe(next_id, Phase::Down, a, t));
            for i in 1..=n {
                let k = i as f64 / n as f64;
                out.push(pe(
                    next_id,
                    Phase::Move,
                    [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k],
                    t + d * k,
                ));
            }
            out.push(pe(next_id, Phase::Up, b, t + d + 0.001));
        }
        if let Some([a, b]) = e.pinch {
            let (i1, i2) = (next_id + 1, next_id + 2);
            next_id += 2;
            let d = e.dur.unwrap_or(0.5);
            let s = e.to.unwrap_or(2.0);
            let c = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
            let at = |p: [f64; 2], f: f64| [c[0] + (p[0] - c[0]) * f, c[1] + (p[1] - c[1]) * f];
            out.push(pe(i1, Phase::Down, a, t));
            out.push(pe(i2, Phase::Down, b, t));
            let n = (d * 60.0).ceil().max(2.0) as usize;
            for i in 1..=n {
                let f = 1.0 + (s - 1.0) * i as f64 / n as f64;
                let tt = t + d * i as f64 / n as f64;
                out.push(pe(i1, Phase::Move, at(a, f), tt));
                out.push(pe(i2, Phase::Move, at(b, f), tt));
            }
            out.push(pe(i1, Phase::Up, at(a, s), t + d + 0.001));
            out.push(pe(i2, Phase::Up, at(b, s), t + d + 0.001));
        }
        let id = e.id.unwrap_or(1);
        if let Some(p) = e.down {
            out.push(pe(id, Phase::Down, p, t));
        }
        if let Some(p) = e.mv {
            out.push(pe(id, Phase::Move, p, t));
        }
        if let Some(p) = e.up {
            out.push(pe(id, Phase::Up, p, t));
        }
        if e.shake {
            out.push((t, None));
        }
    }
    out.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    out
}

// The slots of an event's places: tap, hold, drag's two ends, pinch's two fingers, down, move, up.
const TAP: usize = 0;
const HOLD: usize = 1;
const DRAG: usize = 2;
const PINCH: usize = 4;
const DOWN: usize = 6;
const MOVE: usize = 7;
const UP: usize = 8;

/// Where a scheduled pointer is, from its event's places once they are resolved (the same
/// arithmetic as `pointers`, so a plan of points plays exactly as it always did).
#[derive(Clone, Copy, Debug)]
enum Pos {
    Slot(usize),
    /// Between two slots, `k` of the way.
    Lerp(usize, usize, f64),
    /// A pinch's finger (`PINCH` or `PINCH + 1`) moved out from the fingers' centre by `f`.
    Pinch(usize, f64),
}

impl Pos {
    fn at(self, p: &[[f64; 2]; 9]) -> [f64; 2] {
        match self {
            Pos::Slot(s) => p[s],
            Pos::Lerp(a, b, k) => {
                let (a, b) = (p[a], p[b]);
                [a[0] + (b[0] - a[0]) * k, a[1] + (b[1] - a[1]) * k]
            }
            Pos::Pinch(s, f) => {
                let (a, b) = (p[PINCH], p[PINCH + 1]);
                let c = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0];
                let q = p[s];
                [c[0] + (q[0] - c[0]) * f, c[1] + (q[1] - c[1]) * f]
            }
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum What {
    Pointer {
        id: u64,
        phase: Phase,
        pos: Pos,
    },
    Shake,
    /// The event's `open`: a sound file handed to the toy as if the reader chose it.
    Open,
}

/// One thing that happens at `t`, from the plan's event `ev`.
#[derive(Clone, Copy, Debug)]
struct Item {
    t: f64,
    ev: usize,
    what: What,
}

/// The plan's events as a schedule, in time order (as `pointers`, keeping which event each came from).
fn schedule(events: &[Event]) -> Vec<Item> {
    let mut out = Vec::new();
    let mut next_id = 1000u64;
    for (ev, e) in events.iter().enumerate() {
        let s = e.places.slots();
        let t = e.t;
        let mut ptr = |t: f64, id: u64, phase: Phase, pos: Pos| {
            out.push(Item {
                t,
                ev,
                what: What::Pointer { id, phase, pos },
            })
        };
        if s[TAP].is_some() {
            next_id += 1;
            ptr(t, next_id, Phase::Down, Pos::Slot(TAP));
            ptr(t + 0.08, next_id, Phase::Up, Pos::Slot(TAP));
        }
        if s[HOLD].is_some() {
            next_id += 1;
            ptr(t, next_id, Phase::Down, Pos::Slot(HOLD));
            ptr(
                t + e.dur.unwrap_or(1.0),
                next_id,
                Phase::Up,
                Pos::Slot(HOLD),
            );
        }
        if s[DRAG].is_some() {
            next_id += 1;
            let d = e.dur.unwrap_or(0.5);
            let n = (d * 60.0).ceil().max(2.0) as usize;
            ptr(t, next_id, Phase::Down, Pos::Slot(DRAG));
            for i in 1..=n {
                let k = i as f64 / n as f64;
                ptr(
                    t + d * k,
                    next_id,
                    Phase::Move,
                    Pos::Lerp(DRAG, DRAG + 1, k),
                );
            }
            ptr(t + d + 0.001, next_id, Phase::Up, Pos::Slot(DRAG + 1));
        }
        if s[PINCH].is_some() {
            let (i1, i2) = (next_id + 1, next_id + 2);
            next_id += 2;
            let d = e.dur.unwrap_or(0.5);
            let sc = e.to.unwrap_or(2.0);
            ptr(t, i1, Phase::Down, Pos::Slot(PINCH));
            ptr(t, i2, Phase::Down, Pos::Slot(PINCH + 1));
            let n = (d * 60.0).ceil().max(2.0) as usize;
            for i in 1..=n {
                let f = 1.0 + (sc - 1.0) * i as f64 / n as f64;
                let tt = t + d * i as f64 / n as f64;
                ptr(tt, i1, Phase::Move, Pos::Pinch(PINCH, f));
                ptr(tt, i2, Phase::Move, Pos::Pinch(PINCH + 1, f));
            }
            ptr(t + d + 0.001, i1, Phase::Up, Pos::Pinch(PINCH, sc));
            ptr(t + d + 0.001, i2, Phase::Up, Pos::Pinch(PINCH + 1, sc));
        }
        let id = e.id.unwrap_or(1);
        if s[DOWN].is_some() {
            ptr(t, id, Phase::Down, Pos::Slot(DOWN));
        }
        if s[MOVE].is_some() {
            ptr(t, id, Phase::Move, Pos::Slot(MOVE));
        }
        if s[UP].is_some() {
            ptr(t, id, Phase::Up, Pos::Slot(UP));
        }
        if e.shake {
            out.push(Item {
                t,
                ev,
                what: What::Shake,
            });
        }
        if e.open.is_some() {
            out.push(Item {
                t,
                ev,
                what: What::Open,
            });
        }
    }
    out.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
    out
}

/// A place as a point now: a point is itself, a name is looked up in the toy's `anchors`, a
/// longitude and latitude go through the toy's `project`.
pub fn resolve(
    place: &Place,
    anchors: impl FnOnce() -> Vec<(String, [f64; 2])>,
    project: impl FnOnce([f64; 2]) -> Option<[f64; 2]>,
) -> anyhow::Result<[f64; 2]> {
    match place {
        Place::At(p) => Ok(*p),
        Place::Named(n) => {
            let all = anchors();
            match all.iter().find(|(k, _)| k == n) {
                Some((_, p)) => Ok(*p),
                None => {
                    let names: Vec<&str> = all.iter().map(|(k, _)| k.as_str()).collect();
                    Err(anyhow::anyhow!("plan: no place named {n:?} (the toy names {})", if names.is_empty() { "none".to_string() } else { names.join(", ") }))
                }
            }
        }
        Place::LonLat { lonlat } => project(*lonlat).ok_or_else(|| anyhow::anyhow!("plan: {lonlat:?} has no point (the toy has no globe projection, or the place is behind the globe)")),
    }
}

/// A headless GPU (Vulkan on Linux, Metal on a Mac).
pub fn gpu() -> anyhow::Result<Gpu> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL,
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    block_on(Gpu::new(&instance, None))
}

/// The same device again (a run of its own, the GPU shared).
fn share(g: &Gpu) -> Gpu {
    Gpu {
        device: g.device.clone(),
        queue: g.queue.clone(),
        format: g.format,
        adapter_info: g.adapter_info.clone(),
    }
}

/// The toy's data directory: `TOY_ASSETS`, or `toys/build/<toy>/assets` (tools/toys/assets.py).
pub fn assets_dir(toy: &str) -> PathBuf {
    if let Ok(d) = std::env::var("TOY_ASSETS") {
        return d.into();
    }
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../build")
        .join(toy)
        .join("assets")
}

pub fn write_png(path: &Path, w: u32, h: u32, rgba: &[u8]) -> anyhow::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    image::save_buffer(path, rgba, w, h, image::ExtendedColorType::Rgba8)?;
    Ok(())
}

pub fn write_wav(path: &Path, samples: &[f32], channels: u16, rate: u32) -> anyhow::Result<()> {
    let mut b = Vec::with_capacity(44 + samples.len() * 2);
    let data = (samples.len() * 2) as u32;
    b.extend_from_slice(b"RIFF");
    b.extend_from_slice(&(36 + data).to_le_bytes());
    b.extend_from_slice(b"WAVEfmt ");
    b.extend_from_slice(&16u32.to_le_bytes());
    b.extend_from_slice(&1u16.to_le_bytes());
    b.extend_from_slice(&channels.to_le_bytes());
    b.extend_from_slice(&rate.to_le_bytes());
    b.extend_from_slice(&(rate * channels as u32 * 2).to_le_bytes());
    b.extend_from_slice(&(channels * 2).to_le_bytes());
    b.extend_from_slice(&16u16.to_le_bytes());
    b.extend_from_slice(b"data");
    b.extend_from_slice(&data.to_le_bytes());
    for s in samples {
        b.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, b)?;
    Ok(())
}

/// The loudness of interleaved stereo at 48 kHz: integrated LUFS (ITU-R BS.1770-4: K-weighted,
/// 400 ms blocks, gated at -70 LUFS and 10 LU below), RMS and peak in dBFS. Silence is -inf.
pub fn loudness(samples: &[f32], rate: f64) -> (f64, f64, f64) {
    let db = |x: f64| {
        if x > 0.0 {
            20.0 * x.log10()
        } else {
            f64::NEG_INFINITY
        }
    };
    let frames = samples.len() / 2;
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs())) as f64;
    let rms = if frames > 0 {
        (samples.iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
    } else {
        0.0
    };
    // the K-weighting's two biquads (BS.1770's coefficients at 48 kHz)
    let (b1, a1) = (
        [1.53512485958697, -2.69169618940638, 1.19839281085285],
        [-1.69065929318241, 0.73248077421585],
    );
    let (b2, a2) = ([1.0, -2.0, 1.0], [-1.99004745483398, 0.99007225036621]);
    let mut sq = vec![0.0f64; frames];
    for ch in 0..2 {
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        let (mut u1, mut u2, mut z1, mut z2) = (0.0, 0.0, 0.0, 0.0);
        for (i, s) in sq.iter_mut().enumerate() {
            let x = samples[i * 2 + ch] as f64;
            let y = b1[0] * x + b1[1] * x1 + b1[2] * x2 - a1[0] * y1 - a1[1] * y2;
            (x2, x1, y2, y1) = (x1, x, y1, y);
            let z = b2[0] * y + b2[1] * u1 + b2[2] * u2 - a2[0] * z1 - a2[1] * z2;
            (u2, u1, z2, z1) = (u1, y, z1, z);
            *s += z * z;
        }
    }
    let block = (0.4 * rate) as usize;
    let hop = (0.1 * rate) as usize;
    let mut blocks = Vec::new();
    let mut i = 0;
    while block > 0 && i + block <= frames {
        blocks.push(sq[i..i + block].iter().sum::<f64>() / block as f64);
        i += hop;
    }
    let lufs = |z: f64| -0.691 + 10.0 * z.log10();
    let mean = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
    let abs: Vec<f64> = blocks.into_iter().filter(|z| lufs(*z) > -70.0).collect();
    let integrated = if abs.is_empty() {
        f64::NEG_INFINITY
    } else {
        let rel = lufs(mean(&abs)) - 10.0;
        let gated: Vec<f64> = abs.iter().copied().filter(|z| lufs(*z) > rel).collect();
        if gated.is_empty() {
            f64::NEG_INFINITY
        } else {
            lufs(mean(&gated))
        }
    };
    (integrated, db(rms), db(peak))
}

/// A contrast-stretched copy of a frame: each channel's 0.5th to 99.5th percentile of luminance
/// spread over the whole range, so a faint panel or a seam shows.
pub fn stretch(rgba: &[u8]) -> Vec<u8> {
    let mut hist = [0usize; 256];
    for p in rgba.as_chunks::<4>().0 {
        hist[((p[0] as u32 * 54 + p[1] as u32 * 183 + p[2] as u32 * 19) >> 8) as usize] += 1;
    }
    let n = rgba.len() / 4;
    let pct = |q: f64| {
        let mut acc = 0;
        for (v, c) in hist.iter().enumerate() {
            acc += c;
            if acc as f64 >= q * n as f64 {
                return v as f64;
            }
        }
        255.0
    };
    let (lo, hi) = (pct(0.005), pct(0.995));
    let span = (hi - lo).max(1.0);
    rgba.as_chunks::<4>()
        .0
        .iter()
        .flat_map(|p| {
            let s = |c: u8| ((c as f64 - lo) / span * 255.0).round().clamp(0.0, 255.0) as u8;
            [s(p[0]), s(p[1]), s(p[2]), 255]
        })
        .collect()
}

/// How a plan is played (the command line's flags).
#[derive(Clone, Debug)]
pub struct Options {
    pub size: Option<[u32; 2]>,
    pub ppp: Option<f64>,
    /// Play every finger event again without it, and say which did nothing.
    pub tap_check: bool,
    /// A touch that does nothing fails the check.
    pub strict_taps: bool,
    /// A contrast-stretched copy of every shot.
    pub stretch: bool,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            size: None,
            ppp: None,
            tap_check: std::env::var("KIT_TAP_CHECK").is_ok_and(|v| v == "1"),
            strict_taps: std::env::var("KIT_STRICT_TAPS").is_ok_and(|v| v == "1"),
            stretch: false,
        }
    }
}

/// What a played plan found.
#[derive(Default, Debug)]
pub struct Report {
    /// Finger events checked, and the times and kinds of those that did nothing.
    pub touches: usize,
    pub did_nothing: Vec<(f64, String)>,
}

/// A toy being played through a plan's schedule (the real run, or a shadow without some events).
struct Run<T: Toy> {
    host: Host<T>,
    items: Vec<Item>,
    next: usize,
    resolved: Vec<Option<[[f64; 2]; 9]>>,
}

impl<T: Toy> Run<T> {
    fn new(
        gpu: Gpu,
        name: &str,
        plan: &Plan,
        w: u32,
        h: u32,
        ppp: f64,
        items: Vec<Item>,
    ) -> anyhow::Result<Run<T>> {
        let mut ctx = Ctx::new(gpu, Box::new(DirAssets(assets_dir(name))), w, h, ppp);
        ctx.night = plan.night;
        ctx.reduce_motion = plan.reduce_motion;
        let mut host = Host::<T>::new(ctx)?;
        host.toy.settings(&mut host.ctx);
        if let Some(s) = &plan.restore {
            host.toy.restore(&mut host.ctx, s);
        }
        Ok(Run {
            host,
            items,
            next: 0,
            resolved: vec![None; plan.events.len()],
        })
    }

    /// Hands the toy everything due by `t` (places resolved when their event first acts).
    fn dispatch(&mut self, events: &[Event], t: f64) -> anyhow::Result<()> {
        while self.next < self.items.len() && self.items[self.next].t <= t {
            let it = self.items[self.next];
            match it.what {
                What::Pointer { id, phase, pos } => {
                    if self.resolved[it.ev].is_none() {
                        let mut p = [[0.0; 2]; 9];
                        for (s, place) in events[it.ev].places.slots().iter().enumerate() {
                            if let Some(place) = place {
                                p[s] = resolve(
                                    place,
                                    || self.host.anchors(),
                                    |ll| self.host.toy.project(ll),
                                )
                                .map_err(|e| {
                                    anyhow::anyhow!("event at t={}: {e}", events[it.ev].t)
                                })?;
                            }
                        }
                        self.resolved[it.ev] = Some(p);
                    }
                    let [x, y] = pos.at(self.resolved[it.ev].as_ref().unwrap());
                    self.host.pointer(PointerEvent {
                        id,
                        phase,
                        x,
                        y,
                        t: it.t,
                    });
                }
                What::Shake => self.host.toy.shaken(&mut self.host.ctx),
                What::Open => {
                    let path = events[it.ev].open.as_deref().unwrap_or_default();
                    let f = crate::AudioFile::wav(Path::new(path)).map_err(|e| {
                        anyhow::anyhow!("event at t={}: open {path}: {e}", events[it.ev].t)
                    })?;
                    self.host.opened(f);
                }
            }
            self.next += 1;
        }
        Ok(())
    }
}

/// A frame made small for comparing: the mean of every 4×4 pixels' RGB.
fn small(rgba: &[u8], w: u32, h: u32) -> Vec<u8> {
    let (sw, sh) = (w.div_ceil(4) as usize, h.div_ceil(4) as usize);
    let mut sum = vec![[0u32; 4]; sw * sh];
    for y in 0..h as usize {
        for x in 0..w as usize {
            let p = &rgba[(y * w as usize + x) * 4..][..3];
            let s = &mut sum[(y / 4) * sw + x / 4];
            s[0] += p[0] as u32;
            s[1] += p[1] as u32;
            s[2] += p[2] as u32;
            s[3] += 1;
        }
    }
    sum.iter()
        .flat_map(|s| {
            [
                (s[0] / s[3]) as u8,
                (s[1] / s[3]) as u8,
                (s[2] / s[3]) as u8,
            ]
        })
        .collect()
}

/// Which 4×4 blocks differ by 2 levels or more in a channel.
fn changed(a: &[u8], b: &[u8]) -> Vec<bool> {
    a.as_chunks::<3>()
        .0
        .iter()
        .zip(b.as_chunks::<3>().0)
        .map(|(p, q)| p.iter().zip(q.iter()).any(|(x, y)| x.abs_diff(*y) >= 2))
        .collect()
}

/// How near the finger a touch is judged on a toy that isn't deterministic, in points.
const REACH: f64 = 80.0;

/// Fewer changed blocks than this (about 130 pixels) is a touch that did nothing.
const MIN_CHANGED: usize = 8;

/// A finger event the touch check plays without: the events left out (a `down` takes its moves
/// and up with it), and the moments compared.
struct Touch {
    ev: usize,
    kind: &'static str,
    omit: Vec<usize>,
    frames: Vec<usize>,
}

fn touches(events: &[Event], fps: f64, last_frame: usize) -> Vec<Touch> {
    let mut out = Vec::new();
    let mut order: Vec<usize> = (0..events.len()).collect();
    order.sort_by(|a, b| events[*a].t.partial_cmp(&events[*b].t).unwrap());
    for (k, &i) in order.iter().enumerate() {
        let e = &events[i];
        let s = e.places.slots();
        if e.inert {
            continue;
        }
        let d = e.dur.unwrap_or(if s[HOLD].is_some() { 1.0 } else { 0.5 });
        let mut omit = vec![i];
        let (kind, end) = if s[PINCH].is_some() {
            ("pinch", e.t + d + 0.001)
        } else if s[DRAG].is_some() {
            ("drag", e.t + d + 0.001)
        } else if s[HOLD].is_some() {
            ("hold", e.t + d)
        } else if s[TAP].is_some() {
            ("tap", e.t + 0.08)
        } else if s[DOWN].is_some() {
            // the finger's moves and up, until it is lifted
            let id = e.id.unwrap_or(1);
            let mut end = last_frame as f64 / fps;
            if s[UP].is_none() {
                for &j in &order[k + 1..] {
                    let f = &events[j];
                    if f.id.unwrap_or(1) != id {
                        continue;
                    }
                    let fs = f.places.slots();
                    if fs[DOWN].is_some() {
                        break;
                    }
                    if fs[MOVE].is_some() || fs[UP].is_some() {
                        omit.push(j);
                    }
                    if fs[UP].is_some() {
                        end = f.t;
                        break;
                    }
                }
            } else {
                end = e.t;
            }
            ("down", end)
        } else {
            continue;
        };
        let frame = |t: f64| (((t - 1e-9) * fps).ceil().max(0.0) as usize).min(last_frame);
        let mut frames = vec![];
        if e.t + 0.4 < end {
            frames.push(frame(e.t + 0.4));
        }
        frames.push(frame(end + 0.4));
        frames.dedup();
        out.push(Touch {
            ev: i,
            kind,
            omit,
            frames,
        });
    }
    out
}

/// Plays `plan` with toy `T` into `out`.
pub fn play<T: Toy>(
    name: &str,
    plan: &Plan,
    out: &Path,
    size: Option<[u32; 2]>,
    ppp: Option<f64>,
) -> anyhow::Result<()> {
    let opts = Options {
        size,
        ppp,
        ..Options::default()
    };
    play_with::<T>(&gpu()?, name, plan, out, &opts, &mut std::io::stdout()).map(|_| ())
}

/// Plays `plan` with toy `T` on `gpu` into `out`, its lines to `log`.
pub fn play_with<T: Toy>(
    gpu: &Gpu,
    name: &str,
    plan: &Plan,
    out: &Path,
    opts: &Options,
    log: &mut dyn Write,
) -> anyhow::Result<Report> {
    std::fs::create_dir_all(out)?;
    let [w, h] = opts.size.or(plan.size).unwrap_or([1179, 2556]);
    let ppp = opts.ppp.or(plan.ppp).unwrap_or(3.0);
    let target = Offscreen::new(gpu, w, h);
    let mut run = Run::<T>::new(share(gpu), name, plan, w, h, ppp, schedule(&plan.events))?;
    let fps = plan.fps.unwrap_or(30.0);
    let mut shots: Vec<(f64, String)> = plan
        .shots
        .iter()
        .map(|s| match s {
            Shot::At(t) => (*t, format!("{t:06.2}")),
            Shot::Named { t, name } => (*t, name.clone()),
        })
        .collect();
    shots.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let last_shot = shots.last().map(|s| s.0).unwrap_or(0.0);
    let film_end = plan.film.as_ref().map(|f| f.to).unwrap_or(0.0);
    let seconds = plan
        .seconds
        .unwrap_or(last_shot.max(film_end))
        .max(last_shot)
        .max(film_end);
    let frames = (seconds * fps).ceil() as usize;
    let checks = if opts.tap_check || opts.strict_taps || plan.strict_taps {
        touches(&plan.events, fps, frames)
    } else {
        Vec::new()
    };
    let mut seen: BTreeMap<usize, Vec<u8>> = checks
        .iter()
        .flat_map(|c| c.frames.iter().map(|f| (*f, Vec::new())))
        .collect();
    let mut si = 0;
    let mut film_next = plan.film.as_ref().map(|f| f.from);
    let mut film_i = 0;
    let finger_track = pointers(&plan.events);
    let mut worst = (0.0f64, 0.0f64);
    let rate = 48_000.0;
    let mut sound: Vec<f32> = Vec::new();
    for f in 0..=frames {
        let t = f as f64 / fps;
        // a frame's CPU cost: the fingers' events handed to the toy (where a push does its work) and the update
        let start = std::time::Instant::now();
        run.dispatch(&plan.events, t)?;
        let host = &mut run.host;
        host.update(t);
        let cost = start.elapsed().as_secs_f64() * 1000.0;
        if cost > worst.0 {
            worst = (cost, t);
        }
        let _ = host.take_haptic();
        if plan.audio && f > 0 {
            let n = ((t * rate) as usize).saturating_sub(sound.len() / 2);
            let mut buf = vec![0.0f32; n * 2];
            Host::<T>::audio(host.synth.as_ref(), &mut buf, 2, rate);
            sound.extend_from_slice(&buf);
        }
        let shot_now = si < shots.len() && shots[si].0 <= t + 1e-9;
        let film_now = matches!(film_next, Some(n) if n <= t + 1e-9 && t <= film_end + 1e-9);
        let check_now = seen.contains_key(&f);
        if shot_now || film_now || check_now {
            host.draw(&target.view);
            let px = target.read(&host.ctx.gpu);
            if check_now {
                seen.insert(f, small(&px, w, h));
            }
            while si < shots.len() && shots[si].0 <= t + 1e-9 {
                let p = out.join(format!("{}.png", shots[si].1));
                write_png(&p, w, h, &px)?;
                if opts.stretch {
                    write_png(
                        &out.join(format!("{}.stretch.png", shots[si].1)),
                        w,
                        h,
                        &stretch(&px),
                    )?;
                }
                writeln!(
                    log,
                    "shot {:>7.2}s {} (update {cost:.2} ms)",
                    t,
                    p.display()
                )?;
                // the shell's text showing then (the info line, the rules), which the shell sets in
                // the system's text and this frame lacks: tools/kit/shell_text.py draws it on the shot
                let title = host.toy.title();
                let (line, rules, origins) =
                    (host.info.line(), host.info.rules(), host.info.origins());
                if line.is_some() || rules.is_some() || origins.is_some() || title.is_some() {
                    let s = |c: Option<&std::ffi::CStr>| {
                        serde_json::Value::from(c.map(|c| c.to_string_lossy().into_owned()))
                    };
                    let j = serde_json::json!({ "size": [w, h], "ppp": ppp, "line": s(line), "rules": s(rules),
                        "origins": s(origins), "top": host.card_at_top(), "title": title });
                    std::fs::write(
                        out.join(format!("{}.shell.json", shots[si].1)),
                        j.to_string(),
                    )?;
                }
                si += 1;
            }
            if film_now {
                let film = plan.film.as_ref().unwrap();
                if film.fingers {
                    let mut px = px.clone();
                    draw_fingers(&mut px, w, h, ppp, &fingers_at(&finger_track, t));
                    write_png(
                        &out.join("film").join(format!("{film_i:04}.png")),
                        w,
                        h,
                        &px,
                    )?;
                } else {
                    write_png(
                        &out.join("film").join(format!("{film_i:04}.png")),
                        w,
                        h,
                        &px,
                    )?;
                }
                film_i += 1;
                let every = plan.film.as_ref().unwrap().every;
                film_next = film_next.map(|n| n + every);
            }
        }
    }
    if plan.audio {
        let p = out.join("audio.wav");
        write_wav(&p, &sound, 2, rate as u32)?;
        let peak = sound.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        writeln!(
            log,
            "audio {} ({:.1} s, peak {peak:.2})",
            p.display(),
            sound.len() as f64 / 2.0 / rate
        )?;
        let (lufs, rms, peak) = loudness(&sound, rate);
        writeln!(
            log,
            "loudness {lufs:.1} LUFS integrated, RMS {rms:.1} dBFS, peak {peak:.1} dBFS"
        )?;
    }
    if film_i > 0 {
        writeln!(
            log,
            "film {} frames in {}",
            film_i,
            out.join("film").display()
        )?;
    }
    writeln!(
        log,
        "worst frame {:.2} ms at {:.2}s (CPU: the fingers' events and the update)",
        worst.0, worst.1
    )?;
    let resolved = std::mem::take(&mut run.resolved);
    drop(run);
    let mut report = Report::default();
    if !checks.is_empty() {
        let shadow_target = target;
        let (sw, sh) = (w.div_ceil(4) as usize, h.div_ceil(4) as usize);
        // a shadow run without the events `omit`: at each frame of `upto`, which 4×4 blocks differ
        // from the real run
        let shadow =
            |omit: &[usize], upto: &[usize]| -> anyhow::Result<BTreeMap<usize, Vec<bool>>> {
                let items = schedule(&plan.events)
                    .into_iter()
                    .filter(|i| !omit.contains(&i.ev))
                    .collect();
                let mut run = Run::<T>::new(share(gpu), name, plan, w, h, ppp, items)?;
                let last = *upto.iter().max().unwrap();
                let mut out = BTreeMap::new();
                let mut heard = 0usize;
                for f in 0..=last {
                    let t = f as f64 / fps;
                    run.dispatch(&plan.events, t)?;
                    run.host.update(t);
                    let _ = run.host.take_haptic();
                    // the sound is pulled as the real run pulled it (a synth may share state with the toy)
                    if plan.audio && f > 0 {
                        let n = ((t * rate) as usize).saturating_sub(heard);
                        let mut buf = vec![0.0f32; n * 2];
                        Host::<T>::audio(run.host.synth.as_ref(), &mut buf, 2, rate);
                        heard += n;
                    }
                    if upto.contains(&f) {
                        run.host.draw(&shadow_target.view);
                        let px = small(&shadow_target.read(&run.host.ctx.gpu), w, h);
                        out.insert(f, changed(&px, &seen[&f]));
                    }
                }
                Ok(out)
            };
        // the control: a shadow with nothing left out. A deterministic toy matches it exactly; one
        // that isn't (GPU atomics racing, say) is judged near the finger, against its own noise.
        let all: Vec<usize> = seen.keys().copied().collect();
        let control = shadow(&[], &all)?;
        let noisy = control.values().any(|m| m.iter().any(|c| *c));
        if noisy {
            let most = control
                .values()
                .map(|m| m.iter().filter(|c| **c).count())
                .max()
                .unwrap_or(0);
            writeln!(log, "touch check: the toy isn't deterministic (a run with every touch differs from itself in {most} of {} blocks); touches judged within {REACH} points of the finger, against that noise", sw * sh)?;
        }
        let strict = opts.strict_taps || plan.strict_taps;
        for c in &checks {
            let e = &plan.events[c.ev];
            let diff = shadow(&c.omit, &c.frames)?;
            // the blocks near the finger (every place the gesture touched, and REACH points round)
            let mut near = vec![!noisy; sw * sh];
            if noisy {
                let mut pts: Vec<[f64; 2]> = Vec::new();
                for &i in &c.omit {
                    if let Some(r) = resolved[i] {
                        for (s, p) in plan.events[i].places.slots().iter().enumerate() {
                            if p.is_some() {
                                pts.push(r[s]);
                            }
                        }
                    }
                }
                let lo = pts
                    .iter()
                    .fold([f64::MAX; 2], |m, p| [m[0].min(p[0]), m[1].min(p[1])]);
                let hi = pts
                    .iter()
                    .fold([f64::MIN; 2], |m, p| [m[0].max(p[0]), m[1].max(p[1])]);
                let b = |v: f64| (v * ppp / 4.0).floor().max(0.0) as usize;
                if !pts.is_empty() {
                    for y in b(lo[1] - REACH)..=b(hi[1] + REACH).min(sh - 1) {
                        for x in b(lo[0] - REACH)..=b(hi[0] + REACH).min(sw - 1) {
                            near[y * sw + x] = true;
                        }
                    }
                }
            }
            let count = |m: &Vec<bool>| m.iter().zip(&near).filter(|(c, n)| **c && **n).count();
            let d = diff.values().map(count).max().unwrap_or(0);
            let noise = c
                .frames
                .iter()
                .map(|f| count(&control[f]))
                .max()
                .unwrap_or(0);
            report.touches += 1;
            let at = c
                .frames
                .iter()
                .map(|f| format!("{:.2}", *f as f64 / fps))
                .collect::<Vec<_>>()
                .join(", ");
            let nothing = d < MIN_CHANGED
                || (noise > 0 && (d as f64) < 1.5 * noise as f64 + MIN_CHANGED as f64);
            if nothing {
                let against = if noise > 0 {
                    format!(", against its noise of {noise}")
                } else {
                    String::new()
                };
                let verb = if noise > 0 {
                    "may have done nothing"
                } else {
                    "did nothing"
                };
                let line = format!(
                    "{} at t={:.2} {verb} ({d} blocks of 4x4 px changed at {at} s{against})",
                    c.kind, e.t
                );
                report.did_nothing.push((e.t, c.kind.to_string()));
                if strict {
                    writeln!(log, "EXPECT FAILED: {line}")?;
                } else {
                    writeln!(log, "WARN: {line}")?;
                }
            } else if std::env::var("KIT_TAP_VERBOSE").is_ok_and(|v| v == "1") {
                writeln!(
                    log,
                    "touch: {} at t={:.2} changed {d} blocks (noise {noise})",
                    c.kind, e.t
                )?;
            }
        }
        writeln!(
            log,
            "touches: {} checked, {} did nothing",
            report.touches,
            report.did_nothing.len()
        )?;
        if strict && !report.did_nothing.is_empty() {
            anyhow::bail!(
                "EXPECT FAILED: {} of {} touches did nothing",
                report.did_nothing.len(),
                report.touches
            );
        }
    }
    Ok(report)
}

/// Every plan named (a plan, or a directory of them), in order.
fn plans_in(args: &[&String]) -> anyhow::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for a in args {
        let p = Path::new(a.as_str());
        if p.is_dir() {
            let mut v: Vec<PathBuf> = std::fs::read_dir(p)?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "json"))
                .collect();
            v.sort();
            out.extend(v);
        } else {
            out.push(p.to_path_buf());
        }
    }
    Ok(out)
}

fn read_plan(p: &Path) -> anyhow::Result<Plan> {
    let s = std::fs::read_to_string(p).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))?;
    serde_json::from_str::<Plan>(&s).map_err(|e| anyhow::anyhow!("{}: {e}", p.display()))
}

/// Plays many plans in one process: the GPU made once, the plans on `jobs` threads. Says whether
/// every plan passed.
pub fn play_all<T: Toy>(
    name: &str,
    plans: &[PathBuf],
    out: &Path,
    jobs: usize,
    opts: &Options,
) -> anyhow::Result<bool> {
    std::fs::create_dir_all(out)?;
    let gpu = gpu()?;
    let next = std::sync::atomic::AtomicUsize::new(0);
    let failed = std::sync::atomic::AtomicUsize::new(0);
    let mut names: Vec<String> = Vec::new();
    for p in plans {
        let stem = p
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut n = stem.clone();
        let mut k = 2;
        while names.contains(&n) {
            n = format!("{stem}-{k}");
            k += 1;
        }
        names.push(n);
    }
    let start = std::time::Instant::now();
    std::thread::scope(|s| {
        for _ in 0..jobs.max(1).min(plans.len().max(1)) {
            s.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                if i >= plans.len() {
                    break;
                }
                let t0 = std::time::Instant::now();
                let dir = out.join(&names[i]);
                let mut log: Vec<u8> = Vec::new();
                let r = read_plan(&plans[i])
                    .and_then(|plan| play_with::<T>(&gpu, name, &plan, &dir, opts, &mut log));
                if let Err(e) = &r {
                    let _ = writeln!(log, "check: {e:#}");
                }
                let _ = std::fs::write(out.join(format!("{}.log", names[i])), &log);
                let secs = t0.elapsed().as_secs_f64();
                match r {
                    Ok(rep) => {
                        let touches = if rep.touches > 0 {
                            format!(
                                "; {} touches, {} did nothing",
                                rep.touches,
                                rep.did_nothing.len()
                            )
                        } else {
                            String::new()
                        };
                        println!("ok   {} ({secs:.1} s{touches})", plans[i].display());
                    }
                    Err(e) => {
                        failed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                        println!("FAIL {} ({secs:.1} s): {e:#}", plans[i].display());
                    }
                }
            });
        }
    });
    let failed = failed.into_inner();
    println!(
        "{} plans, {} failed, in {:.1} s ({} threads)",
        plans.len(),
        failed,
        start.elapsed().as_secs_f64(),
        jobs
    );
    Ok(failed == 0)
}

/// The clip encoder: `KIT_CLIP_SH`, or this repository's `tools/kit/clip.sh`.
fn clip_sh() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("KIT_CLIP_SH") {
        return Some(p.into());
    }
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tools/kit/clip.sh");
    p.exists().then_some(p)
}

/// Renders `plan` as a clip: every frame at 30 fps, 1080×1920 unless `size` says, with its sound,
/// encoded by tools/kit/clip.sh into `out` (an MP4).
pub fn clip<T: Toy>(
    name: &str,
    plan: &Plan,
    out: &Path,
    size: Option<[u32; 2]>,
    ppp: Option<f64>,
) -> anyhow::Result<()> {
    let [w, h] = size.unwrap_or([1080, 1920]);
    // the plan's points keep their width
    let plan_w = match plan.size {
        Some([pw, _]) => pw as f64 / plan.ppp.unwrap_or(3.0),
        None => 1179.0 / plan.ppp.unwrap_or(3.0),
    };
    let ppp = ppp.unwrap_or(w as f64 / plan_w);
    let fps = 30.0;
    let frames_dir = out.with_extension("frames");
    let _ = std::fs::remove_dir_all(&frames_dir);
    std::fs::create_dir_all(&frames_dir)?;
    let gpu = gpu()?;
    let target = Offscreen::new(&gpu, w, h);
    let mut run = Run::<T>::new(share(&gpu), name, plan, w, h, ppp, schedule(&plan.events))?;
    let last_shot = plan
        .shots
        .iter()
        .map(|s| match s {
            Shot::At(t) => *t,
            Shot::Named { t, .. } => *t,
        })
        .fold(0.0, f64::max);
    let film_end = plan.film.as_ref().map(|f| f.to).unwrap_or(0.0);
    let seconds = plan
        .seconds
        .unwrap_or(last_shot.max(film_end))
        .max(last_shot)
        .max(film_end);
    let frames = (seconds * fps).round() as usize;
    let rate = 48_000.0;
    let mut sound: Vec<f32> = Vec::new();
    for f in 0..frames {
        let t = f as f64 / fps;
        run.dispatch(&plan.events, t)?;
        run.host.update(t);
        let _ = run.host.take_haptic();
        // the sound up to the frame's end, so the clip's sound is as long as its frames
        let n = (((t + 1.0 / fps) * rate) as usize).saturating_sub(sound.len() / 2);
        let mut buf = vec![0.0f32; n * 2];
        Host::<T>::audio(run.host.synth.as_ref(), &mut buf, 2, rate);
        sound.extend_from_slice(&buf);
        run.host.draw(&target.view);
        let px = target.read(&run.host.ctx.gpu);
        let img =
            image::RgbaImage::from_raw(w, h, px).ok_or_else(|| anyhow::anyhow!("frame size"))?;
        // fast compression: the frames are only on their way to the encoder
        let file = std::io::BufWriter::new(std::fs::File::create(
            frames_dir.join(format!("{f:05}.png")),
        )?);
        image::codecs::png::PngEncoder::new_with_quality(
            file,
            image::codecs::png::CompressionType::Fast,
            image::codecs::png::FilterType::Sub,
        )
        .write_image(&img, w, h, image::ExtendedColorType::Rgba8)?;
        if f % 30 == 0 {
            eprint!("\rclip {:.0}/{seconds:.0} s", t);
        }
    }
    eprintln!();
    write_wav(&frames_dir.join("audio.wav"), &sound, 2, rate as u32)?;
    let (lufs, rms, peak) = loudness(&sound, rate);
    println!("loudness {lufs:.1} LUFS integrated, RMS {rms:.1} dBFS, peak {peak:.1} dBFS");
    let sh = clip_sh().ok_or_else(|| {
        anyhow::anyhow!(
            "no clip.sh (set KIT_CLIP_SH); the frames and sound are in {}",
            frames_dir.display()
        )
    })?;
    let st = std::process::Command::new("bash")
        .arg(&sh)
        .arg(&frames_dir)
        .arg(out)
        .arg(format!("{fps}"))
        .status()?;
    anyhow::ensure!(
        st.success(),
        "{} failed; the frames and sound are in {}",
        sh.display(),
        frames_dir.display()
    );
    std::fs::remove_dir_all(&frames_dir)?;
    println!(
        "clip {} ({w}x{h}, {frames} frames at {fps} fps, {ppp:.3} px a point)",
        out.display()
    );
    Ok(())
}

/// The command line of a toy's `check` binary.
pub fn main<T: Toy>(name: &str) {
    let args: Vec<String> = std::env::args().skip(1).collect();
    main_with::<T>(name, &args)
}

/// The same, with the arguments given (without the program's name): a binary that checks several
/// toys picks one first and passes the rest on. (From busquets' copy of the kit.)
pub fn main_with<T: Toy>(name: &str, given: &[String]) {
    let args: Vec<String> = std::iter::once(String::new())
        .chain(given.iter().cloned())
        .collect();
    let flag = |k: &str| {
        args.iter()
            .position(|a| a == k)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    let has = |k: &str| args.iter().any(|a| a == k);
    let size = flag("--size").and_then(|s| {
        let (a, b) = s.split_once('x')?;
        Some([a.parse().ok()?, b.parse().ok()?])
    });
    let ppp = flag("--ppp").and_then(|s| s.parse().ok());
    let jobs = flag("-j").and_then(|s| s.parse().ok()).unwrap_or_else(|| {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    });
    let switches = [
        "--stretch",
        "--strict-taps",
        "--tap-check",
        "--no-tap-check",
    ];
    let pos: Vec<&String> = {
        let mut v = Vec::new();
        let mut skip = false;
        for a in args.iter().skip(1) {
            if skip {
                skip = false;
                continue;
            }
            if switches.contains(&a.as_str()) {
                continue;
            }
            if a.starts_with("--") || a == "-j" {
                skip = true;
                continue;
            }
            v.push(a);
        }
        v
    };
    let mut opts = Options {
        size,
        ppp,
        ..Options::default()
    };
    opts.stretch = has("--stretch");
    opts.strict_taps |= has("--strict-taps");
    opts.tap_check = (opts.tap_check || has("--tap-check")) && !has("--no-tap-check");
    let usage = "usage: check play <plan.json> <out-dir> [--size WxH] [--ppp n] [--stretch] [--tap-check] [--strict-taps] [--no-tap-check]\n       check play-all <plans-dir or plan.json>... <out-dir> [-j N] [same flags]\n       check clip <plan.json> <out.mp4> [--size WxH] [--ppp n]";
    let r = match pos.as_slice() {
        [cmd, plan, out] if cmd.as_str() == "play" => {
            read_plan(Path::new(plan.as_str())).and_then(|p| {
                gpu()
                    .and_then(|g| {
                        play_with::<T>(
                            &g,
                            name,
                            &p,
                            Path::new(out.as_str()),
                            &opts,
                            &mut std::io::stdout(),
                        )
                    })
                    .map(|_| ())
            })
        }
        [cmd, rest @ .., out] if cmd.as_str() == "play-all" && !rest.is_empty() => plans_in(rest)
            .and_then(|plans| {
                if play_all::<T>(name, &plans, Path::new(out.as_str()), jobs, &opts)? {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("some plans failed"))
                }
            }),
        [cmd, plan, out] if cmd.as_str() == "clip" => read_plan(Path::new(plan.as_str()))
            .and_then(|p| clip::<T>(name, &p, Path::new(out.as_str()), size, ppp)),
        _ => Err(anyhow::anyhow!("{usage}")),
    };
    if let Err(e) = r {
        eprintln!("check: {e:#}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(json: &str) -> Plan {
        serde_json::from_str(json).unwrap()
    }

    /// A plan of points schedules exactly the pointers it always did.
    #[test]
    fn schedule_matches_pointers() {
        let p = plan(
            r#"{"events": [
                {"t": 1.0, "tap": [200, 400]},
                {"t": 2.0, "drag": [[100, 500], [300, 450]], "for": 0.6},
                {"t": 3.0, "pinch": [[150, 400], [250.3, 401]], "to": 2.7, "for": 0.5},
                {"t": 4.0, "hold": [200, 300], "for": 1.5},
                {"t": 5.0, "down": [10, 10], "id": 7}, {"t": 5.2, "move": [40, 10], "id": 7}, {"t": 5.3, "up": [40, 10], "id": 7},
                {"t": 6.0, "shake": true}]}"#,
        );
        let old = pointers(&p.events);
        let new = schedule(&p.events);
        assert_eq!(old.len(), new.len());
        let mut resolved = vec![[[0.0; 2]; 9]; p.events.len()];
        for (i, e) in p.events.iter().enumerate() {
            for (s, place) in e.places.slots().iter().enumerate() {
                if let Some(pl) = place {
                    resolved[i][s] = resolve(pl, Vec::new, |_| None).unwrap();
                }
            }
        }
        for (o, n) in old.iter().zip(&new) {
            assert_eq!(o.0.to_bits(), n.t.to_bits());
            match (o.1, n.what) {
                (Some(pe), What::Pointer { id, phase, pos }) => {
                    let [x, y] = pos.at(&resolved[n.ev]);
                    assert_eq!(
                        (
                            pe.id,
                            pe.phase,
                            pe.x.to_bits(),
                            pe.y.to_bits(),
                            pe.t.to_bits()
                        ),
                        (id, phase, x.to_bits(), y.to_bits(), n.t.to_bits())
                    );
                }
                (None, What::Shake) => {}
                _ => panic!("different kinds"),
            }
        }
    }

    #[test]
    fn places_by_name_and_lonlat() {
        let p = plan(
            r#"{"events": [{"t": 1, "tap": "whale"}, {"t": 2, "drag": [[1, 2], {"lonlat": [10, 20]}]}, {"t": 3, "hold": "nowhere", "inert": true}]}"#,
        );
        let anchors = || vec![("whale".to_string(), [50.0, 60.0])];
        let e = &p.events;
        assert_eq!(e[0].tap, None);
        assert_eq!(
            resolve(e[0].places.tap.as_ref().unwrap(), anchors, |_| None).unwrap(),
            [50.0, 60.0]
        );
        let d = e[1].places.drag.as_ref().unwrap();
        assert_eq!(resolve(&d[0], anchors, |_| None).unwrap(), [1.0, 2.0]);
        assert_eq!(
            resolve(&d[1], anchors, |ll| Some([ll[0] * 2.0, ll[1]])).unwrap(),
            [20.0, 20.0]
        );
        assert!(resolve(&d[1], anchors, |_| None).is_err());
        assert!(resolve(e[2].places.hold.as_ref().unwrap(), anchors, |_| None).is_err());
        assert!(e[2].inert);
    }

    #[test]
    fn touches_and_their_moments() {
        let p = plan(
            r#"{"events": [{"t": 1, "tap": [1, 1]}, {"t": 2, "hold": [1, 1], "for": 1.5}, {"t": 4, "tap": [1, 1], "inert": true},
            {"t": 5, "down": [1, 1], "id": 3}, {"t": 5.5, "move": [2, 1], "id": 3}, {"t": 6, "up": [2, 1], "id": 3}, {"t": 7, "shake": true}]}"#,
        );
        let t = touches(&p.events, 30.0, 300);
        let kinds: Vec<&str> = t.iter().map(|t| t.kind).collect();
        assert_eq!(kinds, ["tap", "hold", "down"]);
        assert_eq!(t[0].frames, [45]); // the first frame at or after 1.08 + 0.4 s
        assert_eq!(t[1].frames, [72, 117]); // 2.4 s and 3.9 s
        assert_eq!(t[2].omit, [3, 4, 5]);
        assert_eq!(t[2].frames, [162, 192]);
    }

    #[test]
    fn loudness_of_a_tone() {
        // a full-scale 1 kHz sine is -3.0 LUFS in one channel (BS.1770's reference), 0 in both
        let s: Vec<f32> = (0..48_000 * 3)
            .flat_map(|i| {
                let v = (i as f64 * 1000.0 * std::f64::consts::TAU / 48_000.0).sin() as f32;
                [v, v]
            })
            .collect();
        let (lufs, rms, peak) = loudness(&s, 48_000.0);
        assert!(lufs.abs() < 0.2, "lufs {lufs}");
        assert!((rms + 3.01).abs() < 0.05, "rms {rms}");
        assert!(peak.abs() < 0.01);
    }
}
