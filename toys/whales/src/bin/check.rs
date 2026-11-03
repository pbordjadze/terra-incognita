//! The headless check: `cargo run -p whales --bin whales-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<whales::Whales>("whales")
}
