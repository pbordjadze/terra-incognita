//! Curves and the beat's lengths. (R0: the few the skeleton needs.)

/// Seconds a finger rests before a tap becomes a put.
pub const BEAT: f64 = 0.45;
/// Points a finger may wander and still be resting.
pub const REST_SLOP: f64 = 8.0;
/// Seconds a tap may last.
pub const TAP: f64 = 0.3;

pub fn ease_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

pub fn smoothstep(a: f64, b: f64, x: f64) -> f64 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
