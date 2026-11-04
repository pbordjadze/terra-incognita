//! The headless check: `cargo run -p walk --bin walk-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<walk::Walk>("walk")
}
