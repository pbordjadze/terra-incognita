//! The sea as measured, and the rules a float follows on it (Truth's module; DECISIONS.md §2).
//!
//! R0 (Engineering's skeleton): the month's mean current from the drifter climatology's sheet, the
//! coast from the globe's distance field, the month's ice; a float moves with the current alone.
//! Truth adds the eddy wander, the leeway on the wind, the maybes' sampler, the drifters' tracks,
//! and `rules.wgsl`, the GPU mirror of `Rules::step`.

pub mod data;
pub mod rules;

pub use data::{Coast, Currents, Ice};
pub use rules::{Rules, Step};
