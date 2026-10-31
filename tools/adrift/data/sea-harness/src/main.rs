//! See Cargo.toml. The sea module is included by path; `model` is the stub it needs.
#![allow(dead_code)]
pub mod model {
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct LonLat {
        pub lon: f64,
        pub lat: f64,
    }
}
#[path = "../../../../../adrift/crates/adrift-core/src/sea/mod.rs"]
pub mod sea;

fn main() {
    let data = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../../adrift/data");
    let rules = sea::Rules::load(&data).unwrap();
    let mut at = model::LonLat { lon: -74.0, lat: 35.5 };
    let mut day = 15.2 + 6.0 * 30.4375;
    for _ in 0..30 {
        let s = rules.step(at, day, 1.0, 7, 0);
        println!("{:.3} {:.3} {:?} {:.2}", s.at.lon, s.at.lat, s.held, s.measured);
        at = s.at;
        day += 1.0;
    }
    // the maybes against the record: where 500 of them are thirty days after Cape Hatteras in a
    // typical July (the bundled GDP runs that passed there: 9 of 18 east of 65 W, median 64.8 W)
    let from = model::LonLat { lon: -74.5, lat: 35.8 };
    let july = 15.2 + 6.0 * 30.4375;
    let ends = rules.maybes(from, july, july + 30.0, 0.25, 11, 500);
    let east = ends.iter().filter(|s| s.at.lon >= -65.0).count();
    let coast = ends.iter().filter(|s| s.held == sea::Held::Coast).count();
    let mut lons: Vec<f64> = ends.iter().map(|s| s.at.lon).collect();
    lons.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("walk's maybes after 30 days: {} of 500 east of 65 W, {} on a coast, median {:.1}", east, coast, lons[250]);
    let ends = rules.flights(sea::Drift::put(from), july, july + 30.0, 0.25, 11, 500);
    let east = ends.iter().filter(|f| f.drift.at.lon >= -65.0).count();
    let coast = ends.iter().filter(|f| f.held == sea::Held::Coast).count();
    let mut lons: Vec<f64> = ends.iter().map(|f| f.drift.at.lon).collect();
    lons.sort_by(|a, b| a.partial_cmp(b).unwrap());
    println!("flight's maybes after 30 days: {} of 500 east of 65 W, {} on a coast, median {:.1}", east, coast, lons[250]);
}
