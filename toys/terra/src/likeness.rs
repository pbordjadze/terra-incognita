//! The small likenesses some lights hold of their scene toys (doors.rs draws the rest): a window of dark
//! glass growing frost; lightning over a night lake; crabs' starbursts on sand; crescent dunes walking.
//! `c` is the light's middle (pixels), `r` its radius (points) and `k` pixels a point, `kf` the same as
//! f32, `a` its strength, `halo` its window's, `tm` the clock.

use crate::doors::Door;
use crate::rooms;
use toy_kit::paint::Painter;

/// The scenes (toys without a globe of their own), which open out of their light as a window.
pub const SCENES: [&str; 7] = [
    "murmuration",
    "meander",
    "frost",
    "throats",
    "catatumbo",
    "bubblers",
    "barchans",
];

/// The lights of the scenes that came last (painting-company row 327), after doors.rs's.
pub fn doors() -> [Door; 3] {
    [
        Door {
            id: "catatumbo",
            name: "Catatumbo",
            at: [catatumbo::PLACE.0, catatumbo::PLACE.1],
            tint: [0.70, 0.62, 1.0],
            night: true,
            degree: 1,
            open: rooms::open::<catatumbo::Catatumbo>,
        },
        Door {
            id: "bubblers",
            name: "Bubblers",
            at: [bubblers::PLACE.0, bubblers::PLACE.1],
            tint: [0.90, 0.80, 0.62],
            night: false,
            degree: 8,
            open: rooms::open::<bubblers::Bubblers>,
        },
        Door {
            id: "barchans",
            name: "Barchans",
            at: [barchans::PLACE.0, barchans::PLACE.1],
            tint: [0.93, 0.64, 0.36],
            night: false,
            degree: -1,
            open: rooms::open::<barchans::Barchans>,
        },
    ]
}

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
        "frost" => frost(p, sp, tm),
        "catatumbo" => catatumbo(p, sp, tm),
        "bubblers" => bubblers(p, sp, tm),
        "barchans" => barchans(p, sp, tm),
        _ => {}
    }
}

fn frost(p: &mut Painter, sp: &Spot, tm: f64) {
    let (c, kf, a) = (sp.c, sp.kf, sp.a);
    let at = |x: f64, y: f64| sp.at(x, y);
    // a window of dark glass at -50°: frost growing on it in six-armed feathers, then fading
    // as if a finger had warmed it, and growing back
    sp.glass(p, [0.06, 0.10, 0.20]);
    let ph = (tm / 9.0).rem_euclid(1.0);
    let grow = smooth(0.0, 0.7, ph) * (1.0 - smooth(0.85, 1.0, ph));
    let ice = [0.80, 0.90, 1.0, 0.9 * a];
    for arm in 0..6 {
        let ang = arm as f64 * std::f64::consts::FRAC_PI_3 + 0.3;
        let (sa, ca) = ang.sin_cos();
        let len = 0.85 * grow;
        p.line(c, at(ca * len, sa * len), 0.9 * kf, 0.8, ice);
        for b in 1..4 {
            let along = len * b as f64 / 4.0;
            let bl = (len - along) * 0.45;
            let o = at(ca * along, sa * along);
            for side in [-1.0f64, 1.0] {
                let ba = ang + side * 1.0;
                let tip = at(ca * along + ba.cos() * bl, sa * along + ba.sin() * bl);
                p.line(o, tip, 0.6 * kf, 0.8, ice);
            }
        }
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
    let fl = (0.5 + 0.5 * (tm * 7.3).sin() * (tm * 2.9 + 1.0).sin()).powi(3) as f32;
    sp.glass(p, [0.10 + 0.10 * fl, 0.08 + 0.08 * fl, 0.22 + 0.16 * fl]);
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
