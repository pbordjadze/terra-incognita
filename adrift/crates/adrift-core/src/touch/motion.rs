//! Curves, springs and the beat's lengths: every number that says how Adrift moves under a finger,
//! in one place, each with its reason (VOCABULARY.md › The numbers). The look reads `BEAT` and
//! `thicken` so the ring thickens in time with the finger's rest.

/// The one beat to learn: a finger that rests this long means what it rests on. Longer than a slow
/// tap (about 0.25 s), shorter than the system's long press (0.5 s).
pub const BEAT: f64 = 0.40;
/// How far a resting fingertip may stray (reading points) and still be resting.
pub const SLOP: f64 = 8.0;
/// A fingertip's reach for a thing drawn smaller than one (reading points).
pub const REACH: f64 = 22.0;
/// A second tap within this time and distance of the first is a double tap.
pub const DOUBLE_TAP_S: f64 = 0.30;
pub const DOUBLE_TAP_PT: f64 = 40.0;
/// A fling's velocity decays by `exp(-FLING_DECAY * dt)` and stops under `FLING_REST` points a second.
pub const FLING_DECAY: f64 = 3.2;
pub const FLING_REST: f64 = 2.0;
/// The velocity window: the last samples within this many seconds of the lift make the fling.
pub const FLING_WINDOW: f64 = 0.08;
/// A finger that stopped this long before lifting has no fling.
pub const FLING_STOPPED: f64 = 0.15;
/// A flight (double tap) takes this long, eased in and out.
pub const FLIGHT_S: f64 = 0.70;
/// The ride's catch-up: a critically damped spring with this time constant (from rest it covers
/// 90% of the way in about 4τ, a second, and never overshoots).
pub const RIDE_TAU: f64 = 0.25;
/// Left alone this long, with nothing on the sea, the globe turns at this rate (degrees a second).
pub const IDLE_AFTER: f64 = 8.0;
pub const IDLE_RATE: f64 = 4.0;
/// How far in and out the camera goes.
pub const ZOOM_MIN: f64 = 0.8;
pub const ZOOM_MAX: f64 = 8.0;
/// The camera's centre never passes this latitude: the pole never flips.
pub const LAT_LIMIT: f64 = 80.0;
/// A three-finger swipe this far is undo (left) or redo (right).
pub const SWIPE: f64 = 40.0;
/// Scrubbing: a year across the screen's width, never less than a day a point.
pub fn days_per_point(screen_width: f64) -> f64 {
    (365.0 / screen_width.max(1.0)).max(1.0)
}

/// Reduce Motion, read once from the system by the shell and given to `Gestures` and `CameraRig`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Motion {
    pub reduce: bool,
}

/// 0..1 clamped.
pub fn clamp01(t: f64) -> f64 {
    t.clamp(0.0, 1.0)
}

/// Terra's ease: cubic in, cubic out.
pub fn ease_in_out(t: f64) -> f64 {
    let t = clamp01(t);
    if t < 0.5 {
        4.0 * t * t * t
    } else {
        1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
    }
}

/// Hermite smoothstep between two edges.
pub fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = clamp01((x - a) / (b - a));
    t * t * (3.0 - 2.0 * t)
}

/// How thick the ring is during a rest, 0 a plain ring, 1 a float, for `since` seconds of rest.
/// Eased in: the first 150 ms look like a tap's ring, so a lift short of the beat shows no
/// half-float; it reaches 1 exactly at `BEAT`.
pub fn thicken(since: f64) -> f64 {
    let u = clamp01(since / BEAT);
    // a cubic ease in, then a soft arrival
    let v = u * u * u;
    smoothstep(0.0, 1.0, v)
}

/// A critically damped spring's step toward `target`: returns the new (position, velocity) after
/// `dt` with time constant `tau` (it settles in about 4·tau and never overshoots).
pub fn spring(x: f64, v: f64, target: f64, tau: f64, dt: f64) -> (f64, f64) {
    if tau <= 0.0 || dt <= 0.0 {
        return (target, 0.0);
    }
    let w = 1.0 / tau;
    let e = (-w * dt).exp();
    let dx = x - target;
    let c2 = v + w * dx;
    let nx = (dx + c2 * dt) * e;
    let nv = (c2 - w * (dx + c2 * dt)) * e;
    (target + nx, nv)
}

/// Decays a velocity by the fling's law over `dt`.
pub fn fling_decay(v: f64, dt: f64) -> f64 {
    v * (-FLING_DECAY * dt).exp()
}

/// Wraps a longitude into (-180, 180].
pub fn wrap_lon(lon: f64) -> f64 {
    let l = (lon + 540.0).rem_euclid(360.0) - 180.0;
    if l <= -180.0 { l + 360.0 } else { l }
}

/// The shortest signed way from one longitude to another.
pub fn lon_delta(from: f64, to: f64) -> f64 {
    let mut d = to - from;
    if d > 180.0 {
        d -= 360.0
    } else if d < -180.0 {
        d += 360.0
    }
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thicken_is_quiet_early_and_full_at_the_beat() {
        assert!(thicken(0.0) == 0.0);
        assert!(thicken(0.15) < 0.15, "{}", thicken(0.15));
        assert!((thicken(BEAT) - 1.0).abs() < 1e-9);
        assert!(thicken(0.35) < 1.0);
        let mut last = 0.0;
        for i in 0..=40 {
            let t = thicken(BEAT * i as f64 / 40.0);
            assert!(t >= last);
            last = t;
        }
    }

    #[test]
    fn spring_settles_without_overshoot() {
        let (mut x, mut v) = (10.0, 0.0);
        let mut min = x;
        for _ in 0..600 {
            let (nx, nv) = spring(x, v, 0.0, RIDE_TAU, 1.0 / 120.0);
            x = nx;
            v = nv;
            min = min.min(x);
        }
        assert!(x.abs() < 1e-3, "{}", x);
        assert!(min >= -1e-9, "overshot to {}", min);
        // about four time constants in
        let (mut x, mut v) = (10.0, 0.0);
        let mut t = 0.0;
        while x > 0.1 {
            let (nx, nv) = spring(x, v, 0.0, RIDE_TAU, 1.0 / 120.0);
            x = nx;
            v = nv;
            t += 1.0 / 120.0;
        }
        // from rest, (1 + u)·e^-u = 0.01 at u ≈ 6.6 time constants
        assert!(t > 6.0 * RIDE_TAU && t < 7.5 * RIDE_TAU, "{}", t);
    }

    #[test]
    fn ease_and_wrap() {
        assert_eq!(ease_in_out(0.0), 0.0);
        assert_eq!(ease_in_out(1.0), 1.0);
        assert!((ease_in_out(0.5) - 0.5).abs() < 1e-12);
        assert_eq!(wrap_lon(190.0), -170.0);
        assert_eq!(wrap_lon(-190.0), 170.0);
        assert_eq!(wrap_lon(180.0), 180.0);
        assert_eq!(lon_delta(170.0, -170.0), 20.0);
        assert_eq!(days_per_point(1024.0), 1.0);
        assert!((days_per_point(100.0) - 3.65).abs() < 1e-12);
    }
}
