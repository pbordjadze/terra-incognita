//! Elkmont engraved on black film, as Norman McLaren made *Blinkity Blank* (National Film Board of
//! Canada, Montreal, 1955): "engraved directly onto black film leader", images that "blink in and out of
//! existence", some frames left blank, what he called "sprinkling on the empty band of time"; he set out
//! "to investigate the possibilities of intermittent animation" (SOURCES.md).
//!
//! Here the woods are drawn as before into a picture of their own; `fs_surface` reads their light into a
//! grid, and `fs_main` cuts the frame (`engraved.wgsl`): the fireflies scratched through as bursts, the
//! trees as boiling scratches on some runs of frames and not others. Twenty-four frames a second.

use toy_kit::film::Film;

/// Cells across the screen's short side.
pub const CELLS: f32 = 260.0;
/// Frames a second: the film's.
pub const FRAMES_HZ: f64 = 24.0;

pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Film {
    Film::new(
        gpu,
        "engraved",
        include_str!("engraved.wgsl"),
        CELLS,
        FRAMES_HZ,
        size,
    )
}

/// The frame's values: how often a run of frames carries the woods' lines (more as the woods darken,
/// so the trees still show between bursts), and Reduce Motion's steady frame: every frame cut, no boil.
pub fn values(dusk: f32, reduce_motion: bool) -> [f32; 4] {
    let lines = 0.45 + 0.25 * dusk.clamp(0.0, 1.0);
    [lines, if reduce_motion { 1.0 } else { 0.0 }, 1.6, 0.0]
}
