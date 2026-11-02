//! The headless check: `cargo run -p mold --bin mold-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<mold::Mold>("mold")
}
