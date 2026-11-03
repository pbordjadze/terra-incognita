//! The headless check: `cargo run -p battle --bin battle-check --release -- play <plan.json> <out-dir>` (toy-kit check.rs).
fn main() {
    toy_kit::check::main::<battle::Battle>("battle")
}
