//! The headless check: `cargo run -p shimmering --bin shimmering-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<shimmering::Shimmering>("shimmering")
}
