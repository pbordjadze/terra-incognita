//! The flood's reach: run from the opening for a minute with the phone flat, it covers a fair share of the
//! fan and sinks into the sand before the screen's bottom, as the real one never reaches the sea.
use delta::land::{Land, WET};

#[test]
fn the_flood_spreads_and_sinks() {
    let (w, h) = (72, 156);
    let mut l = Land::new(w, h, delta::SEED);
    for _ in 0..60 {
        l.run(1.0, 1440);
    }
    let wet = l.water.iter().filter(|d| **d > WET).count();
    let deepest_row = (0..l.water.len())
        .filter(|i| l.water[*i] > WET)
        .map(|i| i / w)
        .max()
        .unwrap_or(0);
    assert!(
        wet * 100 > w * h * 8,
        "the flood covers {wet} of {} cells",
        w * h
    );
    assert!(
        deepest_row < h - 4,
        "the flood reaches row {deepest_row} of {h}: it should sink first"
    );
}
