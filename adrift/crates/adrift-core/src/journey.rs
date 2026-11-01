//! A journey clip's pace (R14, Engineering's seam; the measure of what is worth seeing is Truth's
//! this round). A journey clip is a take with no hand in it (`App::journey_take`): its picked lamps
//! from the day the last was put, for a length the reader picks. Its date need not run evenly: where
//! the lamps do something worth seeing (come near each other, turn, loop) it slows, and where they
//! only travel it hurries, so the length is filled with what makes the edit. `pace` gives the day
//! each frame shows; the take carries it as a pace before each frame, so it replays exactly.

use crate::model::Float;
use crate::sea::Rules;

/// The day each of `frames` frames (at `fps`) shows, from `from` (the frame before the first) to about
/// `to`, never going back. The first cut: even.
pub fn pace(_rules: &Rules, _floats: &[Float], from: f64, to: f64, frames: usize, _fps: f64) -> Vec<f64> {
    let n = frames.max(1) as f64;
    (1..=frames).map(|i| from + (to - from) * i as f64 / n).collect()
}
