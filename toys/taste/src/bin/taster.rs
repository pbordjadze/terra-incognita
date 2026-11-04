//! A pretend reader with a taste of their own, for tuning the walk and writing plans (no GPU).
//!
//!     cargo run -p taste --bin taste-taster --release -- stats
//!     cargo run -p taste --bin taste-taster --release -- plan <taste> <seconds> <out.json>
//!
//! A taste is a favourite in two or three of the four numbers (the rest the reader doesn't mind). The reader
//! holds a finger on the creature while what they saw a moment ago is close enough to it, and lifts
//! when it isn't (with a little hysteresis). `stats` plays several tastes and seeds and prints how much
//! of each minute the creature spent near the taste, against a reader who never touches; `plan` writes
//! the holds of one such reader as a plan for the check (the walk is deterministic: the toy plays the
//! same walk from the same holds, frame for frame at the plan's fps).

use taste::walk::{Form, Walker, D};

struct Taste {
    name: &'static str,
    /// (number, favourite value)
    fav: &'static [(usize, f64)],
    width: f64,
}

const TASTES: &[Taste] = &[
    Taste { name: "warm-spiky", fav: &[(0, 0.04), (1, 0.85)], width: 0.2 },
    Taste { name: "blue-jelly", fav: &[(0, 0.58), (3, 0.9), (1, 0.15)], width: 0.22 },
    Taste { name: "many-matte", fav: &[(2, 0.9), (3, 0.1)], width: 0.2 },
    Taste { name: "green-calm", fav: &[(0, 0.33), (2, 0.1)], width: 0.2 },
];

fn closeness(t: &Taste, f: &Form) -> f64 {
    let mut d2 = 0.0;
    for &(i, v) in t.fav {
        let mut d = f[i] - v;
        if i == 0 {
            d = (d + 0.5).rem_euclid(1.0) - 0.5;
        }
        d2 += d * d;
    }
    (-0.5 * d2 / (t.width * t.width)).exp()
}

pub const START: Form = taste::START;

/// Plays a reader of taste `t` (who touches only when `touch`; the closeness is to `t` either way) for `seconds` at `fps`; returns per-frame
/// (holding, closeness of the creature's kept form) and the walker.
fn play(t: Option<&Taste>, touch: bool, rests: bool, seed: u64, seconds: f64, fps: f64) -> (Vec<bool>, Vec<f64>, Walker) {
    let mut w = Walker::new(seed, START);
    let dt = 1.0 / fps;
    let frames = (seconds * fps).ceil() as usize;
    let mut seen: std::collections::VecDeque<Form> = Default::default();
    let lag = (0.45 * fps) as usize;
    let mut holding = false;
    let mut since = 0.0;
    let mut free_since = f64::NEG_INFINITY;
    // a reader who rests now and then: after a long hold, sometimes a pause of 6-15 s without touching
    let mut resting_until = 0.0;
    let mut r = taste::walk::Rng(seed.wrapping_mul(0x2545_f491) | 1);
    let (mut holds, mut near) = (Vec::new(), Vec::new());
    for f in 0..=frames {
        let time = f as f64 / fps;
        seen.push_back(w.display());
        if seen.len() > lag {
            seen.pop_front();
        }
        let leaving = std::env::var("TASTER_LEAVE").ok().and_then(|v| v.parse::<f64>().ok()).is_some_and(|l| time >= l);
        if leaving && holding {
            holding = false;
            free_since = time - dt;
        }
        if let (Some(t), true, false) = (t, touch, leaving) {
            let c = closeness(t, &seen[0]);
            since += dt;
            // they start after a first look, and a hold or a pause lasts at least 0.6 s
            if time > 3.0 && since > 0.6 && time >= resting_until {
                if !holding && c > 0.35 {
                    holding = true;
                    since = 0.0;
                } else if holding && (c < 0.2 || (since > 8.0 && rests && r.f() < 0.004)) {
                    if c >= 0.2 {
                        resting_until = time + 6.0 + 9.0 * r.f();
                    }
                    holding = false;
                    since = 0.0;
                    free_since = time - dt;
                }
            }
        }
        holds.push(holding);
        near.push(t.map(|t| closeness(t, &w.x)).unwrap_or(0.0));
        // the frame: the check dispatches events at t, then steps by dt (dt is 0 on the first frame)
        // (as the host does: this frame's time less the last's, not a constant 1/fps, so the walk's
        // phases end on the same frames)
        let step = if f == 0 { 0.0 } else { (f as f64 / fps - (f - 1) as f64 / fps).clamp(0.0, 0.1) };
        let engaged = holding || time - free_since < taste::walk::ENGAGED || (free_since == f64::NEG_INFINITY && touch);
        w.step(step, if holding { step } else { 0.0 }, engaged);
    }
    (holds, near, w)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("stats") => {
            let rests = args.get(2).map(String::as_str) != Some("steady");
            let fps = 30.0;
            let mut total = Vec::new();
            for t in TASTES {
                let mut rows = Vec::new();
                let mut score = 0.0;
                for seed in 1..=6u64 {
                    let (_, near, w) = play(Some(t), true, rests, taste::SEED ^ (seed * 0x9e37_79b9), 240.0, fps);
                    let (_, near0, _) = play(Some(t), false, false, taste::SEED ^ (seed * 0x9e37_79b9), 240.0, fps);
                    // the share of each minute the kept form is close to the taste (> 0.5)
                    let share = |v: &[f64], a: f64, b: f64| {
                        let s = &v[(a * fps) as usize..(b * fps) as usize];
                        s.iter().sum::<f64>() / s.len() as f64
                    };
                    score += (share(&near, 120.0, 240.0) - share(&near0, 120.0, 240.0)) / 6.0;
                    rows.push(format!(
                        "  seed {seed}: near {:.2} {:.2} {:.2} {:.2} (no touch {:.2})  steps {} kept {}",
                        share(&near, 0.0, 60.0),
                        share(&near, 60.0, 120.0),
                        share(&near, 120.0, 180.0),
                        share(&near, 180.0, 240.0),
                        share(&near0, 0.0, 240.0),
                        w.steps,
                        w.path.len() - 1
                    ));
                }
                println!("{}:\n{}", t.name, rows.join("\n"));
                total.push(score);
            }
            println!("score (share near the taste, minutes 3-4, over a reader who never touches): {:.3?} mean {:.3}", total, total.iter().sum::<f64>() / total.len() as f64);
        }
        Some("plan") => {
            let name = &args[2];
            let seconds: f64 = args[3].parse().unwrap();
            let t = TASTES.iter().find(|t| t.name == name).expect("taste");
            let fps = 30.0;
            let (holds, near, w) = play(Some(t), true, true, taste::SEED, seconds, fps);
            let mut events = Vec::new();
            let mut start = None;
            for (f, h) in holds.iter().enumerate() {
                let time = f as f64 / fps;
                match (h, start) {
                    (true, None) => start = Some(time),
                    (false, Some(s)) => {
                        events.push(format!("    {{ \"t\": {:.5}, \"hold\": \"creature\", \"for\": {:.5} }}", s - 0.001, time - s));
                        start = None;
                    }
                    _ => {}
                }
            }
            if let Some(s) = start {
                events.push(format!("    {{ \"t\": {:.5}, \"hold\": \"creature\", \"for\": {:.5} }}", s - 0.001, seconds - s + 0.5));
            }
            eprintln!("{} holds; kept form near the taste at the end: {:.2}; steps {}", events.len(), near.last().unwrap(), w.steps);
            let _ = D;
            println!("{}", events.join(",\n"));
        }
        _ => eprintln!("usage: taste-taster stats | plan <taste> <seconds>"),
    }
}
