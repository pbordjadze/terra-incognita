//! Meander (a working title): a river on the Rio Beni's floodplain that wanders and leaves oxbow lakes.
pub mod sim;

use sim::{dist, rand, resample};

/// The time-lapse: years of the river's life a second on screen.
pub const YEARS_A_SECOND: f64 = 10.0;

/// The river the toy opens on: down the middle of the belt, entering at the bottom (upstream, south)
/// and leaving at the top (north), with the gentle bends of a young course.
pub fn opening_course(seed: u64) -> Vec<[f64; 2]> {
    let mut s = seed | 1;
    let ph: Vec<f64> = (0..3).map(|_| rand(&mut s) * 6.283).collect();
    (0..=110)
        .map(|i| {
            let y = 11000.0 - i as f64 * 200.0;
            let x = 1100.0 + 700.0 * (y / 1500.0 + ph[0]).sin() + 300.0 * (y / 700.0 + ph[1]).sin() + 200.0 * (y / 3700.0 + ph[2]).sin();
            [x, y]
        })
        .collect()
}

/// A drawn course with the small bends any real valley gives a river: a few waves along its length, a
/// little shorter than the bends the river will grow (so the first ones come soon), at a random phase.
pub fn seed_wobble(course: &[[f64; 2]], seed: u64) -> Vec<[f64; 2]> {
    let p = resample(course, 60.0);
    if p.len() < 3 {
        return p;
    }
    let mut s = seed | 1;
    let waves: Vec<(f64, f64, f64)> = (0..3).map(|k| (1900.0 + 900.0 * k as f64 + 600.0 * rand(&mut s), 170.0 + 80.0 * rand(&mut s), rand(&mut s) * 6.283)).collect();
    let mut out = Vec::with_capacity(p.len());
    let mut along = 0.0;
    for i in 0..p.len() {
        if i > 0 {
            along += dist(p[i], p[i - 1]);
        }
        let a = p[i.saturating_sub(1)];
        let b = p[(i + 1).min(p.len() - 1)];
        let l = dist(a, b).max(1e-6);
        let nrm = [-(b[1] - a[1]) / l, (b[0] - a[0]) / l];
        let off: f64 = waves.iter().map(|&(wl, amp, ph)| amp * (along / wl * 6.283 + ph).sin()).sum();
        out.push([p[i][0] + nrm[0] * off, p[i][1] + nrm[1] * off]);
    }
    out
}

/// The course carried on straight past both of its ends until it is well outside `bounds`
/// ([x0, y0, x1, y1], m), so the river comes from somewhere and goes somewhere.
pub fn extend(course: &[[f64; 2]], bounds: [f64; 4]) -> Vec<[f64; 2]> {
    let p = resample(course, 60.0);
    if p.len() < 2 {
        return p;
    }
    let dir = |from: [f64; 2], to: [f64; 2]| {
        let l = dist(from, to).max(1e-6);
        [(to[0] - from[0]) / l, (to[1] - from[1]) / l]
    };
    let k = (p.len() - 1).min(8);
    let outside = |q: [f64; 2]| q[0] < bounds[0] - 2500.0 || q[0] > bounds[2] + 2500.0 || q[1] < bounds[1] - 2500.0 || q[1] > bounds[3] + 2500.0;
    let mut head = Vec::new();
    let d0 = dir(p[k], p[0]);
    let mut q = p[0];
    while !outside(q) && head.len() < 400 {
        q = [q[0] + d0[0] * 200.0, q[1] + d0[1] * 200.0];
        head.push(q);
    }
    head.reverse();
    let n = p.len();
    let d1 = dir(p[n - 1 - k], p[n - 1]);
    let mut tail = Vec::new();
    let mut q = p[n - 1];
    while !outside(q) && tail.len() < 400 {
        q = [q[0] + d1[0] * 200.0, q[1] + d1[1] * 200.0];
        tail.push(q);
    }
    let mut out = head;
    out.extend_from_slice(&p);
    out.extend(tail);
    out
}

mod toy;
pub use toy::Meander;
