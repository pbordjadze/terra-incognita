//! The info mark (docs/kit.md › First after the reset, 2): a tiny mark in a corner; a tap on it shows
//! for a few seconds what the app is, a ghost finger doing the app's one gesture over the live scene
//! (real touches: the scene answers as it would a finger) and at most one line; a tap anywhere closes
//! it. Each toy supplies its own [`Demo`] (`Toy::demo`; None, the default, shows no mark). The kit
//! draws the mark and the ghost; the line is the shell's, in the system's text (VoiceOver reads it, it
//! follows the reader's text size; `host.info.line()`). Under Reduce Motion the ghost rests where the
//! gesture starts, still, with the line, and no touch is played. A plan taps it as `"tap": "info"`.
//!
//! Beside it, the rules mark (a "?"): a tap shows the toy's [`Rules`] (`Toy::rules`): the few rules
//! that make what is on screen, then one line of what is measured and what is drawn. The shell sets
//! them in the system's text over the live scene (`host.info.rules()`); a tap anywhere closes them,
//! and nothing closes them on a timer (they are read at the reader's pace, or not at all). A plan
//! taps it as `"tap": "rules"`. painting-company row 214.
//!
//! Beside that, the origins mark (an asterisk, a footnote's mark): a tap shows where the rules came
//! from (`Toy::origins`, a [`Rules`] too: its lines the science and history behind them, its note what
//! is real and what is drawn), so the "?" can be the rules alone. Same card, same closing tap. A
//! toy may hang the card from the top instead of over the marks (`Toy::card_at_top`), to keep it off
//! what matters in its scene. A plan taps it as `"tap": "origins"`. painting-company row 219.

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

/// What the rules mark (or the origins mark) shows: lines, a line each, and an optional dimmer note
/// (empty: none), e.g. what is real and what is drawn.
#[derive(Clone, Debug)]
pub struct Rules {
    pub lines: Vec<String>,
    pub note: String,
}

impl Rules {
    pub fn new(lines: &[&str], note: &str) -> Rules {
        Rules { lines: lines.iter().map(|l| l.to_string()).collect(), note: note.into() }
    }
    /// As the shell takes it: the lines, one a line, then a blank line and the note.
    pub fn text(&self) -> String {
        let mut t = self.lines.join("\n");
        if !self.note.is_empty() {
            t.push_str("\n\n");
            t.push_str(&self.note);
        }
        t.replace('\0', "")
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
    /// The rules while they show (the shell's text).
    rules: Option<std::ffi::CString>,
    /// Where the rules came from, while that shows (the shell's text).
    origins: Option<std::ffi::CString>,
}

/// Which marks a toy has (each shown only if the toy gives its text).
#[derive(Clone, Copy, Default, Debug)]
pub struct Marks {
    pub demo: bool,
    pub rules: bool,
    pub origins: bool,
}

/// What the host should do with a finger.
pub enum Route {
    Toy,
    Taken,
    /// The mark was tapped: open the demo.
    Open,
    /// The rules mark was tapped: show the rules.
    Rules,
    /// The origins mark was tapped: show where the rules came from.
    Origins,
}

impl Info {
    /// Where the mark is, in points: the bottom right corner, clear of the home indicator.
    pub fn spot(points: [f64; 2]) -> [f64; 2] {
        [points[0] - 26.0, points[1] - 34.0]
    }
    /// Where the rules mark is: beside the info mark, toward the middle.
    pub fn rules_spot(points: [f64; 2]) -> [f64; 2] {
        [points[0] - 26.0 - Self::GAP, points[1] - 34.0]
    }
    /// Where the origins mark is: beside the rules mark, further toward the middle.
    pub fn origins_spot(points: [f64; 2]) -> [f64; 2] {
        [points[0] - 26.0 - 2.0 * Self::GAP, points[1] - 34.0]
    }
    /// A fingertip's reach for either, points (half the gap between them: no tap is both's).
    pub const REACH: f64 = 22.0;
    const GAP: f64 = 44.0;

    /// The demo, the rules or the origins showing.
    pub fn is_open(&self) -> bool {
        self.open.is_some() || self.rules.is_some() || self.origins.is_some()
    }

    /// Where the rules came from, while that shows (for the shell's text; `Rules::text`).
    pub fn origins(&self) -> Option<&std::ffi::CStr> {
        self.origins.as_deref()
    }

    /// Shows the origins (closing the demo or the rules), or hides them.
    pub fn show_origins(&mut self, origins: Option<Rules>) {
        if origins.is_some() {
            if self.open.is_some() {
                self.close_pending = true;
            }
            self.rules = None;
        }
        self.origins = origins.map(|r| std::ffi::CString::new(r.text()).unwrap_or_default());
    }

    /// The rules while they show (for the shell to set in the system's text; `Rules::text`).
    pub fn rules(&self) -> Option<&std::ffi::CStr> {
        self.rules.as_deref()
    }

    /// Shows the rules (closing the demo, whose ghost goes up on the next frame), or hides them.
    pub fn show_rules(&mut self, rules: Option<Rules>) {
        if rules.is_some() {
            if self.open.is_some() {
                self.close_pending = true;
            }
            self.origins = None;
        }
        self.rules = rules.map(|r| std::ffi::CString::new(r.text()).unwrap_or_default());
    }

    /// The line while the demo shows (for the shell to set in the system's text).
    pub fn line(&self) -> Option<&std::ffi::CStr> {
        self.line.as_deref()
    }

    /// Routes a real finger: a mark's, the closing tap's, or the toy's. `m` says which marks the toy has.
    pub fn route(&mut self, e: &PointerEvent, points: [f64; 2], m: Marks) -> Route {
        let near = |s: [f64; 2], k: f64| (e.x - s[0]).hypot(e.y - s[1]) <= Self::REACH * k;
        if self.taken.contains(&e.id) {
            if matches!(e.phase, Phase::Up | Phase::Cancel) {
                self.taken.retain(|&i| i != e.id);
                if e.phase == Phase::Up && !self.is_open() {
                    // (lifted within reach of where it went down: the marks are 44 points apart)
                    if m.demo && near(Self::spot(points), 1.0) {
                        return Route::Open;
                    }
                    if m.rules && near(Self::rules_spot(points), 1.0) {
                        return Route::Rules;
                    }
                    if m.origins && near(Self::origins_spot(points), 1.0) {
                        return Route::Origins;
                    }
                }
            }
            return Route::Taken;
        }
        if e.phase != Phase::Down {
            return Route::Toy;
        }
        if self.is_open() {
            // a tap anywhere closes it, and is not the toy's
            self.close();
            self.taken.push(e.id);
            return Route::Taken;
        }
        if (m.demo && near(Self::spot(points), 1.0))
            || (m.rules && near(Self::rules_spot(points), 1.0))
            || (m.origins && near(Self::origins_spot(points), 1.0))
        {
            self.taken.push(e.id);
            return Route::Taken;
        }
        Route::Toy
    }
}

// (the closing tap is carried out by the host, which can send the ghost's fingers up)
impl Info {
    /// Closes whatever shows (the demo, the rules, the origins), as a tap anywhere does.
    pub fn close(&mut self) {
        if self.is_open() {
            self.close_pending = true;
        }
        self.rules = None;
        self.origins = None;
    }

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

    /// Draws the marks, and the ghost while the demo plays, over the toy's frame.
    pub fn draw(&mut self, gpu: &Gpu, target: &wgpu::TextureView, size: [u32; 2], ppp: f64, m: Marks, reduce_motion: bool) {
        if !m.demo && !m.rules && !m.origins {
            return;
        }
        let painter = self.painter.get_or_insert_with(|| Painter::new(gpu));
        let k = ppp as f32;
        let points = [size[0] as f64 / ppp, size[1] as f64 / ppp];
        // a soft dark ground so each reads on any scene, then a small ring and its letter
        let ground = |painter: &mut Painter, c: [f32; 2], lit: f32| {
            painter.disc(c, 9.5 * k, 3.0 * k, [0.0, 0.0, 0.0, 0.42]);
            painter.ring(c, 7.5 * k, 1.1 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        };
        if m.demo {
            let s = Self::spot(points);
            let c = [s[0] as f32 * k, s[1] as f32 * k];
            let lit = if self.open.is_some() { 0.85 } else { 0.5 };
            ground(painter, c, lit);
            painter.disc([c[0], c[1] - 3.1 * k], 0.95 * k, 0.7, [1.0, 1.0, 1.0, lit]);
            painter.line([c[0], c[1] - 0.6 * k], [c[0], c[1] + 3.4 * k], 0.62 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        }
        if m.rules {
            let s = Self::rules_spot(points);
            let c = [s[0] as f32 * k, s[1] as f32 * k];
            let lit = if self.rules.is_some() { 0.85 } else { 0.5 };
            ground(painter, c, lit);
            // a "?": an arc over the top, a short stem down from its right, a dot
            let (r, o) = (2.3f32, [c[0], c[1] - 1.7 * k]);
            let arc: Vec<[f32; 2]> = (0..=10)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.0 + 1.4 * i as f32 / 10.0);
                    [o[0] + a.cos() * r * k, o[1] + a.sin() * r * k]
                })
                .collect();
            for w in arc.windows(2) {
                painter.line(w[0], w[1], 0.62 * k, 0.7, [1.0, 1.0, 1.0, lit]);
            }
            let end = arc[arc.len() - 1];
            painter.line(end, [c[0], c[1] + 1.4 * k], 0.62 * k, 0.7, [1.0, 1.0, 1.0, lit]);
            painter.disc([c[0], c[1] + 3.6 * k], 0.85 * k, 0.7, [1.0, 1.0, 1.0, lit]);
        }
        if m.origins {
            let s = Self::origins_spot(points);
            let c = [s[0] as f32 * k, s[1] as f32 * k];
            let lit = if self.origins.is_some() { 0.85 } else { 0.5 };
            ground(painter, c, lit);
            // an asterisk, a footnote's mark: six spokes from a small open middle
            for i in 0..6 {
                let a = std::f32::consts::PI * (0.5 + i as f32 / 3.0);
                let (dx, dy) = (a.cos() * k, a.sin() * k);
                painter.line([c[0] + dx * 0.9, c[1] - dy * 0.9], [c[0] + dx * 3.5, c[1] - dy * 3.5], 0.62 * k, 0.7, [1.0, 1.0, 1.0, lit]);
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tap(info: &mut Info, id: u64, at: [f64; 2], demo: bool, rules: bool) -> Route {
        tap3(info, id, at, Marks { demo, rules, origins: false })
    }
    fn tap3(info: &mut Info, id: u64, at: [f64; 2], m: Marks) -> Route {
        let pts = [390.0, 844.0];
        let down = PointerEvent { id, phase: Phase::Down, x: at[0], y: at[1], t: 0.0 };
        let r = info.route(&down, pts, m);
        if !matches!(r, Route::Taken) {
            return r;
        }
        info.route(&PointerEvent { phase: Phase::Up, ..down }, pts, m)
    }

    #[test]
    fn the_origins_mark_opens_and_takes_the_rules_place() {
        let pts = [390.0, 844.0];
        let all = Marks { demo: true, rules: true, origins: true };
        let mut info = Info::default();
        assert!(matches!(tap3(&mut info, 1, Info::origins_spot(pts), all), Route::Origins));
        // without it its place is the toy's
        assert!(matches!(tap3(&mut info, 2, Info::origins_spot(pts), Marks { origins: false, ..all }), Route::Toy));
        info.show_rules(Some(Rules::new(&["a"], "")));
        assert_eq!(info.rules().unwrap().to_str().unwrap(), "a");
        info.show_origins(Some(Rules::new(&["b"], "c")));
        assert!(info.rules().is_none());
        assert_eq!(info.origins().unwrap().to_str().unwrap(), "b\n\nc");
        assert!(matches!(tap3(&mut info, 3, [100.0, 100.0], all), Route::Taken));
        assert!(info.origins().is_none() && !info.is_open());
    }

    #[test]
    fn the_rules_mark_opens_and_any_tap_closes() {
        let pts = [390.0, 844.0];
        let mut info = Info::default();
        assert!(matches!(tap(&mut info, 1, Info::rules_spot(pts), true, true), Route::Rules));
        assert!(matches!(tap(&mut info, 2, Info::spot(pts), true, true), Route::Open));
        // without a rules mark its place is the toy's
        assert!(matches!(tap(&mut info, 3, Info::rules_spot(pts), true, false), Route::Toy));
        info.show_rules(Some(Rules::new(&["a", "b"], "c")));
        assert_eq!(info.rules().unwrap().to_str().unwrap(), "a\nb\n\nc");
        assert!(info.is_open());
        // a tap anywhere closes them and is not the toy's
        assert!(matches!(tap(&mut info, 4, [100.0, 100.0], true, true), Route::Taken));
        assert!(info.rules().is_none() && !info.is_open());
    }
}
