//! The mark (painting-company row 355, Nov 15 2026: five marks became one): a tiny mark in the bottom right
//! corner, three dots (or the frame's own button: `native`). A tap on it opens the toy's sheet, and a tap
//! anywhere else closes it; nothing closes it on a timer. The sheet is the frame's, in its platform's own
//! controls over the live scene (the chrome's `sheet`, chrome.rs):
//!
//! - the toy's [`Rules`] (`Toy::rules`): the few rules the real thing follows and how a finger joins in,
//!   in two to four short lines (`host.info.rules()`);
//! - under them the rules as switches (`Toy::switches`, switches.rs), which stay as the reader left them
//!   when the sheet closes; the mark wears a small amber dot while any is off;
//! - and its buttons (the sheet's `demo` and `origins`): "Show me", the demo again (`Toy::demo`: a ghost finger doing
//!   the toy's one gesture over the live scene, real touches the scene answers, and at most one line),
//!   and "Where this comes from" (`Toy::origins`, a [`Rules`] too, the science and history behind the
//!   rules), which takes the sheet's place.
//!
//! A toy drawn in a style from the history of animation says whose (`Toy::style`, a [`Style`], the sheet's
//! `style`): one dimmer line at the foot of the rules, "Drawn in the style of BEFLIX: Ken Knowlton, Bell Labs, New Jersey, 1963." (Paul,
//! Nov 22 2026: "have it say somewhere in the toy what the style is").
//!
//! A toy with a demo alone has no sheet: its mark plays the demo. The demo also opens by itself on a first
//! visit (`First`). Under Reduce Motion the ghost rests where the gesture starts, still, with the line, and
//! no touch is played. A toy may hang the sheet from the top instead (`Toy::card_at_top`), to keep it off
//! what matters in its scene. A plan taps the mark as `"tap": "info"` (or `"rules"`, or `"switches"`: one
//! place now), and presses a sheet's button as `"show": "demo"` or `"show": "origins"` (check.rs).
//! painting-company rows 214, 219, 335 and 351.

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
        Stroke {
            start,
            seconds: 0.12,
            path: vec![at],
        }
    }
    pub fn drag(start: f64, seconds: f64, from: [f64; 2], to: [f64; 2]) -> Stroke {
        Stroke {
            start,
            seconds,
            path: vec![from, to],
        }
    }
    pub fn hold(start: f64, seconds: f64, at: [f64; 2]) -> Stroke {
        Stroke {
            start,
            seconds,
            path: vec![at],
        }
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
        Demo {
            line: line.into(),
            strokes,
        }
    }
    /// Two fingers about `c`, `r` points apart each side, turned by `turn` radians (clockwise on the
    /// screen) and spread by `spread` (2: twice as far apart), over `seconds` from `start`.
    pub fn two_fingers(
        start: f64,
        seconds: f64,
        c: [f64; 2],
        r: f64,
        turn: f64,
        spread: f64,
    ) -> Vec<Stroke> {
        let path = |sign: f64| {
            (0..=12)
                .map(|i| {
                    let u = i as f64 / 12.0;
                    let a = std::f64::consts::PI
                        + turn * u
                        + if sign > 0.0 {
                            0.0
                        } else {
                            std::f64::consts::PI
                        };
                    let d = r * (1.0 + (spread - 1.0) * u);
                    [c[0] + a.cos() * d, c[1] + a.sin() * d]
                })
                .collect()
        };
        // (each finger's path already eased by its points; the stroke eases it again only lightly)
        vec![
            Stroke {
                start,
                seconds,
                path: path(1.0),
            },
            Stroke {
                start,
                seconds,
                path: path(-1.0),
            },
        ]
    }
    fn end(&self) -> f64 {
        self.strokes.iter().map(Stroke::end).fold(0.0, f64::max)
    }
}

/// What the rules mark (or the origins mark) shows: lines, a line each, and an optional dimmer note
/// (empty: none; Screw's says the reader's song stays on the phone).
#[derive(Clone, Debug, PartialEq)]
pub struct Rules {
    pub lines: Vec<String>,
    pub note: String,
}

impl Rules {
    pub fn new(lines: &[&str], note: &str) -> Rules {
        Rules {
            lines: lines.iter().map(|l| l.to_string()).collect(),
            note: note.into(),
        }
    }
    /// The lines alone, no note.
    pub fn lines(lines: &[&str]) -> Rules {
        Rules::new(lines, "")
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

/// The style a toy is drawn in, from the history of animation (`Toy::style`): the film or process it
/// follows, who made it, where and when. Shown as one line at the foot of the sheet's rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Style {
    /// The film or process ("BEFLIX", "Growth", "the pinscreen").
    pub name: &'static str,
    /// Who made it ("Ken Knowlton").
    pub maker: &'static str,
    /// Where ("Bell Labs, New Jersey").
    pub place: &'static str,
    pub year: u16,
}

impl Style {
    /// The sheet's line: "Drawn in the style of BEFLIX: Ken Knowlton, Bell Labs, New Jersey, 1963."
    pub fn credit(&self) -> String {
        format!(
            "Drawn in the style of {}: {}, {}, {}.",
            self.name, self.maker, self.place, self.year
        )
    }
    /// The rules with the credit at their foot (under any note of their own).
    pub fn under(&self, mut rules: Rules) -> Rules {
        rules.note = if rules.note.is_empty() {
            self.credit()
        } else {
            format!("{}\n{}", rules.note, self.credit())
        };
        rules
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
    /// The finger whose touch closed what showed (lifted on the mark, it is not a tap to open it again).
    closer: Option<u64>,
    /// The ghost's fingers down now (stroke index).
    ghosts: Vec<usize>,
    /// Strokes already played (each once).
    played: Vec<usize>,
    /// A tap closed it: the ghost's fingers go up on the next frame.
    close_pending: bool,
    now: f64,
    line: Option<String>,
    /// The rules while they show (the frame's sheet: chrome.rs).
    rules: Option<Rules>,
    /// Where the rules came from, while that shows (the frame's sheet).
    origins: Option<Rules>,
    /// The frame draws the mark itself, natively (`Ctx::native_chrome`): the kit neither draws it nor
    /// routes a finger to it, and only plays the demo's ghost.
    pub native: bool,
}

/// Which marks a toy has (each shown only if the toy gives its text).
#[derive(Clone, Copy, Default, Debug)]
pub struct Marks {
    pub demo: bool,
    pub rules: bool,
    pub origins: bool,
    /// The toy has rules to switch (`Toy::switches`).
    pub switches: bool,
    /// One of them is off now: the mark wears its amber dot.
    pub off: bool,
}

impl Marks {
    /// Whether the toy has a mark at all.
    pub fn any(&self) -> bool {
        self.demo || self.rules || self.origins || self.switches
    }
    /// Whether the mark opens a sheet (else it plays the demo).
    pub fn sheet(&self) -> bool {
        self.rules || self.origins || self.switches
    }
}

/// What the host should do with a finger.
pub enum Route {
    Toy,
    Taken,
    /// The mark was tapped: open the sheet (or the demo, for a toy with no sheet).
    Open,
}

impl Info {
    /// Where the mark is, in points: the bottom right corner, clear of the home indicator.
    pub fn spot(points: [f64; 2]) -> [f64; 2] {
        [points[0] - 26.0, points[1] - 34.0]
    }
    /// A fingertip's reach for the mark, points.
    pub const REACH: f64 = 22.0;

    /// The demo, the rules or the origins showing.
    pub fn is_open(&self) -> bool {
        self.open.is_some() || self.rules.is_some() || self.origins.is_some()
    }

    /// Where the rules came from, while that shows (the frame's sheet).
    pub fn origins(&self) -> Option<&Rules> {
        self.origins.as_ref()
    }

    /// The sheet showing: its rules page or its origins page.
    pub fn sheet_open(&self) -> bool {
        self.rules.is_some() || self.origins.is_some()
    }

    /// Shows the origins (closing the demo or the rules), or hides them.
    pub fn show_origins(&mut self, origins: Option<Rules>) {
        if origins.is_some() {
            if self.open.is_some() {
                self.close_pending = true;
            }
            self.rules = None;
        }
        self.origins = origins;
    }

    /// The rules while they show (the frame's sheet).
    pub fn rules(&self) -> Option<&Rules> {
        self.rules.as_ref()
    }

    /// Shows the rules (closing the demo, whose ghost goes up on the next frame), or hides them.
    pub fn show_rules(&mut self, rules: Option<Rules>) {
        if rules.is_some() {
            if self.open.is_some() {
                self.close_pending = true;
            }
            self.origins = None;
        }
        self.rules = rules;
    }

    /// The line while the demo shows (the frame sets it in the system's text).
    pub fn line(&self) -> Option<&str> {
        self.line.as_deref()
    }

    /// Routes a real finger: the mark's, the closing tap's, or the toy's. `m` says what the toy has.
    pub fn route(&mut self, e: &PointerEvent, points: [f64; 2], m: Marks) -> Route {
        let near = |s: [f64; 2]| (e.x - s[0]).hypot(e.y - s[1]) <= Self::REACH;
        if self.taken.contains(&e.id) {
            if matches!(e.phase, Phase::Up | Phase::Cancel) {
                self.taken.retain(|&i| i != e.id);
                let closed = self.closer.take_if(|&mut c| c == e.id).is_some();
                // (lifted within reach of the mark, with nothing showing: a tap on it)
                if e.phase == Phase::Up
                    && !closed
                    && !self.is_open()
                    && m.any()
                    && !self.native
                    && near(Self::spot(points))
                {
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
            self.close();
            self.taken.push(e.id);
            self.closer = Some(e.id);
            return Route::Taken;
        }
        if m.any() && !self.native && near(Self::spot(points)) {
            self.taken.push(e.id);
            return Route::Taken;
        }
        Route::Toy
    }
}

/// The demo opening by itself on a first visit (Paul, Nov 13: "The info should pop up when you first
/// open an experience"): a toy's own a moment after it starts (`Toy::info_at_start`), unless a finger
/// came down first; and whenever the toy asks (`Ctx::show_info`: Terra, as each toy it holds first
/// opens). Off (`on: false`) in the headless check unless a plan says `"first_info": true`, so a plan's
/// fingers stay its own.
pub struct First {
    pub on: bool,
    /// A real finger came down (the reader is already in: the start's demo stays shut).
    pub touched: bool,
    /// The real fingers down now (a demo waits for them to lift: its ghost would play among them).
    down: Vec<u64>,
    /// The toy's first frame, and whether the start's demo is done with.
    began: Option<f64>,
    done: bool,
    /// A toy's ask, waiting for the fingers to lift and any card to close (until `FIRST_WAIT` passes).
    asked: Option<f64>,
}

impl Default for First {
    fn default() -> First {
        First {
            on: true,
            touched: false,
            down: Vec::new(),
            began: None,
            done: false,
            asked: None,
        }
    }
}

impl First {
    /// A real finger, as the host is handed it (never the ghost's).
    pub fn finger(&mut self, e: &PointerEvent) {
        match e.phase {
            Phase::Down => {
                self.touched = true;
                self.down.push(e.id);
            }
            Phase::Up | Phase::Cancel => self.down.retain(|&i| i != e.id),
            Phase::Move => {}
        }
    }
}

/// How long after the toy's first frame its demo opens by itself.
const FIRST_AFTER: f64 = 1.0;
/// How long a toy's ask waits for the fingers to lift or a card to close before it lapses.
const FIRST_WAIT: f64 = 3.0;

// the info marks' side of the host
impl<T: crate::Toy> crate::Host<T> {
    /// Opens the demo by itself when a first visit calls for it (`First`).
    pub(crate) fn first_info_step(&mut self, t: f64) {
        let asked = self.ctx.take_show_info();
        if self.ctx.take_hide_info() && self.info.is_open() {
            self.info.close();
        }
        let f = &mut self.first_info;
        if !f.on {
            return;
        }
        if asked {
            f.asked = Some(t);
        }
        let began = *f.began.get_or_insert(t);
        let start = !f.done && t - began >= FIRST_AFTER;
        if start {
            f.done = true;
        }
        let free = f.down.is_empty() && !self.info.is_open();
        let waiting = f.asked.is_some_and(|a| t - a <= FIRST_WAIT);
        if waiting && free {
            f.asked = None;
            self.open_info(t);
        } else if !waiting {
            f.asked = None;
            if start && !f.touched && free && self.toy.info_at_start() {
                self.open_info(t);
            }
        }
    }

    /// Shows where the rules came from in the sheet's place (its button, or VoiceOver's), or, if that
    /// shows, closes it.
    pub fn open_origins(&mut self) {
        let r = if self.info.origins().is_some() {
            None
        } else {
            self.toy.origins()
        };
        if r.is_some() && !self.info.sheet_open() {
            self.sheet_top = self.toy.card_at_top(&self.ctx);
        }
        self.info.show_origins(r);
    }

    /// Whether the shell hangs the sheet from the top (`Toy::card_at_top`).
    pub fn card_at_top(&self) -> bool {
        self.toy.card_at_top(&self.ctx)
    }

    /// Opens the sheet on its rules page, or, if it shows, closes it.
    pub fn open_rules(&mut self) {
        if self.info.sheet_open() {
            self.info.close();
            return;
        }
        // (a toy with switches and no rules of its own: the sheet is its switches alone)
        // (the style's credit goes with it in the frame's sheet: `chrome::Sheet::style`)
        let r = self.toy.rules().unwrap_or_else(|| Rules::lines(&[]));
        self.sheet_top = self.toy.card_at_top(&self.ctx);
        self.info.show_rules(Some(r));
    }

    /// The mark's tap (or VoiceOver's activation of it): the sheet, opened or closed; for a toy with a demo
    /// alone, the demo.
    pub fn open_mark(&mut self, t: f64) {
        if self.info.open.is_some() {
            self.info.close();
        } else if self.marks.sheet() {
            self.open_rules();
        } else {
            self.open_info(t);
        }
    }

    /// Plays the demo (its own opening on a first visit, the sheet's "Show me"), closing the sheet.
    pub fn open_info(&mut self, t: f64) {
        if self.info.open.is_some() {
            return;
        }
        if let Some(d) = self.toy.demo(&self.ctx) {
            // (the sheet closes; no ghost is down to lift)
            self.info.close();
            self.info.close_pending = false;
            self.info.open(d, t);
        }
    }

    /// The mark's place in points, if the toy has one.
    pub fn info_spot(&self) -> Option<[f64; 2]> {
        self.marks.any().then(|| Info::spot(self.ctx.points()))
    }

    /// Each frame: which of the mark's things the toy has now (a host's change with what it holds).
    pub(crate) fn marks_step(&mut self) {
        let sw = self.toy.switches();
        self.marks = Marks {
            demo: self.toy.demo(&self.ctx).is_some(),
            rules: self.toy.rules().is_some(),
            origins: self.toy.origins().is_some(),
            switches: !sw.is_empty(),
            off: sw.iter().any(|s| !s.on),
        };
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
        self.line = (!demo.line.is_empty()).then(|| demo.line.clone());
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
        let Some((demo, at)) = &self.open else {
            return out;
        };
        let el = now - at;
        if !reduce_motion {
            for (i, s) in demo.strokes.iter().enumerate() {
                let id = GHOST_ID + i as u64;
                let down = self.ghosts.contains(&i);
                if !down && !close && el >= s.start && !self.played.contains(&i) {
                    let p = s.at(0.0);
                    out.push(PointerEvent {
                        id,
                        phase: Phase::Down,
                        x: p[0],
                        y: p[1],
                        t: now,
                    });
                    self.ghosts.push(i);
                    self.played.push(i);
                } else if down {
                    let u = if s.seconds > 0.0 {
                        (el - s.start) / s.seconds
                    } else {
                        1.0
                    };
                    let p = s.at(u);
                    if close {
                        out.push(PointerEvent {
                            id,
                            phase: Phase::Cancel,
                            x: p[0],
                            y: p[1],
                            t: now,
                        });
                        self.ghosts.retain(|&g| g != i);
                    } else if u >= 1.0 {
                        out.push(PointerEvent {
                            id,
                            phase: Phase::Up,
                            x: p[0],
                            y: p[1],
                            t: now,
                        });
                        self.ghosts.retain(|&g| g != i);
                    } else {
                        out.push(PointerEvent {
                            id,
                            phase: Phase::Move,
                            x: p[0],
                            y: p[1],
                            t: now,
                        });
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
    pub fn draw(
        &mut self,
        gpu: &Gpu,
        target: &wgpu::TextureView,
        size: [u32; 2],
        ppp: f64,
        m: Marks,
        reduce_motion: bool,
    ) {
        if !m.any() {
            return;
        }
        let lit = if self.is_open() { 0.85 } else { 0.5 };
        let native = self.native;
        let painter = self.painter.get_or_insert_with(|| Painter::new(gpu));
        let k = ppp as f32;
        let points = [size[0] as f64 / ppp, size[1] as f64 / ppp];
        // a soft dark ground so it reads on any scene, a small ring, and three dots (more, as iOS has it);
        // none where the frame draws its own (`native`)
        let s = Self::spot(points);
        let c = [s[0] as f32 * k, s[1] as f32 * k];
        if !native {
            painter.disc(c, 9.5 * k, 3.0 * k, [0.0, 0.0, 0.0, 0.42]);
            painter.ring(c, 7.5 * k, 1.1 * k, 0.7, [1.0, 1.0, 1.0, lit]);
            for i in -1..=1 {
                painter.disc(
                    [c[0] + 2.9 * k * i as f32, c[1]],
                    0.95 * k,
                    0.7,
                    [1.0, 1.0, 1.0, lit],
                );
            }
        }
        if m.off && !native {
            // a rule is off: a small amber dot on the mark's shoulder, so the toy is never mistaken for the real thing
            let d = [c[0] + 5.6 * k, c[1] - 5.6 * k];
            painter.disc(d, 3.3 * k, 0.8, [0.0, 0.0, 0.0, 0.5]);
            painter.disc(d, 2.5 * k, 0.7, [0.96, 0.67, 0.24, 1.0]);
        }
        if let Some((demo, at)) = &self.open {
            let el = self.now - at;
            for s in &demo.strokes {
                // the fingertip: shown a beat before it lands and a beat after it lifts
                let (u, a) = if reduce_motion {
                    (0.0, 1.0)
                } else {
                    let a = ((el - s.start + 0.25) / 0.25).clamp(0.0, 1.0)
                        * (1.0 - ((el - s.end()) / 0.35).clamp(0.0, 1.0));
                    (
                        if s.seconds > 0.0 {
                            (el - s.start) / s.seconds
                        } else {
                            1.0
                        },
                        a as f32,
                    )
                };
                if a <= 0.0 {
                    continue;
                }
                let p = s.at(u);
                let p = [p[0] as f32 * k, p[1] as f32 * k];
                let pressed = if !reduce_motion && el >= s.start && el <= s.end() {
                    1.0
                } else {
                    0.6
                };
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

    fn tap(info: &mut Info, id: u64, at: [f64; 2], m: Marks) -> Route {
        let pts = [390.0, 844.0];
        let down = PointerEvent {
            id,
            phase: Phase::Down,
            x: at[0],
            y: at[1],
            t: 0.0,
        };
        let r = info.route(&down, pts, m);
        if !matches!(r, Route::Taken) {
            return r;
        }
        info.route(
            &PointerEvent {
                phase: Phase::Up,
                ..down
            },
            pts,
            m,
        )
    }

    #[test]
    fn one_mark_opens_and_any_tap_closes() {
        let pts = [390.0, 844.0];
        let all = Marks {
            demo: true,
            rules: true,
            origins: true,
            switches: true,
            off: false,
        };
        let mut info = Info::default();
        assert!(matches!(
            tap(&mut info, 1, Info::spot(pts), all),
            Route::Open
        ));
        // where the other marks were is the toy's now
        assert!(matches!(
            tap(&mut info, 2, [pts[0] - 26.0 - 44.0, pts[1] - 34.0], all),
            Route::Toy
        ));
        // a toy with no mark: its place is the toy's
        assert!(matches!(
            tap(&mut info, 3, Info::spot(pts), Marks::default()),
            Route::Toy
        ));
        info.show_rules(Some(Rules::new(&["a", "b"], "c")));
        assert_eq!(info.rules().unwrap().text(), "a\nb\n\nc");
        assert!(info.is_open() && info.sheet_open());
        // the origins take the rules' place
        info.show_origins(Some(Rules::new(&["d"], "")));
        assert!(info.rules().is_none() && info.origins().is_some());
        // a tap anywhere closes it and is not the toy's
        assert!(matches!(
            tap(&mut info, 4, [100.0, 100.0], all),
            Route::Taken
        ));
        assert!(!info.is_open() && !info.sheet_open());
        // a tap on the mark while the sheet shows only closes it
        info.show_rules(Some(Rules::new(&["a"], "")));
        assert!(matches!(
            tap(&mut info, 5, Info::spot(pts), all),
            Route::Taken
        ));
        assert!(!info.is_open());
        // where the frame draws its own mark, the corner is the toy's
        info.native = true;
        assert!(matches!(
            tap(&mut info, 6, Info::spot(pts), all),
            Route::Toy
        ));
    }

    #[test]
    fn a_style_is_credited_under_the_rules() {
        let s = Style {
            name: "BEFLIX",
            maker: "Ken Knowlton",
            place: "Bell Labs, New Jersey",
            year: 1963,
        };
        let r = s.under(Rules::lines(&["a"]));
        assert_eq!(
            r.text(),
            "a\n\nDrawn in the style of BEFLIX: Ken Knowlton, Bell Labs, New Jersey, 1963."
        );
        assert!(s.under(Rules::new(&["a"], "n")).text().ends_with(
            "n\nDrawn in the style of BEFLIX: Ken Knowlton, Bell Labs, New Jersey, 1963."
        ));
    }

    #[test]
    fn a_demo_alone_has_no_sheet() {
        let m = Marks {
            demo: true,
            ..Marks::default()
        };
        assert!(m.any() && !m.sheet());
        let s = Marks {
            switches: true,
            ..Marks::default()
        };
        assert!(s.any() && s.sheet());
    }
}
