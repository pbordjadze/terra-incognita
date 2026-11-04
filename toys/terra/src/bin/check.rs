//! The headless check: `cargo run -p terra --bin terra-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<terra::Terra>("terra")
}
