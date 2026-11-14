//! The stand the toy opens on, grown rather than drawn: old growth planted five centuries back, then
//! the years run, with three giants felled at chosen times so that the first look holds every stage:
//! an old log long gone, its trees standing in a row on stilts; a log of a century with its young trees
//! thinning; a fresh log near the eye with its first line of seedlings.

use crate::forest::{Forest, FALL_S, X, Z1};

/// When each chosen giant went over (years before the opening), where its foot was, its way down, and
/// how tall it was (m).
const FELLED: [(f32, [f32; 2], [f32; 2], f32); 3] = [
    (-330.0, [-11.0, 17.0], [0.85, 0.527], 66.0),
    (-110.0, [12.0, 22.0], [0.2, 0.98], 62.0),
    (-14.0, [-6.5, 9.2], [0.94, 0.341], 56.0),
];
const START: f32 = -480.0;

fn clear_of(p: [f32; 2], room: f32) -> bool {
    FELLED.iter().all(|&(_, a, d, h)| {
        let len = h * 0.8;
        let ap = [p[0] - a[0], p[1] - a[1]];
        let t = (ap[0] * d[0] + ap[1] * d[1]).clamp(-2.0, len);
        let q = [a[0] + d[0] * t - p[0], a[1] + d[1] * t - p[1]];
        (q[0] * q[0] + q[1] * q[1]).sqrt() > room
    })
}

pub fn opening(seed: u64) -> Forest {
    let mut f = Forest::new(seed);
    // the old growth: giants some nine metres apart, none where a chosen giant will lie
    let mut at: Vec<[f32; 2]> = Vec::new();
    for _ in 0..3000 {
        let p = [f.rng.range(-X, X), f.rng.range(12.0, Z1 * 0.8)];
        // (and none between the eye and the fresh log and its row behind it, but at the sides)
        let open = p[1] < 16.0 && p[0].abs() < p[1] * 0.6;
        if open || !clear_of(p, 3.0) || at.iter().any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < 8.5) {
            continue;
        }
        at.push(p);
        let h = f.rng.range(46.0, 70.0);
        // some stood on logs long gone themselves
        let base = if f.rng.f() < 0.3 {
            f.rng.range(0.6, 2.2)
        } else {
            0.0
        };
        f.plant(p[0], p[1], h, base);
    }
    f.year = START as f64;
    // only the chosen giants fall in the history, so the first look is clear
    f.natural = false;
    let mut next = 0;
    while f.year < 0.0 {
        while next < FELLED.len() && f.year as f32 >= FELLED[next].0 {
            let (_, a, d, h) = FELLED[next];
            f.plant(a[0], a[1], h, 0.0);
            let k = f.trees.len() - 1;
            f.felled_long_ago(k, d);
            next += 1;
        }
        f.run(1.0);
        // what fell by itself in those years is long down
        f.tick(FALL_S);
    }
    f.landed.clear();
    f.started = 0;
    f.natural = true;
    f.year = 0.0;
    f
}
