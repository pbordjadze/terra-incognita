//! The plotter's pen, for scales that draw themselves as line drawings (the grove, the stem): each call
//! is a pen stroke of the one width, or a patch of paper laid down to hide what is behind (the hidden
//! lines). `plotter::ink` passes these through as they are, and outlines everything else.

use crate::plotter::{INK, PAPER};
use toy_kit::paint::{Painter, Shape};

/// `_pad` marks: a shape already in ink, or a patch of paper.
pub const INKED: f32 = 2.0;
pub const PAPERED: f32 = 3.0;
/// The pen's width (points).
pub const WIDTH: f32 = 0.55;

pub struct Pen {
    /// The pen's width (px) and the ink's alpha (a dive's fade).
    pub w: f32,
    pub a: f32,
}

impl Pen {
    pub fn new(ppp: f32, fade: f32) -> Pen {
        Pen {
            w: WIDTH * ppp,
            a: fade,
        }
    }
    /// A straight stroke from `a` to `b`.
    pub fn line(&self, p: &mut Painter, a: [f32; 2], b: [f32; 2]) {
        p.shapes_over.push(Shape {
            a,
            b,
            r: self.w * 0.5,
            ring: 0.0,
            soft: 0.75,
            _pad: INKED,
            color: [INK[0] * self.a, INK[1] * self.a, INK[2] * self.a, self.a],
        });
    }
    /// A stroke through the points, closed back to the first if `closed`.
    pub fn path(&self, p: &mut Painter, pts: &[[f32; 2]], closed: bool) {
        for w in pts.windows(2) {
            self.line(p, w[0], w[1]);
        }
        if closed && pts.len() > 2 {
            self.line(p, pts[pts.len() - 1], pts[0]);
        }
    }
    /// A small circle drawn with the pen (radius px).
    pub fn circle(&self, p: &mut Painter, c: [f32; 2], r: f32) {
        p.shapes_over.push(Shape {
            a: c,
            b: c,
            r,
            ring: self.w,
            soft: 0.75,
            _pad: INKED,
            color: [INK[0] * self.a, INK[1] * self.a, INK[2] * self.a, self.a],
        });
    }
    /// A patch of paper, a capsule from `a` to `b` of radius `r` (px), over what was drawn before.
    pub fn paper(&self, p: &mut Painter, a: [f32; 2], b: [f32; 2], r: f32) {
        p.shapes_over.push(Shape {
            a,
            b,
            r,
            ring: 0.0,
            soft: 0.75,
            _pad: PAPERED,
            color: [
                PAPER[0] * self.a,
                PAPER[1] * self.a,
                PAPER[2] * self.a,
                self.a,
            ],
        });
    }
}
