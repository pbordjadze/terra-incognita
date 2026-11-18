//! The headless check: `cargo run -p marsh --bin marsh-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<marsh::MagnetMarsh>("marsh")
}
