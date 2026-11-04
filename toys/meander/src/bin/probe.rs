//! The river alone, no GPU: how long a few kinds of drawn course take to their first oxbow, and
//! whether the line stays clean (no self-crossing, no kinks). For tuning the time-lapse.
//!
//!     cargo run -p meander --bin meander-probe --release [-- <years a second>]
use meander::sim::{dist, River};

fn crosses(p: &[[f64; 2]]) -> bool {
    let seg = |a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]| {
        let o = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
        o(a, b, c) * o(a, b, d) < 0.0 && o(c, d, a) * o(c, d, b) < 0.0
    };
    for i in 0..p.len() - 1 {
        for j in i + 2..p.len() - 1 {
            if seg(p[i], p[i + 1], p[j], p[j + 1]) {
                return true;
            }
        }
    }
    false
}

fn max_turn(p: &[[f64; 2]]) -> f64 {
    let mut m: f64 = 0.0;
    for w in p.windows(3) {
        let a = [w[1][0] - w[0][0], w[1][1] - w[0][1]];
        let b = [w[2][0] - w[1][0], w[2][1] - w[1][1]];
        let ang = (a[0] * b[1] - a[1] * b[0]).atan2(a[0] * b[0] + a[1] * b[1]).abs();
        m = m.max(ang);
    }
    m.to_degrees()
}

fn main() {
    let yps: f64 = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(meander::YEARS_A_SECOND);
    let courses: Vec<(&str, Vec<[f64; 2]>)> = vec![
        ("opening", meander::opening_course(7)),
        ("straight", (0..60).map(|i| [0.0, 12000.0 - i as f64 * 400.0]).collect()),
        ("wobbly hand", (0..60).map(|i| { let y = 12000.0 - i as f64 * 400.0; [300.0 * (y / 2300.0).sin() + 90.0 * (y / 700.0).cos(), y] }).collect()),
        ("diagonal", (0..60).map(|i| [-4000.0 + i as f64 * 140.0, 12000.0 - i as f64 * 380.0]).collect()),
    ];
    for (name, c) in courses {
        let line = meander::seed_wobble(&meander::extend(&c, [-11000.0, -11000.0, 11000.0, 11000.0]), 11);
        let mut r = River::new(line, 5);
        let mut first = None;
        let mut bad = String::new();
        let dump = std::env::var("PROBE_DUMP").ok();
        let mut frames: Vec<String> = Vec::new();
        for f in 0..(60 * 30) {
            if dump.is_some() && f % 60 == 0 {
                let l: Vec<String> = r.line.iter().map(|p| format!("[{:.0},{:.0}]", p[0], p[1])).collect();
                let o: Vec<String> = r.oxbows.iter().map(|ox| { let v: Vec<String> = ox.line.iter().map(|p| format!("[{:.0},{:.0}]", p[0], p[1])).collect(); format!("[{}]", v.join(",")) }).collect();
                frames.push(format!("{{\"t\":{},\"line\":[{}],\"ox\":[{}]}}", f / 30, l.join(","), o.join(",")));
            }
            r.advance(yps / 30.0);
            if first.is_none() && !r.cuts.is_empty() {
                first = Some(f as f64 / 30.0);
            }
            if f % 30 == 0 && bad.is_empty() {
                if crosses(&r.line) {
                    bad = format!("crossed at {:.1}s", f as f64 / 30.0);
                }
                let t = max_turn(&r.line);
                if t > 60.0 {
                    bad = format!("kink {t:.0} deg at {:.1}s", f as f64 / 30.0);
                }
            }
        }
        if let Some(d) = &dump {
            std::fs::write(format!("{d}/{}.json", name.replace(' ', "_")), format!("[{}]", frames.join(","))).unwrap();
        }
        let gaps: Vec<String> = r.cuts.iter().take(8).map(|c| format!("{:.1}", c.0 / yps)).collect();
        println!(
            "{name:12} first oxbow {:>5} s; cuts in 60 s: {:2} (at {}); length {:.0} km; {}",
            first.map(|f| format!("{f:.1}")).unwrap_or("-".into()),
            r.cuts.len(),
            gaps.join(" "),
            r.length() / 1000.0,
            if bad.is_empty() { "clean".to_string() } else { bad }
        );
        let _ = dist;
    }
}
