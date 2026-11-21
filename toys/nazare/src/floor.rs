//! Nazaré's floor as the swell feels it: EMODnet's depths (floor.bin, about 115 m a sounding, resampled to
//! 50 m), with the canyon's northern edge in its last kilometre drawn in from where it was measured, and the
//! canyon itself marked as the wall a long swell meets.
//!
//! EMODnet's soundings blur the canyon's head: its edge, a drop of about 60° over 20 m from a shelf 15 to
//! 20 m deep, comes out as a slope over several cells. Hope et al. (2026, section 1) give the edge as
//! measured: from the lighthouse it runs 660 m at a bearing of 228° (segment 1), then to 912 m at 251°
//! (segment 2), then west at 266° (segment 3); the shelf slopes down 2.4° toward it; the thalweg is about
//! 150 m deep at the canyon's terminus. That is drawn in here; the rest of the floor is EMODnet's.

use crate::sea::Floor;

/// The northern edge from the lighthouse seaward: (bearing in degrees, length in m) of each segment. The
/// last segment is held to 800 m, where the soundings show the canyon again.
const EDGE: [(f64, f64); 3] = [(228.0, 660.0), (251.0, 252.0), (266.0, 800.0)];
/// How far either side of the edge it is drawn (m): the canyon's side at least this deep, the shelf's no
/// deeper than its slope gives.
const CANYON_SIDE_M: f64 = 250.0;
const SHELF_SIDE_M: f64 = 500.0;
const CANYON_DEPTH: f64 = 100.0;
const SHELF_AT_EDGE: f64 = 20.0;
const SHELF_SLOPE_DEG: f64 = 2.4;
/// The wall: the canyon cut into the shelf (deeper than this, and this many times the shelf around it),
/// within this distance of the lighthouse: its upper part, which the swell on the shelf meets.
/// How far into the canyon from the measured edge it is the wall (m).
const EDGE_WALL_M: f64 = 150.0;
const WALL_DEPTH: f64 = 45.0;
const WALL_INCISED: f64 = 1.6;
const WALL_REACH_M: f64 = 6000.0;

/// The edge as points (cells, x east, y south), from the lighthouse at `light`.
pub fn edge(light: [f64; 2], cell_m: f64) -> Vec<[f64; 2]> {
    let mut pts = vec![light];
    for (brg, len) in EDGE {
        let b = brg.to_radians();
        let p = pts[pts.len() - 1];
        pts.push([p[0] + b.sin() * len / cell_m, p[1] - b.cos() * len / cell_m]);
    }
    pts
}

/// The floor from its heights (m, negative under the sea), with the edge drawn in and the wall marked.
pub fn build(w: usize, h: usize, cell_m: f64, mut ground: Vec<f32>, light: [f64; 2]) -> Floor {
    let pts = edge(light, cell_m);
    let mut edge_wall = vec![false; w * h];
    for y in 0..h {
        for x in 0..w {
            let q = [x as f64 + 0.5, y as f64 + 0.5];
            let (dist, side) = nearest(&pts, q);
            let m = dist * cell_m;
            let i = y * w + x;
            let z = ground[i] as f64;
            // the edge runs seaward (west): the canyon is on its left, the shelf on its right
            // (the sea only: the headland behind the lighthouse stays land)
            if side < 0.0 && m < CANYON_SIDE_M && z < 0.0 {
                // the measured edge itself is the wall's face
                edge_wall[i] = m < EDGE_WALL_M;
                ground[i] = z.min(-CANYON_DEPTH - 0.2 * m) as f32;
            } else if side > 0.0 && m < SHELF_SIDE_M && z < 0.0 {
                let shelf = (SHELF_AT_EDGE - SHELF_SLOPE_DEG.to_radians().tan() * m).max(3.0);
                ground[i] = z.max(-shelf) as f32;
            }
        }
    }
    let mut canyon = wall(w, h, cell_m, &ground, light);
    for (c, e) in canyon.iter_mut().zip(&edge_wall) {
        *c |= *e;
    }
    Floor {
        w,
        h,
        cell_m,
        ground,
        canyon,
    }
}

/// How far a point is from the polyline (cells), and on which side of it (negative: left of its direction).
fn nearest(pts: &[[f64; 2]], q: [f64; 2]) -> (f64, f64) {
    let mut best = (f64::INFINITY, 0.0);
    for s in pts.windows(2) {
        let (a, b) = (s[0], s[1]);
        let d = [b[0] - a[0], b[1] - a[1]];
        let l2 = d[0] * d[0] + d[1] * d[1];
        let t = (((q[0] - a[0]) * d[0] + (q[1] - a[1]) * d[1]) / l2).clamp(0.0, 1.0);
        let p = [a[0] + t * d[0], a[1] + t * d[1]];
        let dist = (q[0] - p[0]).hypot(q[1] - p[1]);
        if dist < best.0 {
            // with y running south, a point left of the direction of travel has a negative cross product
            let cross = d[0] * (q[1] - a[1]) - d[1] * (q[0] - a[0]);
            best = (dist, cross.signum());
        }
    }
    best
}

/// The canyon's upper part: deeper than WALL_DEPTH and WALL_INCISED times the shelf around it (the floor
/// widely blurred, its depths held to 150 m), within WALL_REACH_M of the lighthouse.
fn wall(w: usize, h: usize, cell_m: f64, ground: &[f32], light: [f64; 2]) -> Vec<bool> {
    let depth: Vec<f64> = ground.iter().map(|&g| (-g as f64).max(0.0)).collect();
    let mut around: Vec<f64> = depth.iter().map(|&d| d.min(150.0)).collect();
    for _ in 0..8 {
        box5(&mut around, w, h);
    }
    (0..w * h)
        .map(|i| {
            let (x, y) = ((i % w) as f64 + 0.5, (i / w) as f64 + 0.5);
            let near = (x - light[0]).hypot(y - light[1]) * cell_m < WALL_REACH_M;
            near && depth[i] > WALL_DEPTH && depth[i] > WALL_INCISED * around[i]
        })
        .collect()
}

/// A 5 × 5 box blur, edges clamped.
fn box5(v: &mut [f64], w: usize, h: usize) {
    let mut tmp = vec![0.0; v.len()];
    for y in 0..h {
        for x in 0..w {
            let s: f64 = (-2i64..=2)
                .map(|d| v[y * w + (x as i64 + d).clamp(0, w as i64 - 1) as usize])
                .sum();
            tmp[y * w + x] = s / 5.0;
        }
    }
    for y in 0..h {
        for x in 0..w {
            let s: f64 = (-2i64..=2)
                .map(|d| tmp[(y as i64 + d).clamp(0, h as i64 - 1) as usize * w + x])
                .sum();
            v[y * w + x] = s / 5.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_edge_runs_south_west_then_west_from_the_lighthouse() {
        let pts = edge([100.0, 100.0], 50.0);
        // 660 m at 228°: west and south
        assert!(pts[1][0] < 100.0 && pts[1][1] > 100.0);
        assert!(((pts[1][0] - 100.0).hypot(pts[1][1] - 100.0) * 50.0 - 660.0).abs() < 1.0);
        // the last segment nearly due west
        assert!((pts[3][1] - pts[2][1]).abs() < 0.2 * (pts[3][0] - pts[2][0]).abs());
        // a point south of the edge is on its left (the canyon's side)
        let (_, side) = nearest(&pts, [pts[2][0], pts[2][1] + 3.0]);
        assert!(side < 0.0);
    }
}
