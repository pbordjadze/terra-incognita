//! Meander in sand on a lightbox, as Caroline Leaf made *Sand, or Peter and the Wolf* at Harvard in 1969:
//! "pouring sand on a light box and manipulating the shapes frame-by-frame", beach sand on the glass,
//! lit from below, pushed with her fingers, black-and-white silhouettes (SOURCES.md).
//!
//! Here the floodplain is drawn as before into a picture of its own; `fs_surface` reads it as the depth
//! of sand (the river cleared to the glass, the lakes swept thin, the forest deep), and `fs_main` lets
//! the lamp through it grain by grain (`sand.wgsl`). A new frame twelve times a second, each
//! photographed twice.

use toy_kit::film::Film;

/// The style, credited at the foot of the sheet (`Toy::style`).
pub const STYLE: toy_kit::info::Style = toy_kit::info::Style {
    name: "Sand, or Peter and the Wolf",
    maker: "Caroline Leaf",
    place: "Harvard, Cambridge, Massachusetts",
    year: 1969,
};

/// Cells across the screen's short side.
pub const CELLS: f32 = 320.0;
/// Frames a second.
pub const FRAMES_HZ: f64 = 12.0;

pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Film {
    Film::new(
        gpu,
        "sand",
        include_str!("sand.wgsl"),
        CELLS,
        FRAMES_HZ,
        size,
    )
}

/// The frame's values: whether the edges boil from frame to frame.
pub fn values(reduce_motion: bool) -> [f32; 4] {
    [if reduce_motion { 0.0 } else { 1.0 }, 0.0, 0.0, 0.0]
}
