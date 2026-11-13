//! What Terra's own info and "?" say on the Earth (each toy's own take their place while it is open).

use toy_kit::info::{Demo, Rules, Stroke};

/// A fingertip touches the light at `at`, and in it goes.
pub fn earth_demo(at: [f64; 2]) -> Demo {
    Demo::new(
        "Each light is a toy. Touch one to go in.",
        vec![Stroke::tap(0.6, at)],
    )
}

pub fn earth_rules() -> Rules {
    Rules::lines(&[
        "Every light is a toy, set where the thing it shows happens for real.",
        "Touch a light to go in. Pinch out, or touch the Earth mark at the bottom left, to come back.",
    ])
}
