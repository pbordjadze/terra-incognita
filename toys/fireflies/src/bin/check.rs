//! The headless check: `cargo run -p fireflies --bin fireflies-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<fireflies::Fireflies>("fireflies")
}
