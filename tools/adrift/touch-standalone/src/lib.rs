//! See README.md: the touch module compiled on its own, against the real `model.rs` and a stand-in
//! `app` (the state the gestures probe, without the GPU).
#[path = "../../../../adrift/crates/adrift-core/src/model.rs"]
pub mod model;
pub mod app;
pub mod gpu;
#[path = "../../../../adrift/crates/adrift-core/src/touch/mod.rs"]
pub mod touch;
