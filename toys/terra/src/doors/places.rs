//! Where Terra's lights are: one line for each toy it holds.

use super::Door;
use crate::rooms;

/// Every toy Terra holds, one entry each. A new toy: its crate in Cargo.toml, its entry here (its name, where
/// it is and one line for its postcard, where it opens, its colour, its note), and its name in
/// tools/toys/terra/manifest.py. That is all: every light is the same plain light of its colour.
pub fn doors() -> Vec<Door> {
    let mut all = globes();
    all.extend(scenes());
    all.extend(ground_scenes());
    all.extend(homages());
    all
}

/// The toys with a globe of their own.
fn globes() -> Vec<Door> {
    vec![
        Door {
            id: "fireflies",
            name: "Elkmont",
            place: "Great Smoky Mountains, Tennessee",
            line: "Fireflies that flash together, all at once.",
            at: [elkmont::PLACE.0, elkmont::PLACE.1],
            tint: [0.62, 0.92, 0.36],
            degree: 6,
            open: rooms::open::<elkmont::Elkmont>,
        },
        Door {
            id: "swell",
            name: "Nazaré",
            place: "Praia do Norte, Portugal",
            line: "The Atlantic swell, bent and stacked by a canyon.",
            at: [nazare::PLACE.0, nazare::PLACE.1],
            tint: [0.20, 0.45, 0.70],
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
            place: "Somerset Levels, England",
            line: "Starlings at dusk, thousands turning as one.",
            at: [murmuration::PLACE.0, murmuration::PLACE.1],
            tint: [0.93, 0.58, 0.42],
            degree: 4,
            open: rooms::open::<murmuration::Murmuration>,
        },
        Door {
            id: "meander",
            name: "Meander",
            place: "Río Beni, Bolivia",
            line: "A river that wanders, and cuts off its own bends.",
            at: [-67.03, -12.97],
            tint: [0.30, 0.52, 0.62],
            degree: 7,
            open: rooms::open::<meander::Meander>,
        },
        Door {
            id: "throats",
            name: "Throats",
            place: "Near Los Banos, California",
            line: "Lizards playing rock, paper, scissors with their colours.",
            at: [-120.85, 37.06],
            tint: [0.93, 0.62, 0.30],
            degree: 5,
            open: rooms::open::<throats::Throats>,
        },
        Door {
            id: "bats",
            name: "Bats",
            place: "Bracken Cave, Texas",
            line: "Millions of bats leaving the cave at dusk.",
            at: [-98.35, 29.69],
            tint: [0.94, 0.56, 0.36],
            degree: 1,
            open: rooms::open::<bats::Bats>,
        },
        Door {
            id: "huddle",
            name: "Huddle",
            place: "Atka Bay, Antarctica",
            line: "Emperor penguins sharing the warmth in a winter wind.",
            at: [-8.25, -70.65],
            tint: [0.72, 0.78, 0.98],
            degree: 8,
            open: rooms::open::<huddle::Huddle>,
        },
        Door {
            id: "wave",
            name: "The Wave",
            place: "Estadio Azteca, Mexico City",
            line: "A Mexican wave, if enough people stand.",
            at: [-99.15, 19.30],
            tint: [1.0, 0.86, 0.62],
            degree: -1,
            open: rooms::open::<wave::Wave>,
        },
        Door {
            id: "frost",
            name: "Frost",
            place: "Oymyakon, Siberia",
            line: "Frost growing on a cabin window at fifty below.",
            at: [142.79, 63.46],
            tint: [0.22, 0.42, 0.78],
            degree: 9,
            open: rooms::open::<frost::Frost>,
        },
    ]
}

/// The scenes whose rule runs on the ground (and Pando's grove), split from `scenes` to keep each short.
fn ground_scenes() -> Vec<Door> {
    vec![
        Door {
            id: "catatumbo",
            name: "Catatumbo",
            place: "Lake Maracaibo, Venezuela",
            line: "The lightning that comes back almost every night.",
            at: [catatumbo::PLACE.0, catatumbo::PLACE.1],
            tint: [0.70, 0.62, 1.0],
            degree: 1,
            open: rooms::open::<catatumbo::Catatumbo>,
        },
        Door {
            id: "bubblers",
            name: "Bubblers",
            place: "Phuket, Thailand",
            line: "Crabs drawing starbursts of sand at low tide.",
            at: [bubblers::PLACE.0, bubblers::PLACE.1],
            tint: [0.90, 0.80, 0.62],
            degree: 8,
            open: rooms::open::<bubblers::Bubblers>,
        },
        Door {
            id: "barchans",
            name: "Barchans",
            place: "Near Tarfaya, Morocco",
            line: "Crescent dunes walking across the desert.",
            at: [barchans::PLACE.0, barchans::PLACE.1],
            tint: [0.93, 0.64, 0.36],
            degree: -1,
            open: rooms::open::<barchans::Barchans>,
        },
        Door {
            id: "pando",
            name: "Pando",
            place: "Fish Lake, Utah",
            line: "A forest of aspen that is one single tree.",
            at: [pando::PLACE.0, pando::PLACE.1],
            tint: [0.96, 0.70, 0.18],
            degree: 4,
            open: rooms::open::<pando::Pando>,
        },
        Door {
            id: "hoh",
            name: "Hoh",
            place: "Hoh Rain Forest, Washington",
            line: "Trees that grow in rows along fallen logs.",
            at: [hoh::PLACE.0, hoh::PLACE.1],
            tint: [0.52, 0.80, 0.40],
            degree: 6,
            open: rooms::open::<hoh::Hoh>,
        },
    ]
}

/// The scenes made in homage to the owner's other apps: Moon Road, after Paint by Moonlight.
fn homages() -> Vec<Door> {
    vec![Door {
        id: "moonroad",
        name: "Moon Road",
        place: "Marquette, on Lake Superior",
        line: "The moon paints a road across the water.",
        at: [moonroad::PLACE.0, moonroad::PLACE.1],
        tint: [0.98, 0.90, 0.68],
        degree: 10,
        open: rooms::open::<moonroad::MoonRoad>,
    }]
}
