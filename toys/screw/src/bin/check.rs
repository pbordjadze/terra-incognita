//! The headless check: `cargo run -p screw --bin screw-check --release -- play <plan.json> <out-dir>`
//! (toy-kit check.rs), from `toys/`. The plans open `build/screw/practice.wav`, a practice record
//! synthesized here (src/practice.rs; nobody's song), written first if it isn't there.
fn main() {
    let p = std::path::Path::new("build/screw/practice.wav");
    if !p.exists() {
        let (rate, l, r) = screw::practice::record();
        let mut x = Vec::with_capacity(l.len() * 2);
        for (a, b) in l.iter().zip(&r) {
            x.push(*a);
            x.push(*b);
        }
        let _ = std::fs::create_dir_all(p.parent().unwrap());
        toy_kit::check::write_wav(p, &x, 2, rate as u32).expect("the practice record");
    }
    toy_kit::check::main::<screw::Screw>("screw")
}
