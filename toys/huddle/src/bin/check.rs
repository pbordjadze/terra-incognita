//! The headless check: `cargo run -p huddle --bin huddle-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<huddle::Huddle>("huddle")
}
