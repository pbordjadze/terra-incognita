//! A journey clip's pace (R14, Engineering's seam; the measure of what is worth seeing is Truth's
//! this round). A journey clip is a take with no hand in it (`App::journey_take`): its picked lamps
//! from the day the last was put, for a length the reader picks. Its date need not run evenly: where
//! the lamps do something worth seeing (come near each other, turn, loop) it slows, and where they
//! only travel it hurries, so the length is filled with what makes the edit. `pace` gives the day
//! each frame shows; the take carries it as a pace before each frame, so it replays exactly.

use crate::model::Float;
use crate::sea::Rules;

/// How much slower the date may run where the lamps are worth seeing than where they only travel.
pub const PACE_RANGE: f64 = 20.0;
/// The interest is smoothed over this many days (a Gaussian's sigma) before it is spent.
pub const PACE_SMOOTH: f64 = 1.0;
/// The pace itself (its logarithm) is then smoothed over this many seconds of the clip, so even a
/// twentyfold change eases in and out: the clip's camera follows it (R15).
pub const PACE_EASE: f64 = 1.0;
/// A pair this close (km) counts as near (the interest of closeness falls off as exp(-d / NEAR_KM)).
pub const NEAR_KM: f64 = 60.0;

/// The day each of `frames` frames (at `fps`) shows, from `from` (the frame before the first) to `to`,
/// never going back (R14, Truth). Time is spent where the lamps are worth seeing: each quarter-day
/// mark of their own paths (`path`, each leg under its law) is weighed by how near the pairs are,
/// how fast their distance changes, how far the line between each pair winds, and how much each lamp
/// turns (a turn weighed by the steps either side of it, so a stall's flips count for little). Each
/// term is scaled by its own mean over the journey, so none drowns the others; the sum, plus one for
/// plain travel, is smoothed over `PACE_SMOOTH` days and held within `PACE_RANGE`; the frames are
/// spaced evenly in that weight, so the date slows where it is high. Deterministic.
pub fn pace(rules: &Rules, floats: &[Float], from: f64, to: f64, frames: usize, _fps: f64) -> Vec<f64> {
    let frames = frames.max(1);
    if !(to > from) {
        return vec![from; frames];
    }
    let w = interest(rules, floats, from, to);
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
pub fn interest(rules: &Rules, floats: &[Float], from: f64, to: f64) -> Vec<f64> {
    let marks = (((to - from) / 0.25).ceil() as usize).max(1) + 1;
    let step = (to - from) / (marks - 1) as f64;
    // each lamp's place at every mark, from its own path
    let places: Vec<Vec<crate::model::LonLat>> = floats
        .iter()
        .map(|f| {
            let p = path(rules, f, to + 0.25);
            (0..marks).map(|i| place_on(&p, from + i as f64 * step)).collect()
        })
        .collect();
    let km = |a: crate::model::LonLat, b: crate::model::LonLat| crate::sea::drifters::distance_km(a.lon as f32, a.lat as f32, b.lon as f32, b.lat as f32) as f64;
    let mut terms = vec![vec![0.0; marks]; 4];
    // the pairs: closeness, the distance's rate of change, the line's winding
    for a in 0..places.len() {
        for b in a + 1..places.len() {
            let mut last_ang: Option<f64> = None;
            for i in 0..marks {
                let (p, q) = (places[a][i], places[b][i]);
                let d = km(p, q);
                terms[0][i] += (-d / NEAR_KM).exp();
                if i > 0 {
                    terms[1][i] += (d - km(places[a][i - 1], places[b][i - 1])).abs() / (1.0 + d / NEAR_KM);
                }
                let dx = ((q.lon - p.lon + 540.0).rem_euclid(360.0) - 180.0) * p.lat.to_radians().cos();
                let ang = (q.lat - p.lat).atan2(dx);
                if let Some(l) = last_ang {
                    let turn = ((ang - l + 3.0 * std::f64::consts::PI).rem_euclid(2.0 * std::f64::consts::PI) - std::f64::consts::PI).abs();
                    // a winding counts by how near they are (far apart, the line's angle means little)
                    terms[2][i] += turn * (-d / (2.0 * NEAR_KM)).exp();
                }
                last_ang = Some(ang);
            }
        }
    }
    // each lamp's turning, weighed by the steps either side against its median step
    for p in &places {
        let steps: Vec<(f64, f64)> = p.windows(2).map(|w| (((w[1].lon - w[0].lon + 540.0).rem_euclid(360.0) - 180.0) * w[0].lat.to_radians().cos(), w[1].lat - w[0].lat)).collect();
        let mut lens: Vec<f64> = steps.iter().map(|s| s.0.hypot(s.1)).collect();
        let mut sorted = lens.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let med = sorted.get(sorted.len() / 2).copied().unwrap_or(0.0).max(1e-9);
        for l in lens.iter_mut() {
            *l = (*l / med).min(1.0);
        }
        for i in 1..steps.len() {
            let (u, v) = (steps[i - 1], steps[i]);
            if u.0.hypot(u.1) < 1e-12 || v.0.hypot(v.1) < 1e-12 {
                continue;
            }
            let turn = (u.0 * v.1 - u.1 * v.0).atan2(u.0 * v.0 + u.1 * v.1).abs();
            terms[3][i] += turn * lens[i - 1].min(lens[i]);
        }
    }
    // each term by its own mean, so none drowns the others, the dance's own (closeness, the line's
    // winding) weighed most; the mix grows as its square, so the dance takes most of the clip and
    // plain travel (weighing 1) passes quickly (R15: the critic's "the real dance arrives late")
    let weights = [0.3, 0.3, 2.0, 1.5];
    let mut x = vec![0.0; marks];
    let total_w: f64 = weights.iter().sum();
    for (t, wt) in terms.iter().zip(weights) {
        let mean = t.iter().sum::<f64>() / marks as f64;
        if mean > 1e-12 {
            for i in 0..marks {
                x[i] += wt * t[i] / mean / total_w;
            }
        }
    }
    let w: Vec<f64> = x.iter().map(|v| 1.0 + 3.0 * v * v).collect();
    if std::env::var_os("ADRIFT_PACE_DEBUG").is_some() {
        for b in 0..(marks / 20) {
            let r = b * 20..(b + 1) * 20;
            let m = |t: &Vec<f64>| t[r.clone()].iter().sum::<f64>() / 20.0 / (t.iter().sum::<f64>() / marks as f64).max(1e-12);
            eprintln!("days {:5.1}: close {:.2} dchange {:.2} wind {:.2} turn {:.2}", b as f64 * 20.0 * step, m(&terms[0]), m(&terms[1]), m(&terms[2]), m(&terms[3]));
        }
    }
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
    let least = smooth.iter().cloned().fold(f64::INFINITY, f64::min).max(1e-9);
    smooth.iter().map(|x| x.min(least * PACE_RANGE)).collect()
}

/// A path's place on `day`, between its marks.
fn place_on(p: &[(f64, crate::model::LonLat)], day: f64) -> crate::model::LonLat {
    let i = p.partition_point(|m| m.0 <= day + 1e-9).saturating_sub(1);
    match p.get(i + 1) {
        Some(b) if b.0 > p[i].0 => crate::app::lerp_place(p[i].1, b.1, ((day - p[i].0) / (b.0 - p[i].0)).clamp(0.0, 1.0)),
        _ => p[i].1,
    }
}

/// A lamp's own path (its legs, each under its own law and world), every quarter-day mark from its
/// first leg to `to`: (day, place). The lamp's path as the App computes it (`App::extend_path`).
pub fn path(rules: &Rules, f: &Float, to: f64) -> Vec<(f64, crate::model::LonLat)> {
    use crate::sea::Drift;
    let legs = f.legs();
    let mut out = Vec::new();
    for (i, leg) in legs.iter().enumerate() {
        let until = legs.get(i + 1).map_or(to, |l| l.day);
        let e = leg.eddy.map(|e| (e[0], e[1])).unwrap_or_else(|| rules.put(leg.at, leg.day, leg.seed, leg.world).eddy);
        let mut d = Drift { at: leg.at, eddy: e };
        let mut day = leg.day;
        out.push((day, d.at));
        while day < until - 1e-9 {
            let next = crate::app::grid_next(day);
            d = rules.flight_under(leg.law, d, day, next - day, leg.seed, leg.world).drift;
            day = next;
            out.push((day, d.at));
        }
    }
    out
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

    pub(crate) fn rules() -> Rules {
        Rules::load(&DirAssets(assets())).unwrap()
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
                        if !r.coast.is_sea(a.lon as f32, a.lat as f32) {
                            lat += 0.75;
                            continue;
                        }
                        for (dx, dy) in [(0.4, 0.0), (0.0, 0.4), (0.3, 0.3), (0.6, -0.2)] {
                            let b = LonLat { lon: lon + dx, lat: lat + dy };
                            if !r.coast.is_sea(b.lon as f32, b.lat as f32) {
                                continue;
                            }
                            let (fa, fb) = (lamp(1, a, day0, 1), lamp(2, b, day0, 2));
                            let span: f64 = std::env::var("DANCE_DAYS").ok().and_then(|v| v.parse().ok()).unwrap_or(60.0);
                            let (pa, pb) = (path(&r, &fa, day0 + span), path(&r, &fb, day0 + span));
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
        let (a, b) = (path(&r, &f[0], to), path(&r, &f[1], to));
        let (w, near, closest) = dance(&a, &b);
        eprintln!("the dance: the line between them wound {w:+.1} rad in 60 days, 15-150 km apart {:.0}% of the time, closest {closest:.0} km", near * 100.0);
        assert!(w.abs() > 4.0 * std::f64::consts::PI && near > 0.8);
        if let Some(out) = std::env::var_os("ADRIFT_DANCE_OUT") {
            let copy = crate::app::LampsCopy { version: 1, day: to, floats: f };
            std::fs::write(out, format!("{}\n{}\n", crate::app::LAMPS_HEADER, serde_json::to_string(&copy).unwrap())).unwrap();
        }
    }

    #[test]
    fn the_pace_lingers_where_the_lamps_dance() {
        let r = rules();
        let (f, to) = the_dance();
        let from = f[0].put_day;
        let (frames, fps) = (900usize, 30.0);
        let days = pace(&r, &f, from, to, frames, fps);
        assert_eq!(days.len(), frames);
        assert_eq!(*days.last().unwrap(), to, "it ends on the last day");
        assert!(days.windows(2).all(|w| w[1] >= w[0]) && days[0] > from, "never back");
        assert_eq!(days, pace(&r, &f, from, to, frames, fps), "deterministic");
        let per: Vec<f64> = std::iter::once(days[0] - from).chain(days.windows(2).map(|w| w[1] - w[0])).collect();
        let (lo, hi) = per.iter().fold((f64::INFINITY, 0.0f64), |(a, b), x| (a.min(*x), b.max(*x)));
        // smooth: the pace changes by little from frame to frame
        let lurch = per.windows(2).map(|w| (w[1] - w[0]).abs() / w[0].max(1e-12)).fold(0.0, f64::max);
        // slower where they are near: the pace at their nearest marks against the pace at their farthest
        let (a, b) = (path(&r, &f[0], to), path(&r, &f[1], to));
        let dist = |day: f64| {
            let (p, q) = (super::place_on(&a, day), super::place_on(&b, day));
            distance_km(p.lon as f32, p.lat as f32, q.lon as f32, q.lat as f32) as f64
        };
        let mut by: Vec<(f64, f64)> = days.iter().zip(&per).map(|(d, p)| (dist(*d), *p)).collect();
        by.sort_by(|x, y| x.0.partial_cmp(&y.0).unwrap());
        let tenth = by.len() / 10;
        let near = by[..tenth].iter().map(|x| x.1).sum::<f64>() / tenth as f64;
        let far = by[by.len() - tenth..].iter().map(|x| x.1).sum::<f64>() / tenth as f64;
        eprintln!(
            "the dance in 30 s: {:.2}-{:.2} days a second; nearest tenth of frames at {:.2}, farthest at {:.2}; the most a frame's pace changes: {:.1}%",
            lo * fps, hi * fps, near * fps, far * fps, lurch * 100.0
        );
        // the run apart along the coast (days 10-35, 42% of the days, where the critic saw a camera
        // chasing two lamps) passes quickly; the tangle before it and the loops after take the clip
        let coast = days.iter().filter(|d| **d > from + 10.0 && **d < from + 35.0).count() as f64 / frames as f64;
        eprintln!("the run along the coast (days 10-35) takes {:.0}% of the clip", coast * 100.0);
        assert!(hi / lo <= PACE_RANGE * 1.05 && hi / lo > 3.0, "the pace varies boldly, within its range");
        assert!(lurch < 0.05, "no lurch: under 5% from one frame to the next");
        assert!(near < far * 0.5, "much slower where the pair is near");
        // honest about it: the measure finds winding and changing distance along the coast too, so
        // it passes only a little quicker than its share of the days (37% of the clip for 42%)
        assert!(coast < 0.40, "the run along the coast passes quicker than its share of the days");
    }

    #[test]
    fn one_plain_lamp_runs_near_evenly() {
        let r = rules();
        let f = vec![lamp(1, LonLat { lon: -30.0, lat: 20.0 }, 9131.0, 1)];
        let days = pace(&r, &f, 9131.0, 9191.0, 300, 30.0);
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
        let day0 = 9131.0 + 8.0 * 30.44 + 1.0;
        let f = vec![lamp(1, LonLat { lon: 18.75, lat: -42.0 }, day0, 1), lamp(2, LonLat { lon: 18.75, lat: -41.6 }, day0, 2)];
        let to = day0 + 30.0;
        let (a, b) = (path(&r, &f[0], to), path(&r, &f[1], to));
        let (w, near, closest) = dance(&a, &b);
        eprintln!("the early dance: the line wound {w:+.1} rad in 30 days, 15-150 km apart {:.0}% of the time, closest {closest:.0} km", near * 100.0);
        assert!(w.abs() > 4.0 * std::f64::consts::PI && near > 0.8);
        assert_eq!(f[0].legs[0].law, crate::sea::rules::LAW);
        if let Some(out) = std::env::var_os("ADRIFT_DANCE_EARLY_OUT") {
            let copy = crate::app::LampsCopy { version: 1, day: to, floats: f };
            std::fs::write(out, format!("{}\n{}\n", crate::app::LAMPS_HEADER, serde_json::to_string(&copy).unwrap())).unwrap();
        }
    }
}
