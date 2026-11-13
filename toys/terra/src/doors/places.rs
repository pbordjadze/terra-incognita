//! Where Terra's lights are: one line for each toy it holds.

use super::Door;
use crate::rooms;

/// The scenes (toys without a globe of their own), which open out of their light as a window.
pub const SCENES: [&str; 13] = [
    "murmuration",
    "fireflies",
    "mold",
    "swell",
    "meander",
    "frost",
    "throats",
    "bats",
    "huddle",
    "wave",
    "catatumbo",
    "bubblers",
    "barchans",
];

/// Every toy Terra holds, one line each. A new toy: its crate in Cargo.toml, its line here (where it opens,
/// its colour, its note), its name in tools/toys/terra/manifest.py, and if it is a scene, in SCENES;
/// a likeness of its own (in `Life::paint` or likeness.rs) is optional (without one it is a
/// breathing light of its colour).
pub fn doors() -> Vec<Door> {
    let mut all = globes();
    all.extend(scenes());
    all
}

/// The toys with a globe of their own.
fn globes() -> Vec<Door> {
    vec![
        Door {
            id: "whales",
            name: "Whales",
            at: [-125.0, 30.0],
            tint: [0.80, 0.60, 0.26],
            night: true,
            degree: 0,
            open: rooms::open::<whales::Whales>,
        },
        Door {
            id: "mold",
            name: "Mold",
            at: [kanto::PLACE.0, kanto::PLACE.1],
            tint: [0.95, 0.76, 0.28],
            night: true,
            degree: 2,
            open: rooms::open::<kanto::Kanto>,
        },
        Door {
            id: "fireflies",
            name: "Fireflies",
            at: [elkmont::PLACE.0, elkmont::PLACE.1],
            tint: [0.62, 0.92, 0.36],
            night: true,
            degree: 6,
            open: rooms::open::<elkmont::Elkmont>,
        },
        Door {
            id: "bell",
            name: "Bell",
            at: [-73.05, -38.24],
            tint: [0.62, 0.42, 0.22],
            night: false,
            degree: -2,
            open: rooms::open::<bell::Bell>,
        },
        Door {
            id: "swell",
            name: "Swell",
            at: [nazare::PLACE.0, nazare::PLACE.1],
            tint: [0.20, 0.45, 0.70],
            night: false,
            degree: 3,
            open: rooms::open::<nazare::Nazare>,
        },
    ]
}

/// The scenes' lights.
fn scenes() -> Vec<Door> {
    vec![
        Door {
            id: "murmuration",
            name: "Murmuration",
            at: [murmuration::PLACE.0, murmuration::PLACE.1],
            tint: [0.93, 0.58, 0.42],
            night: false,
            degree: 4,
            open: rooms::open::<murmuration::Murmuration>,
        },
        Door {
            id: "meander",
            name: "Meander",
            at: [-67.03, -12.97],
            tint: [0.30, 0.52, 0.62],
            night: false,
            degree: 7,
            open: rooms::open::<meander::Meander>,
        },
        Door {
            id: "throats",
            name: "Throats",
            at: [-120.85, 37.06],
            tint: [0.93, 0.62, 0.30],
            night: false,
            degree: 5,
            open: rooms::open::<throats::Throats>,
        },
        Door {
            id: "bats",
            name: "Bats",
            at: [-98.35, 29.69],
            tint: [0.94, 0.56, 0.36],
            night: false,
            degree: 1,
            open: rooms::open::<bats::Bats>,
        },
        Door {
            id: "huddle",
            name: "Huddle",
            at: [-8.25, -70.65],
            tint: [0.72, 0.78, 0.98],
            night: false,
            degree: 8,
            open: rooms::open::<huddle::Huddle>,
        },
        Door {
            id: "wave",
            name: "The Wave",
            at: [-99.15, 19.30],
            tint: [1.0, 0.86, 0.62],
            night: true,
            degree: -1,
            open: rooms::open::<wave::Wave>,
        },
        Door {
            id: "frost",
            name: "Frost",
            at: [142.79, 63.46],
            tint: [0.22, 0.42, 0.78],
            night: false,
            degree: 9,
            open: rooms::open::<frost::Frost>,
        },
        Door {
            id: "catatumbo",
            name: "Catatumbo",
            at: [catatumbo::PLACE.0, catatumbo::PLACE.1],
            tint: [0.70, 0.62, 1.0],
            night: true,
            degree: 1,
            open: rooms::open::<catatumbo::Catatumbo>,
        },
        Door {
            id: "bubblers",
            name: "Bubblers",
            at: [bubblers::PLACE.0, bubblers::PLACE.1],
            tint: [0.90, 0.80, 0.62],
            night: false,
            degree: 8,
            open: rooms::open::<bubblers::Bubblers>,
        },
        Door {
            id: "barchans",
            name: "Barchans",
            at: [barchans::PLACE.0, barchans::PLACE.1],
            tint: [0.93, 0.64, 0.36],
            night: false,
            degree: -1,
            open: rooms::open::<barchans::Barchans>,
        },
    ]
}
