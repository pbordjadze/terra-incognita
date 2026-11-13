//! The headless check: `cargo run -p barchans --bin barchans-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<barchans::Barchans>("barchans")
}
