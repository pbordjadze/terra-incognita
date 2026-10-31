//! Touch: pointer events to intents (`Gestures`), the camera under a finger (`CameraRig`), and
//! the curves and the beat (`motion`). VOCABULARY.md is the state machine, with every ambiguous
//! case resolved; WORKS.md what it is measured against; `plans/` the played sessions that prove it.
//!
//! The seam: `Gestures::event` and `Gestures::tick` take a `Probe` (what is under a point, where a
//! float is, whether the clock plays) and push `model::Intent`s; `App` applies them, the look draws
//! the state and never reads pointers. `CameraRig` owns the camera's truth and reports it as
//! `Intent::Camera` whenever it moves.

pub mod camera;
pub mod gestures;
pub mod motion;

pub use camera::{CameraRig, Frame};
pub use gestures::{Gestures, Hit, Key, Probe};
pub use motion::Motion;
