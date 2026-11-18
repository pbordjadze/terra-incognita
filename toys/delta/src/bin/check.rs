//! The headless check: `cargo run -p delta --bin delta-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<delta::Delta>("delta")
}
