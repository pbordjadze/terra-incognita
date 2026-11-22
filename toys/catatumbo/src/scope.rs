//! Catatumbo drawn as Mary Ellen Bute drew on an oscilloscope: the storm as calligraphic traces of light
//! on a cathode-ray tube, filmed, then "optically printed with other images and color filters"
//! (*Abstronic*, New York, 1952, with Ted Nemeth; her scope a Precision Series ES-500A; SOURCES.md).
//!
//! Here the storm is drawn as before into a picture of its own; `fs_surface` reads its light into a
//! grid, and `fs_main` traces the levels of that light as the beam would draw them, each level through
//! its own filter (`scope.wgsl`). A new frame twenty-four times a second, as the camera filmed it.

use toy_kit::film::Film;

/// Cells across the screen's short side (the levels are traced between them, smoothly).
pub const CELLS: f32 = 300.0;
/// Frames a second: the film's.
pub const FRAMES_HZ: f64 = 24.0;

pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Film {
    Film::new(
        gpu,
        "scope",
        include_str!("scope.wgsl"),
        CELLS,
        FRAMES_HZ,
        size,
    )
}

/// How far the storm's light is turned up before its levels are traced.
pub fn values() -> [f32; 4] {
    [3.4, 0.0, 0.0, 0.0]
}
