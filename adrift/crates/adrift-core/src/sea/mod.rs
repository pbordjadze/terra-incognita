//! The sea as the rules read it (Truth's module: `adrift/DECISIONS.md` §2). The data sheets in
//! `adrift/data` (`sheets`), where a float goes in a step (`rules`, mirrored on the GPU by
//! `rules.wgsl`), and the real drifters it is measured against (`drifters`). What is measured and
//! what is drawn, with the credit owed, is `adrift/SOURCES.md`.
//!
//! **The honest reading of the date.** Every sheet is a climatology: a typical year, not a given
//! one. `Clock.day` picks a day of that typical year (`sheets::months`), which mixes the two
//! nearest months' means; a float's path is what a typical year's sea would do from that day,
//! never what the sea did on it. The screen should say "a typical <month>", and never a year.

pub mod drifters;
pub mod gpu;
pub mod rules;
pub mod sheets;
#[cfg(test)]
mod tests;

pub use drifters::{Drifters, Nearest, Run};
pub use gpu::{GpuPath, Maybes, Params};
pub use rules::{Drift, Flight, Rules, Step};
pub use sheets::{Coast, Currents, Eddy, EddyTime, Ice, Wind};

/// The GPU mirror of `Rules::step`, for the maybes by the thousand.
pub const RULES_WGSL: &str = include_str!("rules.wgsl");
