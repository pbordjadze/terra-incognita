//! The stand from above, in numbers: `cargo run -p hoh --release --example census [years]` prints a line
//! per log (`L ax az bx bz r age`) and per tree (`T x z base h dying`), to tune the history without
//! rendering (tools/toys/hoh/map.py draws them as a map).
fn main() {
    let years: f32 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0);
    let mut f = hoh::history::opening(hoh::SEED);
    f.run(years);
    for l in &f.logs {
        println!(
            "L {} {} {} {} {} {}",
            l.a[0],
            l.a[1],
            l.b[0],
            l.b[1],
            l.r(),
            l.age
        );
    }
    for t in &f.trees {
        println!("T {} {} {} {} {}", t.x, t.z, t.base, t.h, t.dying() as u8);
    }
}
