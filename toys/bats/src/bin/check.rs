//! The headless check: `cargo run -p bats --bin bats-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<bats::Bats>("bats")
}
