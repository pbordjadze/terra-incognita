//! Pando drawn as a line drawing on a microfilm plotter, after Charles Csuri and James Shaffer's
//! *Hummingbird* (Ohio State University, 1967): "the subject was a line drawing of a hummingbird", and
//! "a microfilm plotter recorded the images directly to film", some 30,000 drawings from FORTRAN on the
//! university's IBM 7094 (SOURCES.md). The grove and the stem draw themselves with the pen (`pen.rs`);
//! every other shape the scales paint becomes its outline in one pen weight, filled with paper so that
//! what is nearer hides the lines behind it; light (the painter's glows) is not drawn at all. The
//! backdrop draws the same way (`scene.wgsl`): ridges, the leaf's edge and veins, the skin's cell
//! walls, ink on paper.
use toy_kit::paint::{Painter, Shape};

/// The paper and the ink, straight colours.
pub const PAPER: [f32; 3] = [0.96, 0.95, 0.91];
pub const INK: [f32; 3] = [0.08, 0.08, 0.10];
/// The pen's width, points.
const PEN: f32 = crate::pen::WIDTH;
/// A shape fainter than this isn't drawn: a pen is down or up.
const FAINT: f32 = 0.25;

fn premul(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0] * a, c[1] * a, c[2] * a, a]
}

/// Turns the painter's shapes into the plotter's: each a paper fill under an ink outline, in the same
/// order, so a later (nearer) shape hides what is behind it. A shape larger than the screen (the veil
/// a dive draws over everything) stays a wash of paper, as faint as it was.
pub fn ink(painter: &mut Painter, size: [f32; 2], ppp: f32) {
    let big = size[0].hypot(size[1]) * 0.9;
    let pen = PEN * ppp;
    let shapes = std::mem::take(&mut painter.shapes_over);
    painter.shapes_add.clear();
    for s in shapes {
        let a = s.color[3];
        if s._pad == crate::pen::INKED || s._pad == crate::pen::PAPERED {
            // drawn by the scale itself with the pen
            if a >= FAINT || s._pad == crate::pen::PAPERED {
                painter.shapes_over.push(s);
            }
            continue;
        }
        if s.r >= big {
            painter.shapes_over.push(Shape {
                color: premul(PAPER, a),
                ..s
            });
            continue;
        }
        if a < FAINT {
            continue;
        }
        if s.ring <= 0.0 {
            painter.shapes_over.push(Shape {
                soft: 0.75,
                color: premul(PAPER, 1.0),
                ..s
            });
        }
        // the outline: a ring of the pen's width on the shape's edge (or on a ring's own line)
        let (r, ring) = if s.ring > 0.0 {
            (s.r, s.ring.max(pen))
        } else {
            (s.r, pen)
        };
        painter.shapes_over.push(Shape {
            r,
            ring,
            soft: 0.75,
            color: premul(INK, 1.0),
            ..s
        });
    }
}
