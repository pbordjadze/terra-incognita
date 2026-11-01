//! Together (painting-company row 199): one lamp a day, set on the real sea and let go, drifting in
//! real time among everyone else's.
//!
//! The mode is the app's own state machine with a few rules on top (`App.together`):
//! - **The clock is the wall's.** `Clock.day` is the real day (the shell's `set_wall`) plus `skip`,
//!   the days a hidden rest has let pass; nothing scrubs it.
//! - **One lamp a day.** A lamp may be set while no lamp of yours was set on this local day. The
//!   release is a long rest on the sea: the ring gathers for `FORM` seconds into a lit lamp under the
//!   finger, and lifting the finger sets it down; lifting early, or moving, sets nothing. Once set it
//!   can't be held, carried, undone or taken out: the intents that would are dropped.
//! - **Everyone else's lamps** are simulated here, on this device, from a seeded population: about
//!   thirty a day, set off coasts where people live and a few far out, each stepped by the same
//!   rules as yours (`Rules::flight`, its own world k, so their wander is their own). Nothing is
//!   sent or fetched. A lamp burns for `LIFE` days and goes out.
//! - **Meeting.** When two lamps (yours or anyone's) come within `MEET_KM`, both glow toward each
//!   other for a few seconds.
//! - **Letting time pass.** Two fingers resting still on the screen (or one on the dark off the
//!   globe) for a moment let the days run, about one every `SKIP_SECONDS`, until they lift or move:
//!   the hidden fast-forward, for trying it in one sitting. The days passed are kept (`skip`) so the sea stays where it was let go to.

use crate::model::{Held, LonLat};
use crate::sea::rules::{mix, uniform, Drift, Rules};
use std::collections::{HashMap, HashSet};

/// Seconds of a still finger before the lamp is formed and may be let go.
pub const FORM: f64 = 2.4;
/// The days a simulated lamp burns.
pub const LIFE: f64 = 100.0;
/// The days over which it goes out at the end.
const FADE_DAYS: f64 = 12.0;
/// The days a lamp that came ashore takes to go out.
const ASHORE_DAYS: f64 = 1.5;
/// Kilometres apart at which two lamps meet.
pub const MEET_KM: f64 = 22.0;
/// Seconds of a meeting's glow.
const GLOW_S: f64 = 5.0;
/// Seconds a day takes while time is let pass.
pub const SKIP_SECONDS: f64 = 3.0;
/// Seconds of a still finger off the globe before time starts to pass.
pub const SKIP_AFTER: f64 = 0.8;
/// The step of a simulated lamp (half a day: the maybes' own; SOURCES.md › How many maybes).
const STEP: f64 = 0.5;
/// The first world index the simulated lamps live in (yours is world 0, its maybes 1..2048).
const WORLDS: u32 = 1 << 20;

/// A lamp of someone else's, as simulated here.
struct Other {
    id: u32,
    put_day: f64,
    /// The last two steps on its half-day grid; it is drawn between them.
    a: (f64, Drift),
    b: (f64, Drift),
    ashore: bool,
    /// The day it came ashore (it goes out over `ASHORE_DAYS`).
    ashore_day: f64,
    /// 0..1, a little warmer or paler (each lamp its own candle).
    warm: f32,
    /// A meeting's glow, 1 at the moment and fading, and whom it is toward.
    glow: f32,
    toward: LonLat,
    /// The shell's seconds when it was first seen lit (its arrival blooms).
    born_secs: Option<f64>,
}

impl Other {
    fn at(&self, day: f64) -> LonLat {
        let (da, db) = (self.a.0, self.b.0);
        let f = if db > da { ((day - da) / (db - da)).clamp(0.0, 1.0) } else { 1.0 };
        crate::app::lerp_place(self.a.1.at, self.b.1.at, f)
    }
}

/// What the look draws of one simulated lamp.
#[derive(Clone, Copy, Debug)]
pub struct OtherLamp {
    pub at: LonLat,
    pub toward: LonLat,
    pub glow: f32,
    /// 0..1: its light (arrival, burning out).
    pub alpha: f32,
    pub warm: f32,
    /// 0..1 just after it arrives (a brief bloom), else 0.
    pub arrival: f32,
}

/// Where people stand at the sea's edge: (lon, lat, weight). A lamp is set out from one of these,
/// on the water within a few hundred km (or, one in nine, anywhere on the open sea: a ship, an
/// island). Weights follow the size of the coast's cities, roughly.
const SHORES: &[(f64, f64, f64)] = &[
    (-9.4, 38.6, 2.0), (-6.4, 36.4, 1.0), (5.3, 43.2, 1.5), (14.2, 40.7, 1.5), (23.7, 37.8, 1.5), (29.1, 41.2, 2.0),
    (29.9, 31.3, 2.0), (3.4, 6.3, 3.0), (-0.2, 5.5, 1.0), (13.2, -8.8, 1.0), (18.4, -34.0, 1.5), (31.1, -29.9, 1.0),
    (39.7, -4.1, 1.0), (32.6, -26.0, 0.7), (58.4, 23.6, 0.7), (67.0, 24.8, 2.5), (72.8, 18.9, 3.5), (80.3, 13.1, 2.0),
    (79.8, 6.9, 1.0), (91.8, 22.3, 2.0), (103.9, 1.2, 2.0), (106.8, -6.0, 3.0), (120.9, 14.5, 2.5), (114.2, 22.2, 2.5),
    (121.6, 31.2, 3.0), (129.1, 35.1, 1.5), (139.8, 35.5, 3.5), (141.3, 43.1, 0.8), (151.3, -33.9, 2.0), (145.0, -37.9, 1.5),
    (115.7, -32.0, 0.8), (174.8, -36.8, 0.8), (-157.9, 21.3, 0.7), (-123.2, 49.2, 1.0), (-122.5, 37.8, 1.5), (-118.3, 33.7, 2.5),
    (-106.4, 23.2, 0.6), (-79.5, 8.9, 0.8), (-77.2, -12.1, 1.5), (-71.6, -33.0, 1.0), (-58.3, -34.6, 1.5), (-43.2, -22.9, 2.5),
    (-38.5, -13.0, 1.0), (-38.5, -3.7, 1.0), (-66.9, 10.6, 1.0), (-82.4, 23.1, 1.0), (-80.1, 25.8, 1.5), (-74.0, 40.6, 3.0),
    (-63.6, 44.6, 0.6), (-52.7, 47.6, 0.4), (-21.9, 64.1, 0.4), (5.3, 60.4, 0.6), (-2.1, 57.1, 0.6), (-9.0, 53.3, 0.7),
    (-4.5, 48.4, 0.8), (-17.4, 14.7, 1.2), (-70.3, 18.5, 0.8), (-90.0, 29.9, 0.8), (-97.4, 27.8, 0.6), (55.3, 25.3, 1.0),
    (43.1, 11.6, 0.5), (49.8, -18.1, 0.5), (57.5, -20.2, 0.4), (-149.6, -17.5, 0.3), (178.4, -18.1, 0.3), (-70.7, 42.4, 1.0),
    (-117.2, 32.7, 1.0), (12.4, 45.4, 0.8), (2.2, 41.4, 1.5), (35.0, 32.8, 1.0), (-122.4, 47.6, 0.8), (153.0, -27.4, 0.8),
];

/// The together mode's state (`App.together`).
pub struct Together {
    /// The real day (days since 2000-01-01 UTC), from the shell's clock.
    pub wall_day: f64,
    /// The local time's offset from UTC, in days (the day's lamp turns over at local midnight).
    pub tz_days: f64,
    /// Days let pass by the hidden rest, kept across visits.
    pub skip: f64,
    /// How fast the days pass now (days a second), eased toward `skip_rate_to`.
    skip_rate: f64,
    pub skip_rate_to: f64,
    others: Vec<Other>,
    /// The days whose lamps have been made (by their integer day).
    made: HashSet<i64>,
    /// The pairs within `MEET_KM` at the last look (ids; yours are offset by `MINE`).
    close: HashSet<(u32, u32)>,
    paired_once: bool,
    /// A lamp gathering under a still finger: where, and since when (the shell's seconds).
    pub forming: Option<(LonLat, f64)>,
    /// The fingers down: where each went down, when, and whether it was off the globe; and whether
    /// any has moved since the first went down (a still hand lets time pass: two fingers anywhere,
    /// or one on the dark off the globe).
    pub fingers: HashMap<u64, (f64, f64, f64, bool)>,
    pub stirred: bool,
    /// Glows of your own lamps (by float id), toward whom.
    pub mine_glow: HashMap<u32, (f32, LonLat)>,
    next_id: u32,
    /// The shell's seconds at the first frame, and whether the opening flight to your newest lamp
    /// has been taken.
    pub opened: Option<f64>,
    pub flew: bool,
    /// The shell's seconds when the meetings were last looked for.
    met_at: f64,
}

const MINE: u32 = 1 << 31;

impl Together {
    pub fn new(wall_day: f64, tz_days: f64, skip: f64) -> Self {
        Together {
            wall_day,
            tz_days,
            skip,
            skip_rate: 0.0,
            skip_rate_to: 0.0,
            others: Vec::new(),
            made: HashSet::new(),
            close: HashSet::new(),
            paired_once: false,
            forming: None,
            fingers: HashMap::new(),
            stirred: false,
            mine_glow: HashMap::new(),
            next_id: 1,
            opened: None,
            flew: false,
            met_at: f64::NEG_INFINITY,
        }
    }

    /// The day as the sea has it: the wall's and the days let pass.
    pub fn day(&self) -> f64 {
        self.wall_day + self.skip
    }

    /// The local calendar day (an integer) of a day.
    pub fn local_day(&self, day: f64) -> i64 {
        (day + self.tz_days).floor() as i64
    }

    /// Whether the day's lamp is still to be set: none of `puts` (your lamps' put days) fell on today.
    pub fn lamp_ready(&self, puts: impl Iterator<Item = f64>) -> bool {
        let today = self.local_day(self.day());
        !puts.into_iter().any(|d| self.local_day(d) == today)
    }

    /// Lets time pass while `on` (eased in and out).
    pub fn pass_time(&mut self, on: bool) {
        self.skip_rate_to = if on { 1.0 / SKIP_SECONDS } else { 0.0 };
    }

    pub fn passing(&self) -> bool {
        self.skip_rate > 1e-4 || self.skip_rate_to > 0.0
    }

    /// Advances the days let pass by `dt` seconds of the shell.
    pub fn tick(&mut self, dt: f64) {
        let k = 1.0 - (-dt / 0.6).exp();
        self.skip_rate += (self.skip_rate_to - self.skip_rate) * k;
        if self.skip_rate_to == 0.0 && self.skip_rate < 1e-4 {
            self.skip_rate = 0.0;
        }
        self.skip += self.skip_rate * dt;
    }

    /// The lamps set on integer day `d`, made once (deterministic for `d`: the same sea on every
    /// visit and every device).
    fn make_day(&mut self, rules: &Rules, d: i64) {
        if !self.made.insert(d) {
            return;
        }
        let h0 = mix((d as u32).wrapping_mul(0x9E37_79B9) ^ 0x7A3D_11C5);
        let count = 34 + (h0 % 17);
        let total: f64 = SHORES.iter().map(|s| s.2).sum();
        for i in 0..count {
            let h = mix(h0 ^ i.wrapping_mul(0x85EB_CA6B));
            let when = d as f64 + uniform(h, 0) as f64;
            let mut at = None;
            for tries in 0..14u32 {
                let p = if uniform(h, 1) < 0.2 {
                    // the open sea
                    let z = uniform(h, 10 + tries) as f64 * 1.6 - 0.8;
                    LonLat { lon: uniform(h, 30 + tries) as f64 * 360.0 - 180.0, lat: z.asin().to_degrees() }
                } else {
                    let mut pick = uniform(h, 2) as f64 * total;
                    let mut shore = SHORES[0];
                    for s in SHORES {
                        if pick < s.2 {
                            shore = *s;
                            break;
                        }
                        pick -= s.2;
                    }
                    // out onto the water: a bearing and a reach, the nearer more likely
                    let bearing = uniform(h, 50 + tries) as f64 * std::f64::consts::TAU;
                    let km = 30.0 + 650.0 * (uniform(h, 70 + tries) as f64).powf(1.3);
                    let dlat = km / 111.2 * bearing.cos();
                    let dlon = km / (111.2 * shore.1.to_radians().cos().max(0.2)) * bearing.sin();
                    LonLat { lon: shore.0 + dlon, lat: shore.1 + dlat }.normalized()
                };
                if matches!(rules.may_put(p), crate::sea::rules::Put::Sea | crate::sea::rules::Put::Guess) {
                    at = Some(p);
                    break;
                }
            }
            let Some(at) = at else { continue };
            let id = self.next_id;
            self.next_id += 1;
            let k = WORLDS + id;
            let drift = rules.put(at, when, 0, k);
            self.others.push(Other { id, put_day: when, a: (when, drift), b: (when, drift), ashore: false, ashore_day: f64::INFINITY, warm: uniform(h, 90), glow: 0.0, toward: at, born_secs: None });
        }
    }

    /// The simulated sea brought to `day`: lamps made for the days in their life, the burnt out let
    /// go, each stepped on its half-day grid to the day; then who has met. `seconds` is the shell's
    /// clock (glows and arrivals are timed in it), `dt` its frame. `mine` are your lamps (id, place).
    pub fn advance(&mut self, rules: &Rules, day: f64, seconds: f64, dt: f64, mine: &[(u32, LonLat)]) -> Vec<(u32, LonLat)> {
        let today = day.floor() as i64;
        for d in (today - LIFE as i64)..=today {
            self.make_day(rules, d);
        }
        self.made.retain(|&d| d >= today - LIFE as i64 - 2);
        self.others.retain(|o| day - o.put_day < LIFE && day - o.ashore_day < ASHORE_DAYS);
        let mut stepped = false;
        for o in &mut self.others {
            if o.put_day > day {
                continue;
            }
            while o.b.0 < day && !o.ashore {
                o.a = o.b;
                let next = ((o.a.0 / STEP).floor() + 1.0) * STEP;
                let f = rules.flight(o.a.1, o.a.0, next - o.a.0, 0, WORLDS + o.id);
                o.b = (next, f.drift);
                if f.held == Held::Coast {
                    o.ashore = true;
                    o.ashore_day = next;
                }
                stepped = true;
            }
            if o.born_secs.is_none() {
                // (the lamps already out when the sea is first seen are simply there)
                o.born_secs = Some(if self.paired_once { seconds } else { seconds - 10.0 });
            }
            o.glow = (o.glow - (dt / GLOW_S) as f32).max(0.0);
        }
        for g in self.mine_glow.values_mut() {
            g.0 = (g.0 - (dt / GLOW_S) as f32).max(0.0);
        }
        self.mine_glow.retain(|_, g| g.0 > 0.0);
        let mut met = Vec::new();
        // (looked for when anyone has stepped, and every second for your lamps, which glide)
        if stepped || !self.paired_once || (!mine.is_empty() && seconds - self.met_at >= 1.0) {
            self.met_at = seconds;
            met = self.meet(day, mine);
        }
        met
    }

    /// Finds the pairs within `MEET_KM`; a pair that wasn't close before glows toward each other.
    /// Returns your lamps that met someone (id, toward).
    fn meet(&mut self, day: f64, mine: &[(u32, LonLat)]) -> Vec<(u32, LonLat)> {
        const CELL: f64 = 0.25;
        let mut points: Vec<(u32, LonLat)> = self.others.iter().filter(|o| o.put_day <= day).map(|o| (o.id, o.at(day))).collect();
        points.extend(mine.iter().map(|&(id, at)| (id | MINE, at)));
        let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (i, (_, p)) in points.iter().enumerate() {
            grid.entry(((p.lon / CELL).floor() as i32, (p.lat / CELL).floor() as i32)).or_default().push(i);
        }
        let mut close = HashSet::new();
        for (i, (id, p)) in points.iter().enumerate() {
            let (cx, cy) = ((p.lon / CELL).floor() as i32, (p.lat / CELL).floor() as i32);
            // (a cell is narrower than MEET_KM east-west only toward the poles, where few lamps go)
            for dx in -1..=1 {
                for dy in -1..=1 {
                    let Some(cell) = grid.get(&(cx + dx, cy + dy)) else { continue };
                    for &j in cell {
                        if j <= i {
                            continue;
                        }
                        let (jd, q) = points[j];
                        if p.distance_m(q) < MEET_KM * 1000.0 {
                            close.insert(((*id).min(jd), (*id).max(jd)));
                        }
                    }
                }
            }
        }
        let mut met = Vec::new();
        if self.paired_once {
            let place: HashMap<u32, LonLat> = points.iter().copied().collect();
            let index: HashMap<u32, usize> = self.others.iter().enumerate().map(|(i, o)| (o.id, i)).collect();
            for &(a, b) in close.difference(&self.close) {
                let (Some(&pa), Some(&pb)) = (place.get(&a), place.get(&b)) else { continue };
                for (me, other) in [(a, pb), (b, pa)] {
                    if me & MINE != 0 {
                        self.mine_glow.insert(me & !MINE, (1.0, other));
                        met.push((me & !MINE, other));
                    } else if let Some(&i) = index.get(&me) {
                        self.others[i].glow = 1.0;
                        self.others[i].toward = other;
                    }
                }
            }
        }
        self.close = close;
        self.paired_once = true;
        met
    }

    /// What the look draws of the others today.
    pub fn lamps(&self, day: f64, seconds: f64) -> Vec<OtherLamp> {
        self.others
            .iter()
            .filter(|o| o.put_day <= day)
            .map(|o| {
                let age = day - o.put_day;
                let since = o.born_secs.map_or(10.0, |b| seconds - b);
                let out = ((LIFE - age) / FADE_DAYS).clamp(0.0, 1.0) * (1.0 - (day - o.ashore_day) / ASHORE_DAYS).clamp(0.0, 1.0);
                let lit = (since / 1.2).clamp(0.0, 1.0);
                let arrival = if since < 3.0 { (1.0 - since / 3.0).clamp(0.0, 1.0) as f32 } else { 0.0 };
                OtherLamp { at: o.at(day), toward: o.toward, glow: o.glow, alpha: (out * lit) as f32, warm: o.warm, arrival }
            })
            .collect()
    }

    /// The brightest meeting now: where, toward whom, how bright (the checks look there).
    pub fn brightest(&self, day: f64) -> Option<(LonLat, LonLat, f32)> {
        self.others.iter().filter(|o| o.glow > 0.0).max_by(|a, b| a.glow.partial_cmp(&b.glow).unwrap()).map(|o| (o.at(day), o.toward, o.glow))
    }

    pub fn glowing(&self) -> usize {
        self.others.iter().filter(|o| o.glow > 0.0).count()
    }

    pub fn count(&self, day: f64) -> usize {
        self.others.iter().filter(|o| o.put_day <= day).count()
    }
}
