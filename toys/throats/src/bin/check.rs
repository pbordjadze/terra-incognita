//! The headless check: `cargo run -p throats --bin throats-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<throats::Throats>("throats")
}
