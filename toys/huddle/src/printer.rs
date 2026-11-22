//! Huddle as a printout: the huddle typed in characters by a line printer, a sheet a drawing, as *Kitty*
//! (Koshechka) was made at Moscow State University in 1968: a model of a cat's walk solved on the BESM-4,
//! each frame "printing onto paper using the alphanumeric printing device ACPU-128" as "numerical
//! patterns", and the sheets photographed one at a time (SOURCES.md).
//!
//! Here the huddle is drawn as before into a picture of its own; `fs_surface` chooses the character
//! struck at each place by how dark the picture is beneath it (the digits and a few marks, sorted by
//! their ink; the darkest struck twice), and `fs_main` prints the sheet (`printer.wgsl`). A new sheet
//! twelve times a second, each photographed twice.

use toy_kit::film::Film;

/// Character places across the screen's short side.
pub const COLUMNS: f32 = 92.0;
/// Each bird as the program draws it for the printer, a share of its size: about one character place,
/// so each bird prints as a character of its own with paper round it, countable, as a printer plot
/// puts one character at each point.
pub const BODY: f32 = 0.36;
/// Sheets a second.
pub const SHEETS_HZ: f64 = 12.0;

pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Film {
    let wgsl = concat!(
        include_str!("printer.wgsl"),
        "\n",
        include_str!("printer_type.wgsl")
    );
    Film::new(gpu, "printer", wgsl, COLUMNS, SHEETS_HZ, size)
}

/// The sheet's values: whether each sheet lies a little differently under the camera.
pub fn values(reduce_motion: bool) -> [f32; 4] {
    [if reduce_motion { 0.0 } else { 1.0 }, 0.0, 0.0, 0.0]
}
