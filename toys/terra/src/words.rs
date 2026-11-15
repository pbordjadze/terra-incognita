//! What Terra's own mark says on the Earth (each toy's own take their place while it is open).

use toy_kit::info::{Demo, Rules, Stroke};

/// A fingertip touches the light at `at`, and its card comes up.
pub fn earth_demo(at: [f64; 2]) -> Demo {
    Demo::new(
        "Each light is a toy. Touch one to see it.",
        vec![Stroke::tap(0.6, at)],
    )
}

pub fn earth_rules() -> Rules {
    Rules::lines(&[
        "Every light is a toy, set where the thing it shows happens for real.",
        "Touch a light and its card comes up with the toy playing in it. Touch the card to go in; swipe it down to put it away.",
        "A light ringed with small dots is several toys close together: touch it to come nearer.",
        "Inside, the Earth mark at the bottom left brings the toy back into its card.",
    ])
}
