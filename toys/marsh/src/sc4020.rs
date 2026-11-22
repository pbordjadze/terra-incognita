//! The SC 4020's film of the plotted drop (`plot.rs`): the tube face read at its 1024 addressable
//! positions across (S-C 4020 manual, 1964), photographed a frame at a time by a camera "without pin
//! registration", so each frame sits a little differently in the gate, the jitter its films show
//! (SOURCES.md). Black-and-white stock, the lines' slight halation, grain (`sc4020.wgsl`).

use toy_kit::film::Film;

/// Addressable positions across the screen's short side: the 4020's raster.
pub const POSITIONS: f32 = 1024.0;
/// Frames a second, as projected.
pub const FRAMES_HZ: f64 = 24.0;

pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Film {
    Film::new(
        gpu,
        "sc4020",
        include_str!("sc4020.wgsl"),
        POSITIONS,
        FRAMES_HZ,
        size,
    )
}

/// The film's values: how far a frame may sit off in the gate (0 with reduced motion).
pub fn values(reduce_motion: bool) -> [f32; 4] {
    [if reduce_motion { 0.0 } else { 1.0 }, 0.0, 0.0, 0.0]
}
