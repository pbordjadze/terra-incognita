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

pub use drifters::{Drifters, Nearest, Run, Track};
pub use gpu::{GpuPath, Maybes, Params};
pub use rules::{Drift, Flight, Rules, Step};
pub use sheets::{Coast, Currents, Eddy, EddyTime, Ice, Wind};

/// What the foot says of where the sea comes from: one line, and the full credits behind it
/// (`adrift/SOURCES.md` › Credit owed; the GDP's licence asks for its sentence whole).
pub const CREDIT_LINE: &str = "A typical year's sea: NOAA Global Drifter Program · NCEP/NCAR Reanalysis · NSIDC Sea Ice Index · Natural Earth";
pub const CREDITS: &[&str] = &[
    "The currents, their eddies and the real drifters: this study used data collected and made freely available by the NOAA Global Drifter Program (https://www.aoml.noaa.gov/phod/gdp/) accessed from https://erddap.aoml.noaa.gov/gdp/erddap/index.html (CC BY 4.0). Climatology: Laurindo, Mariano and Lumpkin (2017). Trajectories: Lumpkin and Centurioni, NOAA NCEI, doi:10.25921/7ntx-z961.",
    "The wind: NCEP/NCAR Reanalysis 1 data provided by the NOAA PSL, Boulder, Colorado, USA, from their website at https://psl.noaa.gov (Kalnay et al. 1996).",
    "The sea ice: Fetterer, F., K. Knowles, W. N. Meier, M. Savoie and A. K. Windnagel, Sea Ice Index, Version 4, NSIDC.",
    "The coast: made with Natural Earth.",
    "The sea is a typical year, not this one: a float's path is what a typical year's sea would do from that day, never what the sea did on it.",
];

/// The GPU mirror of `Rules::step`, for the maybes by the thousand.
pub const RULES_WGSL: &str = include_str!("rules.wgsl");
