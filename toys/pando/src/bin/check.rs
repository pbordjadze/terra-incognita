//! The headless check: `cargo run -p pando --bin pando-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<pando::Pando>("pando")
}
