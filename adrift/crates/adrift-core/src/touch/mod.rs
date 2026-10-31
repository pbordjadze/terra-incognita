//! Touch: pointer events to intents (`Gestures`), the camera under a finger (`CameraControl`), and
//! the curves and the beat (`motion`). VOCABULARY.md is the state machine, with every ambiguous
//! case resolved; WORKS.md what it is measured against; CRITICS.md the critics' verdicts; `plans/`
//! the played sessions that prove it (`adrift-check -- play`; `tools/adrift/touch-play.sh` plays
//! them all; `tools/adrift/touch-plans.py` writes them).
//!
//! The seam: `Gestures::feed` and `Gestures::tick` take a `Probe` (what is under a point, where a
//! float is, the clock, the frame; `probe.rs` answers for `App`) and return `Out`: intents for the
//! state and `CameraMove`s for the camera, which the app applies (`CameraControl::apply`). The look
//! draws the state and never reads pointers.

pub mod camera;
pub mod gestures;
pub mod motion;
pub mod probe;

pub use camera::{CameraControl, CameraMove};
pub use gestures::{date_band, Frame, Gestures, Hit, Key, Out, Probe, Rect};
pub use motion::Motion;
