//! The headless check: `cargo run -p swell --bin swell-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<swell::Swell>("swell")
}
