//! The headless check: `cargo run -p fairy --bin fairy-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<fairy::FairyCircles>("fairy")
}
