//! The headless check: `cargo run -p taste --bin taste-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<taste::Taste>("taste")
}
