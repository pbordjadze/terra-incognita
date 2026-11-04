//! The headless check: `cargo run -p carol --bin carol-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<carol::Carol>("carol")
}
