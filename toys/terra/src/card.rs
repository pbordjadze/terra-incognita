//! The postcard (painting-company row 351): touched, a light raises a card from the foot of the screen with its
//! toy already going in it, the toy's name, its place and one line under the picture; touched, the card grows
//! until its picture is the glass and the toy is the whole screen. Coming back, the toy shrinks into its card
//! again, and the card stays up until the reader touches the Earth elsewhere or swipes it away. This file is
//! the card's shape at every step; `veil.rs` draws it, the kit's shell sets its words (`Toy::label`).

use crate::veil::Lay;

/// Seconds for a card to rise (or fall away).
pub const RISE: f64 = 0.38;
/// Seconds for a card to grow to the glass (or shrink back).
pub const GROW: f64 = 0.55;

/// The card's shape at rest, in points (left, top, right, bottom): the card, its picture, and the box its
/// words go in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    pub card: [f64; 4],
    pub pic: [f64; 4],
    pub words: [f64; 4],
}

/// The card on a screen of `points`: at most 400 points wide, centred, its foot 64 points above the screen's
/// (clear of the marks and the home indicator); the picture three fifths as tall as it is wide.
pub fn shape(points: [f64; 2]) -> Shape {
    let [w, h] = points;
    let cw = (w - 32.0).min(400.0);
    let pad = 10.0;
    let pic_h = (cw * 0.6).round();
    let words_h = 108.0;
    let ch = pad + pic_h + words_h;
    let (x0, y1) = ((w - cw) / 2.0, h - 64.0);
    let y0 = y1 - ch;
    Shape {
        card: [x0, y0, x0 + cw, y1],
        pic: [x0 + pad, y0 + pad, x0 + cw - pad, y0 + pad + pic_h],
        words: [x0 + 18.0, y0 + pad + pic_h + 12.0, x0 + cw - 18.0, y1 - 8.0],
    }
}

pub fn ease(u: f64) -> f64 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

fn mix(a: [f64; 4], b: [f64; 4], t: f64) -> [f64; 4] {
    [0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// Where in a toy's own frame its card's picture centres (fractions of the frame), where what it is lies in
/// its first frame: Frost's ice grows from the window's foot, the starlings' flock is high in the sky. The
/// middle for every other; a new toy needs nothing here.
pub fn focus(id: &str) -> [f64; 2] {
    match id {
        "frost" => [0.5, 0.78],
        "murmuration" => [0.55, 0.36],
        "fireflies" => [0.5, 0.5],
        _ => [0.5, 0.5],
    }
}

/// The card drawn: `rise` (0 below the screen .. 1 at rest), `g` (0 a card .. 1 the glass), `alpha`; on a screen
/// of `points` at `ppp` pixels a point; `at` the toy's `focus`.
pub fn lay(points: [f64; 2], ppp: f64, at: [f64; 2], rise: f64, g: f64, alpha: f64) -> Lay {
    let [w, h] = points;
    let s = shape(points);
    let dy = (1.0 - ease(rise)) * (h - s.card[1] + 24.0);
    let down = |r: [f64; 4]| [r[0], r[1] + dy, r[2], r[3] + dy];
    let full = [0.0, 0.0, w, h];
    let e = ease(g);
    let card = mix(down(s.card), full, e);
    let pic = mix(down(s.pic), full, e);
    // the frame covers the picture (no edge of it ever shows), its point `at` at the picture's middle, kept
    // inside the frame; at the glass, the frame itself
    let (pw, ph) = (pic[2] - pic[0], pic[3] - pic[1]);
    let scale = (pw / w).max(ph / h).clamp(1e-3, 1.0);
    let (hw, hh) = (pw / 2.0 / scale, ph / 2.0 / scale);
    let fx = (at[0] * w).clamp(hw.min(w / 2.0), (w - hw).max(w / 2.0));
    let fy = (at[1] * h).clamp(hh.min(h / 2.0), (h - hh).max(h / 2.0));
    let px = |r: [f64; 4]| r.map(|v| (v * ppp) as f32);
    Lay {
        card: px(card),
        pic: px(pic),
        radius: (22.0 * (1.0 - e) * ppp) as f32,
        pic_radius: (14.0 * (1.0 - e) * ppp) as f32,
        focus: [(fx * ppp) as f32, (fy * ppp) as f32],
        scale: scale as f32,
        paper: (1.0 - e) as f32,
        alpha: alpha as f32,
    }
}

/// Whether a point (points) is on the card at rest.
pub fn on_card(points: [f64; 2], p: [f64; 2]) -> bool {
    let c = shape(points).card;
    p[0] >= c[0] && p[0] <= c[2] && p[1] >= c[1] && p[1] <= c[3]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_card_sits_above_the_marks_and_grows_to_the_glass() {
        let pts = [390.0, 844.0];
        let s = shape(pts);
        assert!(s.card[3] <= 844.0 - 60.0 && s.card[0] >= 16.0 && s.card[2] <= 390.0 - 16.0);
        assert!(s.pic[3] < s.words[1] && s.words[3] <= s.card[3]);
        let at_rest = lay(pts, 3.0, [0.5, 0.5], 1.0, 0.0, 1.0);
        assert_eq!(at_rest.card[1], (s.card[1] * 3.0) as f32);
        // the picture is covered: the frame shown smaller than the glass
        assert!(at_rest.scale < 1.0);
        let glass = lay(pts, 3.0, [0.5, 0.8], 1.0, 1.0, 1.0);
        assert_eq!(glass.card, [0.0, 0.0, 1170.0, 2532.0]);
        assert_eq!(glass.pic, glass.card);
        assert_eq!(glass.scale, 1.0);
        // the frame's middle at the glass's: the toy's own frame, unscaled and unmoved
        assert_eq!(glass.focus, [585.0, 1266.0]);
        assert_eq!(glass.paper, 0.0);
        // risen from below the screen
        let below = lay(pts, 3.0, [0.5, 0.5], 0.0, 0.0, 1.0);
        assert!(below.card[1] >= 2532.0);
    }
}
