//! The Wave drawn as a BEFLIX film: Ken Knowlton's language for computer-animated movies (Bell
//! Labs, 1963; "Bell flicks"). Its pictures were a surface of cells, 252 by 184 at the fine
//! setting, each holding a value 0 to 7; the IBM 7090 wrote the surfaces to tape, and a
//! Stromberg-Carlson SC-4020 drew each cell on its charactron tube as a character chosen for how
//! much light it held, the beam defocused, while a camera shot the tube frame by frame. Drawings
//! were held by shooting them more than once (computer time was dear), and the colour of the
//! VanDerBeek-Knowlton Poem Fields was dyed in afterwards by an optical printer. SOURCES.md has the
//! sources.
//!
//! Here the stadium is drawn as before into a picture of its own; `fs_surface` reads it into the
//! surface (184 cells across a phone's short side, as across the film's), and `fs_main` draws the
//! surface as the plotter did. A new drawing twelve times a second, each shown twice.

use toy_kit::film::Film;

/// Cells across the screen's short side: the fine surface's short side (252 by 184).
pub const CELLS: f32 = 184.0;
/// Drawings a second (each drawn twice, at 24 frames a second).
pub const DRAWINGS_HZ: f64 = 12.0;
/// Seconds a shot lasts before the next is dyed another way.
const SHOT_S: f64 = 9.0;

pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Film {
    Film::new(
        gpu,
        "beflix",
        include_str!("beflix.wgsl"),
        CELLS,
        DRAWINGS_HZ,
        size,
    )
}

/// The film's values for drawing `d`: which shot (how it is dyed), and whether the gate weaves.
pub fn values(d: i64, reduce_motion: bool) -> [f32; 4] {
    let shot = ((d as f64 / DRAWINGS_HZ / SHOT_S).floor() as i64).rem_euclid(3);
    [shot as f32, if reduce_motion { 0.0 } else { 1.0 }, 0.0, 0.0]
}
