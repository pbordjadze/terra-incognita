//! New lights: a toy an update brought is marked on the Earth until it has been opened once (painting-company
//! row 342). Terra keeps, with its camera, the toys it already knew (`known`); a light whose toy is not among
//! them wears a slow ring of moonlit gold, a second ring breathing out of it, until it is touched and its toy
//! opens. A first launch knows every toy it holds (nothing is new to someone who has seen none of them).

use toy_kit::paint::Painter;

use super::Life;

/// The toys new when this mark came in, for a save written before it (which knew no list): the two that
/// landed just before it. Every later toy is new by its absence from the saved list.
const NEW_BEFORE_THE_LIST: [&str; 2] = ["pando", "moonroad"];

/// The mark's gold: warm, as a lit town's sodium glow is, against the night map's cool.
const GOLD: [f32; 3] = [1.0, 0.82, 0.45];

/// The ring's beat (s).
const BEAT: f64 = 3.2;

impl Life {
    /// The toys no longer new (to keep with the camera).
    pub fn known(&self) -> Vec<String> {
        self.doors
            .iter()
            .zip(&self.new)
            .filter(|(_, &n)| !n)
            .map(|(d, _)| d.id.to_string())
            .collect()
    }

    /// Takes back what was known (`None`: a save from before the list); returns where the first new light
    /// is, so the Earth can open turned toward it.
    pub fn know(&mut self, known: Option<&[String]>) -> Option<[f64; 2]> {
        self.new = self
            .doors
            .iter()
            .map(|d| match known {
                Some(k) => !k.iter().any(|id| id == d.id),
                None => NEW_BEFORE_THE_LIST.contains(&d.id),
            })
            .collect();
        let first = self.new.iter().position(|&n| n)?;
        Some(self.doors[first].at)
    }

    /// Light `i`'s toy was opened: it is new no longer.
    pub fn tried(&mut self, i: usize) {
        if let Some(n) = self.new.get_mut(i) {
            *n = false;
        }
    }

    /// The new mark round light `i` (pixels: centre `c`, the light's size `r` points, `k` pixels a point,
    /// `a` its visibility): a steady ring of gold just outside the light, and another breathing out of it.
    /// Reduce Motion: the steady ring alone.
    #[allow(clippy::too_many_arguments)] // a drawing call: each argument is one thing it draws with
    pub(super) fn paint_new(
        &self,
        p: &mut Painter,
        i: usize,
        c: [f32; 2],
        r: f64,
        k: f64,
        t: f64,
        a: f32,
        still: bool,
    ) {
        if !self.new.get(i).copied().unwrap_or(false) || a <= 0.01 {
            return;
        }
        let kf = k as f32;
        let [g0, g1, g2] = GOLD;
        let ph = if still {
            0.0
        } else {
            (t / BEAT).rem_euclid(1.0)
        };
        let breath = if still {
            0.8
        } else {
            0.65 + 0.25 * (std::f64::consts::TAU * ph).cos()
        } as f32;
        let at = (r * 1.45 * k) as f32;
        p.glow_ring(c, at, 2.6 * kf, 2.4 * kf, [g0, g1, g2, 0.35 * breath * a]);
        p.ring(c, at, 1.1 * kf, 1.0, [g0, g1, g2, 0.9 * breath * a]);
        if !still {
            let out = (r * (1.45 + 1.3 * ph) * k) as f32;
            let fade = ((1.0 - ph).powf(1.6) * 0.6) as f32 * a;
            p.ring(c, out, 1.0 * kf, 1.2, [g0, g1, g2, fade]);
        }
    }
}
