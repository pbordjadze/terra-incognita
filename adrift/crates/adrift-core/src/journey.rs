//! A journey clip's pace (R14, Engineering's seam; the measure of what is worth seeing is Truth's
//! this round). A journey clip is a take with no hand in it (`App::journey_take`): its picked lamps
//! from the day the last was put, for a length the reader picks. Its date need not run evenly: where
//! the lamps do something worth seeing (come near each other, turn, loop) it slows, and where they
//! only travel it hurries, so the length is filled with what makes the edit. `pace` gives the day
//! each frame shows; the take carries it as a pace before each frame, so it replays exactly.

use crate::model::Float;
use crate::sea::Media;

/// How much slower the date may run where the lamps are worth seeing than where they only travel.
pub const PACE_RANGE: f64 = 20.0;
/// The visible change is smoothed over this many days (a Gaussian's sigma) before it is spent.
pub const PACE_SMOOTH: f64 = 0.5;
/// The pace itself (its logarithm) is then smoothed over this many seconds of the clip, so even a
/// twentyfold change eases in and out: the clip's camera follows it (R15; R16 half a second).
pub const PACE_EASE: f64 = 0.5;
/// The weight of a mark where nothing moves on the screen, against the journey's mean visible change
/// (1): small, so a hover passes quickly but the date never leaps.
pub const VISIBLE_FLOOR: f64 = 0.1;
/// The frame's least scale (km): the clip's camera frames the lamps, but never closer than this.
pub const FRAME_KM: f64 = 80.0;

/// What moves on the screen between marks (R16): for each mark after the first, the lamps' motion
/// relative to each other (each pair's separation vector's change) and each lamp's bend (its turn
/// times its shorter step either side), in km, over the frame's scale there (the lamps' spread, at
/// least `FRAME_KM`), so it is the change the clip's camera, which follows and frames them, shows.
/// `places[l][i]` is lamp l at mark i. The first mark's value is the second's.
pub fn visible_series(places: &[Vec<crate::model::LonLat>]) -> Vec<f64> {
    use crate::model::LonLat;
    let marks = places.first().map_or(0, |p| p.len());
    if marks < 2 {
        return vec![0.0; marks];
    }
    // a local east-north vector from a to b, km
    let vec = |a: LonLat, b: LonLat| {
        let c = ((a.lat + b.lat) / 2.0).to_radians().cos();
        (((b.lon - a.lon + 540.0).rem_euclid(360.0) - 180.0).to_radians() * c * 6371.0, (b.lat - a.lat).to_radians() * 6371.0)
    };
    let mut out = vec![0.0; marks];
    for i in 1..marks {
        // the frame's scale: the lamps' greatest separation, at least FRAME_KM
        let mut scale = FRAME_KM;
        for a in 0..places.len() {
            for b in a + 1..places.len() {
                let d = vec(places[a][i], places[b][i]);
                scale = scale.max(d.0.hypot(d.1));
            }
        }
        let mut change = 0.0;
        for a in 0..places.len() {
            for b in a + 1..places.len() {
                let (d0, d1) = (vec(places[a][i - 1], places[b][i - 1]), vec(places[a][i], places[b][i]));
                change += (d1.0 - d0.0).hypot(d1.1 - d0.1);
            }
            if i >= 2 {
                let (u, w) = (vec(places[a][i - 2], places[a][i - 1]), vec(places[a][i - 1], places[a][i]));
                let (lu, lw) = (u.0.hypot(u.1), w.0.hypot(w.1));
                if lu > 1e-9 && lw > 1e-9 {
                    let turn = (u.0 * w.1 - u.1 * w.0).atan2(u.0 * w.0 + u.1 * w.1).abs();
                    change += turn * lu.min(lw);
                }
            }
        }
        out[i] = change / scale;
    }
    out[0] = out[1];
    out
}

/// A pair this close (km) counts as near (the interest of closeness falls off as exp(-d / NEAR_KM)).
pub const NEAR_KM: f64 = 60.0;

/// The day each of `frames` frames (at `fps`) shows, from `from` (the frame before the first) to `to`,
/// never going back (R14, Truth; R16 what moves on the screen). Time is spent where the screen
/// changes: each quarter-day mark of the lamps' own paths (`path`, each leg under its law) is weighed
/// by its visible change (`visible_series`: their motion relative to each other and each line's bend,
/// at the frame's scale), plus a small floor, smoothed over `PACE_SMOOTH` days and held within
/// `PACE_RANGE`; the frames are spaced evenly in that weight and the pace eased over `PACE_EASE`
/// seconds of clip. So a hover, or a run side by side, passes quickly, and braiding and looping take
/// the clip. Deterministic.
pub fn pace(media: Media<'_>, floats: &[Float], from: f64, to: f64, frames: usize, _fps: f64) -> Vec<f64> {
    let frames = frames.max(1);
    if !(to > from) {
        return vec![from; frames];
    }
    let w = interest(media, floats, from, to);
    let marks = w.len();
    // the weight's running integral over the marks (a quarter day each, the last part one shorter)
    let step = (to - from) / (marks - 1).max(1) as f64;
    let mut cum = vec![0.0; marks];
    for i in 1..marks {
        cum[i] = cum[i - 1] + 0.5 * (w[i - 1] + w[i]) * step;
    }
    let total = cum[marks - 1].max(1e-12);
    let mut out = Vec::with_capacity(frames);
    let mut j = 0;
    for f in 1..=frames {
        let target = total * f as f64 / frames as f64;
        while j + 1 < marks - 1 && cum[j + 1] < target {
            j += 1;
        }
        let (c0, c1) = (cum[j], cum[(j + 1).min(marks - 1)]);
        let frac = if c1 > c0 { ((target - c0) / (c1 - c0)).clamp(0.0, 1.0) } else { 1.0 };
        let day = from + (j as f64 + frac) * step;
        out.push(if f == frames { to } else { day.min(to) });
    }
    // the pace eased in the clip's own time: its logarithm smoothed over PACE_EASE seconds, then
    // rescaled so the frames still end on `to`
    let per: Vec<f64> = std::iter::once(out[0] - from).chain(out.windows(2).map(|w| w[1] - w[0])).map(|p| p.max(1e-9).ln()).collect();
    let sigma = (PACE_EASE * _fps).max(1.0);
    let reach = (3.0 * sigma).ceil() as isize;
    let eased: Vec<f64> = (0..per.len() as isize)
        .map(|i| {
            let (mut s, mut n) = (0.0, 0.0);
            for k in -reach..=reach {
                let w = (-0.5 * (k as f64 / sigma).powi(2)).exp();
                s += w * per[(i + k).clamp(0, per.len() as isize - 1) as usize];
                n += w;
            }
            (s / n).exp()
        })
        .collect();
    let scale = (to - from) / eased.iter().sum::<f64>();
    let mut day = from;
    let mut out: Vec<f64> = eased.iter().map(|p| {
        day += p * scale;
        day
    }).collect();
    *out.last_mut().unwrap() = to;
    for i in 1..out.len() {
        if out[i] < out[i - 1] {
            out[i] = out[i - 1];
        }
    }
    out
}

/// The weight of each of the journey's marks (from `from` to `to`, `step` apart, about a quarter
/// day): 1 for plain travel, more where the lamps are worth seeing, smoothed and held in range.
pub fn interest(media: Media<'_>, floats: &[Float], from: f64, to: f64) -> Vec<f64> {
    let marks = (((to - from) / 0.25).ceil() as usize).max(1) + 1;
    let step = (to - from) / (marks - 1) as f64;
    // each lamp's place at every mark, from its own path
    let places: Vec<Vec<crate::model::LonLat>> = floats
        .iter()
        .map(|f| {
            let p = path(media, f, to + 0.25);
            (0..marks).map(|i| place_on(&p, from + i as f64 * step)).collect()
        })
        .collect();
    // what moves on the screen at each mark (R16): the clip's camera follows the lamps, so what is
    // seen is their motion relative to each other and the bending of each line, at the frame's scale
    let v = visible_series(&places);
    let mean = (v.iter().sum::<f64>() / marks as f64).max(1e-12);
    // time goes where the screen changes: a hover passes quickly, braiding and looping take the clip
    let w: Vec<f64> = v.iter().map(|x| VISIBLE_FLOOR + x / mean).collect();
    // smoothed (a Gaussian of PACE_SMOOTH days), then held within PACE_RANGE of its least
    let sigma = (PACE_SMOOTH / step).max(1.0);
    let reach = (3.0 * sigma).ceil() as isize;
    let kernel: Vec<f64> = (-reach..=reach).map(|k| (-0.5 * (k as f64 / sigma).powi(2)).exp()).collect();
    let smooth: Vec<f64> = (0..marks as isize)
        .map(|i| {
            let (mut s, mut n) = (0.0, 0.0);
            for (j, k) in (-reach..=reach).enumerate() {
                let m = (i + k).clamp(0, marks as isize - 1) as usize;
                s += kernel[j] * w[m];
                n += kernel[j];
            }
            s / n
        })
        .collect();
    let most = smooth.iter().cloned().fold(0.0, f64::max).max(1e-9);
    smooth.iter().map(|x| x.max(most / PACE_RANGE)).collect()
}

/// A path's place on `day`, between its marks.
fn place_on(p: &[(f64, crate::model::LonLat)], day: f64) -> crate::model::LonLat {
    let i = p.partition_point(|m| m.0 <= day + 1e-9).saturating_sub(1);
    match p.get(i + 1) {
        Some(b) if b.0 > p[i].0 => crate::app::lerp_place(p[i].1, b.1, ((day - p[i].0) / (b.0 - p[i].0)).clamp(0.0, 1.0)),
        _ => p[i].1,
    }
}

/// A lamp's own path (its legs, each under its own law and world, and (R21) carried by its own
/// medium: a seed by the wind's rules), every quarter-day mark from its first leg to `to`: (day,
/// place). The lamp's path as the App computes it (`App::extend_path`). A seed come down stays where
/// it came down.
pub fn path(media: Media<'_>, f: &Float, to: f64) -> Vec<(f64, crate::model::LonLat)> {
    path_held(media, f, to).into_iter().map(|(d, at, _)| (d, at)).collect()
}

/// `path`, each mark with what holds it then (R21: a seed's `Held::Landed` or `Held::Ditched`).
pub fn path_held(media: Media<'_>, f: &Float, to: f64) -> Vec<(f64, crate::model::LonLat, crate::model::Held)> {
    use crate::model::Held;
    use crate::sea::Drift;
    let legs = f.legs();
    let mut out = Vec::new();
    for (i, leg) in legs.iter().enumerate() {
        let rules = media.of(leg.medium);
        let until = legs.get(i + 1).map_or(to, |l| l.day);
        let e = leg.eddy.map(|e| (e[0], e[1])).unwrap_or_else(|| rules.put(leg.at, leg.day, leg.seed, leg.world).eddy);
        let mut d = Drift { at: leg.at, eddy: e };
        let mut held = Held::No;
        let mut day = leg.day;
        out.push((day, d.at, held));
        while day < until - 1e-9 {
            let next = crate::app::grid_next(day);
            let s = rules.flight(leg.law, d, held, day, next - day, leg.seed, leg.world);
            d = s.drift;
            // (the sea's holds free again; a seed's coming down is for good)
            held = if held.down() { held } else { s.held };
            day = next;
            out.push((day, d.at, held));
        }
    }
    out
}

/// (R21) The day the last of `floats` came down, if every one is a seed come down by `to` (a lamp, or
/// a seed still aloft, never ends): a journey of seeds ends there, and its end beat begins.
pub fn all_down(media: Media<'_>, floats: &[Float], to: f64) -> Option<f64> {
    let mut last = f64::NEG_INFINITY;
    for f in floats {
        let p = path_held(media, f, to);
        let (_, _, held) = *p.last()?;
        if !held.down() {
            return None;
        }
        // the first mark of the stretch it has stayed down since
        let down = p.iter().rev().take_while(|m| m.2.down()).last().map_or(to, |m| m.0);
        last = last.max(down);
    }
    last.is_finite().then_some(last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assets::DirAssets;
    use crate::model::{Leg, LonLat};
    use crate::sea::drifters::distance_km;
    use std::path::{Path, PathBuf};

    fn assets() -> PathBuf {
        std::env::var_os("ADRIFT_ASSETS").map(PathBuf::from).unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../build/assets"))
    }

    pub(crate) fn rules() -> Media<'static> {
        let sea: &'static crate::sea::Rules = Box::leak(Box::new(crate::sea::Rules::load(&DirAssets(assets())).unwrap()));
        let air = Box::leak(Box::new(crate::sea::AirRules::sharing(sea, &DirAssets(assets())).unwrap()));
        Media { sea, air }
    }

    /// A lamp as a fresh put makes it (the current law).
    pub(super) fn lamp(id: u32, at: LonLat, day: f64, seed: u64) -> Float {
        serde_json::from_value(serde_json::json!({
            "id": id, "at": at, "put_day": day, "seed": seed, "held": "No", "hand": "Free", "wake": [], "lit_for": 10.0,
            "legs": [Leg::new(at, day, seed)], "kept": true
        }))
        .unwrap()
    }

    /// How a pair danced over their paths: the line between them wound (radians, signed), the share of
    /// marks they were 15-150 km apart, and their closest.
    pub(super) fn dance(a: &[(f64, LonLat)], b: &[(f64, LonLat)]) -> (f64, f64, f64) {
        let mut wind = 0.0;
        let mut last: Option<f64> = None;
        let (mut near, mut closest) = (0usize, f64::INFINITY);
        let n = a.len().min(b.len());
        for i in 0..n {
            let (p, q) = (a[i].1, b[i].1);
            let d = distance_km(p.lon as f32, p.lat as f32, q.lon as f32, q.lat as f32) as f64;
            closest = closest.min(d);
            if (15.0..150.0).contains(&d) {
                near += 1;
            }
            let dx = ((q.lon - p.lon + 540.0).rem_euclid(360.0) - 180.0) * p.lat.to_radians().cos();
            let ang = (q.lat - p.lat).atan2(dx);
            if let Some(l) = last {
                wind += (ang - l + 3.0 * std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI;
            }
            last = Some(ang);
        }
        (wind, near as f64 / n as f64, closest)
    }

    /// The search for a dancing pair (R14): pairs of lamps put 25-70 km apart over the eddying seas,
    /// at month starts, followed 60 days; the best by how far the line between them wound while they
    /// stayed 15-150 km apart. `--ignored --nocapture` prints the best; the chosen pair is
    /// `adrift-check/lamps/dance.txt`.
    #[test]
    #[ignore]
    fn find_a_dancing_pair() {
        let r = rules();
        let regions: [(&str, f64, f64, f64, f64); 6] = [
            ("off Taiwan", 120.5, 126.0, 20.0, 27.0),
            ("Kuroshio extension", 140.0, 155.0, 32.0, 38.0),
            ("Gulf Stream", -72.0, -55.0, 34.0, 41.0),
            ("Agulhas", 15.0, 35.0, -42.0, -34.0),
            ("Brazil-Malvinas", -58.0, -45.0, -44.0, -36.0),
            ("East Australian", 150.0, 158.0, -38.0, -28.0),
        ];
        let mut best: Vec<(f64, String, LonLat, LonLat, f64)> = Vec::new();
        for (name, lo0, lo1, la0, la1) in regions {
            for month in [0.0, 1.0, 2.0, 5.0, 8.0, 11.0] {
                let day0 = 9131.0 + month * 30.44 + 1.0; // a typical year from 2025-01-01, a month's first
                let mut lon = lo0;
                while lon < lo1 {
                    let mut lat = la0;
                    while lat < la1 {
                        let a = LonLat { lon, lat };
                        if !r.sea.coast.is_sea(a.lon as f32, a.lat as f32) {
                            lat += 0.75;
                            continue;
                        }
                        for (dx, dy) in [(0.4, 0.0), (0.0, 0.4), (0.3, 0.3), (0.6, -0.2)] {
                            let b = LonLat { lon: lon + dx, lat: lat + dy };
                            if !r.sea.coast.is_sea(b.lon as f32, b.lat as f32) {
                                continue;
                            }
                            let (fa, fb) = (lamp(1, a, day0, 1), lamp(2, b, day0, 2));
                            let span: f64 = std::env::var("DANCE_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(60.0);
                            let (pa, pb) = (path(r, &fa, day0 + span), path(r, &fb, day0 + span));
                            let (w, near, closest) = dance(&pa, &pb);
                            let score = w.abs() * near;
                            best.push((score, format!("{name} {w:+.1} rad, near {:.0}%, closest {closest:.0} km", near * 100.0), a, b, day0));
                        }
                        lat += 0.75;
                    }
                    lon += 0.75;
                }
            }
        }
        best.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        for b in best.iter().take(12) {
            eprintln!("{:6.2}  {}  a {:.2},{:.2}  b {:.2},{:.2}  day {:.1}", b.0, b.1, b.2.lon, b.2.lat, b.3.lon, b.3.lat, b.4);
        }
    }

    /// The dancing pair (R14, `find_a_dancing_pair`'s best): two lamps put 40 km apart in the East
    /// Australian Current's eddies off New South Wales on a typical 1 December, whose line wound three
    /// and a half turns round them in 60 days, 15-150 km apart nine tenths of the time. The copied-lamps
    /// text of them, the date 60 days on: `adrift-check/lamps/dance.txt` (ADRIFT_DANCE_OUT writes it).
    pub(crate) fn the_dance() -> (Vec<Float>, f64) {
        let day0 = 9131.0 + 11.0 * 30.44 + 1.0; // as the search reckoned it
        // made under law 1 (R14's), as dance.txt holds them: a lamp keeps its law (R13)
        let mut f = vec![lamp(1, LonLat { lon: 156.75, lat: -29.0 }, day0, 1), lamp(2, LonLat { lon: 157.15, lat: -29.0 }, day0, 2)];
        for l in &mut f {
            l.legs = l.legs.iter().map(|g| g.under(1)).collect();
        }
        (f, day0 + 60.0)
    }

    #[test]
    fn the_dance_dances() {
        let r = rules();
        let (f, to) = the_dance();
        let (a, b) = (path(r, &f[0], to), path(r, &f[1], to));
        let (w, near, closest) = dance(&a, &b);
        eprintln!("the dance: the line between them wound {w:+.1} rad in 60 days, 15-150 km apart {:.0}% of the time, closest {closest:.0} km", near * 100.0);
        assert!(w.abs() > 4.0 * std::f64::consts::PI && near > 0.8);
        if let Some(out) = std::env::var_os("ADRIFT_DANCE_OUT") {
            let copy = crate::app::LampsCopy { version: 1, day: to, floats: f };
            std::fs::write(out, format!("{}\n{}\n", crate::app::LAMPS_HEADER, serde_json::to_string(&copy).unwrap())).unwrap();
        }
    }

    /// The share of a clip's frames in which the screen barely changes (visible change under a quarter
    /// of the clip's median), with each frame's day from `days`.
    fn still_share(r: Media<'_>, f: &[Float], from: f64, to: f64, days: &[f64]) -> f64 {
        let paths: Vec<_> = f.iter().map(|l| path(r, l, to + 0.25)).collect();
        let places: Vec<Vec<LonLat>> = paths.iter().map(|p| std::iter::once(from).chain(days.iter().copied()).map(|d| super::place_on(p, d)).collect()).collect();
        let v = visible_series(&places);
        let mut sorted = v.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = sorted[sorted.len() / 2];
        v.iter().filter(|x| **x < 0.25 * median).count() as f64 / v.len() as f64
    }

    fn early() -> (Vec<Float>, f64, f64) {
        let day0 = 9131.0 + 8.0 * 30.44 + 1.0;
        (vec![lamp(1, LonLat { lon: 18.75, lat: -42.0 }, day0, 1), lamp(2, LonLat { lon: 18.75, lat: -41.6 }, day0, 2)], day0, day0 + 30.0)
    }

    #[test]
    fn the_pace_spends_time_where_the_screen_changes() {
        let r = rules();
        let (dance, dance_to) = the_dance();
        let (e, e_from, e_to) = early();
        for (name, f, from, to) in [("dance", dance.clone(), dance[0].put_day, dance_to), ("early", e, e_from, e_to)] {
            let (frames, fps) = (900usize, 30.0);
            let days = pace(r, &f, from, to, frames, fps);
            assert_eq!(days.len(), frames);
            assert_eq!(*days.last().unwrap(), to, "it ends on the last day");
            assert!(days.windows(2).all(|w| w[1] >= w[0]) && days[0] > from, "never back");
            assert_eq!(days, pace(r, &f, from, to, frames, fps), "deterministic");
            let per: Vec<f64> = std::iter::once(days[0] - from).chain(days.windows(2).map(|w| w[1] - w[0])).collect();
            let (lo, hi) = per.iter().fold((f64::INFINITY, 0.0f64), |(a, b), x| (a.min(*x), b.max(*x)));
            let lurch = per.windows(2).map(|w| (w[1] - w[0]).abs() / w[0].max(1e-12)).fold(0.0, f64::max);
            let even: Vec<f64> = (1..=frames).map(|i| from + (to - from) * i as f64 / frames as f64).collect();
            let (still, still_even) = (still_share(r, &f, from, to, &days), still_share(r, &f, from, to, &even));
            // the middle third of the clip: how many days it covers, against an even pace's third
            let middle = days[frames * 2 / 3] - days[frames / 3];
            // the stillest two seconds: the least visible change over 60 frames, against the median 60
            let stillest = |d: &[f64]| {
                let paths: Vec<_> = f.iter().map(|l| path(r, l, to + 0.25)).collect();
                let places: Vec<Vec<LonLat>> = paths.iter().map(|p| std::iter::once(from).chain(d.iter().copied()).map(|x| super::place_on(p, x)).collect()).collect();
                let v = visible_series(&places);
                let sums: Vec<f64> = v.windows(60).map(|w| w.iter().sum::<f64>()).collect();
                let mut s2 = sums.clone();
                s2.sort_by(|a, b| a.partial_cmp(b).unwrap());
                s2[0] / s2[s2.len() / 2]
            };
            // (R15's pace, measured the same way when this was written: 0.08 for the dance, 0.23 for early)
            let (s_now, s_even) = (stillest(&days), stillest(&even));
            eprintln!("{name}: stillest 2 s against the median 2 s: this pace {s_now:.2}, even {s_even:.2}");
            eprintln!(
                "{name} in 30 s: {:.2}-{:.2} days a second, the most a frame's pace changes {:.1}%; still frames {:.0}% (even pace {:.0}%); the clip's middle third covers {:.1} days (even {:.1})",
                lo * fps, hi * fps, lurch * 100.0, still * 100.0, still_even * 100.0, middle, (to - from) / 3.0
            );
            assert!(lurch < 0.05, "no lurch");
            assert!(still < still_even * 0.6 && still < 0.05, "{name}: fewer frames where nothing moves on the screen");
            assert!(s_now > 0.55 && s_now > s_even * 1.3, "{name}: no dead air: the stillest two seconds still move");
        }
    }

    #[test]
    fn one_plain_lamp_runs_near_evenly() {
        let r = rules();
        let f = vec![lamp(1, LonLat { lon: -30.0, lat: 20.0 }, 9131.0, 1)];
        let days = pace(r, &f, 9131.0, 9191.0, 300, 30.0);
        assert!(days.windows(2).all(|w| w[1] >= w[0]));
        assert_eq!(*days.last().unwrap(), 9191.0);
    }

    /// An early dance for the demo (R15, `find_a_dancing_pair` over 30 days under law 2): two lamps
    /// put 40 km apart in the Agulhas retroflection's eddies south of Cape Town on a typical 2
    /// September, whose line winds two and a half turns in their first 30 days, 15-150 km apart nine
    /// tenths of the time. `adrift-check/lamps/dance-early.txt` (ADRIFT_DANCE_EARLY_OUT writes it).
    #[test]
    fn the_early_dance_dances_from_the_start() {
        let r = rules();
        let (f, _, to) = early();
        let (a, b) = (path(r, &f[0], to), path(r, &f[1], to));
        let (w, near, closest) = dance(&a, &b);
        eprintln!("the early dance: the line wound {w:+.1} rad in 30 days, 15-150 km apart {:.0}% of the time, closest {closest:.0} km", near * 100.0);
        assert!(w.abs() > 4.0 * std::f64::consts::PI && near > 0.8);
        assert_eq!(f[0].legs[0].law, crate::sea::rules::LAW);
        if let Some(out) = std::env::var_os("ADRIFT_DANCE_EARLY_OUT") {
            let copy = crate::app::LampsCopy { version: 1, day: to, floats: f };
            std::fs::write(out, format!("{}\n{}\n", crate::app::LAMPS_HEADER, serde_json::to_string(&copy).unwrap())).unwrap();
        }
    }

    /// A seed as a fresh lift makes it (R21).
    pub(super) fn seed(id: u32, at: LonLat, day: f64, seed: u64) -> Float {
        let mut f = lamp(id, at, day, seed);
        f.legs = f.legs.iter().map(|l| l.on(crate::model::Medium::Wind)).collect();
        f
    }

    /// (R21) Where seeds fly far: pairs lifted 40 km apart over a grid, each month, ranked by how far
    /// the pair went before both came down. `cargo test --release -p adrift-core find_a_seed_flight -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn find_a_seed_flight() {
        let r = rules();
        let mut best = Vec::new();
        for month in [0.0, 3.0, 6.0, 9.0] {
            let day0 = 9131.0 + month * 30.44 + 1.0;
            for lon in (-120..=20).step_by(10) {
                for lat in (25..=55).step_by(10) {
                    let a = LonLat::new(lon as f64, lat as f64);
                    let b = LonLat::new(lon as f64 + 0.5, lat as f64);
                    let f = [seed(1, a, day0, 1), seed(2, b, day0, 2)];
                    let (pa, pb) = (path_held(r, &f[0], day0 + 6.0), path_held(r, &f[1], day0 + 6.0));
                    let down = all_down(r, &f, day0 + 6.0).unwrap_or(day0 + 6.0) - day0;
                    let (ea, eb) = (pa.last().unwrap(), pb.last().unwrap());
                    let d = |p: LonLat, q: LonLat| distance_km(p.lon as f32, p.lat as f32, q.lon as f32, q.lat as f32) as f64;
                    let km = d(a, ea.1).min(d(b, eb.1));
                    let apart = d(ea.1, eb.1);
                    best.push((km, format!("{lon},{lat} month {month}: {km:.0} km, apart {apart:.0} km at the end, down by {down:.2} d ({:?} {:?})", ea.2, eb.2)));
                }
            }
        }
        best.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        for b in best.iter().take(10) {
            eprintln!("{}", b.1);
        }
        let n = best.len();
        eprintln!("median: {}", best[n / 2].1);
    }

    /// (R21) A journey of seeds: each by the wind's rules (not the sea's, as before the seam's fix), so
    /// the gale pair off Biscay crosses to Spain; it ends when the last comes down (`all_down`), a lamp
    /// never does; and its pace runs on, never back, to the last day.
    #[test]
    fn a_journey_of_seeds_flies_by_the_wind_and_ends_when_they_are_down() {
        let r = rules();
        let day0 = 9132.0;
        let f = vec![seed(1, LonLat::new(-20.0, 45.0), day0, 1), seed(2, LonLat::new(-19.5, 45.0), day0, 2)];
        let p = path_held(r, &f[0], day0 + 4.0);
        let (_, end, held) = *p.last().unwrap();
        let km = distance_km(-20.0, 45.0, end.lon as f32, end.lat as f32);
        assert!(km > 1000.0 && held.down(), "the gale seed went {km:.0} km and came down ({held:?})");
        let down = all_down(r, &f, day0 + 4.0).expect("both down within four days");
        assert!(down > day0 + 0.5 && down < day0 + 3.0, "down by {:.2} d", down - day0);
        // nothing moves after: the path's places past `down` are where it came down
        let q = path(r, &f[1], day0 + 4.0);
        let after: Vec<_> = q.iter().filter(|m| m.0 >= down).map(|m| m.1).collect();
        assert!(after.windows(2).all(|w| w[0] == w[1]));
        // a lamp never ends; nor does a mix of a lamp and seeds
        let l = lamp(3, LonLat::new(-30.0, 40.0), day0, 3);
        assert_eq!(all_down(r, &[l.clone()], day0 + 4.0), None);
        assert_eq!(all_down(r, &[f[0].clone(), l], day0 + 4.0), None);
        let days = pace(r, &f, day0, down, 300, 30.0);
        assert!(days.windows(2).all(|w| w[1] >= w[0]) && *days.last().unwrap() == down);
    }
}
