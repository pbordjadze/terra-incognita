//! The headless check: `cargo run -p hoh --bin hoh-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<hoh::Hoh>("hoh")
}
