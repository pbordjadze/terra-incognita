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

/// The worlds a lamp chooses among: 1024, so six lamps conducted at up to 3 days a second step
/// inside about 2 ms a frame on a phone (R11, Truth; `conduct_cost`, CONDUCTOR.md › In the app): a
/// flight step is 0.48 us natively, assumed 2-3x slower on the phone; 1024 worlds / 5 frames a mark
/// x 6 lamps is 1230 steps a frame, 0.59 ms natively, 1.2-1.8 ms on the phone. A gesture matches the
/// same share of them as of the spike's 2048 (5-25%); a sweep's furthest pick is the top one, not two.
pub const WORLDS: u32 = 1024;
/// The date's days a second while a take records and the hand conducts (Truth's measure, R11:
/// at 3 a one-second stroke chooses among worlds about 35 points apart; 2 is too tight, 6 blurs).
pub const TEMPO: f64 = 3.0;
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

/// A grid mark beyond the date being stepped a share at a time (`Worlds::prepare`), so a mark's
/// cost is spread over the frames before the date reaches it instead of falling on one.
struct Pending {
    day: f64,
    w: Vec<W>,
}

/// A lamp's worlds on its current leg.
pub struct Worlds {
    pub leg: Leg,
    pub leg_index: usize,
    marks: VecDeque<Mark>,
    pending: Option<Pending>,
}

/// A mark is prepared over this many frames (R11, Truth: at 3 days a second and 60 frames a second
/// the date crosses a mark every 5 frames, at 2 every 7.5, so the next is ready; a late frame or a
/// faster date finishes it in `step_to`, unbudgeted, so a replay is the same whatever the frames).
pub const PREPARE_FRAMES: u32 = 5;

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
        Worlds { leg, leg_index, marks, pending: None }
    }

    /// Steps every world to the first grid mark at or after `day` (unbudgeted, so a replay steps
    /// alike; what `prepare` already stepped is kept), keeping `HISTORY` marks.
    pub fn step_to(&mut self, rules: &Rules, day: f64) {
        while self.marks.back().unwrap().day < day - 1e-9 {
            self.advance(rules, usize::MAX);
        }
    }

    /// Steps a share of the next mark beyond the date (`WORLDS / PREPARE_FRAMES` worlds), once a
    /// frame: the same steps `step_to` would take, earlier. Returns the worlds stepped.
    pub fn prepare(&mut self, rules: &Rules) -> usize {
        self.advance(rules, WORLDS.div_ceil(PREPARE_FRAMES) as usize)
    }

    /// Steps up to `budget` worlds of the next mark; completes it when all are stepped.
    fn advance(&mut self, rules: &Rules, budget: usize) -> usize {
        let last = self.marks.back().unwrap();
        let next = crate::app::grid_next(last.day);
        let dt = next - last.day;
        let p = self.pending.get_or_insert_with(|| Pending { day: next, w: Vec::with_capacity(WORLDS as usize) });
        let from = p.w.len();
        let to = (from + budget.min(WORLDS as usize)).min(WORLDS as usize);
        for (i, x) in last.w[from..to].iter().enumerate() {
            let s = rules.flight(Drift { at: x.at, eddy: x.eddy }, last.day, dt, self.leg.seed, (from + i) as u32 + 1);
            p.w.push(W { at: s.drift.at, eddy: s.drift.eddy });
        }
        if p.w.len() == WORLDS as usize {
            let p = self.pending.take().unwrap();
            self.marks.push_back(Mark { day: p.day, w: p.w });
            while self.marks.len() > HISTORY {
                self.marks.pop_front();
            }
        }
        to - from
    }

    /// Whether these are the worlds of leg `li` as `leg` now reads, whichever world a cue has since
    /// given it (`Leg::world` names which of these the lamp follows; the worlds themselves are the
    /// same), so a cue does not rebuild them from the leg's start.
    pub fn serves(&self, leg: &Leg, li: usize) -> bool {
        self.leg_index == li && Leg { world: leg.world, ..self.leg } == *leg
    }

    /// Where a cue on `day` cuts the leg for world `k` (1-based): the first quarter-day mark at or
    /// after `day` (stepped already by `step_to(day)`), its day and world k's exact state there.
    /// The lamp follows world k's own line to that mark and the next leg begins on it with that
    /// place and eddy, in world k, so the lamp's path is world k's step for step, bit for bit, until
    /// the next cue: nothing interpolated becomes a leg's start (R11, Truth).
    pub fn cut(&self, k: u32, day: f64) -> (f64, W) {
        let i = self.marks.iter().position(|m| m.day >= day - 1e-9).unwrap_or(self.marks.len() - 1);
        let m = &self.marks[i];
        (m.day, m.w[(k - 1) as usize])
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

/// The conductor held to its honesty (R11, Truth): the App driven as a take is, with its lamps'
/// legs read back and recomputed from the sea's own rules. `cargo test -p adrift-core conduct`
/// (needs a GPU adapter: llvmpipe will do; without one the App tests say so and pass).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{lerp_place, App, Scene};
    use crate::assets::DirAssets;
    use crate::gpu::Gpu;
    use crate::model::{ConductGesture, Intent};
    use std::path::{Path, PathBuf};

    fn assets() -> PathBuf {
        std::env::var_os("ADRIFT_ASSETS").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/assets"))
    }

    fn block<F: std::future::Future>(f: F) -> F::Output {
        use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
        fn noop(_: *const ()) {}
        fn clone(p: *const ()) -> RawWaker {
            RawWaker::new(p, &VTABLE)
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
        let waker = unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &VTABLE)) };
        let mut cx = Context::from_waker(&waker);
        let mut f = std::pin::pin!(f);
        loop {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                return v;
            }
            std::thread::yield_now();
        }
    }

    /// An App on the conduct plan's scene (two lamps off Hatteras in a typical June), or None
    /// without a GPU.
    fn app() -> Option<App> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { backends: wgpu::Backends::VULKAN | wgpu::Backends::METAL, ..Default::default() });
        let gpu = match block(Gpu::new(&instance, None)) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("no GPU here ({e}); the App is not checked");
                return None;
            }
        };
        let mut app = App::new(gpu, Box::new(DirAssets(assets())), 393, 852, 3.0).unwrap();
        let scene: Scene = serde_json::from_str(
            r#"{"camera": {"lon": -66.0, "lat": 35.0, "zoom": 3.0}, "day": 160.0,
                "floats": [{"lon": -72.0, "lat": 35.0, "days_ago": 10}, {"lon": -62.0, "lat": 37.0, "days_ago": 6}]}"#,
        )
        .unwrap();
        app.set_scene(&scene);
        Some(app)
    }

    /// A leg's own flight in world `k` on the quarter-day grid to `upto`: (day, place, eddy) at every
    /// mark, as the lamp's path and the worlds compute it.
    fn flight(rules: &Rules, leg: Leg, k: u32, upto: f64) -> Vec<(f64, LonLat, (f32, f32))> {
        let e = leg.eddy.map(|e| (e[0], e[1])).unwrap_or_else(|| rules.put(leg.at, leg.day, leg.seed, k).eddy);
        let mut out = vec![(leg.day, leg.at, e)];
        while out.last().unwrap().0 < upto - 1e-9 {
            let (day, at, eddy) = *out.last().unwrap();
            let next = crate::app::grid_next(day);
            let s = rules.flight(Drift { at, eddy }, day, next - day, leg.seed, k);
            out.push((next, s.drift.at, s.drift.eddy));
        }
        out
    }

    fn place_on(path: &[(f64, LonLat, (f32, f32))], day: f64) -> LonLat {
        let i = path.iter().rposition(|p| p.0 <= day + 1e-9).unwrap();
        match path.get(i + 1) {
            Some(b) => lerp_place(path[i].1, b.1, ((day - path[i].0) / (b.0 - path[i].0)).clamp(0.0, 1.0)),
            None => path[i].1,
        }
    }

    /// Frames at 60 a second from `t0` to `t1` (the shell's clock), calling `each` after every one.
    fn run(app: &mut App, t0: f64, t1: f64, mut each: impl FnMut(&App)) -> f64 {
        let mut t = t0;
        while t < t1 - 1e-9 {
            t += 1.0 / 60.0;
            app.advance(t);
            each(app);
        }
        t
    }

    #[test]
    fn a_take_start_is_invisible() {
        // the same scene run twice, once with a take begun at an off-grid moment: every frame's
        // lamps are where they would have been, to the bit, and the cut sits on the grid
        let (Some(mut plain), Some(mut taken)) = (app(), app()) else { return };
        let (mut a, mut b) = (Vec::new(), Vec::new());
        let t = run(&mut plain, 0.0, 0.53, |_| {});
        run(&mut taken, 0.0, 0.53, |_| {});
        taken.command(Intent::Record { on: true });
        // a take runs at the conductor's tempo: the plain run is set to it too
        plain.clock.rate = TEMPO;
        let day = taken.clock.day;
        assert!((day * 4.0).fract().abs() > 1e-6, "the take starts between grid marks ({day})");
        for f in &taken.floats {
            let cut = f.legs.last().unwrap();
            assert!(cut.day >= day && cut.day - day < 0.25 + 1e-9 && (cut.day * 4.0).fract() == 0.0, "the cut is the next grid mark: {cut:?} at {day}");
        }
        run(&mut plain, t, 4.0, |app| a.push(app.floats.iter().map(|f| (f.at, f.eddy)).collect::<Vec<_>>()));
        run(&mut taken, t, 4.0, |app| b.push(app.floats.iter().map(|f| (f.at, f.eddy)).collect::<Vec<_>>()));
        assert_eq!(a, b, "every frame of 3.5 s (7 days) after the take began: the lamps as they were");
    }

    #[test]
    fn after_a_cue_the_lamp_is_the_chosen_world() {
        let Some(mut app) = app() else { return };
        run(&mut app, 0.0, 0.3, |_| {});
        app.command(Intent::Record { on: true });
        let t = run(&mut app, 0.3, 1.71, |_| {});
        let day = app.clock.day;
        let before = app.floats[0].legs.clone();
        app.command(Intent::Conduct { float: Some(1), gesture: ConductGesture::Sweep { dx: 0.0, dy: -160.0, secs: 0.4 } });
        let legs = app.floats[0].legs.clone();
        let li = before.len() - 1;
        let (leg, next) = (legs[li], legs[li + 1]);
        let k = leg.world;
        assert!(k >= 1 && k <= WORLDS, "the cue chose a world: {legs:?}");
        assert_eq!(Leg { world: before[li].world, ..leg }, before[li], "the leg is the same leg, now world {k}");
        assert_eq!(next.world, k, "the next leg goes on in world {k}");
        // world k's unbroken flight from the leg's start: the cut is its mark, to the bit
        let rules = &app.rules;
        let world = flight(rules, leg, k, day + 4.0 * TEMPO);
        let mark = world.iter().find(|p| (p.0 - next.day).abs() < 1e-9).expect("the cut is on world k's grid");
        assert!(next.day >= day && next.day - day < 0.25 + 1e-9);
        assert_eq!((mark.1, mark.2), (next.at, (next.eddy.unwrap()[0], next.eddy.unwrap()[1])), "the next leg starts in world {k}'s exact state");
        // and the lamp, frame by frame for 3 s (6 days), is world k's place
        let mut worst = 0.0f64;
        run(&mut app, t, t + 3.0, |app| {
            let f = &app.floats[0];
            let w = place_on(&world, app.clock.day);
            worst = worst.max((f.at.lon - w.lon).abs().max((f.at.lat - w.lat).abs()));
        });
        eprintln!("cue on day {day:.3} chose world {k} of {WORLDS}; cut at {:.2}; the lamp's worst gap from world {k}'s unbroken flight over 3 s: {worst:e} degrees", next.day);
        assert!(worst == 0.0, "the lamp is world {k}'s path exactly");
    }

    /// What the old off-grid cut (a leg begun at the lamp's interpolated place and its last mark's
    /// eddy) did to a lamp: printed, for CONDUCTOR.md.
    #[test]
    fn the_old_cut_moved_a_lamp_by() {
        let rules = Rules::load(&DirAssets(assets())).unwrap();
        let (mut worst, mut sum, mut n) = (0.0f64, 0.0, 0.0);
        for (i, (lon, lat)) in [(-72.0, 35.0), (-62.0, 37.0), (-40.0, 40.0), (-20.0, -30.0), (60.0, -45.0), (150.0, 35.0)].into_iter().enumerate() {
            let leg = Leg::new(LonLat { lon, lat }, 150.0, i as u64);
            let unbroken = flight(&rules, leg, 0, 170.0);
            for d in [153.1, 155.37, 158.2] {
                let j = unbroken.iter().rposition(|p| p.0 <= d).unwrap();
                let cut = Leg { at: place_on(&unbroken, d), day: d, seed: leg.seed, world: 0, eddy: Some([unbroken[j].2 .0, unbroken[j].2 .1]) };
                let after = flight(&rules, cut, 0, d + 7.0);
                let (a, b) = (place_on(&after, d + 7.0), place_on(&unbroken, d + 7.0));
                let km = crate::sea::drifters::distance_km(a.lon as f32, a.lat as f32, b.lon as f32, b.lat as f32) as f64;
                worst = worst.max(km);
                sum += km;
                n += 1.0;
            }
        }
        eprintln!("the old cut: a lamp 7 days later was {:.1} km (mean) and {worst:.1} km (worst) from where it would have been", sum / n);
    }

    /// A lamp's worlds off Hatteras in a typical June, stepped `days`, and a flat projection at the
    /// conduct plan's scale (about 30 points a degree, y down).
    fn worlds(days: f64) -> (Rules, Worlds, f64) {
        let rules = Rules::load(&DirAssets(assets())).unwrap();
        let leg = Leg::new(LonLat { lon: -72.0, lat: 35.0 }, 160.0, 7);
        let mut w = Worlds::new(&rules, leg, 0);
        let day = 160.0 + days;
        w.step_to(&rules, day);
        (rules, w, day)
    }

    fn flat(p: LonLat) -> Option<[f64; 2]> {
        Some([p.lon * 30.0 * 35f64.to_radians().cos(), -p.lat * 30.0])
    }

    #[test]
    fn a_tap_picks_a_world_under_the_finger() {
        let (_, w, day) = worlds(4.1);
        let places = w.places(day);
        let n = places.len();
        let mut counts = Vec::new();
        for (i, j) in [(17usize, 4.0), (600, -9.0), (1000, 15.0)] {
            let [x, y] = flat(places[i]).unwrap();
            let c = w.choose(ConductGesture::Tap { x: x + j, y: y - j }, day, 2.0, &flat).unwrap();
            let [cx, cy] = flat(w.place(c.k, day).at).unwrap();
            assert!((cx - x - j).hypot(cy - y + j) <= j.abs() * 2f64.sqrt() + 1e-9, "the pick is at least as near as the world tapped");
            assert!(c.matches >= 1 && c.near.contains(&c.k));
            counts.push(c.matches);
        }
        // far from every world: still the nearest, but none did it
        let c = w.choose(ConductGesture::Tap { x: 1e5, y: 0.0 }, day, 2.0, &flat).unwrap();
        assert_eq!(c.matches, 0);
        eprintln!("taps matched {counts:?} of {n} worlds (within {TAP_NEAR} pt)");
    }

    #[test]
    fn a_sweep_picks_a_world_that_moved_the_strokes_way() {
        let (_, w, day) = worlds(6.0);
        let mut fr = Vec::new();
        for (dx, dy) in [(0.0, -160.0), (160.0, 0.0), (0.0, 120.0), (-200.0, 40.0)] {
            let secs = 0.5;
            let c = w.choose(ConductGesture::Sweep { dx, dy, secs }, day, 2.0, &flat).unwrap();
            let m = ((secs * 2.0 * 4.0) as usize).max(1);
            let disp = |k: usize| {
                let (a, b) = (flat(w.place(k as u32 + 1, day).at).unwrap(), flat(w.back(m, k, day)).unwrap());
                [a[0] - b[0], a[1] - b[1]]
            };
            let all: Vec<_> = (0..WORLDS as usize).map(disp).collect();
            let mean = [all.iter().map(|d| d[0]).sum::<f64>() / all.len() as f64, all.iter().map(|d| d[1]).sum::<f64>() / all.len() as f64];
            let d = disp(c.k as usize - 1);
            let len = dx.hypot(dy);
            let along = ((d[0] - mean[0]) * dx + (d[1] - mean[1]) * dy) / len;
            let alongs: Vec<f64> = all.iter().map(|e| ((e[0] - mean[0]) * dx + (e[1] - mean[1]) * dy) / len).collect();
            let beaten = alongs.iter().filter(|a| **a < along).count() as f64 / alongs.len() as f64;
            assert!(along > 0.0 && beaten > 0.7, "the pick moved the stroke's way against the mean: along {along:.1} pt, beyond {beaten:.2} of the worlds");
            fr.push(c.matches as f64 / WORLDS as f64);
        }
        eprintln!("sweeps matched {:?} of the worlds (within 30 degrees of the stroke)", fr.iter().map(|f| (f * 1000.0).round() / 10.0).collect::<Vec<_>>());
        assert!(fr.iter().all(|f| *f > 0.03 && *f < 0.4));
    }

    #[test]
    fn a_circle_picks_a_world_that_turned_that_way() {
        let (_, w, day) = worlds(8.0);
        for turn in [5.0, -5.0] {
            let secs = 1.0;
            let c = w.choose(ConductGesture::Circle { turn, secs }, day, 2.0, &flat).unwrap();
            let m = (secs * 2.0 * 4.0) as usize;
            let mut pts: Vec<[f64; 2]> = (1..=m).rev().map(|b| flat(w.back(b, c.k as usize - 1, day)).unwrap()).collect();
            pts.push(flat(w.place(c.k, day).at).unwrap());
            let mut t = 0.0;
            for q in pts.windows(3) {
                let (a, b) = ([q[1][0] - q[0][0], q[1][1] - q[0][1]], [q[2][0] - q[1][0], q[2][1] - q[1][1]]);
                t += (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1]);
            }
            assert!(t * turn.signum() > std::f64::consts::FRAC_PI_2, "the pick turned the circle's way: {t:.2} rad");
            let f = c.matches as f64 / WORLDS as f64;
            eprintln!("a circle of sense {turn:+}: the pick turned {t:+.2} rad; {:.1}% of the worlds turned a quarter or more that way", f * 100.0);
            assert!(f > 0.01 && f < 0.4);
        }
    }

    /// What conducting costs on the CPU (release: `cargo test --release -p adrift-core conduct_cost
    /// -- --ignored --nocapture`): a quarter-day mark of every world, and each gesture's look back.
    #[test]
    #[ignore]
    fn conduct_cost() {
        let (rules, mut w, day) = worlds(16.0);
        let t = std::time::Instant::now();
        let mut d = day;
        for _ in 0..40 {
            d += 0.25;
            w.step_to(&rules, d);
        }
        let mark = t.elapsed().as_secs_f64() / 40.0;
        eprintln!("a mark of {WORLDS} worlds: {:.2} ms ({:.0} ns a flight step)", mark * 1e3, mark / WORLDS as f64 * 1e9);
        let pts: Vec<(f32, f32)> = w.places(d).iter().map(|p| (p.lon as f32, p.lat as f32)).collect();
        let time = |name: &str, f: &dyn Fn(f32, f32) -> f32| {
            let t = std::time::Instant::now();
            let mut acc = 0.0;
            for _ in 0..20 {
                for &(lon, lat) in &pts {
                    acc += f(lon, lat);
                }
            }
            std::hint::black_box(acc);
            eprintln!("  {name}: {:.0} ns", t.elapsed().as_secs_f64() / 20.0 / pts.len() as f64 * 1e9);
        };
        let df = d as f32;
        time("currents", &|lon, lat| rules.currents.at(df, lon, lat).0);
        time("wind", &|lon, lat| rules.wind.at(df, lon, lat).0);
        time("eddy", &|lon, lat| rules.eddy.at(lon, lat).0);
        time("eddy time", &|lon, lat| rules.eddy_time.at(lon, lat).0);
        time("ice", &|lon, lat| rules.ice.at(df, lon, lat));
        time("coast", &|lon, lat| rules.coast.distance(lon, lat));
        time("world kick", &|lon, lat| crate::sea::rules::world_kick(5, df, 0.25, lon, lat).0);
        for (name, g) in [
            ("tap", ConductGesture::Tap { x: -1700.0, y: -1100.0 }),
            ("sweep", ConductGesture::Sweep { dx: 0.0, dy: -160.0, secs: 0.5 }),
            ("circle 1 s", ConductGesture::Circle { turn: 5.0, secs: 1.0 }),
            ("circle 3 s", ConductGesture::Circle { turn: 5.0, secs: 3.0 }),
        ] {
            let t = std::time::Instant::now();
            for _ in 0..10 {
                std::hint::black_box(w.choose(g, d, 2.0, &flat));
            }
            eprintln!("choose {name}: {:.2} ms", t.elapsed().as_secs_f64() / 10.0 * 1e3);
        }
    }

    /// The tempo (R11): at each date rate, what the worlds do in a stroke's half second and second
    /// off Hatteras at the conduct plan's scale (~30 pt a degree): their spread about the mean
    /// drift (RMS, points), the mean drift itself, how far out the sweep's 0.95 pick lies, and the
    /// share that turned a quarter or more in the second. `--ignored --nocapture` prints the table
    /// in CONDUCTOR.md.
    #[test]
    #[ignore]
    fn the_tempo() {
        let (_, w, day) = worlds(15.5);
        eprintln!("rate  secs  days  drift pt  spread pt  sweep 0.95 pt  turned >= 1/4 (1 s)");
        for rate in [1.0, 2.0, 3.0, 4.0, 6.0] {
            for secs in [0.5, 1.0] {
                let m = ((secs * rate * 4.0) as usize).clamp(1, HISTORY - 1);
                let d: Vec<[f64; 2]> = (0..WORLDS as usize)
                    .map(|k| {
                        let (a, b) = (flat(w.place(k as u32 + 1, day).at).unwrap(), flat(w.back(m, k, day)).unwrap());
                        [a[0] - b[0], a[1] - b[1]]
                    })
                    .collect();
                let n = d.len() as f64;
                let mean = [d.iter().map(|x| x[0]).sum::<f64>() / n, d.iter().map(|x| x[1]).sum::<f64>() / n];
                let spread = (d.iter().map(|x| (x[0] - mean[0]).powi(2) + (x[1] - mean[1]).powi(2)).sum::<f64>() / n).sqrt();
                let mut north: Vec<f64> = d.iter().map(|x| -(x[1] - mean[1])).collect();
                north.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let q95 = north[(0.95 * (n - 1.0)) as usize];
                let turned = if secs == 1.0 {
                    let c = w.choose(ConductGesture::Circle { turn: 5.0, secs }, day, rate, &flat).unwrap();
                    format!("{:.0}%", c.matches as f64 / n * 100.0)
                } else {
                    String::new()
                };
                eprintln!("{rate:4}  {secs:4}  {:4.1}  {:8.0}  {spread:9.0}  {q95:13.0}  {turned}", m as f64 / 4.0, mean[0].hypot(mean[1]));
            }
        }
    }
}
