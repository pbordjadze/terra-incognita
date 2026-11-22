//! Prints the level along the upper estuary through one tide: `cargo run --release -p bore --example profile -- 1 1 0.57`.
use bore::tide::{Tide, PERIOD};
fn main() {
    let a: Vec<String> = std::env::args().collect();
    let on = [
        a.get(1).is_none_or(|s| s == "1"),
        a.get(2).is_none_or(|s| s == "1"),
    ];
    let moon: f64 = a.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.57);
    let mut t = Tide::new(moon, on);
    t.run(PERIOD * 2.0);
    for k in 0..48 {
        t.run(PERIOD / 48.0);
        let row: Vec<String> = (0..=20)
            .map(|j| format!("{:5.1}", t.eta_at(j as f64 * 2400.0)))
            .collect();
        println!(
            "{:5.2}h {}",
            (k as f64 + 1.0) * PERIOD / 48.0 / 3600.0,
            row.join("")
        );
    }
    let z: Vec<String> = (0..=20)
        .map(|j| {
            format!(
                "{:5.1}",
                t.z[(((j as f64 * 2400.0) / 50.0) as usize).min(999)]
            )
        })
        .collect();
    println!("bed    {}", z.join(""));
}
