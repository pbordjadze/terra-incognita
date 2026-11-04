//! The headless check: `cargo run -p frost --bin frost-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<frost::Frost>("frost")
}
