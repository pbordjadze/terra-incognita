//! The Delta through the Sandin Image Processor (Dan Sandin, University of Illinois at Chicago Circle,
//! built 1971–73): an analog computer for video, after the Moog synthesizer's modules. A black-and-white
//! camera's signal goes in; the Amplitude Classifier cuts it into "8 contiguous grey levels evenly spaced
//! from black to white"; wired through the adders to the colour encoder, that is "a 'threshold based
//! colorizer'", each level keyed to a colour of the player's choosing; the Differentiator lights "sharp
//! horizontal edges" (SOURCES.md). Here the land is drawn as before and that camera is pointed at it
//! (`ip.wgsl`): the video's lines, each level's colour, the edges, the tape's noise.

use toy_kit::film::Film;

/// Picture elements across the screen's short side: an analog video line's few hundred.
pub const ELEMENTS: f32 = 260.0;
/// Frames a second: video's thirty.
pub const FRAMES_HZ: f64 = 30.0;

pub fn new(gpu: &toy_kit::Gpu, size: [u32; 2]) -> Film {
    Film::new(
        gpu,
        "ip",
        include_str!("ip.wgsl"),
        ELEMENTS,
        FRAMES_HZ,
        size,
    )
}

/// The processor's values: the time (s, for the keys' slow drift and the lines' wobble) and whether the
/// picture may move on its own (0 with reduced motion).
pub fn values(t: f64, reduce_motion: bool) -> [f32; 4] {
    [
        (t % 3600.0) as f32,
        if reduce_motion { 0.0 } else { 1.0 },
        0.0,
        0.0,
    ]
}
