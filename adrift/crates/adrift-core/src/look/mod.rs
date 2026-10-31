//! How Adrift looks: every drawing shader and the Rust that owns its pipeline (Visual's module;
//! DECISIONS.md §2). Engineering's R0 ports Terra's ground and sea plainly and draws the floats as
//! plain forms. Draw order (§3): ground, sea, wakes, maybes, drifter, floats, lamp, chrome.

pub mod ground;
pub mod maybes;
pub mod sea;
pub mod sprites;

use crate::gpu::view::VIEW_WGSL;

pub const COMMON_WGSL: &str = include_str!("common.wgsl");

/// A pass's WGSL: the View struct, the shared helpers, then its own.
pub fn wgsl(own: &str) -> String {
    format!("{VIEW_WGSL}\n{COMMON_WGSL}\n{own}")
}

pub use ground::GroundPass;
pub use maybes::MaybesPass;
pub use sea::SeaPass;
pub use sprites::{Instance, SpritePass};
