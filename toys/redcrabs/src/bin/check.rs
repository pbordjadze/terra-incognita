//! The headless check: `cargo run -p redcrabs --bin redcrabs-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<redcrabs::RedCrabs>("redcrabs")
}
