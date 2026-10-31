//! How Adrift feels under a finger (Motion-and-Interaction's module; DECISIONS.md §2): pointer events
//! to intents, the camera, the motion's curves. Engineering's R0 is the plainest vocabulary that
//! lets the skeleton be used: tap rings, a rest puts, a drag orbits, two fingers zoom.

pub mod camera;
pub mod gestures;
pub mod motion;

pub use camera::CameraControl;
pub use gestures::Gestures;
