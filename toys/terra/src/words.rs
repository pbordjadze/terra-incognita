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
        "Every light is a toy, set where it happens for real. Touch one for its card; touch the card to go in.",
        "A light ringed with dots is several toys: touch it to come nearer.",
    ])
}
