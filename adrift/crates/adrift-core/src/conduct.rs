//! Conducting (R11, Engineering; the chooser's honesty is Truth's this round; painting-company row
//! 199). Step 2 of the studio, grown from the conductor spike (branch `adrift-conductor-spike`,
//! `adrift/CONDUCTOR.md`): while a take is recorded, the hand conducts the lamps. Every lamp's
//! current leg has `WORLDS` possible seas, the worlds `k = 1..=WORLDS` of the sea's own rules (the
//! maybes' law: the same `Rules::flight` on the same quarter-day grid as the lamp's own path, the
//! wander's kicks the world's, shared by every lamp in it). They are stepped here on the CPU as the
//! date runs, and a gesture looks back over them and picks the one that did what the hand did:
//!
//! - **tap**: the world drawn nearest the finger;
//! - **sweep**: the world that, over the last stretch as long as the stroke, veered most the
//!   stroke's way from the others' drift (a longer stroke reaches further out);
//! - **circle** (three quarters of a turn or more): the world whose path turned most the same way.
//!
//! The cue rewrites the lamp's leg to that world (`Leg::world`), so its journey since the last cue
//! becomes that world's path, and a new leg begins where it now is, carrying its eddy velocity
//! (`Leg::eddy`), so the continuation is the same law as one unbroken flight. A take replays its
//! inputs, so a conducted take exports exactly as it was conducted.

use crate::model::{ConductGesture, Leg, LonLat};
use crate::sea::{Drift, Rules};
use std::collections::VecDeque;

/// The worlds a lamp chooses among.
pub const WORLDS: u32 = 2048;
/// Quarter-day marks kept for the look back (16 days).
pub const HISTORY: usize = 65;
/// A tap counts the worlds within this many points of the finger as having done it.
pub const TAP_NEAR: f64 = 24.0;

#[derive(Clone, Copy, Debug)]
pub struct W {
    pub at: LonLat,
    pub eddy: (f32, f32),
}

/// One quarter-day mark of every world.
struct Mark {
    day: f64,
    w: Vec<W>,
}

/// A lamp's worlds on its current leg.
pub struct Worlds {
    pub leg: Leg,
    pub leg_index: usize,
    marks: VecDeque<Mark>,
}

/// What a cue chose: world `k` (1-based, as `Leg::world`), how many worlds did what the hand did, and
/// which (for the look's flash).
#[derive(Clone, Debug)]
pub struct Choice {
    pub k: u32,
    pub matches: u32,
    pub near: Vec<u32>,
}

impl Worlds {
    /// A leg's worlds at its start: its place, each world's first eddy as the lamp's own path takes it
    /// (`Leg::eddy` when the leg carries one, else the world's own put).
    pub fn new(rules: &Rules, leg: Leg, leg_index: usize) -> Worlds {
        let w = (1..=WORLDS)
            .map(|k| {
                let e = match leg.eddy {
                    Some(e) => (e[0], e[1]),
                    None => rules.put(leg.at, leg.day, leg.seed, k).eddy,
                };
                W { at: leg.at, eddy: e }
            })
            .collect();
        let mut marks = VecDeque::new();
        marks.push_back(Mark { day: leg.day, w });
        Worlds { leg, leg_index, marks }
    }

    /// Steps every world to the first grid mark at or after `day` (unbudgeted, so a replay steps
    /// alike), keeping `HISTORY` marks.
    pub fn step_to(&mut self, rules: &Rules, day: f64) {
        loop {
            let last = self.marks.back().unwrap();
            if last.day >= day - 1e-9 {
                break;
            }
            let next = crate::app::grid_next(last.day);
            let dt = next - last.day;
            let w: Vec<W> = last
                .w
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    let s = rules.flight(Drift { at: x.at, eddy: x.eddy }, last.day, dt, self.leg.seed, i as u32 + 1);
                    W { at: s.drift.at, eddy: s.drift.eddy }
                })
                .collect();
            self.marks.push_back(Mark { day: next, w });
            while self.marks.len() > HISTORY {
                self.marks.pop_front();
            }
        }
    }

    /// The index of the newest mark at or before `day`.
    fn at_or_before(&self, day: f64) -> usize {
        self.marks.iter().rposition(|m| m.day <= day + 1e-9).unwrap_or(0)
    }

    /// World `k`'s place (1-based) on `day`, between its marks as the lamp's own is, and the eddy it
    /// carries there (the mark's at or before).
    pub fn place(&self, k: u32, day: f64) -> W {
        let i = self.at_or_before(day);
        let a = &self.marks[i];
        let wa = a.w[(k - 1) as usize];
        match self.marks.get(i + 1) {
            Some(b) if b.day > a.day => {
                let f = ((day - a.day) / (b.day - a.day)).clamp(0.0, 1.0);
                W { at: crate::app::lerp_place(wa.at, b.w[(k - 1) as usize].at, f), eddy: wa.eddy }
            }
            _ => wa,
        }
    }

    /// Every world's place on `day` (for the look's flash), 1-based order.
    pub fn places(&self, day: f64) -> Vec<LonLat> {
        (1..=WORLDS).map(|k| self.place(k, day).at).collect()
    }

    /// World k's place `marks` quarter days before the mark at or before `day` (the oldest kept at
    /// most).
    fn back(&self, marks: usize, k: usize, day: f64) -> LonLat {
        let i = self.at_or_before(day).saturating_sub(marks);
        self.marks[i].w[k].at
    }

    /// The world that did what the hand did on `day`. `project` puts a place on the screen (None
    /// when it faces away); `rate` is the date's days a second (a stroke's seconds become days).
    pub fn choose(&self, g: ConductGesture, day: f64, rate: f64, project: &dyn Fn(LonLat) -> Option<[f64; 2]>) -> Option<Choice> {
        let n = WORLDS as usize;
        let now: Vec<Option<[f64; 2]>> = (1..=WORLDS).map(|k| project(self.place(k, day).at)).collect();
        let marks_for = |secs: f64, least: usize| ((secs * rate * 4.0).round() as usize).clamp(least, HISTORY - 1);
        let pick = |scored: &mut Vec<(f64, usize)>, q: f64| {
            scored.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
            scored.get(((scored.len() as f64 - 1.0) * q).round() as usize).map(|s| s.1)
        };
        match g {
            ConductGesture::Tap { x, y } => {
                let mut best: Option<(f64, usize)> = None;
                let mut near = Vec::new();
                for (k, s) in now.iter().enumerate() {
                    let Some([sx, sy]) = s else { continue };
                    let d = (sx - x).hypot(sy - y);
                    if best.map_or(true, |b| d < b.0) {
                        best = Some((d, k));
                    }
                    if d < TAP_NEAR {
                        near.push(k as u32 + 1);
                    }
                }
                best.map(|b| Choice { k: b.1 as u32 + 1, matches: near.len() as u32, near })
            }
            ConductGesture::Sweep { dx, dy, secs } => {
                let len = dx.hypot(dy).max(1e-9);
                let (ux, uy) = (dx / len, dy / len);
                let m = marks_for(secs, 1);
                // each world's displacement over the stroke's stretch, less the worlds' mean drift
                let disp: Vec<Option<[f64; 2]>> = (0..n).map(|k| match (now[k], project(self.back(m, k, day))) {
                    (Some(s), Some(t)) => Some([s[0] - t[0], s[1] - t[1]]),
                    _ => None,
                }).collect();
                let (mut mx, mut my, mut c) = (0.0, 0.0, 0.0);
                for d in disp.iter().flatten() {
                    mx += d[0];
                    my += d[1];
                    c += 1.0;
                }
                if c == 0.0 {
                    return None;
                }
                let (mx, my) = (mx / c, my / c);
                let mut scored = Vec::new();
                let mut near = Vec::new();
                for (k, d) in disp.iter().enumerate() {
                    let Some(d) = d else { continue };
                    let (rx, ry) = (d[0] - mx, d[1] - my);
                    let along = rx * ux + ry * uy;
                    let perp = (rx * uy - ry * ux).abs();
                    if along > 0.0 && perp < along * 0.577 {
                        near.push(k as u32 + 1);
                    }
                    scored.push((along - perp, k));
                }
                // a longer stroke reaches further out along the cloud's edge
                let reach = (len / 300.0).clamp(0.0, 1.0);
                pick(&mut scored, 0.80 + 0.199 * reach).map(|k| Choice { k: k as u32 + 1, matches: near.len() as u32, near })
            }
            ConductGesture::Circle { turn, secs } => {
                let m = marks_for(secs, 4);
                let sense = turn.signum();
                let mut scored = Vec::new();
                let mut near = Vec::new();
                for k in 0..n {
                    let mut pts: Vec<[f64; 2]> = (1..=m).rev().filter_map(|b| project(self.back(b, k, day))).collect();
                    if let Some(s) = now[k] {
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
                        near.push(k as u32 + 1);
                    }
                    scored.push((s, k));
                }
                pick(&mut scored, 1.0).map(|k| Choice { k: k as u32 + 1, matches: near.len() as u32, near })
            }
        }
    }
}
