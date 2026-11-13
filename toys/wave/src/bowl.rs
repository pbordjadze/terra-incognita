//! The stadium's shape and the camera. Metres: x across the pitch, y up, z along it (away from the
//! camera, up the screen). The stands are one rake round a rounded rectangle: row `v` stands
//! `v * DEPTH` metres back from the pitch's surround and `v * RISE` up; seat `u` is a share of the way
//! round, clockwise from above starting on the east side (so the wave, running to higher seat numbers,
//! goes clockwise).

use crate::crowd::{U, V};

/// The bowl's inner edge: half its width and length, and its corners' radius.
pub const HALF: [f32; 2] = [44.0, 62.0];
pub const CORNER: f32 = 28.0;
/// Each row back and up, metres.
pub const DEPTH: f32 = 0.85;
pub const RISE: f32 = 0.52;

/// The camera: over the south end of the pitch, high up, looking north and down.
pub struct Camera {
    pub eye: [f32; 3],
    /// how far it looks down from level, radians
    pub tilt: f32,
    /// pixels a unit at unit depth, and the screen in pixels
    pub f: f32,
    pub size: [f32; 2],
}

impl Camera {
    pub fn new(size: [f32; 2]) -> Camera {
        // the whole bowl in the frame, its long axis up the screen
        // nearly straight down, a little from the south, the bowl's width filling the screen's
        let f = size[0] * 0.5 / 112.0 * 330.0;
        Camera {
            eye: [0.0, 330.0, -58.0],
            tilt: 80f32.to_radians(),
            f,
            size,
        }
    }

    pub fn basis(&self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let (s, c) = self.tilt.sin_cos();
        ([1.0, 0.0, 0.0], [0.0, c, s], [0.0, -s, c])
    }

    /// Where a point is on screen (pixels), and its depth.
    pub fn project(&self, p: [f32; 3]) -> Option<([f32; 2], f32)> {
        let (r, up, fw) = self.basis();
        let d = [p[0] - self.eye[0], p[1] - self.eye[1], p[2] - self.eye[2]];
        let dot = |a: [f32; 3]| a[0] * d[0] + a[1] * d[1] + a[2] * d[2];
        let z = dot(fw);
        if z < 1.0 {
            return None;
        }
        Some((
            [
                self.size[0] * 0.5 + self.f * dot(r) / z,
                self.size[1] * 0.5 - self.f * dot(up) / z,
            ],
            z,
        ))
    }
}

/// The point `along` metres round the rounded rectangle pushed out by `out` metres, and the outward
/// direction there. `along` starts at the middle of the east side and goes clockwise from above.
fn round(along: f32, out: f32) -> ([f32; 2], [f32; 2]) {
    let (a, b, r) = (HALF[0] - CORNER, HALF[1] - CORNER, CORNER + out);
    let arc = std::f32::consts::FRAC_PI_2 * r;
    // the pieces clockwise from above (x east, z north): east side going south, the south-east
    // corner, the south side going west, and so on
    let legs = [b, arc, 2.0 * a, arc, 2.0 * b, arc, 2.0 * a, arc, b];
    let total: f32 = legs.iter().sum();
    let mut s = along.rem_euclid(total);
    let corners = [[a, -b], [-a, -b], [-a, b], [a, b]];
    let starts = [
        0.0f32,
        -std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        std::f32::consts::FRAC_PI_2,
    ];
    for (k, &len) in legs.iter().enumerate() {
        if s <= len || k == legs.len() - 1 {
            return match k {
                0 => ([HALF[0] + out, -s], [1.0, 0.0]),
                2 => ([a - s, -HALF[1] - out], [0.0, -1.0]),
                4 => ([-HALF[0] - out, -b + s], [-1.0, 0.0]),
                6 => ([-a + s, HALF[1] + out], [0.0, 1.0]),
                8 => ([HALF[0] + out, b - s], [1.0, 0.0]),
                _ => {
                    let c = corners[k / 2];
                    let t = starts[k / 2] - s / r;
                    let n = [t.cos(), t.sin()];
                    ([c[0] + n[0] * r, c[1] + n[1] * r], n)
                }
            };
        }
        s -= len;
    }
    unreachable!()
}

/// The rounded rectangle's length at `out` metres out.
fn perimeter(out: f32) -> f32 {
    4.0 * (HALF[0] - CORNER) + 4.0 * (HALF[1] - CORNER) + std::f32::consts::TAU * (CORNER + out)
}

/// Seat `u` (0..U, fractional) of row `v` (0..V, fractional): where it is.
pub fn seat(u: f32, v: f32) -> [f32; 3] {
    let out = 4.0 + v * DEPTH;
    let (p, _) = round(u / U as f32 * perimeter(out), out);
    [p[0], v * RISE + 1.0, p[1]]
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub pos: [f32; 3],
    /// seat and row (the stands), or -1 and the pitch's own place (the pitch)
    pub uv: [f32; 2],
}

/// The stands as a grid of triangles (fine round the corners), then the pitch and its surround.
pub fn mesh() -> (Vec<Vertex>, Vec<u32>) {
    let (su, sv) = (600usize, 4usize);
    let mut verts = Vec::new();
    let mut idx = Vec::new();
    for j in 0..=sv {
        for i in 0..=su {
            let (u, v) = (
                i as f32 / su as f32 * U as f32,
                j as f32 / sv as f32 * V as f32,
            );
            verts.push(Vertex {
                pos: seat(u, v),
                uv: [u, v],
            });
        }
    }
    for j in 0..sv {
        for i in 0..su {
            let a = (j * (su + 1) + i) as u32;
            let b = a + su as u32 + 1;
            idx.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }
    // the floor inside the stands: a fan from the middle to the stands' foot
    let centre = verts.len() as u32;
    verts.push(Vertex {
        pos: [0.0, 0.0, 0.0],
        uv: [-1.0, 0.0],
    });
    let first = verts.len() as u32;
    for i in 0..=su {
        let mut p = seat(i as f32 / su as f32 * U as f32, 0.0);
        p[1] = 0.0;
        verts.push(Vertex {
            pos: p,
            uv: [-1.0, 0.0],
        });
    }
    for i in 0..su as u32 {
        idx.extend_from_slice(&[centre, first + i, first + i + 1]);
    }
    (verts, idx)
}
