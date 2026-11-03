//! The headless check: `cargo run -p murmuration --bin murmuration-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<murmuration::Murmuration>("murmuration")
}
