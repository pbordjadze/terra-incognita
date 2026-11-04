//! The headless check: `cargo run -p jam --bin jam-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<jam::Jam>("jam")
}
