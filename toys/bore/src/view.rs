//! Where we stand and what we see: the seawall at Yanguan, the river beside it, and the camera, shared by
//! the shader (`river.wgsl` takes the same numbers) and by everything drawn over it (the crowd, the spray).
//!
//! Metres: X across the river from the seawall's edge (+ out over the water, − back over the promenade),
//! Y up from mean sea level, Z down the river from where we stand (the bore comes from +Z toward us).

/// The promenade on top of the wall, and the stone revetment that slopes from its edge down into the
/// river (Yanguan's fish-scale seawall), one in one.
pub const WALL_TOP: f64 = 7.0;
/// The river's breadth at Yanguan (m).
pub const BREADTH: f64 = 2800.0;

#[derive(Clone, Copy, Debug)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// down from level, and toward the river from straight down it (radians)
    pub pitch: f64,
    pub yaw: f64,
    /// the tangent of half the screen's height in view
    pub tan_half: f64,
}

impl Camera {
    /// On the tide-watching stand behind the rail, seven metres over the promenade, looking down the
    /// river along the wall.
    pub fn at_the_rail() -> Camera {
        Camera {
            x: -1.0,
            y: WALL_TOP + 7.0,
            z: 0.0,
            pitch: 0.33,
            yaw: 0.33,
            tan_half: 0.9,
        }
    }

    /// From `at`, looking at `target`, with this much of the height in view.
    pub fn looking(at: [f64; 3], target: [f64; 3], tan_half: f64) -> Camera {
        let d = [target[0] - at[0], target[1] - at[1], target[2] - at[2]];
        let flat = (d[0] * d[0] + d[2] * d[2]).sqrt();
        Camera {
            x: at[0],
            y: at[1],
            z: at[2],
            pitch: (-d[1]).atan2(flat),
            yaw: d[0].atan2(d[2]),
            tan_half,
        }
    }

    /// Where the eye is as the bore comes: up on the stand while it is far off (`near` 0), down at the
    /// revetment's steps by the water, watching the place along the wall where it will strike (`near`
    /// 1, struck at `zi` m).
    pub fn following(near: f64, zi: f64) -> Camera {
        let k = near * near * (3.0 - 2.0 * near);
        let lerp = |a: f64, b: f64| a + (b - a) * k;
        let stand = Camera::at_the_rail();
        let z = zi.clamp(18.0, 400.0);
        let rail = Camera::looking(
            [2.6, WALL_TOP - 2.6 + 1.6, 0.0],
            [40.0, 1.6, z * 0.9 + 20.0],
            0.6,
        );
        Camera {
            x: lerp(stand.x, rail.x),
            y: lerp(stand.y, rail.y),
            z: lerp(stand.z, rail.z),
            pitch: lerp(stand.pitch, rail.pitch),
            yaw: lerp(stand.yaw, rail.yaw),
            tan_half: lerp(stand.tan_half, rail.tan_half),
        }
    }

    /// A point in the world on the screen (points, for a screen of `size` points), and how far it is in
    /// front of the eye (m); None behind it.
    pub fn project(&self, p: [f64; 3], size: [f64; 2]) -> Option<([f64; 2], f64)> {
        let r = [p[0] - self.x, p[1] - self.y, p[2] - self.z];
        let (cy, sy) = (self.yaw.cos(), self.yaw.sin());
        let x1 = r[0] * cy - r[2] * sy;
        let z1 = r[0] * sy + r[2] * cy;
        let (cp, sp) = (self.pitch.cos(), self.pitch.sin());
        let yc = r[1] * cp + z1 * sp;
        let zc = -r[1] * sp + z1 * cp;
        if zc < 0.1 {
            return None;
        }
        let half = size[1] * 0.5;
        let k = half / (self.tan_half * zc);
        Some(([size[0] * 0.5 + x1 * k, half - yc * k], zc))
    }

    /// Points a metre is, at distance `zc` in front of the eye.
    pub fn scale(&self, zc: f64, size: [f64; 2]) -> f64 {
        size[1] * 0.5 / (self.tan_half * zc)
    }

    /// The shader's camera: (x, y, z, pitch), (yaw, tan_half, 0, 0).
    pub fn uniforms(&self) -> [f32; 8] {
        [
            self.x as f32,
            self.y as f32,
            self.z as f32,
            self.pitch as f32,
            self.yaw as f32,
            self.tan_half as f32,
            0.0,
            0.0,
        ]
    }
}

/// The ground under the water: the promenade, then the revetment sloping one in one to the river's bed.
pub fn ground(x: f64) -> f64 {
    if x < 0.0 {
        WALL_TOP
    } else {
        (WALL_TOP - x).max(-3.0)
    }
}

/// How far along the river (m) the front lags at a place across it: a little behind out in the shallows
/// of the far side, and gently wavering.
pub fn bow(x: f64, zf: f64) -> f64 {
    let a = (x / BREADTH).clamp(0.0, 1.2);
    90.0 * a * a + 8.0 * (x * 0.011 + zf * 0.004).sin()
}
