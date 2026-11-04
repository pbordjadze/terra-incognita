//! The info mark (docs/kit.md › First after the reset, 2): a tiny mark in a corner; a tap on it shows
//! for a few seconds what the app is, a ghost finger doing the app's one gesture over the live scene
//! (real touches: the scene answers as it would a finger) and at most one line; a tap anywhere closes
//! it. Each toy supplies its own [`Demo`] (`Toy::demo`; None, the default, shows no mark). The kit
//! draws the mark and the ghost; the line is the shell's, in the system's text (VoiceOver reads it, it
//! follows the reader's text size; `host.info.line()`). Under Reduce Motion the ghost rests where the
//! gesture starts, still, with the line, and no touch is played. A plan taps it as `"tap": "info"`.

use crate::paint::Painter;
use crate::{Gpu, Phase, PointerEvent};

/// One finger of the gesture: down at `path[0]` at `start` seconds, along the path (points from the
/// top left, eased) for `seconds`, then up. A tap is one point and a short time.
#[derive(Clone, Debug)]
pub struct Stroke {
    pub start: f64,
    pub seconds: f64,
    pub path: Vec<[f64; 2]>,
}

impl Stroke {
    pub fn tap(start: f64, at: [f64; 2]) -> Stroke {
        Stroke { start, seconds: 0.12, path: vec![at] }
    }
    pub fn drag(start: f64, seconds: f64, from: [f64; 2], to: [f64; 2]) -> Stroke {
        Stroke { start, seconds, path: vec![from, to] }
    }
    pub fn hold(start: f64, seconds: f64, at: [f64; 2]) -> Stroke {
        Stroke { start, seconds, path: vec![at] }
    }
    /// Where the finger is a share `u` (0..1) of the way.
    pub fn at(&self, u: f64) -> [f64; 2] {
        let n = self.path.len();
        if n == 1 {
            return self.path[0];
        }
        let u = ease(u.clamp(0.0, 1.0)) * (n - 1) as f64;
        let i = (u.floor() as usize).min(n - 2);
        let f = u - i as f64;
        let (a, b) = (self.path[i], self.path[i + 1]);
        [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f]
    }
    fn end(&self) -> f64 {
        self.start + self.seconds
    }
}

fn ease(t: f64) -> f64 {
    t * t * (3.0 - 2.0 * t)
}

/// What a toy shows when its mark is tapped: the gesture's fingers and the line (one, short).
#[derive(Clone, Debug)]
pub struct Demo {
    pub line: String,
    pub strokes: Vec<Stroke>,
}

impl Demo {
    pub fn new(line: &str, strokes: Vec<Stroke>) -> Demo {
        Demo { line: line.into(), strokes }
    }
    /// Two fingers about `c`, `r` points apart each side, turned by `turn` radians (clockwise on the
    /// screen) and spread by `spread` (2: twice as far apart), over `seconds` from `start`.
    pub fn two_fingers(start: f64, seconds: f64, c: [f64; 2], r: f64, turn: f64, spread: f64) -> Vec<Stroke> {
        let path = |sign: f64| {
            (0..=12)
                .map(|i| {
                    let u = i as f64 / 12.0;
                    let a = std::f64::consts::PI + turn * u + if sign > 0.0 { 0.0 } else { std::f64::consts::PI };
                    let d = r * (1.0 + (spread - 1.0) * u);
                    [c[0] + a.cos() * d, c[1] + a.sin() * d]
                })
                .collect()
        };
        // (each finger's path already eased by its points; the stroke eases it again only lightly)
        vec![Stroke { start, seconds, path: path(1.0) }, Stroke { start, seconds, path: path(-1.0) }]
    }
    fn end(&self) -> f64 {
        self.strokes.iter().map(Stroke::end).fold(0.0, f64::max)
    }
}

/// How long the demo lingers after its last stroke before it closes by itself.
const LINGER: f64 = 2.2;
/// The ghost's ids, far from any the shell gives.
const GHOST_ID: u64 = 1 << 48;

/// The mark and its demo, kept by the `Host`.
#[derive(Default)]
pub struct Info {
    painter: Option<Painter>,
    /// The demo playing, and when it opened (the host's seconds).
    open: Option<(Demo, f64)>,
    /// Fingers the mark (or the closing tap) took: never the toy's.
    taken: Vec<u64>,
    /// The ghost's fingers down now (stroke index).
    ghosts: Vec<usize>,
    /// Strokes already played (each once).
    played: Vec<usize>,
    /// A tap closed it: the ghost's fingers go up on the next frame.
    close_pending: bool,
    now: f64,
    line: Option<std::ffi::CString>,
}

/// What the host should do with a finger.
pub enum Route {
    Toy,
    Taken,
    /// The mark was tapped: open the demo.
    Open,
}

impl Info {
    /// Where the mark is, in points: the bottom right corner, clear of the home indicator.
    pub fn spot(points: [f64; 2]) -> [f64; 2] {
        [points[0] - 26.0, points[1] - 34.0]
    }
    /// A fingertip's reach for it, points.
    const REACH: f64 = 22.0;

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// The line while the demo shows (for the shell to set in the system's text).
    pub fn line(&self) -> Option<&std::ffi::CStr> {
        self.line.as_deref()
    }

    /// Routes a real finger: the mark's, the closing tap's, or the toy's. `has_demo` says whether the
    /// toy has a mark at all.
    pub fn route(&mut self, e: &PointerEvent, points: [f64; 2], has_demo: bool) -> Route {
        if self.taken.contains(&e.id) {
            if matches!(e.phase, Phase::Up | Phase::Cancel) {
                self.taken.retain(|&i| i != e.id);
                let s = Self::spot(points);
                if e.phase == Phase::Up && !self.is_open() && (e.x - s[0]).hypot(e.y - s[1]) <= Self::REACH * 1.5 {
                    return Route::Open;
                }
            }
            return Route::Taken;
        }
        if e.phase != Phase::Down {
            return Route::Toy;
        }
        if self.is_open() {
            // a tap anywhere closes it, and is not the toy's
            self.taken.push(e.id);
            self.close_pending = true;
            return Route::Taken;
        }
        let s = Self::spot(points);
        if has_demo && (e.x - s[0]).hypot(e.y - s[1]) <= Self::REACH {
            self.taken.push(e.id);
            return Route::Taken;
        }
        Route::Toy
    }
}

// (the closing tap is carried out by the host, which can send the ghost's fingers up)
impl Info {
    pub fn open(&mut self, demo: Demo, now: f64) {
        self.line = (!demo.line.is_empty()).then(|| std::ffi::CString::new(demo.line.replace('\0', "")).unwrap_or_default());
        self.open = Some((demo, now));
        self.ghosts.clear();
        self.played.clear();
        self.now = now;
    }

    /// The ghost's touches due by `now` (none under Reduce Motion), and whether the demo closed.
    pub fn advance(&mut self, now: f64, reduce_motion: bool) -> Vec<PointerEvent> {
        self.now = now;
        let mut out = Vec::new();
        let close = std::mem::take(&mut self.close_pending);
        let Some((demo, at)) = &self.open else { return out };
        let el = now - at;
        if !reduce_motion {
            for (i, s) in demo.strokes.iter().enumerate() {
                let id = GHOST_ID + i as u64;
                let down = self.ghosts.contains(&i);
                if !down && !close && el >= s.start && !self.played.contains(&i) {
                    let p = s.at(0.0);
                    out.push(PointerEvent { id, phase: Phase::Down, x: p[0], y: p[1], t: now });
                    self.ghosts.push(i);
                    self.played.push(i);
                } else if down {
                    let u = if s.seconds > 0.0 { (el - s.start) / s.seconds } else { 1.0 };
                    let p = s.at(u);
                    if close {
                        out.push(PointerEvent { id, phase: Phase::Cancel, x: p[0], y: p[1], t: now });
                        self.ghosts.retain(|&g| g != i);
                    } else if u >= 1.0 {
                        out.push(PointerEvent { id, phase: Phase::Up, x: p[0], y: p[1], t: now });
                        self.ghosts.retain(|&g| g != i);
                    } else {
                        out.push(PointerEvent { id, phase: Phase::Move, x: p[0], y: p[1], t: now });
                    }
                }
            }
        }
        if close || el > demo.end() + LINGER {
            self.open = None;
            self.line = None;
            self.ghosts.clear();
        }
        out
    }

    /// Draws the mark, and the ghost while the demo plays, over the toy's frame.
    pub fn draw(&mut self, gpu: &Gpu, target: &wgpu::TextureView, size: [u32; 2], ppp: f64, has_demo: bool, reduce_motion: bool) {
        if !has_demo {
            return;
        }
        let painter = self.painter.get_or_insert_with(|| Painter::new(gpu));
        let k = ppp as f32;
        let points = [size[0] as f64 / ppp, size[1] as f64 / ppp];
        let s = Self::spot(points);
        let c = [s[0] as f32 * k, s[1] as f32 * k];
        let lit = if self.open.is_some() { 0.85 } else { 0.5 };
        // a soft dark ground so it reads on any scene, then a small ring and an "i" in it
        painter.disc(c, 9.0 * k, 4.0 * k, [0.0, 0.0, 0.0, 0.28]);
        painter.ring(c, 7.5 * k, 1.1 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        painter.disc([c[0], c[1] - 3.1 * k], 0.95 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        painter.line([c[0], c[1] - 0.6 * k], [c[0], c[1] + 3.4 * k], 0.62 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        if let Some((demo, at)) = &self.open {
            let el = self.now - at;
            for s in &demo.strokes {
                // the fingertip: shown a beat before it lands and a beat after it lifts
                let (u, a) = if reduce_motion {
                    (0.0, 1.0)
                } else {
                    let a = ((el - s.start + 0.25) / 0.25).clamp(0.0, 1.0) * (1.0 - ((el - s.end()) / 0.35).clamp(0.0, 1.0));
                    (if s.seconds > 0.0 { (el - s.start) / s.seconds } else { 1.0 }, a as f32)
                };
                if a <= 0.0 {
                    continue;
                }
                let p = s.at(u);
                let p = [p[0] as f32 * k, p[1] as f32 * k];
                let pressed = if !reduce_motion && el >= s.start && el <= s.end() { 1.0 } else { 0.6 };
                painter.disc(p, 19.0 * k, 5.0 * k, [1.0, 1.0, 1.0, 0.16 * a * pressed]);
                painter.ring(p, 19.0 * k, 1.4 * k, 1.0, [1.0, 1.0, 1.0, 0.55 * a]);
                // (a dark edge outside it, so it reads on paper as on night)
                painter.ring(p, 20.6 * k, 1.0 * k, 1.0, [0.0, 0.0, 0.0, 0.28 * a]);
            }
        }
        let mut enc = gpu.device.create_command_encoder(&Default::default());
        painter.render(gpu, &mut enc, target, size, None);
        gpu.queue.submit([enc.finish()]);
    }
}
