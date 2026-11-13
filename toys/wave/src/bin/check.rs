//! The headless check: `cargo run -p wave --bin wave-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<wave::Wave>("wave")
}
