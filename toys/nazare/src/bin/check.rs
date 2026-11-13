//! The headless check: `cargo run -p nazare --bin nazare-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<nazare::Nazare>("nazare")
}
