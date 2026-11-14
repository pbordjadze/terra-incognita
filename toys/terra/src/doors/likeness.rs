//! The small likenesses in the lights of the toys whose likeness lives outside `doors.rs`: Bats' column
//! out of the cave, Huddle's penguins in the snow, the Wave going round the stadium; and the breathing
//! light of a toy without a likeness of its own yet.

use std::f64::consts::TAU;
use toy_kit::paint::Painter;

mod ground;

/// Where and how a light is drawn: at `c` (pixels), `r` points across at `k` pixels a point; `tm` the
/// time (still when motion is reduced), `a` its visibility, `halo` its window's strength.
struct Light {
    c: [f32; 2],
    r: f64,
    k: f64,
    tm: f64,
    a: f32,
    halo: f32,
}

impl Light {
    /// A point `x`, `y` light-radii from the middle, in pixels.
    fn at(&self, x: f64, y: f64) -> [f32; 2] {
        [
            self.c[0] + (x * self.r * self.k) as f32,
            self.c[1] + (y * self.r * self.k) as f32,
        ]
    }

    /// `f` light-radii, in pixels.
    fn len(&self, f: f64) -> f32 {
        (f * self.r * self.k) as f32
    }

    /// The window's ground: a soft disc of `col`.
    fn window(&self, p: &mut Painter, col: [f32; 3]) {
        p.disc(
            self.c,
            self.len(1.05),
            self.len(0.2),
            [col[0], col[1], col[2], 0.95 * self.a * self.halo],
        );
    }
}

/// One light's likeness (`i` its index, `tint` its colour); the rest as `Light` says.
#[allow(clippy::too_many_arguments)] // a drawing call: each argument is one thing it draws with
pub fn paint(
    p: &mut Painter,
    id: &str,
    c: [f32; 2],
    r: f64,
    k: f64,
    tm: f64,
    a: f32,
    halo: f32,
    i: usize,
    tint: [f32; 3],
) {
    let l = Light {
        c,
        r,
        k,
        tm,
        a,
        halo,
    };
    match id {
        "bats" => bats(p, &l),
        "huddle" => huddle(p, &l),
        "wave" => wave(p, &l),
        "catatumbo" | "bubblers" | "barchans" | "pando" | "moonroad" => {
            ground::draw(p, id, &ground::Spot::new(c, r, k, k as f32, a, halo), tm)
        }
        _ => breathing(p, &l, i, tint),
    }
}

/// A window onto the dusk: the bats climbing out of the cave in a turning column, and at its top
/// streaming away either side.
fn bats(p: &mut Painter, l: &Light) {
    l.window(p, [0.36, 0.30, 0.46]);
    p.disc(
        l.at(0.0, 0.72),
        l.len(0.16),
        1.0,
        [0.95, 0.70, 0.45, 0.8 * l.a],
    );
    let kf = l.k as f32;
    for j in 0..60 {
        let u = (j as f64 / 60.0 + l.tm * 0.12).rem_euclid(1.0);
        let s = j as f64 * 2.39;
        let (x, y) = if u < 0.6 {
            let h = u / 0.6;
            ((s + l.tm * 3.0).sin() * (0.06 + 0.16 * h), 0.7 - 1.15 * h)
        } else {
            let v = (u - 0.6) / 0.4;
            let side = if j % 3 == 0 { -1.0 } else { 1.0 };
            (
                side * v * 0.8 + 0.05 * s.sin(),
                -0.45 - 0.25 * v + 0.05 * s.cos(),
            )
        };
        p.disc(l.at(x, y), 0.55 * kf, 0.6, [0.05, 0.04, 0.06, 0.9 * l.a]);
    }
}

/// A window onto the sea ice: the huddle, packed and dark, slowly turning, and the snow blowing past.
fn huddle(p: &mut Painter, l: &Light) {
    l.window(p, [0.70, 0.72, 0.86]);
    for j in 0..19 {
        let (ring, n) = match j {
            0 => (0.0, 1.0),
            1..=6 => (0.17, 6.0),
            _ => (0.33, 12.0),
        };
        let ang = j as f64 / n * TAU + l.tm * 0.03 * ring;
        let x = ring * ang.cos() + 0.02 * (l.tm * 0.7 + j as f64).sin();
        let y = ring * ang.sin() * 0.85;
        p.disc(
            l.at(x, y),
            l.len(0.085),
            1.0,
            [0.06, 0.07, 0.10, 0.95 * l.a],
        );
    }
    let kf = l.k as f32;
    for j in 0..10 {
        let u = (j as f64 * 0.37 + l.tm * 0.25).rem_euclid(1.0);
        let y = (j as f64 * 0.61).rem_euclid(1.0) * 1.4 - 0.7;
        let x = u * 1.8 - 0.9;
        p.line(
            l.at(x, y),
            l.at(x + 0.14, y - 0.03),
            0.5 * kf,
            0.6,
            [1.0, 1.0, 1.0, 0.8 * l.a],
        );
    }
}

/// A window onto the stadium at night: its oval of phone lights, and the wave going round.
fn wave(p: &mut Painter, l: &Light) {
    l.window(p, [0.05, 0.05, 0.07]);
    p.disc(l.c, l.len(0.3), 1.0, [0.06, 0.16, 0.08, 0.9 * l.a]);
    let kf = l.k as f32;
    let front = (l.tm * 0.9).rem_euclid(TAU);
    for j in 0..56 {
        let ang = j as f64 / 56.0 * TAU;
        let d = (ang - front + TAU * 1.5).rem_euclid(TAU) - TAU * 0.5;
        let up = (-d * d / 0.06).exp() as f32;
        let rr = 0.62 + 0.12 * up as f64;
        let lit = 0.35 + 0.65 * up;
        p.disc(
            l.at(rr * 0.85 * ang.cos(), rr * ang.sin()),
            (0.5 + 0.7 * up) * kf,
            0.8,
            [1.0, 0.88, 0.66, lit * l.a],
        );
    }
}

/// A toy without a likeness of its own yet: a light of its colour, breathing.
fn breathing(p: &mut Painter, l: &Light, i: usize, tint: [f32; 3]) {
    let kf = l.k as f32;
    let ph = (l.tm / 3.0 + i as f64 * 0.37).rem_euclid(1.0);
    p.ring(
        l.c,
        l.len(0.3 + 1.2 * ph),
        2.1 * kf,
        1.2,
        [tint[0], tint[1], tint[2], ((1.0 - ph) * 0.85) as f32 * l.a],
    );
    p.disc(l.c, 3.6 * kf, 1.0, [tint[0], tint[1], tint[2], 0.95 * l.a]);
}
