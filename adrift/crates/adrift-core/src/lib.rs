//! Adrift: glass floats on the real sea. The one library; the shells (`adrift-check`, `adrift-web`,
//! `adrift/ios`) are thin. The map of the modules and their owners is `adrift/DECISIONS.md`.

pub mod app;
pub mod assets;
pub mod gpu;
pub mod look;
pub mod model;
pub mod sea;
pub mod touch;

#[cfg(target_os = "ios")]
pub mod ffi;

pub use app::{App, Scene, SceneFloat, Snapshot};
pub use assets::{Assets, DirAssets, MemoryAssets};
pub use gpu::{Gpu, Offscreen};
pub use model::*;
