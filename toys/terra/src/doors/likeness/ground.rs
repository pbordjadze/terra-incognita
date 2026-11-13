//! The likenesses of the toys whose rule runs on the ground: lightning reaching for a night lake, crabs'
//! starbursts on sand, crescent dunes walking, the moon painting its road down a lake.

use toy_kit::paint::Painter;

fn smooth(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn hash(n: f64) -> f64 {
    ((n * 12.9898).sin() * 43758.5453).fract().abs()
}

/// Where a light's likeness is drawn: its middle `c` (pixels), radius `r` (points), `k` pixels a point
/// (`kf` the same as f32), its strength `a` and its window's `halo`.
pub struct Spot {
    pub c: [f32; 2],
    pub r: f64,
    pub k: f64,
    pub kf: f32,
    pub a: f32,
    pub halo: f32,
}

impl Spot {
    pub fn new(c: [f32; 2], r: f64, k: f64, kf: f32, a: f32, halo: f32) -> Spot {
        Spot {
            c,
            r,
            k,
            kf,
            a,
            halo,
        }
    }

    fn at(&self, x: f64, y: f64) -> [f32; 2] {
        [
            self.c[0] + (x * self.r * self.k) as f32,
            self.c[1] + (y * self.r * self.k) as f32,
        ]
    }

    /// The window's glass: a disc of `rgb` over the light.
    fn glass(&self, p: &mut Painter, rgb: [f32; 3]) {
        p.disc(
            self.c,
            (self.r * 1.05 * self.k) as f32,
            (self.r * 0.2 * self.k) as f32,
            [rgb[0], rgb[1], rgb[2], 0.95 * self.a * self.halo],
        );
    }
}

pub fn draw(p: &mut Painter, id: &str, sp: &Spot, tm: f64) {
    match id {
        "catatumbo" => catatumbo(p, sp, tm),
        "bubblers" => bubblers(p, sp, tm),
        "barchans" => barchans(p, sp, tm),
        "moonroad" => moonroad(p, sp, tm),
        _ => {}
    }
}

fn catatumbo(p: &mut Painter, sp: &Spot, tm: f64) {
    let (r, k, kf, a) = (sp.r, sp.k, sp.kf, sp.a);
    let at = |x: f64, y: f64| sp.at(x, y);
    // night over a lake: a stroke now and then, forking down to the water, the lake lit with it
    let ph = (tm / 1.7).rem_euclid(1.0);
    let n = (tm / 1.7).floor();
    let flash =
        ((1.0 - ph / 0.12).max(0.0) + 0.6 * (1.0 - (ph - 0.18).abs() / 0.06).max(0.0)) as f32;
    // the deck never quite dark: a flicker somewhere in it most of the time
    let fl = (0.5 + 0.5 * (tm * 7.3).sin() * (tm * 2.9 + 1.0).sin()).powi(2) as f32;
    sp.glass(p, [0.20 + 0.25 * fl, 0.16 + 0.20 * fl, 0.38 + 0.30 * fl]);
    p.disc(
        at(0.0, 0.45),
        (r * 0.7 * k) as f32,
        (r * 0.4 * k) as f32,
        [0.75, 0.66, 1.0, 0.35 * flash * a],
    );
    p.disc(
        at(0.0, -0.5),
        (r * 0.8 * k) as f32,
        (r * 0.6 * k) as f32,
        [0.75, 0.66, 1.0, 0.45 * flash * a],
    );
    let mut q = at((hash(n) - 0.5) * 0.5, -0.85);
    let mut x = (hash(n) - 0.5) * 0.5;
    for j in 1..=8 {
        let y = -0.85 + j as f64 * 0.16;
        x += (hash(n * 7.0 + j as f64) - 0.5) * 0.3;
        let nq = at(x, y);
        p.glow(
            q,
            nq,
            0.5 * kf,
            2.5 * kf,
            [0.9, 0.85, 1.0, flash.min(1.0) * a],
        );
        if j == 3 || j == 5 {
            let side = if hash(n + j as f64) > 0.5 { 1.0 } else { -1.0 };
            p.glow(
                nq,
                at(x + side * 0.25, y + 0.2),
                0.3 * kf,
                1.5 * kf,
                [0.8, 0.75, 1.0, 0.7 * flash * a],
            );
        }
        q = nq;
    }
}

fn bubblers(p: &mut Painter, sp: &Spot, tm: f64) {
    let (kf, a) = (sp.kf, sp.a);
    let at = |x: f64, y: f64| sp.at(x, y);
    // pale sand from above: a few burrows, the crabs' balls laid out from each in spokes, the spokes
    // growing round, and the tide taking them now and then
    sp.glass(p, [0.86, 0.80, 0.68]);
    let ph = (tm / 14.0).rem_euclid(1.0);
    let shown = smooth(0.0, 0.8, ph) * (1.0 - smooth(0.92, 1.0, ph));
    for (b, (bx, by)) in [(-0.35, -0.3), (0.38, -0.1), (-0.05, 0.42)]
        .iter()
        .enumerate()
    {
        p.disc(at(*bx, *by), 1.6 * kf, 1.0, [0.15, 0.12, 0.10, 0.9 * a]);
        let spokes = (shown * 9.0) as usize;
        for s in 0..spokes {
            let ang = s as f64 * 0.7 + b as f64 * 2.0;
            for d in 1..5 {
                let rr = 0.06 * d as f64;
                let q = at(bx + ang.cos() * rr, by + ang.sin() * rr);
                p.disc(q, 1.0 * kf, 0.7, [0.72, 0.64, 0.50, 0.95 * a]);
            }
        }
    }
}

fn barchans(p: &mut Painter, sp: &Spot, tm: f64) {
    let (kf, a) = (sp.kf, sp.a);
    let at = |x: f64, y: f64| sp.at(x, y);
    // pale reg from high above, crescents walking down it, horns ahead, each with its dark
    // slip face in the hollow between the horns
    sp.glass(p, [0.74, 0.66, 0.55]);
    for (j, (x0, s)) in [(-0.35, 1.0), (0.3, 0.75), (0.0, 0.55)].iter().enumerate() {
        let y = ((tm * 0.05 / s + j as f64 * 0.37).rem_euclid(1.0)) * 1.6 - 0.8;
        let fade = (1.0 - (y / 0.8).abs().powi(4)) as f32;
        let arc = |i: usize, inner: f64| {
            let u = i as f64 / 12.0 * std::f64::consts::PI;
            at(
                x0 + u.cos() * 0.22 * s * inner,
                y - u.sin() * 0.12 * s * inner + 0.1 * s,
            )
        };
        for i in 0..12 {
            let kw = (1.6 * s) as f32 * kf;
            p.line(
                arc(i, 1.0),
                arc(i + 1, 1.0),
                kw,
                0.8,
                [0.98, 0.88, 0.70, 0.95 * a * fade],
            );
            if (2..10).contains(&i) {
                p.line(
                    arc(i, 0.8),
                    arc(i + 1, 0.8),
                    kw * 0.5,
                    0.8,
                    [0.30, 0.24, 0.20, 0.8 * a * fade],
                );
            }
        }
    }
}

/// Moon Road: a plum night over a dark template lake, the low moon, and under it its road of cells painted in moonlight,
/// a glint flashing in it here and there.
fn moonroad(p: &mut Painter, sp: &Spot, tm: f64) {
    let (kf, a) = (sp.kf, sp.a);
    let at = |x: f64, y: f64| sp.at(x, y);
    sp.glass(p, [0.24, 0.13, 0.29]);
    // the lake: the lower half of the glass, the dark paper
    for j in 0..5 {
        let y = 0.12 + j as f64 * 0.17;
        let half = (1.0 - y * y).max(0.0).sqrt() * 0.92;
        p.line(
            at(-half, y),
            at(half, y),
            (0.1 * sp.r * sp.k) as f32,
            kf,
            [0.11, 0.08, 0.15, 0.95 * a],
        );
    }
    p.disc(at(0.0, -0.22), 3.2 * kf, 0.8 * kf, [0.98, 0.90, 0.68, a]);
    // the road: cells in the three paints, widening toward you, one flashing
    let paints = [[0.97, 0.95, 0.88], [0.94, 0.87, 0.72], [0.80, 0.75, 0.89]];
    for i in 0..14 {
        let y = 0.06 + (i as f64 / 13.0).powf(1.3) * 0.82;
        let x = (hash(i as f64 + 3.0) - 0.5) * (0.06 + 0.3 * y);
        let c = paints[i % 3];
        let flash = (0.5 + 0.5 * (tm * 2.3 + i as f64 * 1.7).sin()).powi(8) as f32;
        let col = [
            c[0] + (1.0 - c[0]) * flash,
            c[1] + (0.98 - c[1]) * flash,
            c[2] + (0.92 - c[2]) * flash,
            a,
        ];
        p.disc(at(x, y), (0.8 + 2.6 * y) as f32 * kf, 0.5 * kf, col);
    }
}
