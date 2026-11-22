//! The headless check: `cargo run -p bore --bin bore-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<bore::Bore>("bore")
}
