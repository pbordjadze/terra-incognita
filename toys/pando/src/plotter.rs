//! Pando drawn as a line drawing on a microfilm plotter, after Charles Csuri and James Shaffer's
//! *Hummingbird* (Ohio State University, 1967): "the subject was a line drawing of a hummingbird", and
//! "a microfilm plotter recorded the images directly to film", some 30,000 drawings from FORTRAN on the
//! university's IBM 7094 (SOURCES.md). Every shape the scales paint becomes its outline in one pen
//! weight, filled with paper so that what is nearer hides the lines behind it; light (the painter's
//! glows) is not drawn at all. The backdrop draws the same way (`scene.wgsl`): ridges, the leaf's edge
//! and veins, the skin's cell walls, ink on paper.

use toy_kit::paint::{Painter, Shape};

/// The paper and the ink, straight colours.
pub const PAPER: [f32; 3] = [0.96, 0.95, 0.91];
pub const INK: [f32; 3] = [0.08, 0.08, 0.10];
/// The pen's width, points.
const PEN: f32 = 0.55;
/// A shape fainter than this isn't drawn: a pen is down or up.
const FAINT: f32 = 0.25;
/// In the grove, a round shape smaller than this many pen widths across (a crown's leaf cluster) isn't
/// outlined: the pen dots it, and only some of them, so a crown is stippled inside the outline of its
/// body rather than packed with circles.
const DOT: f32 = 60.0;
pub const DOTS_KEPT: f32 = 0.45;

fn hash(p: [f32; 2]) -> f32 {
    let h = (p[0] * 12.9898 + p[1] * 78.233).sin() * 43_758.547;
    h - h.floor()
}

fn premul(c: [f32; 3], a: f32) -> [f32; 4] {
    [c[0] * a, c[1] * a, c[2] * a, a]
}

/// Turns the painter's shapes into the plotter's: each a paper fill under an ink outline, in the same
/// order, so a later (nearer) shape hides what is behind it. A shape larger than the screen (the veil
/// a dive draws over everything) stays a wash of paper, as faint as it was.
pub fn ink(painter: &mut Painter, size: [f32; 2], ppp: f32, grove: bool) {
    let big = size[0].hypot(size[1]) * 0.9;
    let pen = PEN * ppp;
    let shapes = std::mem::take(&mut painter.shapes_over);
    painter.shapes_add.clear();
    let mut cloud: Vec<Shape> = Vec::new();
    for s in shapes {
        let a = s.color[3];
        if s._pad == CLOUD && a >= FAINT {
            cloud.push(s);
            continue;
        }
        outline_cloud(painter, &mut cloud, pen);
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
        let len = ((s.b[0] - s.a[0]).powi(2) + (s.b[1] - s.a[1]).powi(2)).sqrt();
        if grove && s.ring <= 0.0 && s.r * 2.0 < DOT * pen && len < s.r * 0.5 {
            // a small leaf cluster: a dot of the pen (the grove draws only DOTS_KEPT of them, chosen by
            // their seeds, so the stipple holds still as they sway)
            painter.shapes_over.push(Shape {
                r: pen * 0.9,
                soft: 0.75,
                color: premul(INK, 1.0),
                ..s
            });
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
        if s.ring <= 0.0 && len > s.r * 10.0 && s.r > pen {
            eyes(painter, &s, len, pen);
        }
    }
    outline_cloud(painter, &mut cloud, pen);
}

/// A shape marked with this in `_pad` is one lobe of a cloud (a crown's body): the lobes are drawn as
/// one outline, their union's, not each its own.
pub const CLOUD: f32 = 1.0;

/// Draws the lobes gathered so far as one cloud: every lobe's ring just outside its edge, then every
/// lobe's paper over them, so only the union's outer edge stays inked.
fn outline_cloud(painter: &mut Painter, cloud: &mut Vec<Shape>, pen: f32) {
    for s in cloud.iter() {
        painter.shapes_over.push(Shape {
            r: s.r + pen * 0.5,
            ring: pen,
            soft: 0.75,
            _pad: 0.0,
            color: premul(INK, 1.0),
            ..*s
        });
    }
    for s in cloud.drain(..) {
        painter.shapes_over.push(Shape {
            ring: 0.0,
            soft: 0.75,
            _pad: 0.0,
            color: premul(PAPER, 1.0),
            ..s
        });
    }
}

/// An aspen's stem (a long thin stroke) gets its bark's eyes: short dark marks across it where
/// branches fell, a few along its length.
fn eyes(painter: &mut Painter, s: &Shape, len: f32, pen: f32) {
    let d = [(s.b[0] - s.a[0]) / len, (s.b[1] - s.a[1]) / len];
    let n = [-d[1], d[0]];
    let count = (len / (s.r * 9.0)).min(8.0) as usize;
    for i in 0..count {
        // placed by their number along the stem, not by where it is on the screen, so they hold still
        let f = (i as f32 + 0.3 + 0.4 * hash([i as f32, 1.0])) / count as f32;
        let c = [
            s.a[0] + (s.b[0] - s.a[0]) * f,
            s.a[1] + (s.b[1] - s.a[1]) * f,
        ];
        let w = s.r * (0.5 + 0.4 * hash([i as f32, 2.0]));
        // an aspen's eye is a dark chevron where a branch fell: two strokes meeting below the scar
        let apex = [c[0] + d[0] * w * 0.45, c[1] + d[1] * w * 0.45];
        for side in [-1.0f32, 1.0] {
            painter.shapes_over.push(Shape {
                a: apex,
                b: [c[0] + n[0] * w * side, c[1] + n[1] * w * side],
                r: pen * 0.9,
                ring: 0.0,
                soft: 0.75,
                _pad: 0.0,
                color: premul(INK, 1.0),
            });
        }
    }
}
