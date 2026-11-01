//! Conducting a cloud (a spike, painting-company row 199; not part of any round). One lamp's 2048
//! worlds run on the CPU on a fixed half-day grid from the lamp's last cue, so what is drawn, what a
//! gesture is matched against and what a replay computes are the same numbers. A gesture looks back
//! over the worlds that already ran and picks the one whose path did what the hand did:
//!
//! - **point** (a tap): the world drawn nearest the finger;
//! - **sweep** (a stroke): the world that, over the last stretch as long as the stroke (in days at
//!   the conducting rate), veered most the stroke's way from the cloud's own drift; a longer stroke
//!   picks a more extreme one;
//! - **stir** (a loop drawn, three quarters of a turn or more): the world whose path turned most in
//!   the same sense over that stretch;
//! - **hold** (a finger resting): a fermata; the date stands while it rests. No narrowing.
//!
//! The take is then rewritten behind the gesture: the lamp's journey since its last cue becomes that
//! world's path, and a new cloud of 2048 worlds blooms from where it is, carrying its eddy velocity
//! (the flight's memory) so the continuation is the same law as one unbroken flight: the worlds'
//! kicks are fresh each quarter day, so which world a path lives in after a cue is independent of
//! before it. A take is its cues (day, chosen world); `again` replays them exactly.

use crate::look::{LookBead, LookCloud, LookFloat};
use crate::model::{Camera, Disc, LonLat, Phase, PointerEvent};
use crate::sea::{Drift, GpuPath, Rules};

/// Days a second while conducting: a cloud in a jet spreads a few hundred km in a second or two.
pub const RATE: f64 = 6.0;
pub const WORLDS: u32 = 2048;
const STEP: f64 = 0.5;
/// Half-day samples of every world kept for the look back (16 days).
const HISTORY: usize = 33;
/// The id the conducted lamp wears (its hue).
pub const LAMP: u32 = 1;

#[derive(Clone, Copy, Debug)]
struct W {
    at: LonLat,
    eddy: (f32, f32),
}

#[derive(Clone, Copy, Debug)]
struct Leg {
    day: f64,
    at: LonLat,
    eddy: (f32, f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Gesture {
    Point { x: f64, y: f64 },
    Sweep { dx: f64, dy: f64, secs: f64 },
    Stir { turn: f64, secs: f64 },
}

impl Gesture {
    fn name(&self) -> String {
        match *self {
            Gesture::Point { .. } => "tap".into(),
            Gesture::Sweep { dx, dy, .. } => {
                let a = (-dy).atan2(dx).to_degrees();
                let arrows = ["→", "↗", "↑", "↖", "←", "↙", "↓", "↘"];
                format!("sweep {}", arrows[(((a + 360.0 + 22.5) % 360.0) / 45.0) as usize % 8])
            }
            Gesture::Stir { turn, .. } => format!("circle {}", if turn < 0.0 { "↻" } else { "↺" }),
        }
    }
}

/// One cue of a take: on `day`, `gesture` chose world `k`; `matches` of the worlds did it.
#[derive(Clone, Copy, Debug)]
pub struct Cue {
    pub day: f64,
    pub gesture: Gesture,
    pub k: u32,
    pub matches: u32,
    /// The look back's cost, ms.
    pub ms: f64,
}

/// A cloud folding into its chosen world: positions as they stood at the cue.
struct Ghost {
    paths: Vec<GpuPath>,
    at: LonLat,
    since: f64,
    secs: f64,
    slot: u32,
}

#[derive(Clone, Copy, Debug)]
struct Touch {
    id: u64,
    pts_start: usize,
    t0: f64,
}

pub struct Conductor {
    start: Leg,
    legs: Vec<Leg>,
    /// The take decided so far: the chosen worlds' paths, cue to cue, as places a half day apart.
    take: Vec<LonLat>,
    /// The worlds of the current leg: their state at `ahead` and their places at the last
    /// `HISTORY` half-day marks (ring[i][k], `head` the newest).
    now: Vec<W>,
    ring: Vec<Vec<LonLat>>,
    head: usize,
    filled: usize,
    ahead: f64,
    /// What is drawn this frame, k = 1..=WORLDS, and the day it is for.
    pub drawn: Vec<LonLat>,
    drawn_day: f64,
    ghosts: Vec<Ghost>,
    bloom: f32,
    pub cues: Vec<Cue>,
    /// Cues still to replay (`again`), soonest last.
    replay: Vec<Cue>,
    /// A cue to make on the next tick (the gestures are read here; the camera in the app).
    pending: Option<Gesture>,
    pub line: String,
    seconds: f64,
    replaying: bool,
    /// A cue was made since the shell last asked (`take_cued`).
    pub cued: bool,
    /// The lamp's id (its hue), and whether it is conducted as a line (its journey, no cloud drawn
    /// but at the cue: the possibilities flash up and fold into the one chosen).
    pub id: u32,
    pub lines: bool,
    /// Its buffer slots: the live cloud's, then the two folds'.
    slots: [u32; 3],
    /// World 1's path this leg (the line's provisional journey), a half day apart.
    prov: Vec<LonLat>,
}

impl Conductor {
    pub fn new(rules: &Rules, at: LonLat, day: f64) -> Conductor {
        let e = rules.put(at, day, 0, 0).eddy;
        let start = Leg { day, at, eddy: e };
        let mut c = Conductor {
            start,
            legs: Vec::new(),
            take: vec![at],
            now: Vec::new(),
            ring: Vec::new(),
            head: 0,
            filled: 0,
            ahead: day,
            drawn: Vec::new(),
            drawn_day: day,
            ghosts: Vec::new(),
            bloom: 0.0,
            cues: Vec::new(),
            replay: Vec::new(),
            pending: None,
            id: LAMP,
            lines: false,
            slots: [0, 6, 7],
            prov: Vec::new(),
            line: "conduct: tap, sweep or stir the cloud; rest a finger to hold".into(),
            seconds: 0.0,
            replaying: false,
            cued: false,
        };
        c.begin(start);
        c
    }

    pub fn start_at(&self) -> LonLat {
        self.start.at
    }

    pub fn start_day(&self) -> f64 {
        self.start.day
    }

    fn leg(&self) -> Leg {
        *self.legs.last().unwrap()
    }

    /// A new leg: every world at its place with its eddy velocity.
    fn begin(&mut self, leg: Leg) {
        self.legs.push(leg);
        self.now = vec![W { at: leg.at, eddy: leg.eddy }; WORLDS as usize];
        self.ring = vec![vec![leg.at; WORLDS as usize]; HISTORY];
        self.head = 0;
        self.filled = 1;
        self.ahead = leg.day;
        self.drawn = vec![leg.at; WORLDS as usize];
        self.drawn_day = leg.day;
        self.bloom = 0.0;
        self.prov = vec![leg.at];
    }

    /// Steps every world on until the newest mark is at or past `day`.
    fn step_to(&mut self, rules: &Rules, day: f64) {
        let leg = self.leg();
        while self.ahead < day - 1e-9 {
            let t = self.ahead;
            for (i, w) in self.now.iter_mut().enumerate() {
                let f = rules.flight(Drift { at: w.at, eddy: w.eddy }, t, STEP, 0, i as u32 + 1);
                *w = W { at: f.drift.at, eddy: f.drift.eddy };
            }
            self.ahead = t + STEP;
            self.head = (self.head + 1) % HISTORY;
            for (i, w) in self.now.iter().enumerate() {
                self.ring[self.head][i] = w.at;
            }
            self.filled = (self.filled + 1).min(HISTORY);
            self.prov.push(self.now[0].at);
        }
        let _ = leg;
    }

    /// The place world k had `back` marks before the newest.
    fn back(&self, back: usize, k: usize) -> LonLat {
        let b = back.min(self.filled - 1);
        self.ring[(self.head + HISTORY - b) % HISTORY][k]
    }

    /// World k of `leg` stepped exactly to `day` (half-day marks from the leg's day, then the rest):
    /// its places on the way and its state there. The replay's and the cue's one source.
    fn world_at(rules: &Rules, leg: Leg, k: u32, day: f64) -> (Vec<LonLat>, W) {
        let mut w = W { at: leg.at, eddy: leg.eddy };
        let mut t = leg.day;
        let mut path = vec![w.at];
        while t + STEP <= day + 1e-9 {
            let f = rules.flight(Drift { at: w.at, eddy: w.eddy }, t, STEP, 0, k);
            w = W { at: f.drift.at, eddy: f.drift.eddy };
            t += STEP;
            path.push(w.at);
        }
        if day - t > 1e-9 {
            let f = rules.flight(Drift { at: w.at, eddy: w.eddy }, t, day - t, 0, k);
            w = W { at: f.drift.at, eddy: f.drift.eddy };
            path.push(w.at);
        }
        (path, w)
    }

    /// The take from the start again: the same cues, the same worlds, the same journey.
    pub fn again(&mut self, rules: &Rules) -> f64 {
        let mut cues = std::mem::take(&mut self.cues);
        cues.reverse();
        let n = cues.len();
        let start = self.start;
        let (id, lines, slots) = (self.id, self.lines, self.slots);
        *self = Conductor::new(rules, start.at, start.day).voice(id, lines, slots);
        self.replay = cues;
        self.line = format!("again: replaying {n} cues");
        start.day
    }

    /// A new take from the same place and day.
    pub fn fresh(&mut self, rules: &Rules) -> f64 {
        let start = self.start;
        let (id, lines, slots) = (self.id, self.lines, self.slots);
        *self = Conductor::new(rules, start.at, start.day).voice(id, lines, slots);
        start.day
    }

    /// This lamp as one voice of several: its id, whether it is a line, its buffer slots.
    pub fn voice(mut self, id: u32, lines: bool, slots: [u32; 3]) -> Conductor {
        self.id = id;
        self.lines = lines;
        self.slots = slots;
        self
    }

    /// A gesture for this voice, read on its next tick.
    pub fn cue(&mut self, g: Gesture) {
        self.pending = Some(g);
    }

    /// Where its lamp is drawn now (a line's: world 1 of its leg; a cloud's: where it bloomed from).
    pub fn lamp_at(&self) -> LonLat {
        if self.lines {
            self.drawn[0]
        } else {
            self.leg().at
        }
    }

    /// A frame at `day` (the clock's), `seconds` the wall clock: replayed cues fall due, a gesture
    /// is read against what is drawn, the worlds step on and the drawn places are interpolated.
    pub fn tick(&mut self, rules: &Rules, day: f64, seconds: f64, disc: &Disc, cam: &Camera) {
        self.seconds = seconds;
        while let Some(c) = self.replay.last().copied() {
            if c.day > day + 1e-9 {
                break;
            }
            self.replay.pop();
            self.replaying = true;
            self.cue_world(rules, c.day, c.k, c.gesture, c.matches, 0.0, 0);
            self.replaying = false;
        }
        self.step_to(rules, day.max(self.leg().day));
        // drawn: between the two newest marks
        let t1 = self.ahead;
        let t0 = (t1 - STEP).max(self.leg().day);
        let f = if t1 - t0 > 1e-9 { ((day - t0) / (t1 - t0)).clamp(0.0, 1.0) } else { 1.0 };
        for k in 0..WORLDS as usize {
            let a = if self.filled > 1 { self.back(1, k) } else { self.back(0, k) };
            let b = self.back(0, k);
            self.drawn[k] = lerp(a, b, f);
        }
        self.drawn_day = day;
        if let Some(g) = self.pending.take() {
            if !self.replay.is_empty() {
                self.replay.clear(); // a gesture during a replay takes over from there
            }
            let t0 = web_time();
            let (k, matches, near) = self.look_back(g, disc, cam, day);
            let ms = (web_time() - t0) * 1000.0;
            self.cue_world(rules, day, k, g, matches, ms, 0);
            self.add_match_ghost(&near);
        }
        self.bloom = (self.bloom + (1.0 / 60.0 / 0.35) as f32).min(1.0);
        self.ghosts.retain(|g| seconds - g.since < g.secs);
    }

    /// The worlds as the screen shows them, the gesture's match and how many matched, and the
    /// matching worlds (for the fold).
    fn look_back(&self, g: Gesture, disc: &Disc, cam: &Camera, day: f64) -> (u32, u32, Vec<usize>) {
        let screen: Vec<Option<[f64; 2]>> = self
            .drawn
            .iter()
            .map(|&p| {
                let (xy, facing) = disc.project(cam, p);
                (facing > 0.0).then_some(xy)
            })
            .collect();
        let n = WORLDS as usize;
        match g {
            Gesture::Point { x, y } => {
                let mut best = (f64::INFINITY, 0usize);
                let mut near = Vec::new();
                for (k, s) in screen.iter().enumerate() {
                    if let Some([sx, sy]) = s {
                        let d = (sx - x).hypot(sy - y);
                        if d < best.0 {
                            best = (d, k);
                        }
                        if d < 24.0 {
                            near.push(k);
                        }
                    }
                }
                (best.1 as u32 + 1, near.len() as u32, near)
            }
            Gesture::Sweep { dx, dy, secs } => {
                let len = dx.hypot(dy).max(1e-9);
                let (ux, uy) = (dx / len, dy / len);
                let marks = ((secs * RATE / STEP).round() as usize).clamp(1, HISTORY - 1);
                // each world's displacement over the stroke's stretch, less the cloud's own drift
                let disp: Vec<Option<[f64; 2]>> = (0..n)
                    .map(|k| {
                        let then = disc.project(cam, self.back_drawn(marks, k, day)).0;
                        screen[k].map(|s| [s[0] - then[0], s[1] - then[1]])
                    })
                    .collect();
                let (mut mx, mut my, mut c) = (0.0f64, 0.0f64, 0.0f64);
                for d in disp.iter().flatten() {
                    mx += d[0];
                    my += d[1];
                    c += 1.0;
                }
                let (mx, my) = (mx / c.max(1.0), my / c.max(1.0));
                let mut scored: Vec<(f64, usize)> = Vec::new();
                let mut near = Vec::new();
                for (k, d) in disp.iter().enumerate() {
                    let Some(d) = d else { continue };
                    let (rx, ry) = (d[0] - mx, d[1] - my);
                    let along = rx * ux + ry * uy;
                    let perp = (rx * uy - ry * ux).abs();
                    if along > 0.0 && perp < along * 0.577 {
                        near.push(k);
                    }
                    scored.push((along - perp, k));
                }
                scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                // a longer stroke reaches further out along the cloud's edge
                let reach = (len / 300.0).clamp(0.0, 1.0);
                let q = 0.80 + 0.199 * reach;
                let pick = scored.get(((scored.len() as f64 - 1.0) * q) as usize).map_or(0, |s| s.1);
                (pick as u32 + 1, near.len() as u32, near)
            }
            Gesture::Stir { turn, secs } => {
                let marks = ((secs * RATE / STEP).round() as usize).clamp(4, HISTORY - 1);
                let sense = turn.signum();
                let mut scored: Vec<(f64, usize)> = Vec::new();
                let mut near = Vec::new();
                for k in 0..n {
                    let mut pts: Vec<[f64; 2]> = (1..=marks.min(self.filled - 1)).rev().map(|b| disc.project(cam, self.back(b, k)).0).collect();
                    if let Some(s) = screen[k] {
                        pts.push(s);
                    }
                    let mut tsum = 0.0;
                    let mut last: Option<[f64; 2]> = None;
                    for w in pts.windows(2) {
                        let d = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
                        if d[0].hypot(d[1]) < 0.25 {
                            continue;
                        }
                        if let Some(a) = last {
                            tsum += (a[0] * d[1] - a[1] * d[0]).atan2(a[0] * d[0] + a[1] * d[1]);
                        }
                        last = Some(d);
                    }
                    let s = tsum * sense;
                    if s > std::f64::consts::FRAC_PI_2 {
                        near.push(k);
                    }
                    scored.push((s, k));
                }
                scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
                let pick = scored.last().map_or(0, |s| s.1);
                (pick as u32 + 1, near.len() as u32, near)
            }
        }
    }

    /// Where world k (0-based) was `marks` half days before `day` (the leg's start at most).
    fn back_drawn(&self, marks: usize, k: usize, day: f64) -> LonLat {
        let _ = day;
        self.back(marks, k)
    }

    /// The cue: the leg's journey becomes world k's up to `day`, and a new cloud blooms from there.
    #[allow(clippy::too_many_arguments)]
    fn cue_world(&mut self, rules: &Rules, day: f64, k: u32, gesture: Gesture, matches: u32, ms: f64, _x: u32) {
        let leg = self.leg();
        let day = day.max(leg.day);
        let (path, w) = Self::world_at(rules, leg, k, day);
        // the fold: the cloud as it stood, folding into the chosen world
        let paths: Vec<GpuPath> = self.drawn.iter().map(|p| gp(*p)).collect();
        let s = self.slots[1];
        self.ghosts.retain(|g| g.slot != s);
        self.ghosts.push(Ghost { paths, at: w.at, since: self.seconds, secs: if self.lines { 0.5 } else { 0.3 }, slot: s });
        self.take.extend(path.into_iter().skip(1));
        self.cues.push(Cue { day, gesture, k, matches, ms });
        let km = {
            let a = leg.at.to_xyz();
            let b = w.at.to_xyz();
            (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0).acos() * 6371.0
        };
        self.line = if self.replaying {
            format!("again · cue {} · {} · sea {}", self.cues.len(), gesture.name(), k)
        } else if ms > 0.0 {
            format!("{} · {} of {} worlds did it · world {} · {:.0} km since the last cue · look back {:.1} ms", gesture.name(), matches, WORLDS, k, km, ms)
        } else {
            format!("{} · {} of {} possible seas did that · this is sea {}", gesture.name(), matches, WORLDS, k)
        };
        self.cued = true;
        self.begin(Leg { day, at: w.at, eddy: w.eddy });
    }

    fn add_match_ghost(&mut self, near: &[usize]) {
        let (s1, s2) = (self.slots[1], self.slots[2]);
        let Some(g) = self.ghosts.iter().find(|g| g.slot == s1) else { return };
        let paths: Vec<GpuPath> = near.iter().map(|&k| g.paths[k]).collect();
        let at = g.at;
        self.ghosts.retain(|g| g.slot != s2);
        if !paths.is_empty() {
            self.ghosts.push(Ghost { paths, at, since: self.seconds, secs: 0.7, slot: s2 });
        }
    }

    /// The cloud's middle (for the camera; a line's, its lamp).
    pub fn middle(&self) -> LonLat {
        if self.lines {
            return self.drawn[0];
        }
        let mut v = [0.0; 3];
        for p in &self.drawn {
            let x = p.to_xyz();
            v[0] += x[0];
            v[1] += x[1];
            v[2] += x[2];
        }
        LonLat::from_xyz(v)
    }

    /// The buffer's ranges to write this frame: (base, paths).
    pub fn uploads(&self, maybes: u32) -> Vec<(u32, Vec<GpuPath>)> {
        let mut out = Vec::new();
        if !self.lines {
            out.push((self.slots[0] * maybes, self.drawn.iter().map(|p| gp(*p)).collect()));
        }
        for g in &self.ghosts {
            out.push((g.slot * maybes, g.paths.clone()));
        }
        out
    }

    /// The clouds for the look: the live one, and any folding into their cue.
    pub fn clouds(&self, maybes: u32) -> Vec<LookCloud> {
        let mut out = Vec::new();
        for g in &self.ghosts {
            let t = ((self.seconds - g.since) / g.secs).clamp(0.0, 1.0) as f32;
            out.push(LookCloud { id: self.id, at: g.at, origin: g.at, base: g.slot * maybes, count: g.paths.len() as u32, bloom: 1.0 - t, front: false });
        }
        if !self.lines {
            let leg = self.leg();
            out.push(LookCloud { id: self.id, at: leg.at, origin: leg.at, base: self.slots[0] * maybes, count: WORLDS, bloom: self.bloom.max(0.02), front: true });
        }
        out
    }

    /// The lamp: at its last cue, its journey the take so far.
    pub fn lamp(&self) -> LookFloat {
        let mut wake: Vec<LookBead> = self.take.iter().map(|&at| LookBead { at, measured: 1.0 }).collect();
        if self.lines {
            wake.extend(self.prov.iter().skip(1).chain(std::iter::once(&self.drawn[0])).map(|&at| LookBead { at, measured: 1.0 }));
        }
        LookFloat { id: self.id, at: self.lamp_at(), lit: 1.0, held: false, chosen: true, shown: false, followed: true, origin: self.start.at, put_day: self.start.day, wake, measured: 1.0 }
    }
}

/// The conductor's hands and its voices: one lamp's cloud, or a few lamps conducted as lines.
pub struct Orchestra {
    /// Where and when the cloud starts (the lines start beside it).
    pub at: LonLat,
    pub day: f64,
    pub voices: Vec<Conductor>,
    pub lines: bool,
    touch: Option<Touch>,
    pts: Vec<(f64, f64, f64)>,
    pub fermata: bool,
    /// Gestures read, for which voice, made on the next tick.
    pending: Vec<(usize, Gesture)>,
    pub line: String,
    pub cued: bool,
}

/// Where the lines start: three lamps a little apart off Cape Hatteras, in the Gulf Stream.
const LINES: [[f64; 2]; 3] = [[-74.4, 35.0], [-73.4, 35.9], [-72.3, 36.4]];

impl Orchestra {
    pub fn new(rules: &Rules, at: LonLat, day: f64, lines: bool) -> Orchestra {
        let voices = if lines {
            LINES.iter().enumerate().map(|(i, p)| Conductor::new(rules, LonLat::new(p[0], p[1]), day).voice(i as u32 + 1, true, [7, 2 * i as u32, 2 * i as u32 + 1])).collect()
        } else {
            vec![Conductor::new(rules, at, day)]
        };
        Orchestra { at, day, voices, lines, touch: None, pts: Vec::new(), fermata: false, pending: Vec::new(), line: String::new(), cued: false }
    }

    /// The pointer, in points. Returns true while a finger rests (the app holds the date).
    pub fn pointer(&mut self, ev: PointerEvent, lamps: &[[f64; 2]]) {
        match ev.phase {
            Phase::Down => {
                if self.touch.is_none() {
                    self.pts.clear();
                    self.pts.push((ev.x, ev.y, ev.t));
                    self.touch = Some(Touch { id: ev.id, pts_start: 0, t0: ev.t });
                }
            }
            Phase::Move => {
                if self.touch.map_or(false, |t| t.id == ev.id) {
                    self.pts.push((ev.x, ev.y, ev.t));
                }
            }
            Phase::Up | Phase::Cancel => {
                let Some(t) = self.touch.filter(|t| t.id == ev.id) else { return };
                self.touch = None;
                self.pts.push((ev.x, ev.y, ev.t));
                if self.fermata {
                    self.fermata = false;
                    self.line = "…and on".into();
                    return;
                }
                if ev.phase == Phase::Cancel {
                    return;
                }
                let _ = t.pts_start;
                let g = Self::read(&self.pts, ev.t - t.t0);
                // a line's gesture goes to the lamp it began nearest; begun away from them all, to
                // every lamp at once (tutti)
                let (x0, y0, _) = self.pts[0];
                let near = lamps.iter().enumerate().map(|(i, p)| ((p[0] - x0).hypot(p[1] - y0), i)).fold((f64::INFINITY, 0), |a, b| if b.0 < a.0 { b } else { a });
                self.pending = if lamps.len() > 1 && near.0 > 90.0 { (0..lamps.len()).map(|i| (i, g)).collect() } else { vec![(near.1, g)] };
            }
        }
    }

    /// The baton: a swing of the phone, as a sweep on the screen (points), `secs` long.
    pub fn swing(&mut self, dx: f64, dy: f64, secs: f64) {
        self.pending = vec![(0, Gesture::Sweep { dx, dy, secs })];
    }

    /// What a stroke was.
    fn read(pts: &[(f64, f64, f64)], secs: f64) -> Gesture {
        let (x0, y0, _) = pts[0];
        let (x1, y1, _) = *pts.last().unwrap();
        let mut length = 0.0;
        let mut turn = 0.0;
        let mut last: Option<(f64, f64)> = None;
        let mut prev = (x0, y0);
        for &(x, y, _) in &pts[1..] {
            let (dx, dy) = (x - prev.0, y - prev.1);
            let d = dx.hypot(dy);
            if d < 4.0 {
                continue;
            }
            length += d;
            if let Some((ax, ay)) = last {
                let a = (ax * dy - ay * dx).atan2(ax * dx + ay * dy);
                turn += a;
            }
            last = Some((dx, dy));
            prev = (x, y);
        }
        if turn.abs() > 1.5 * std::f64::consts::PI {
            Gesture::Stir { turn, secs }
        } else if length > 24.0 {
            Gesture::Sweep { dx: x1 - x0, dy: y1 - y0, secs }
        } else {
            Gesture::Point { x: x0, y: y0 }
        }
    }

    /// Stands the date while a finger rests unmoved (a fermata).
    pub fn resting(&mut self, now: f64) -> bool {
        if let Some(t) = self.touch {
            let (x0, y0, _) = self.pts[0];
            let moved = self.pts.iter().any(|&(x, y, _)| (x - x0).hypot(y - y0) > 10.0);
            if !moved && now - t.t0 > 0.35 && !self.fermata {
                self.fermata = true;
                self.line = "hold: the sea waits while your finger rests".into();
            }
        }
        self.fermata
    }

    pub fn tick(&mut self, rules: &Rules, day: f64, seconds: f64, disc: &Disc, cam: &Camera) {
        for (i, g) in std::mem::take(&mut self.pending) {
            if let Some(v) = self.voices.get_mut(i) {
                v.cue(g);
            }
        }
        for v in &mut self.voices {
            v.tick(rules, day, seconds, disc, cam);
            if std::mem::take(&mut v.cued) {
                self.line = v.line.clone();
                self.cued = true;
            }
        }
    }

    pub fn again(&mut self, rules: &Rules) -> f64 {
        let mut day = 0.0;
        for v in &mut self.voices {
            day = v.again(rules);
        }
        self.line = "again: the same take, cue for cue".into();
        day
    }

    pub fn fresh(&mut self, rules: &Rules) -> f64 {
        let mut day = 0.0;
        for v in &mut self.voices {
            day = v.fresh(rules);
        }
        self.line = String::new();
        day
    }

    /// Every lamp's place on the screen (for routing a line's gesture).
    pub fn lamps_on(&self, disc: &Disc, cam: &Camera) -> Vec<[f64; 2]> {
        self.voices.iter().map(|v| disc.project(cam, v.lamp_at()).0).collect()
    }

    /// What the camera holds: the cloud's middle, or the lamps' middle.
    pub fn middle(&self) -> LonLat {
        let mut m = [0.0; 3];
        for v in &self.voices {
            let x = v.middle().to_xyz();
            m[0] += x[0];
            m[1] += x[1];
            m[2] += x[2];
        }
        LonLat::from_xyz(m)
    }

    pub fn uploads(&self, maybes: u32) -> Vec<(u32, Vec<GpuPath>)> {
        self.voices.iter().flat_map(|v| v.uploads(maybes)).collect()
    }

    pub fn clouds(&self, maybes: u32) -> Vec<LookCloud> {
        self.voices.iter().flat_map(|v| v.clouds(maybes)).collect()
    }

    pub fn lamps(&self) -> Vec<LookFloat> {
        self.voices.iter().map(|v| v.lamp()).collect()
    }
}

fn gp(p: LonLat) -> GpuPath {
    GpuPath { lon: p.lon as f32, lat: p.lat as f32, held: 0, measured: 1.0, eddy_u: 0.0, eddy_v: 0.0 }
}

fn lerp(a: LonLat, b: LonLat, f: f64) -> LonLat {
    let mut dlon = b.lon - a.lon;
    if dlon > 180.0 {
        dlon -= 360.0;
    } else if dlon < -180.0 {
        dlon += 360.0;
    }
    let mut lon = a.lon + dlon * f;
    if lon > 180.0 {
        lon -= 360.0;
    } else if lon < -180.0 {
        lon += 360.0;
    }
    LonLat { lon, lat: a.lat + (b.lat - a.lat) * f }
}

/// Seconds, for the look back's cost on native (the web shell times its frame instead).
fn web_time() -> f64 {
    #[cfg(target_arch = "wasm32")]
    {
        0.0
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::DirAssets;
    use crate::model::Camera;
    use std::path::Path;

    /// A take of three gestures (tap, sweep, circle) on the lines, then `again`: the same cues pick
    /// the same seas and every lamp's journey comes back the same, to the last place.
    #[test]
    fn again_replays_the_take() {
        let dir = std::env::var_os("ADRIFT_ASSETS").map(std::path::PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/assets"));
        let rules = Rules::load(&DirAssets(dir)).unwrap();
        let disc = Disc { cx: 196.0, cy: 426.0, r: 180.0 * 4.5 };
        let cam = Camera::new(-70.0, 37.0, 4.5);
        let play = |o: &mut Orchestra, gestures: bool| {
            let mut day = 160.0;
            for frame in 0..600 {
                let t = frame as f64 / 60.0;
                day += RATE / 60.0;
                if gestures {
                    let lamps = o.lamps_on(&disc, &cam);
                    let at = lamps[frame % 3];
                    let g = match frame {
                        100 => Some(Gesture::Point { x: at[0] + 20.0, y: at[1] }),
                        250 => Some(Gesture::Sweep { dx: 120.0, dy: -40.0, secs: 0.3 }),
                        400 => Some(Gesture::Stir { turn: 7.0, secs: 0.6 }),
                        _ => None,
                    };
                    if let Some(g) = g {
                        o.voices[frame % 3].cue(g);
                    }
                }
                o.tick(&rules, day, t, &disc, &cam);
            }
            o.voices.iter().map(|v| v.lamp().wake.iter().map(|b| (b.at.lon, b.at.lat)).collect::<Vec<_>>()).collect::<Vec<_>>()
        };
        let mut o = Orchestra::new(&rules, LonLat::new(-74.0, 35.6), 160.0, true);
        let first = play(&mut o, true);
        let cues: usize = o.voices.iter().map(|v| v.cues.len()).sum();
        assert_eq!(cues, 3);
        o.again(&rules);
        let second = play(&mut o, false);
        assert_eq!(first, second);
    }
}
