//! The headless check: `cargo run -p bell --bin bell-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<bell::Bell>("bell")
}
