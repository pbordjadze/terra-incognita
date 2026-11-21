//! Bats drawn on a pinscreen: Alexandre Alexeieff and Claire Parker's board pierced by many thousands
//! of pins (Paris, 1933, for *Une nuit sur le mont chauve*). Lit from one side at a raking angle, a pin
//! pushed out throws a long shadow and one pushed flush throws none, so every grey on the board is the
//! shadow of pins; a frame was made by pushing pins in and out by hand, photographed, and pushed again.
//! SOURCES.md has the sources.
//!
//! Here the dusk is drawn as before into a picture of its own; `fs_surface` pushes each pin out as far
//! as the picture is dark beneath it, and `fs_main` lights the board (`pinscreen.wgsl`). A new frame
//! of pins twelve times a second, each photographed twice.

use toy_kit::film::Film;

/// Pins across the screen's short side.
pub const PINS: f32 = 360.0;
/// Frames of pins a second.
pub const FRAMES_HZ: f64 = 12.0;

pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Film {
    Film::new(
        gpu,
        "pinscreen",
        include_str!("pinscreen.wgsl"),
        PINS,
        FRAMES_HZ,
        size,
    )
}

/// The board's values: how far the darkening sky is lifted (so the bats stay darker than the sky as
/// dusk deepens), and whether the lamp flickers.
pub fn values(dusk: f32, reduce_motion: bool) -> [f32; 4] {
    let exposure = 2.6 + 4.0 * dusk;
    [exposure, if reduce_motion { 0.0 } else { 1.0 }, 0.0, 0.0]
}
