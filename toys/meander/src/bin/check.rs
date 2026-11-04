//! The headless check: `cargo run -p meander --bin meander-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<meander::Meander>("meander")
}
