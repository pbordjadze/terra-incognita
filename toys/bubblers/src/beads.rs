//! Bubblers made of beads, after Ishu Patel's *Bead Game* (National Film Board of Canada, 1977), in which
//! "thousands of beads are arranged and manipulated, assuming shapes of creatures both mythical and real",
//! moved "under an ever-zooming-outward camera" (SOURCES.md). Here every ball a crab rolls is a bead, every
//! crab a figure of beads, every burrow a ring of them, laid on a dark ground; the sand's ripples and the
//! tide are beads too (`sand.wgsl`). The toy already opens close and pulls back, as Patel's camera did.

use crate::beach::{Beach, Crab, State};
use toy_kit::paint::Painter;

/// Bright beads for the balls, as the film's "brightly colored beads" (straight colours).
const BRIGHT: [[f32; 3]; 7] = [
    [0.86, 0.16, 0.12],
    [0.96, 0.52, 0.10],
    [0.98, 0.82, 0.18],
    [0.12, 0.70, 0.68],
    [0.16, 0.34, 0.80],
    [0.94, 0.92, 0.86],
    [0.30, 0.66, 0.24],
];
/// The crabs' beads, a pink no ball takes: shell, a darker one for the legs, and the eyes'.
const SHELL: [f32; 3] = [0.97, 0.66, 0.70];
const MOTTLE: [f32; 3] = [0.84, 0.46, 0.54];
/// The shell's rim, darker, so the carapace has an edge; the claws, cream.
const RIM: [f32; 3] = [0.58, 0.22, 0.30];
const CLAW: [f32; 3] = [0.98, 0.93, 0.82];
const JET: [f32; 3] = [0.06, 0.06, 0.07];

fn hash(x: f32, y: f32) -> f32 {
    let h = (x * 12.9898 + y * 78.233).sin() * 43_758.547;
    h - h.floor()
}

/// One glass bead at `c` (px), radius `r` (px): its body, the shade toward its lower right, the hole
/// threaded through it, and the light on its upper left.
pub fn bead(p: &mut Painter, c: [f32; 2], r: f32, col: [f32; 3], a: f32) {
    if r < 0.4 {
        return;
    }
    let pm = |k: f32, al: f32| [col[0] * k * al, col[1] * k * al, col[2] * k * al, al];
    p.disc(c, r, (r * 0.18).max(0.6), pm(0.55, a));
    p.disc(
        [c[0] - r * 0.12, c[1] - r * 0.12],
        r * 0.8,
        r * 0.3,
        pm(1.0, a),
    );
    p.disc(c, r * 0.24, (r * 0.1).max(0.5), [0.0, 0.0, 0.0, 0.85 * a]);
    p.disc(
        [c[0] - r * 0.42, c[1] - r * 0.42],
        r * 0.2,
        r * 0.15,
        [0.9 * a, 0.9 * a, 0.88 * a, 0.9 * a],
    );
}

/// The balls: each a bead, its colour shared by the balls a crab rolled on one run, so a starburst is
/// spokes of colour.
pub fn balls(p: &mut Painter, b: &Beach, px: &dyn Fn([f32; 2]) -> [f32; 2], s: f32) {
    for q in &b.pellets {
        let grow = ((b.t - q.born) / 0.18).clamp(0.0, 1.0);
        let melt = if q.gone >= 0.0 {
            1.0 - ((b.t - q.gone) / 1.5).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let r = q.r * grow * (0.5 + 0.5 * melt);
        if r <= 0.0 {
            continue;
        }
        let run = (q.born / 3.0).floor();
        let k = hash(run, (q.p[0] / 2.5).floor() * 7.0 + (q.p[1] / 2.5).floor());
        let col = BRIGHT[(k * BRIGHT.len() as f32) as usize % BRIGHT.len()];
        bead(p, px(q.p), r * s, col, melt);
    }
}

/// A burrow: a ring of the sand's beads round its dark mouth.
pub fn burrows(p: &mut Painter, b: &Beach, px: &dyn Fn([f32; 2]) -> [f32; 2], s: f32) {
    let front = b.front().unwrap_or(f32::MAX);
    for c in &b.crabs {
        if c.home[1] > front || !c.dug {
            continue;
        }
        let n = 9;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU + c.home[0];
            let q = [c.home[0] + a.cos() * 0.36, c.home[1] + a.sin() * 0.36];
            bead(p, px(q), 0.11 * s, [0.78, 0.68, 0.50], 1.0);
        }
    }
}

/// A crab of beads: a round body packed with them, mottled, legs as strings of beads, claws, and two
/// jet beads for its eyes (only the eyes, at the mouth of the hole, while it peeks).
pub fn crab(p: &mut Painter, c: &Crab, s: f32, t: f32, px: &dyn Fn([f32; 2]) -> [f32; 2]) {
    let (fx, fy) = (c.facing.cos(), c.facing.sin());
    let at = |along: f32, across: f32| {
        [
            c.p[0] + fx * along - fy * across,
            c.p[1] + fy * along + fx * across,
        ]
    };
    // the same beads as the balls', about: a crab is a few dozen of them
    let br = 0.11 * c.size;
    match c.state {
        State::Hidden(_) => {}
        State::Peek(_) => {
            let wob = (t * 3.0 + c.gait).sin() * 0.03;
            for side in [-1.0f32, 1.0] {
                bead(p, px(at(0.05, side * 0.16 + wob)), 0.07 * s, JET, 1.0);
            }
        }
        _ => {
            // eight legs, each a strand of three touching beads, a black gap between them and the shell,
            // the last bead bent back
            let step = 2.05 * br;
            for side in [-1.0f32, 1.0] {
                for leg in 0..4 {
                    let a = (leg as f32 - 1.5) * 0.55;
                    let swing =
                        (c.gait + leg as f32 * 1.7 + side * 0.8).sin() * 0.2 * c.moving.min(1.0);
                    let dir = -(a + swing);
                    let mut q = at(step * 2.35 * dir.sin(), side * step * 2.35 * dir.cos());
                    for k in 0..3 {
                        bead(p, px(q), br * 0.8 * s, MOTTLE, 1.0);
                        // the next bead touching this one; the foot's bent back, away from the claws
                        let d = if k == 1 { dir - 0.55 } else { dir };
                        q = [
                            q[0] + (fx * d.sin() - fy * side * d.cos()) * br * 1.6,
                            q[1] + (fy * d.sin() + fx * side * d.cos()) * br * 1.6,
                        ];
                    }
                }
            }
            // the carapace: a big pale bead inside a rim of eight darker ones
            bead(p, px(c.p), br * 1.3 * s, SHELL, 1.0);
            for i in 0..8 {
                let a = i as f32 / 8.0 * std::f32::consts::TAU + c.facing;
                let q = [c.p[0] + a.cos() * step * 1.1, c.p[1] + a.sin() * step * 1.1];
                bead(p, px(q), br * 0.8 * s, RIM, 1.0);
            }
            // the claws, held forward, cream: an arm bead and a big pincer
            for side in [-1.0f32, 1.0] {
                bead(
                    p,
                    px(at(step * 2.0, side * step * 1.15)),
                    br * 0.85 * s,
                    CLAW,
                    1.0,
                );
                bead(
                    p,
                    px(at(step * 3.0, side * step * 1.0)),
                    br * 1.45 * s,
                    CLAW,
                    1.0,
                );
            }
            // the eyes at the front, each a jet bead on a short stalk
            for side in [-1.0f32, 1.0] {
                bead(
                    p,
                    px(at(step * 1.7, side * step * 0.38)),
                    br * 0.4 * s,
                    RIM,
                    1.0,
                );
                bead(
                    p,
                    px(at(step * 2.1, side * step * 0.42)),
                    br * 0.6 * s,
                    JET,
                    1.0,
                );
            }
        }
    }
}
