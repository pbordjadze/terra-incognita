//! The headless check: `cargo run -p bubblers --bin bubblers-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<bubblers::Bubblers>("bubblers")
}
