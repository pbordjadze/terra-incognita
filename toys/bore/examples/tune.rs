//! Prints the bore at Yanguan for a few moons and switches: `cargo run --release -p bore --example tune`.
use bore::tide::{Tide, PERIOD, YANGUAN};
fn main() {
    for on in [[true, true], [false, true], [true, false], [false, false]] {
        for moon in [0.57, 0.32] {
            let mut t = Tide::new(moon, on);
            let start = std::time::Instant::now();
            t.run(PERIOD * 2.0);
            let spin = start.elapsed();
            let mut best: Option<bore::tide::Front> = None;
            let (mut lo, mut hi) = (f64::MAX, f64::MIN);
            let mut prev: Option<f64> = None;
            let mut speed = 0.0;
            for _ in 0..(PERIOD / 60.0) as usize {
                t.run(60.0);
                let e = t.eta_at(YANGUAN);
                lo = lo.min(e);
                hi = hi.max(e);
                if let Some(f) = t.front() {
                    if (f.x - YANGUAN).abs() < 3000.0 {
                        if let Some(p) = prev {
                            speed = (f.x - p) / 60.0;
                        }
                        prev = Some(f.x);
                        if best.is_none_or(|b| f.height() > b.height()) {
                            best = Some(f);
                        }
                    }
                }
            }
            println!(
                "on {:?} moon {:.2}: spin {:?}; Yanguan range {:.2}..{:.2}; front {:?} speed {:.1}",
                on,
                moon,
                spin,
                lo,
                hi,
                best.map(|f| (f.height(), f.froude, f.steep, f.depth)),
                speed
            );
        }
    }
}
