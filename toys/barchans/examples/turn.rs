//! How the opening field answers a turned wind: the field after `n` sweeps of an east wind, as hill-shaded
//! previews (a study for the toy's tuning; prints where it wrote them).
//!
//!     cd toys && cargo run --release -p barchans --example turn -- <out-dir> [sweeps, e.g. 30 60 120]
use barchans::dunes::Dunes;
use barchans::{SEED, TRADE};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let out = std::path::PathBuf::from(a.get(1).cloned().unwrap_or(".".into()));
    let marks: Vec<usize> = a[2..].iter().filter_map(|s| s.parse().ok()).collect();
    let bytes = std::fs::read("barchans/data/opening.bin").expect("the opening field");
    let mut d = Dunes::from_bytes(&bytes, SEED, TRADE).expect("a dune field");
    d.wind = [1.0, 0.0];
    let mut done = 0;
    for m in marks {
        while done < m {
            d.run(d.w * d.h);
            done += 1;
        }
        let p = out.join(format!("turn-{m}.bin"));
        std::fs::write(&p, d.to_bytes()).expect("write");
        println!("{}", p.display());
    }
}
