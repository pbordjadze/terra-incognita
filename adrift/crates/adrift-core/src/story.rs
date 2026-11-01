//! Cast the fate (painting-company row 199): two lamps, a story told as the shape of the distance
//! between them, and the worlds where exactly that happened, found among the 2048 the maybes live in.
//!
//! Nothing here is drawn or invented. World k is the sea the k-th maybe of every lamp meets
//! (`Rules::flight`, R6), so a lamp's path in world k is the same quarter-day flight the app steps
//! its lamp with (`App::extend_path` with `k` the world), and two lamps in one world meet one sea.
//! A story is only a way of ranking: every world's two paths are stepped from their puts over the
//! story's span, their distance read once a day, and the worlds whose distance has the story's shape
//! best are the takes. Playing a take sets the app's world to it: the lamps then go exactly as they
//! went in the search, so what the reader watches is what was ranked.

use crate::model::{Held, Leg, LonLat};
use crate::sea::{Drift, Rules};

/// The worlds searched: every maybe's (k = 1..=WORLDS; the clouds' own, `app::MAYBES`).
pub const WORLDS: u32 = 2048;
/// How long a story runs from the later lamp's put, days.
pub const SPAN: f64 = 180.0;
/// The takes offered for a story (the best few).
pub const TAKES: usize = 5;
/// The step, as the app's lamps take it (`app::SUBSTEP`): a take replays only on the same grid.
pub const STEP: f64 = 0.25;
/// "Together", km: about one speck at the whole Earth; under it two lamps read as one.
pub const NEAR_KM: f64 = 40.0;

/// The stories: shapes of the distance between two lamps over the span.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Story {
    /// They keep together the whole way.
    Together,
    /// They start together and the sea parts them for good.
    Parted,
    /// They lose each other for a long time and find each other at the very end.
    Found,
}

impl Story {
    pub const ALL: [Story; 3] = [Story::Together, Story::Parted, Story::Found];

    /// The story's ideal shape at s in 0..1 of the span: the distance as a fraction of a wide one
    /// (the marks draw this; the ranking below is its measure).
    pub fn shape(self, s: f32) -> f32 {
        match self {
            Story::Together => 0.07,
            Story::Parted => 0.07 + 0.93 * s.powf(1.4),
            Story::Found => 0.07 + 0.93 * (std::f32::consts::PI * s).sin().powf(0.6),
        }
    }

    /// How well a world's daily distances (km, day 0 the later put's) tell this story: larger better.
    pub fn score(self, d: &[f32]) -> f64 {
        let n = d.len();
        if n < 4 {
            return 0.0;
        }
        let near = NEAR_KM as f32;
        match self {
            // never apart: the farthest they ever were, the less the better
            Story::Together => -(d.iter().copied().fold(0.0, f32::max) as f64),
            // apart for good: as far as they end, held over the last fifth (not a passing gap)
            Story::Parted => {
                let tail = &d[n * 4 / 5..];
                tail.iter().copied().fold(f32::MAX, f32::min) as f64
            }
            // far for a long time, together at the very end: the middle's typical distance (its
            // median over the middle half, so a long loss, not a moment's) against the end's, and the
            // end must be the closest they come after the middle
            Story::Found => {
                let mut mid: Vec<f32> = d[n / 4..n * 3 / 4].to_vec();
                mid.sort_by(|a, b| a.partial_cmp(b).unwrap());
                let m = mid[mid.len() / 2];
                let end = d[n - 1];
                ((m + near) as f64 / (end + near) as f64).ln()
            }
        }
    }
}

/// One lamp's path in world `k`, from its leg's put to `until`, on the quarter-day grid: (day, place,
/// held ashore) at each step. The same stepping as `App::extend_path` (a test holds them together).
pub fn path(rules: &Rules, leg: Leg, k: u32, until: f64) -> Vec<(f64, LonLat, bool)> {
    let e = rules.put(leg.at, leg.day, leg.seed, k).eddy;
    let mut d = Drift { at: leg.at, eddy: e };
    let mut day = leg.day;
    let mut out = vec![(day, d.at, false)];
    while day < until - 1e-9 {
        let next = grid_next(day);
        let f = rules.flight(d, day, next - day, leg.seed, k);
        d = f.drift;
        day = next;
        out.push((day, d.at, f.held == Held::Coast));
    }
    out
}

/// The next quarter-day mark after `t` (as `app::grid_next`).
pub fn grid_next(t: f64) -> f64 {
    let q = (t / STEP).floor() * STEP + STEP;
    if q - t < 1e-9 {
        q + STEP
    } else {
        q
    }
}

/// Where a path is on `day` (between steps, along the short way round).
pub fn place_on(p: &[(f64, LonLat, bool)], day: f64) -> LonLat {
    let i = p.partition_point(|s| s.0 <= day + 1e-9).saturating_sub(1);
    let (d0, a, _) = p[i];
    match p.get(i + 1) {
        Some(&(d1, b, _)) => {
            let f = ((day - d0) / (d1 - d0)).clamp(0.0, 1.0);
            let mut dlon = b.lon - a.lon;
            if dlon > 180.0 {
                dlon -= 360.0;
            } else if dlon < -180.0 {
                dlon += 360.0;
            }
            LonLat { lon: a.lon + dlon * f, lat: a.lat + (b.lat - a.lat) * f }.normalized()
        }
        None => a,
    }
}

/// The two lamps' daily distance in world `k`, km, from `t0` for `days` days (days + 1 values), and
/// the share of the span's steps either spent held on a coast.
pub fn distances(rules: &Rules, a: Leg, b: Leg, k: u32, t0: f64, days: f64) -> (Vec<f32>, f32) {
    let until = t0 + days;
    let pa = path(rules, a, k, until);
    let pb = path(rules, b, k, until);
    let d = (0..=days as usize)
        .map(|i| {
            let day = t0 + i as f64;
            (place_on(&pa, day).distance_m(place_on(&pb, day)) / 1000.0) as f32
        })
        .collect();
    let held = |p: &[(f64, LonLat, bool)]| p.iter().filter(|s| s.0 > t0 && s.2).count() as f32 / (days / STEP) as f32;
    (d, held(&pa).max(held(&pb)))
}

/// A search under way, or done: every world's distances, filled a slice at a time.
#[derive(Clone, Debug)]
pub struct Search {
    pub a: Leg,
    pub b: Leg,
    /// The later put: the story's day 0.
    pub t0: f64,
    /// `curves[k - 1]` is world k's daily distances once searched.
    pub curves: Vec<Vec<f32>>,
    /// `aground[k - 1]`: the share of the span the more stranded of world k's two lamps lay ashore.
    pub aground: Vec<f32>,
}

impl Search {
    pub fn new(a: Leg, b: Leg) -> Search {
        Search { a, b, t0: a.day.max(b.day), curves: Vec::with_capacity(WORLDS as usize), aground: Vec::with_capacity(WORLDS as usize) }
    }

    pub fn done(&self) -> bool {
        self.curves.len() >= WORLDS as usize
    }

    /// 0..1, how far the search has come.
    pub fn progress(&self) -> f32 {
        self.curves.len() as f32 / WORLDS as f32
    }

    /// Searches `n` more worlds.
    pub fn advance(&mut self, rules: &Rules, n: usize) {
        for _ in 0..n {
            if self.done() {
                return;
            }
            let k = self.curves.len() as u32 + 1;
            let (d, held) = distances(rules, self.a, self.b, k, self.t0, SPAN);
            self.curves.push(d);
            self.aground.push(held);
        }
    }

    /// The best `TAKES` worlds for a story, best first (k, its daily distances).
    pub fn takes(&self, story: Story) -> Vec<u32> {
        // (kept together on a beach is not kept together: for that story a world whose lamps lay
        // ashore more than a tenth of the span ranks after every world whose lamps stayed afloat)
        let mut ranked: Vec<(f64, u32)> = self
            .curves
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let stranded = story == Story::Together && self.aground[i] > 0.1;
                (story.score(d) - if stranded { 1e9 } else { 0.0 }, i as u32 + 1)
            })
            .collect();
        ranked.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap().then(x.1.cmp(&y.1)));
        ranked.into_iter().take(TAKES).map(|(_, k)| k).collect()
    }

    /// The distance the marks draw a take's curve against: wide enough that most worlds' widest
    /// gap fits (the 90th percentile), never under 200 km.
    pub fn scale_km(&self) -> f32 {
        let mut far: Vec<f32> = self.curves.iter().map(|d| d.iter().copied().fold(0.0, f32::max)).collect();
        if far.is_empty() {
            return 200.0;
        }
        far.sort_by(|a, b| a.partial_cmp(b).unwrap());
        far[far.len() * 9 / 10].max(200.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::DirAssets;
    use std::path::{Path, PathBuf};

    fn rules() -> Rules {
        let dir = std::env::var_os("ADRIFT_ASSETS").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/assets"));
        Rules::load(&DirAssets(dir)).unwrap()
    }

    /// A pair put 20 km apart off Hatteras: every story has takes, each told better than by most
    /// worlds, and what the search measured is what the paths do (the same numbers replayed).
    #[test]
    fn every_story_finds_its_worlds_off_hatteras() {
        let rules = rules();
        let a = Leg { at: LonLat::new(-74.0, 35.5), day: 150.0, seed: 1 };
        let b = Leg { at: LonLat::new(-73.8, 35.5), day: 150.0, seed: 2 };
        let mut s = Search::new(a, b);
        let t = std::time::Instant::now();
        s.advance(&rules, WORLDS as usize);
        let secs = t.elapsed().as_secs_f64();
        eprintln!("searched {} worlds in {secs:.2} s", WORLDS);
        let scale = s.scale_km();
        eprintln!("scale {scale:.0} km");
        for story in Story::ALL {
            let takes = s.takes(story);
            assert_eq!(takes.len(), TAKES);
            let mut all: Vec<f64> = s.curves.iter().map(|d| story.score(d)).collect();
            all.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let median = all[all.len() / 2];
            for &k in &takes {
                let d = &s.curves[k as usize - 1];
                assert!(story.score(d) >= median);
                let pick = |f: f32| d[((d.len() - 1) as f32 * f) as usize];
                eprintln!("{story:?} k={k}: {:.0} {:.0} {:.0} {:.0} {:.0} km", pick(0.0), pick(0.25), pick(0.5), pick(0.75), pick(1.0));
            }
            // replayed: the same world's distances again are the same numbers
            let k = takes[0];
            assert_eq!(distances(&rules, a, b, k, s.t0, SPAN).0, s.curves[k as usize - 1]);
        }
        let found = &s.curves[s.takes(Story::Found)[0] as usize - 1];
        assert!(*found.last().unwrap() < 100.0, "the best found ends within 100 km: {}", found.last().unwrap());
    }
}
